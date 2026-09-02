#nullable enable

using System;
using System.IO;
using System.Net;
using System.Text;
using System.Threading;

namespace Battlement
{
    /// <summary>Synchronous main-thread transport for a localhost Battlement service.</summary>
    public sealed class BattlementHttpTransport : IBattlementTransport
    {
        public const int MaximumPayloadBytes = 16 * 1024 * 1024;
        public static readonly TimeSpan ConnectTimeout = TimeSpan.FromSeconds(2);
        public static readonly TimeSpan RequestTimeout = TimeSpan.FromMilliseconds(100);

        private const string DispositionHeader = "Battlement-UI-Event-Disposition";
        private static readonly UTF8Encoding StrictUtf8 = new(false, true);
        private readonly object callGate = new();
        private readonly int owningThreadId;
        private readonly Uri baseUri;
        private readonly string connectionGroupName = $"battlement-{Guid.NewGuid():N}";
        private bool isConnected;
        private bool isDisposed;

        /// <summary>Creates a transport for a localhost service.</summary>
        public BattlementHttpTransport(string baseUrl)
        {
            if (
                !Uri.TryCreate(baseUrl, UriKind.Absolute, out Uri? parsed)
                || parsed.Scheme != Uri.UriSchemeHttp
                || !parsed.IsLoopback
            )
            {
                throw new ArgumentException(
                    "The HTTP transport requires an absolute localhost HTTP URL.",
                    nameof(baseUrl)
                );
            }
            if (!string.IsNullOrEmpty(parsed.Query) || !string.IsNullOrEmpty(parsed.Fragment))
            {
                throw new ArgumentException(
                    "The HTTP transport URL cannot contain a query or fragment.",
                    nameof(baseUrl)
                );
            }

            baseUri = parsed.AbsoluteUri.EndsWith("/", StringComparison.Ordinal)
                ? parsed
                : new Uri(parsed.AbsoluteUri + "/");
            owningThreadId = Thread.CurrentThread.ManagedThreadId;
        }

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> json)
        {
            lock (callGate)
            {
                RequireAvailable();
                isConnected = false;
                BattlementTransportResult result = Send("POST", "connect", json, ConnectTimeout);
                isConnected = result.Status == BattlementTransportStatus.Success;
                return result;
            }
        }

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> json)
        {
            lock (callGate)
            {
                RequireAvailable();
                if (!isConnected)
                    return Failure("Connect must succeed before submitting a message.");
                return Send("POST", "messages", json, RequestTimeout);
            }
        }

        public BattlementUiEventTransportResult SubmitUiEvent(ReadOnlyMemory<byte> json)
        {
            lock (callGate)
            {
                RequireAvailable();
                if (!isConnected)
                    return UiFailure("Connect must succeed before submitting a UI event.");
                return SendUiEvent(json);
            }
        }

        public BattlementTransportResult Poll()
        {
            lock (callGate)
            {
                RequireAvailable();
                if (!isConnected)
                    return Failure("Connect must succeed before polling.");
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
                    return;
                RequireOwningThread();
                isConnected = false;
                isDisposed = true;
#pragma warning disable SYSLIB0014
                ServicePointManager
                    .FindServicePoint(baseUri)
                    .CloseConnectionGroup(connectionGroupName);
#pragma warning restore SYSLIB0014
            }
        }

        private BattlementTransportResult Send(
            string method,
            string path,
            ReadOnlyMemory<byte> body,
            TimeSpan timeout,
            bool allowNoContent = false
        )
        {
            try
            {
                using HttpWebResponse response = Request(method, path, body, timeout);
                return ReadOrdinary(response, allowNoContent);
            }
            catch (WebException exception) when (exception.Response is HttpWebResponse response)
            {
                using (response)
                    return ReadOrdinary(response, allowNoContent);
            }
            catch (Exception exception) when (IsTransportException(exception))
            {
                isConnected = false;
                return Failure($"HTTP transport failure: {exception.Message}");
            }
        }

        private BattlementUiEventTransportResult SendUiEvent(ReadOnlyMemory<byte> body)
        {
            try
            {
                using HttpWebResponse response = Request("POST", "ui-events", body, RequestTimeout);
                return ReadUiEvent(response);
            }
            catch (WebException exception) when (exception.Response is HttpWebResponse response)
            {
                using (response)
                    return ReadUiEvent(response);
            }
            catch (Exception exception) when (IsTransportException(exception))
            {
                isConnected = false;
                return UiFailure($"HTTP transport failure: {exception.Message}");
            }
        }

        private HttpWebResponse Request(
            string method,
            string path,
            ReadOnlyMemory<byte> body,
            TimeSpan timeout
        )
        {
#pragma warning disable SYSLIB0014
            var request = (HttpWebRequest)WebRequest.Create(new Uri(baseUri, path));
#pragma warning restore SYSLIB0014
            request.Method = method;
            request.Accept = "application/json";
            request.KeepAlive = true;
            request.ConnectionGroupName = connectionGroupName;
            request.Timeout = checked((int)timeout.TotalMilliseconds);
            request.ReadWriteTimeout = checked((int)timeout.TotalMilliseconds);
            if (method == "POST")
            {
                request.ContentType = "application/json";
                request.ContentLength = body.Length;
                using Stream requestStream = request.GetRequestStream();
                byte[] bytes = body.ToArray();
                requestStream.Write(bytes, 0, bytes.Length);
            }
            return (HttpWebResponse)request.GetResponse();
        }

        private BattlementTransportResult ReadOrdinary(
            HttpWebResponse response,
            bool allowNoContent
        )
        {
            int status = (int)response.StatusCode;
            if (status == (int)HttpStatusCode.NoContent)
            {
                if (allowNoContent)
                    return new BattlementTransportResult(BattlementTransportStatus.NoMessage);
                isConnected = false;
                return Failure("HTTP 204 is valid only for poll.");
            }

            byte[] bytes = ReadBody(response);
            if (status == (int)HttpStatusCode.BadRequest)
                return Diagnostic(BattlementTransportStatus.InvalidArgument, bytes);
            if (status == (int)HttpStatusCode.InternalServerError)
                return Diagnostic(BattlementTransportStatus.EngineError, bytes);
            if (status != (int)HttpStatusCode.OK)
            {
                isConnected = false;
                return Failure($"Unexpected HTTP status {status}.");
            }
            if (!IsJson(response.ContentType) || bytes.Length == 0)
            {
                isConnected = false;
                return Failure("Successful HTTP response was not nonempty application/json.");
            }
            return new BattlementTransportResult(BattlementTransportStatus.Success, bytes);
        }

        private BattlementUiEventTransportResult ReadUiEvent(HttpWebResponse response)
        {
            int status = (int)response.StatusCode;
            byte[] bytes = ReadBody(response);
            if (status == (int)HttpStatusCode.BadRequest)
                return UiDiagnostic(BattlementTransportStatus.InvalidArgument, bytes);
            if (status == (int)HttpStatusCode.InternalServerError)
                return UiDiagnostic(BattlementTransportStatus.EngineError, bytes);
            if (status != (int)HttpStatusCode.OK)
            {
                isConnected = false;
                return UiFailure($"Unexpected HTTP status {status}.");
            }
            if (!IsJson(response.ContentType) || bytes.Length == 0)
            {
                isConnected = false;
                return UiFailure("Successful UI event response was not nonempty application/json.");
            }

            UiEventDisposition disposition = response.Headers[DispositionHeader] switch
            {
                "0" => UiEventDisposition.Continue,
                "1" => UiEventDisposition.PreventDefault,
                null => throw new InvalidDataException(
                    $"Successful UI event response omitted {DispositionHeader}."
                ),
                string value => throw new InvalidDataException(
                    $"Successful UI event response had invalid {DispositionHeader} value {value}."
                ),
            };
            return new BattlementUiEventTransportResult(
                BattlementTransportStatus.Success,
                disposition,
                bytes
            );
        }

        private static byte[] ReadBody(HttpWebResponse response)
        {
            if (response.ContentLength > MaximumPayloadBytes)
                throw new InvalidDataException("HTTP response body exceeded the payload limit.");
            using Stream stream = response.GetResponseStream();
            using var output = new MemoryStream();
            var buffer = new byte[8192];
            while (true)
            {
                int read = stream.Read(buffer, 0, buffer.Length);
                if (read == 0)
                    return output.ToArray();
                if (output.Length + read > MaximumPayloadBytes)
                    throw new InvalidDataException(
                        "HTTP response body exceeded the payload limit."
                    );
                output.Write(buffer, 0, read);
            }
        }

        private static bool IsJson(string? contentType) =>
            contentType
                ?.Split(';')[0]
                .Trim()
                .Equals("application/json", StringComparison.OrdinalIgnoreCase) == true;

        private static bool IsTransportException(Exception exception) =>
            exception
                is WebException
                    or IOException
                    or InvalidDataException
                    or ObjectDisposedException
                    or InvalidOperationException;

        private static BattlementTransportResult Diagnostic(
            BattlementTransportStatus status,
            byte[] bytes
        ) => new(status, diagnostic: DecodeDiagnostic(bytes));

        private static BattlementUiEventTransportResult UiDiagnostic(
            BattlementTransportStatus status,
            byte[] bytes
        ) => new(status, UiEventDisposition.Continue, default, DecodeDiagnostic(bytes));

        private static string? DecodeDiagnostic(byte[] bytes)
        {
            try
            {
                return bytes.Length == 0 ? null : StrictUtf8.GetString(bytes);
            }
            catch (DecoderFallbackException)
            {
                return "HTTP diagnostic was not valid UTF-8.";
            }
        }

        private void RequireAvailable()
        {
            if (isDisposed)
                throw new ObjectDisposedException(nameof(BattlementHttpTransport));
            RequireOwningThread();
        }

        private void RequireOwningThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != owningThreadId)
                throw new InvalidOperationException(
                    "HTTP transport calls must remain on one thread."
                );
        }

        private static BattlementTransportResult Failure(string diagnostic) =>
            new(BattlementTransportStatus.TransportError, diagnostic: diagnostic);

        private static BattlementUiEventTransportResult UiFailure(string diagnostic) =>
            new(
                BattlementTransportStatus.TransportError,
                UiEventDisposition.Continue,
                default,
                diagnostic
            );
    }
}
