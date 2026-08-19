#nullable enable

using System;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading.Tasks;
using NUnit.Framework;
using UnityEngine;

namespace Masonry.Tests
{
    public sealed class MasonryHttpTransportTests
    {
        private static readonly byte[] ConnectBytes = { 0x81, 0xa1, 0x63, 0x01 };
        private static readonly byte[] ResponseBytes = { 0x81, 0xa1, 0x72, 0x02 };

        [Test]
        public void RunnerUsesRequiredRoutesHeadersBodiesAndOneConnection()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(204);
            using var transport = new MasonryHttpTransport(server.BaseUrl, ConnectBytes);
            var host = new GameObject("Masonry HTTP test host");
            try
            {
                MasonryRunner runner = host.AddComponent<MasonryRunner>();
                runner.Configure(new MasonryRunnerOptions(transport, new FakeAssetStorage()));
                runner.Connect();

                Assert.That(transport.Submit(new byte[] { 1, 2, 3 }).Status, Is.EqualTo(Success));
                Assert.That(transport.Poll().Status, Is.EqualTo(MasonryTransportStatus.NoMessage));
                Assert.That(server.WaitForRequests(3), Is.True);

                LoopbackHttpServer.RecordedRequest[] requests = server.Requests.ToArray();
                Assert.That(
                    requests.Select(request => $"{request.Method} {request.Path}"),
                    Is.EqualTo(new[] { "POST /connect", "POST /messages", "GET /poll" })
                );
                Assert.That(requests[0].Body, Is.EqualTo(ConnectBytes));
                Assert.That(requests[1].Body, Is.EqualTo(new byte[] { 1, 2, 3 }));
                Assert.That(
                    requests.Take(2).Select(request => request.Headers["Content-Type"]),
                    Is.All.EqualTo("application/msgpack")
                );
                Assert.That(server.ConnectionCount, Is.EqualTo(1));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(host);
            }
        }

        [Test]
        public void MapsDiagnosticsAndRejectsOtherStatusesOrContentTypes()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(400, Encoding.UTF8.GetBytes("bad input"), "text/plain");
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(500, Encoding.UTF8.GetBytes("engine failed"), "text/plain");
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(201, ResponseBytes);
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(200, ResponseBytes, "application/json");
            using var transport = new MasonryHttpTransport(server.BaseUrl, ConnectBytes);

            MasonryTransportResult invalid = transport.Connect();
            Assert.That(invalid.Status, Is.EqualTo(MasonryTransportStatus.InvalidArgument));
            Assert.That(invalid.Diagnostic, Is.EqualTo("bad input"));
            Assert.That(transport.Connect().Status, Is.EqualTo(Success));
            MasonryTransportResult engineError = transport.Submit(new byte[] { 1 });
            Assert.That(engineError.Status, Is.EqualTo(MasonryTransportStatus.EngineError));
            Assert.That(engineError.Diagnostic, Is.EqualTo("engine failed"));
            Assert.That(transport.Connect().Status, Is.EqualTo(Success));
            Assert.That(transport.Submit(new byte[] { 2 }).Status, Is.EqualTo(Failure));
            Assert.That(transport.Connect().Status, Is.EqualTo(Success));
            Assert.That(transport.Submit(new byte[] { 3 }).Status, Is.EqualTo(Failure));
        }

        [Test]
        public void EnforcesBodyLimitAndPollOnlyNoContent()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(204);
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(200, declaredLength: MasonryHttpTransport.MaximumPayloadBytes + 1L);
            using var transport = new MasonryHttpTransport(server.BaseUrl, ConnectBytes);

            Assert.That(transport.Connect().Status, Is.EqualTo(Failure));
            Assert.That(transport.Connect().Status, Is.EqualTo(Success));
            MasonryTransportResult oversized = transport.Submit(new byte[] { 1 });
            Assert.That(oversized.Status, Is.EqualTo(Failure));
            Assert.That(oversized.Diagnostic, Does.Contain("limit"));
        }

        [Test]
        public void TimesOutWithoutRetryAndRequiresExplicitReconnect()
        {
            Assert.That(MasonryHttpTransport.ConnectTimeout, Is.EqualTo(TimeSpan.FromSeconds(2)));
            Assert.That(
                MasonryHttpTransport.RequestTimeout,
                Is.EqualTo(TimeSpan.FromMilliseconds(100))
            );

            using var server = new LoopbackHttpServer();
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(200, ResponseBytes, delayMilliseconds: 300);
            using var transport = new MasonryHttpTransport(server.BaseUrl, ConnectBytes);
            Assert.That(transport.Connect().Status, Is.EqualTo(Success));

            MasonryTransportResult timedOut = transport.Submit(new byte[] { 1 });
            Assert.That(timedOut.Status, Is.EqualTo(Failure));
            Assert.That(transport.Poll().Status, Is.EqualTo(Failure));
            Assert.That(server.WaitForRequests(2), Is.True);
            Assert.That(server.Requests.Count, Is.EqualTo(2));
        }

        [Test]
        public void RefusesNonLoopbackAndOffThreadCalls()
        {
            Assert.Throws<ArgumentException>(() =>
                new MasonryHttpTransport("https://example.com", ConnectBytes)
            );

            using var server = new LoopbackHttpServer();
            using var transport = new MasonryHttpTransport(server.BaseUrl, ConnectBytes);
            AggregateException exception = Assert.Throws<AggregateException>(() =>
                Task.Run(() => transport.Connect()).Wait()
            )!;
            Assert.That(exception.InnerException, Is.TypeOf<InvalidOperationException>());
        }

        [Test]
        public void ConnectionRefusalIsFatal()
        {
            var reservation = new TcpListener(IPAddress.Loopback, 0);
            reservation.Start();
            int unusedPort = ((IPEndPoint)reservation.LocalEndpoint).Port;
            reservation.Stop();
            using var transport = new MasonryHttpTransport(
                $"http://127.0.0.1:{unusedPort}",
                ConnectBytes
            );

            Assert.That(transport.Connect().Status, Is.EqualTo(Failure));
        }

        private static MasonryTransportStatus Success => MasonryTransportStatus.Success;

        private static MasonryTransportStatus Failure => MasonryTransportStatus.TransportError;

        private sealed class FakeAssetStorage : IMasonryAssetStorage
        {
            public IMasonryAssetHandle Prepare(PreparedAsset asset) =>
                throw new NotSupportedException();

            public void Dispose() { }
        }
    }
}
