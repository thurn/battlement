#nullable enable

using System;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading.Tasks;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class BattlementHttpTransportTests
    {
        private static readonly byte[] ConnectBytes = { 0x81, 0xa1, 0x63, 0x01 };
        private static readonly byte[] ResponseBytes = { 0x81, 0xa1, 0x72, 0x02 };

        [Test]
        public void RunnerUsesRequiredRoutesHeadersBodiesAndOneConnection()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(200, InitialResponseBytes());
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(204);
            using var transport = new BattlementHttpTransport(server.BaseUrl);
            var host = new GameObject("Battlement HTTP test host");
            try
            {
                BattlementRunner runner = host.AddComponent<BattlementRunner>();
                runner.Configure(
                    new BattlementRunnerOptions(
                        transport,
                        new FakeBattlementAssetStorage(),
                        BattlementMessagePack.Instance
                    )
                );
                runner.Connect();

                Assert.That(transport.Submit(new byte[] { 1, 2, 3 }).Status, Is.EqualTo(Success));
                Assert.That(
                    transport.Poll().Status,
                    Is.EqualTo(BattlementTransportStatus.NoMessage)
                );
                Assert.That(server.WaitForRequests(3), Is.True);

                LoopbackHttpServer.RecordedRequest[] requests = server.Requests.ToArray();
                Assert.That(
                    requests.Select(request => $"{request.Method} {request.Path}"),
                    Is.EqualTo(new[] { "POST /connect", "POST /messages", "GET /poll" })
                );
                Connect connect = BattlementMessagePack.DeserializeConnect(requests[0].Body);
                Assert.That(connect.Platform, Is.Not.Empty);
                Assert.That(connect.UnityVersion, Is.EqualTo(Application.unityVersion));
                Assert.That(connect.PersistentDataPath, Is.Null);
                Assert.That(connect.StreamingAssetsPath, Is.Null);
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
            using var transport = new BattlementHttpTransport(server.BaseUrl);

            BattlementTransportResult invalid = transport.Connect(ConnectBytes);
            Assert.That(invalid.Status, Is.EqualTo(BattlementTransportStatus.InvalidArgument));
            Assert.That(invalid.Diagnostic, Is.EqualTo("bad input"));
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));
            BattlementTransportResult engineError = transport.Submit(new byte[] { 1 });
            Assert.That(engineError.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
            Assert.That(engineError.Diagnostic, Is.EqualTo("engine failed"));
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));
            Assert.That(transport.Submit(new byte[] { 2 }).Status, Is.EqualTo(Failure));
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));
            Assert.That(transport.Submit(new byte[] { 3 }).Status, Is.EqualTo(Failure));
        }

        [Test]
        public void EnforcesBodyLimitAndPollOnlyNoContent()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(204);
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(200, declaredLength: BattlementHttpTransport.MaximumPayloadBytes + 1L);
            using var transport = new BattlementHttpTransport(server.BaseUrl);

            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Failure));
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));
            BattlementTransportResult oversized = transport.Submit(new byte[] { 1 });
            Assert.That(oversized.Status, Is.EqualTo(Failure));
            Assert.That(oversized.Diagnostic, Does.Contain("limit"));
        }

        [Test]
        public void TimesOutWithoutRetryAndRequiresExplicitReconnect()
        {
            Assert.That(
                BattlementHttpTransport.ConnectTimeout,
                Is.EqualTo(TimeSpan.FromSeconds(2))
            );
            Assert.That(
                BattlementHttpTransport.RequestTimeout,
                Is.EqualTo(TimeSpan.FromMilliseconds(100))
            );

            using var server = new LoopbackHttpServer();
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(200, ResponseBytes, delayMilliseconds: 300);
            using var transport = new BattlementHttpTransport(server.BaseUrl);
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));

            BattlementTransportResult timedOut = transport.Submit(new byte[] { 1 });
            Assert.That(timedOut.Status, Is.EqualTo(Failure));
            Assert.That(transport.Poll().Status, Is.EqualTo(Failure));
            Assert.That(server.WaitForRequests(2), Is.True);
            Assert.That(server.Requests.Count, Is.EqualTo(2));
        }

        [Test]
        public void RefusesNonLoopbackAndOffThreadCalls()
        {
            Assert.Throws<ArgumentException>(() =>
                new BattlementHttpTransport("https://example.com")
            );

            using var server = new LoopbackHttpServer();
            using var transport = new BattlementHttpTransport(server.BaseUrl);
            AggregateException exception = Assert.Throws<AggregateException>(() =>
                Task.Run(() => transport.Connect(ConnectBytes)).Wait()
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
            using var transport = new BattlementHttpTransport($"http://127.0.0.1:{unusedPort}");

            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Failure));
        }

        private static BattlementTransportStatus Success => BattlementTransportStatus.Success;

        private static BattlementTransportStatus Failure =>
            BattlementTransportStatus.TransportError;

        private static byte[] InitialResponseBytes() =>
            FakeBattlementTransport.SnapshotResponse().Payload.ToArray();
    }
}
