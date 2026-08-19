#nullable enable

using System;
using System.IO;
using System.Net;
using System.Text;
using System.Threading;

namespace Masonry
{
    /// <summary>Synchronous main-thread transport for a localhost Masonry service.</summary>
    public sealed class MasonryHttpTransport : IMasonryTransport
    {
        public const int MaximumPayloadBytes = 16 * 1024 * 1024;
        public static readonly TimeSpan ConnectTimeout = TimeSpan.FromSeconds(2);
        public static readonly TimeSpan RequestTimeout = TimeSpan.FromMilliseconds(100);

        private static readonly UTF8Encoding StrictUtf8 = new(false, true);

        private readonly object callGate = new();
        private readonly int owningThreadId;
        private readonly Uri baseUri;
        private readonly byte[] connectMessage;
        private readonly string connectionGroupName = $"masonry-{Guid.NewGuid():N}";
        private bool isConnected;
        private bool isDisposed;

        /// <summary>Creates a transport from a localhost URL and encoded connect message.</summary>
        public MasonryHttpTransport(string baseUrl, ReadOnlyMemory<byte> connectMessagePack)
        {
            if (
                !Uri.TryCreate(baseUrl, UriKind.Absolute, out Uri? parsed)
                || parsed.Scheme != Uri.UriSchemeHttp
                || !parsed.IsLoopback
                || !string.IsNullOrEmpty(parsed.Query)
                || !string.IsNullOrEmpty(parsed.Fragment)
            )
            {
                throw new ArgumentException(
                    "The HTTP transport requires an absolute localhost HTTP URL.",
                    nameof(baseUrl)
                );
            }

            baseUri = parsed.AbsoluteUri.EndsWith("/", StringComparison.Ordinal)
                ? parsed
                : new Uri(parsed.AbsoluteUri + "/");
            connectMessage = connectMessagePack.ToArray();
            owningThreadId = Thread.CurrentThread.ManagedThreadId;
        }

        public MasonryTransportResult Connect()
        {
            lock (callGate)
            {
                RequireAvailable();
                isConnected = false;
                MasonryTransportResult result = Send(
                    "POST",
                    "connect",
                    connectMessage,
                    ConnectTimeout,
                    false
                );
                isConnected = result.Status == MasonryTransportStatus.Success;
                return result;
            }
        }

        public MasonryTransportResult Submit(ReadOnlyMemory<byte> messagePack)
        {
            lock (callGate)
            {
                RequireAvailable();
                if (!isConnected)
                {
                    return Failure("Connect must succeed before submitting a message.");
                }

                return Send("POST", "messages", messagePack, RequestTimeout, false);
            }
        }

        public MasonryTransportResult Poll()
        {
            lock (callGate)
            {
                RequireAvailable();
                if (!isConnected)
                {
                    return Failure("Connect must succeed before polling.");
                }

                return Send("GET", "poll", default, RequestTimeout, true);
            }
        }

        public void Stop()
        {
            lock (callGate)
            {
                RequireAvailable();
                isConnected = false;
            }
        }

        public void Dispose()
        {
            lock (callGate)
            {
                if (isDisposed)
                {
                    return;
                }

                RequireOwningThread();
                isConnected = false;
                isDisposed = true;
                // Match the legacy request API so disposal closes only this transport's
                // uniquely named persistent connection group, not other localhost clients.
#pragma warning disable SYSLIB0014
                ServicePointManager
                    .FindServicePoint(baseUri)
                    .CloseConnectionGroup(connectionGroupName);
#pragma warning restore SYSLIB0014
            }
        }

        private MasonryTransportResult Send(
            string method,
            string path,
            ReadOnlyMemory<byte> body,
            TimeSpan timeout,
            bool allowNoContent
        )
        {
            try
            {
                // HttpWebRequest is intentional despite SYSLIB0014. UnityWebRequest is
                // asynchronous, and Unity's .NET profile exposes SendAsync but not the newer
                // HttpClient.Send. Blocking SendAsync would complicate bounded response reads
                // and timeout coverage; this API directly supplies the required synchronous
                // calls, per-request connect/read/write timeouts, and connection grouping.
#pragma warning disable SYSLIB0014
                var request = (HttpWebRequest)WebRequest.Create(new Uri(baseUri, path));
#pragma warning restore SYSLIB0014
                request.Method = method;
                request.Accept = "application/msgpack";
                request.KeepAlive = true;
                request.ConnectionGroupName = connectionGroupName;
                request.Timeout = checked((int)timeout.TotalMilliseconds);
                request.ReadWriteTimeout = checked((int)timeout.TotalMilliseconds);

                if (method == "POST")
                {
                    request.ContentType = "application/msgpack";
                    request.ContentLength = body.Length;
                    using Stream requestStream = request.GetRequestStream();
                    byte[] bytes = body.ToArray();
                    requestStream.Write(bytes, 0, bytes.Length);
                }

                using var response = (HttpWebResponse)request.GetResponse();
                return ReadResponse(response, allowNoContent);
            }
            catch (WebException exception) when (exception.Response is HttpWebResponse response)
            {
                using (response)
                {
                    return ReadResponse(response, allowNoContent);
                }
            }
            catch (Exception exception)
                when (exception
                        is WebException
                            or IOException
                            or ObjectDisposedException
                            or InvalidOperationException
                )
            {
                isConnected = false;
                return Failure($"HTTP transport failure: {exception.Message}");
            }
        }

        private MasonryTransportResult ReadResponse(HttpWebResponse response, bool allowNoContent)
        {
            int status = (int)response.StatusCode;
            if (status == (int)HttpStatusCode.NoContent)
            {
                if (allowNoContent)
                {
                    return new MasonryTransportResult(MasonryTransportStatus.NoMessage);
                }

                isConnected = false;
                return Failure("HTTP 204 is valid only for poll.");
            }

            byte[] bytes = ReadBody(response);
            if (status == (int)HttpStatusCode.BadRequest)
            {
                return Diagnostic(MasonryTransportStatus.InvalidArgument, bytes);
            }

            if (status == (int)HttpStatusCode.InternalServerError)
            {
                return Diagnostic(MasonryTransportStatus.EngineError, bytes);
            }

            if (status != (int)HttpStatusCode.OK)
            {
                isConnected = false;
                return Failure($"Unexpected HTTP status {status}.");
            }

            if (!IsMessagePack(response.ContentType))
            {
                isConnected = false;
                return Failure("Successful HTTP response was not application/msgpack.");
            }

            if (bytes.Length == 0)
            {
                isConnected = false;
                return Failure("Successful HTTP response had an empty body.");
            }

            return new MasonryTransportResult(MasonryTransportStatus.Success, bytes);
        }

        private static byte[] ReadBody(HttpWebResponse response)
        {
            if (response.ContentLength > MaximumPayloadBytes)
            {
                throw new InvalidOperationException(
                    $"HTTP body exceeded the {MaximumPayloadBytes}-byte limit."
                );
            }

            using Stream stream = response.GetResponseStream();
            using var output = new MemoryStream();
            var buffer = new byte[8192];
            while (true)
            {
                int read = stream.Read(buffer, 0, buffer.Length);
                if (read == 0)
                {
                    return output.ToArray();
                }

                if (output.Length + read > MaximumPayloadBytes)
                {
                    throw new InvalidOperationException(
                        $"HTTP body exceeded the {MaximumPayloadBytes}-byte limit."
                    );
                }

                output.Write(buffer, 0, read);
            }
        }

        private static bool IsMessagePack(string? contentType) =>
            contentType
                ?.Split(';')[0]
                .Trim()
                .Equals("application/msgpack", StringComparison.OrdinalIgnoreCase) == true;

        private static MasonryTransportResult Diagnostic(
            MasonryTransportStatus status,
            byte[] bytes
        )
        {
            try
            {
                return new MasonryTransportResult(
                    status,
                    diagnostic: bytes.Length == 0 ? null : StrictUtf8.GetString(bytes)
                );
            }
            catch (DecoderFallbackException)
            {
                return Failure("HTTP diagnostic was not valid UTF-8.");
            }
        }

        private void RequireAvailable()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryHttpTransport));
            }

            RequireOwningThread();
        }

        private void RequireOwningThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != owningThreadId)
            {
                throw new InvalidOperationException(
                    "HTTP transport calls must remain on their creating Unity thread."
                );
            }
        }

        private static MasonryTransportResult Failure(string diagnostic) =>
            new(MasonryTransportStatus.TransportError, diagnostic: diagnostic);
    }
}
