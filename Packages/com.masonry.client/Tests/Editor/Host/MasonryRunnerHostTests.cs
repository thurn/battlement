#nullable enable

using System;
using System.Linq;
using NUnit.Framework;

namespace Masonry.Tests
{
    public sealed class MasonryRunnerHostTests
    {
        [Test]
        public void PublicHostBoundaryDrivesAndTearsDownAnIsolatedRunner()
        {
            MasonryTestHarness harness = MasonryTestHarness.Create();
            MasonryRunner runner = harness.Runner;
            FakeMasonryTransport transport = harness.Transport;
            FakeMasonryAssetStorage assetStorage = harness.AssetStorage;
            var handle = (FakeAssetHandle)
                assetStorage.Prepare(new PreparedAsset.Prefab(new PrefabAddress("test/prefab")));

            try
            {
                Assert.That(runner.TransportKind, Is.EqualTo(MasonryTransportKind.Native));
                Assert.That(runner.NativeTransport.LibraryName, Is.EqualTo("masonry_rules"));
                Assert.That(runner.HttpTransport.BaseUrl, Is.EqualTo("http://127.0.0.1:8080"));

                runner.Connect();
                harness.Clock.Advance(TimeSpan.FromTicks(125_000));
                runner.RunFrame();
                runner.Reconnect();
                runner.Stop();

                Assert.That(
                    transport.Calls,
                    Is.EqualTo(new[] { "connect", "stop", "connect", "stop" }),
                    "Connect, reconnect, and stop should reach the transport in order."
                );
                Assert.That(
                    harness.Logger.Records.Select(record => record.EventName),
                    Is.EqualTo(
                        new[]
                        {
                            "masonry.host.connected",
                            "masonry.host.reconnected",
                            "masonry.host.stopped",
                        }
                    ),
                    "Runner lifecycle logs should identify the operation that emitted each record."
                );
            }
            finally
            {
                harness.Dispose();
            }

            Assert.That(runner == null, Is.True);
            Assert.That(harness.Scene.IsValid(), Is.False);
            Assert.That(transport.IsDisposed, Is.True);
            Assert.That(assetStorage.IsDisposed, Is.True);
            Assert.That(handle.IsDisposed, Is.True);
            Assert.That(assetStorage.LiveHandleCount, Is.Zero);
        }

        [Test]
        public void RunFrameRejectsAClockThatMovesBackward()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            harness.Clock.Advance(TimeSpan.FromSeconds(1));
            harness.Runner.Connect();
            harness.Clock.Advance(TimeSpan.FromSeconds(-2));

            Assert.Throws<InvalidOperationException>(
                () => harness.Runner.RunFrame(),
                "RunFrame should reject a clock that moved backward from the prior frame."
            );
        }
    }
}
