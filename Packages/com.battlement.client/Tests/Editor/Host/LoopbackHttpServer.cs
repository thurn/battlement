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

namespace Battlement.Tests
{
    internal sealed class LoopbackHttpServer : IDisposable
    {
        private readonly TcpListener listener = new(IPAddress.Loopback, 0);
        private readonly ConcurrentQueue<ScriptedResponse> responses = new();
        private readonly ConcurrentQueue<RecordedRequest> requests = new();
        private readonly Thread thread;
        private volatile bool stopping;

        public LoopbackHttpServer()
        {
            listener.Start();
            int port = ((IPEndPoint)listener.LocalEndpoint).Port;
            BaseUrl = $"http://127.0.0.1:{port}";
            thread = new Thread(Serve) { IsBackground = true };
            thread.Start();
        }

        public string BaseUrl { get; }

        public IReadOnlyList<RecordedRequest> Requests => requests.ToArray();

        public void Enqueue(
            int status,
            byte[]? body = null,
            string? contentType = "application/json",
            IReadOnlyDictionary<string, string>? headers = null
        ) =>
            responses.Enqueue(
                new ScriptedResponse(
                    status,
                    body ?? Array.Empty<byte>(),
                    contentType,
                    headers ?? new Dictionary<string, string>()
                )
            );

        public bool WaitForRequests(int count)
        {
            DateTime deadline = DateTime.UtcNow.AddSeconds(2);
            while (requests.Count < count && DateTime.UtcNow < deadline)
                Thread.Sleep(5);
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
                    return;
                requests.Enqueue(request);
                if (!responses.TryDequeue(out ScriptedResponse response))
                {
                    response = new ScriptedResponse(
                        500,
                        Encoding.UTF8.GetBytes("No scripted response."),
                        "text/plain",
                        new Dictionary<string, string>()
                    );
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
                    return header.Length == 0 ? null : throw new EndOfStreamException();
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
            for (int offset = 0; offset < body.Length; )
            {
                int read = stream.Read(body, offset, body.Length - offset);
                if (read == 0)
                    throw new EndOfStreamException();
                offset += read;
            }
            return new RecordedRequest(requestLine[0], requestLine[1], body);
        }

        private static void WriteResponse(Stream stream, ScriptedResponse response)
        {
            var headers = new StringBuilder()
                .Append("HTTP/1.1 ")
                .Append(response.Status)
                .Append(" Response\r\nContent-Length: ")
                .Append(response.Body.LongLength)
                .Append("\r\nConnection: keep-alive\r\n");
            if (response.ContentType is not null)
                headers.Append("Content-Type: ").Append(response.ContentType).Append("\r\n");
            foreach (KeyValuePair<string, string> header in response.Headers)
                headers.Append(header.Key).Append(": ").Append(header.Value).Append("\r\n");
            byte[] headerBytes = Encoding.ASCII.GetBytes(headers.Append("\r\n").ToString());
            stream.Write(headerBytes, 0, headerBytes.Length);
            stream.Write(response.Body, 0, response.Body.Length);
            stream.Flush();
        }

        internal sealed record RecordedRequest(string Method, string Path, byte[] Body);

        private sealed record ScriptedResponse(
            int Status,
            byte[] Body,
            string? ContentType,
            IReadOnlyDictionary<string, string> Headers
        );
    }
}
