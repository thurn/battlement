#nullable enable

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

namespace Masonry.Tests
{
    internal sealed class LoopbackHttpServer : IDisposable
    {
        private readonly TcpListener listener = new(IPAddress.Loopback, 0);
        private readonly ConcurrentQueue<ScriptedResponse> responses = new();
        private readonly ConcurrentQueue<RecordedRequest> requests = new();
        private readonly Thread thread;
        private volatile bool stopping;
        private int connectionCount;

        public LoopbackHttpServer()
        {
            listener.Start();
            int port = ((IPEndPoint)listener.LocalEndpoint).Port;
            BaseUrl = $"http://127.0.0.1:{port}";
            thread = new Thread(Serve) { IsBackground = true };
            thread.Start();
        }

        public string BaseUrl { get; }

        public int ConnectionCount => Volatile.Read(ref connectionCount);

        public IReadOnlyList<RecordedRequest> Requests => requests.ToArray();

        public void Enqueue(
            int status,
            byte[]? body = null,
            string? contentType = "application/msgpack",
            int delayMilliseconds = 0,
            long? declaredLength = null
        ) =>
            responses.Enqueue(
                new ScriptedResponse(
                    status,
                    body ?? Array.Empty<byte>(),
                    contentType,
                    delayMilliseconds,
                    declaredLength
                )
            );

        public bool WaitForRequests(int count, int milliseconds = 2000)
        {
            DateTime deadline = DateTime.UtcNow.AddMilliseconds(milliseconds);
            while (requests.Count < count && DateTime.UtcNow < deadline)
            {
                Thread.Sleep(5);
            }

            return requests.Count >= count;
        }

        public void Dispose()
        {
            stopping = true;
            listener.Stop();
            thread.Join(2000);
        }

        private void Serve()
        {
            while (!stopping)
            {
                try
                {
                    using TcpClient client = listener.AcceptTcpClient();
                    Interlocked.Increment(ref connectionCount);
                    ServeConnection(client);
                }
                catch (SocketException) when (stopping) { }
                catch (ObjectDisposedException) when (stopping) { }
            }
        }

        private void ServeConnection(TcpClient client)
        {
            using NetworkStream stream = client.GetStream();
            while (!stopping)
            {
                RecordedRequest? request;
                try
                {
                    request = ReadRequest(stream);
                }
                catch (IOException)
                {
                    return;
                }

                if (request is null)
                {
                    return;
                }

                requests.Enqueue(request);
                if (!responses.TryDequeue(out ScriptedResponse response))
                {
                    response = new ScriptedResponse(
                        500,
                        Encoding.UTF8.GetBytes("No scripted response."),
                        "text/plain",
                        0,
                        null
                    );
                }

                if (response.DelayMilliseconds > 0)
                {
                    Thread.Sleep(response.DelayMilliseconds);
                }

                try
                {
                    WriteResponse(stream, response);
                }
                catch (IOException)
                {
                    return;
                }
            }
        }

        private static RecordedRequest? ReadRequest(Stream stream)
        {
            using var header = new MemoryStream();
            int matched = 0;
            byte[] terminator = { 13, 10, 13, 10 };
            while (matched < terminator.Length)
            {
                int nextByte = stream.ReadByte();
                if (nextByte < 0)
                {
                    return header.Length == 0 ? null : throw new EndOfStreamException();
                }

                header.WriteByte((byte)nextByte);
                matched =
                    nextByte == terminator[matched] ? matched + 1
                    : nextByte == terminator[0] ? 1
                    : 0;
            }

            string[] lines = Encoding
                .ASCII.GetString(header.ToArray())
                .Split(new[] { "\r\n" }, StringSplitOptions.RemoveEmptyEntries);
            string[] requestLine = lines[0].Split(' ');
            var headers = lines
                .Skip(1)
                .Select(line => line.Split(new[] { ':' }, 2))
                .ToDictionary(
                    pair => pair[0],
                    pair => pair[1].Trim(),
                    StringComparer.OrdinalIgnoreCase
                );
            int length = headers.TryGetValue("Content-Length", out string value)
                ? int.Parse(value)
                : 0;
            var body = new byte[length];
            int offset = 0;
            while (offset < body.Length)
            {
                int read = stream.Read(body, offset, body.Length - offset);
                if (read == 0)
                {
                    throw new EndOfStreamException();
                }

                offset += read;
            }

            return new RecordedRequest(requestLine[0], requestLine[1], headers, body);
        }

        private static void WriteResponse(Stream stream, ScriptedResponse response)
        {
            string reason = response.Status switch
            {
                200 => "OK",
                204 => "No Content",
                400 => "Bad Request",
                500 => "Internal Server Error",
                _ => "Scripted",
            };
            long length = response.DeclaredLength ?? response.Body.LongLength;
            var headers = new StringBuilder()
                .Append("HTTP/1.1 ")
                .Append(response.Status)
                .Append(' ')
                .Append(reason)
                .Append("\r\nContent-Length: ")
                .Append(length)
                .Append("\r\nConnection: keep-alive\r\n");
            if (response.ContentType is not null)
            {
                headers.Append("Content-Type: ").Append(response.ContentType).Append("\r\n");
            }

            byte[] headerBytes = Encoding.ASCII.GetBytes(headers.Append("\r\n").ToString());
            stream.Write(headerBytes, 0, headerBytes.Length);
            stream.Write(response.Body, 0, response.Body.Length);
            stream.Flush();
        }

        internal sealed class RecordedRequest
        {
            public RecordedRequest(
                string method,
                string path,
                IReadOnlyDictionary<string, string> headers,
                byte[] body
            )
            {
                Method = method;
                Path = path;
                Headers = headers;
                Body = body;
            }

            public string Method { get; }
            public string Path { get; }
            public IReadOnlyDictionary<string, string> Headers { get; }
            public byte[] Body { get; }
        }

        private sealed class ScriptedResponse
        {
            public ScriptedResponse(
                int status,
                byte[] body,
                string? contentType,
                int delayMilliseconds,
                long? declaredLength
            )
            {
                Status = status;
                Body = body;
                ContentType = contentType;
                DelayMilliseconds = delayMilliseconds;
                DeclaredLength = declaredLength;
            }

            public int Status { get; }
            public byte[] Body { get; }
            public string? ContentType { get; }
            public int DelayMilliseconds { get; }
            public long? DeclaredLength { get; }
        }
    }
}
