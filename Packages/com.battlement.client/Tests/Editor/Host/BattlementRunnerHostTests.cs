#nullable enable

using System;
using System.IO;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;

namespace Battlement.Tests
{
    public sealed class BattlementRunnerHostTests
    {
        [Test]
        public void PublicHostBoundaryDrivesAndTearsDownAnIsolatedRunner()
        {
            BattlementTestHarness harness = BattlementTestHarness.Create();
            BattlementRunner runner = harness.Runner;
            FakeBattlementTransport transport = harness.Transport;
            FakeBattlementAssetStorage assetStorage = harness.AssetStorage;
            var handle = (FakeAssetHandle)
                assetStorage.Prepare(new PreparedAsset.Prefab(new PrefabAddress("test/prefab")));

            try
            {
                Assert.That(runner.TransportKind, Is.EqualTo(BattlementTransportKind.Native));
                Assert.That(runner.NativeTransport.LibraryName, Is.EqualTo("battlement_rules"));
                Assert.That(runner.HttpTransport.BaseUrl, Is.EqualTo("http://127.0.0.1:8080"));

                runner.Connect();
                harness.Clock.Advance(TimeSpan.FromTicks(125_000));
                runner.RunFrame();
                runner.Reconnect();
                runner.Stop();

                Assert.That(
                    transport.Calls,
                    Is.EqualTo(new[] { "connect", "poll", "stop", "connect", "stop" }),
                    "Connect, frame polling, reconnect, and stop should reach the transport "
                        + "in order."
                );
                Assert.That(
                    harness.Logger.Records.Select(record => record.EventName),
                    Is.EqualTo(
                        new[]
                        {
                            "battlement.host.connected",
                            "battlement.host.reconnected",
                            "battlement.host.stopped",
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
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Clock.Advance(TimeSpan.FromSeconds(1));
            harness.Runner.Connect();
            harness.Clock.Advance(TimeSpan.FromSeconds(-2));

            Assert.Throws<InvalidOperationException>(
                () => harness.Runner.RunFrame(),
                "RunFrame should reject a clock that moved backward from the prior frame."
            );
        }

        [Test]
        public void ConnectBuildsCurrentNativeEnvironmentAndAppliesInputGate()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                customCommandTypes: new[] { "mygame.zap", "mygame.flash" }
            );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(inputDisabled: true)
            );

            harness.Runner.Connect();

            Connect connect = BattlementMessagePack.DeserializeConnect(
                harness.Transport.ConnectMessages.Single()
            );
            Assert.That(connect.Platform, Is.EqualTo(ExpectedPlatform()));
            Assert.That(connect.UnityVersion, Is.EqualTo(Application.unityVersion));
            Assert.That(connect.Screen.Width, Is.EqualTo((uint)Screen.width));
            Assert.That(connect.Screen.Height, Is.EqualTo((uint)Screen.height));
            Assert.That(
                connect.CustomCommandTypes,
                Is.EqualTo(new[] { "mygame.flash", "mygame.zap" })
            );
            Assert.That(
                connect.PersistentDataPath,
                Is.EqualTo(Absolute(Application.persistentDataPath))
            );
            Assert.That(
                connect.StreamingAssetsPath,
                Is.EqualTo(Absolute(Application.streamingAssetsPath))
            );
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
        }

        [Test]
        public void HttpConnectOmitsLocalPathsAndEnablesInputAfterSnapshot()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                BattlementTransportKind.Http
            );

            harness.Runner.Connect();

            Connect connect = BattlementMessagePack.DeserializeConnect(
                harness.Transport.ConnectMessages.Single()
            );
            Assert.That(connect.PersistentDataPath, Is.Null);
            Assert.That(connect.StreamingAssetsPath, Is.Null);
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void MissingOrMismatchedInitialSnapshotStopsWithoutRetry()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                )
            );

            harness.Runner.Connect();
            harness.Runner.RunFrame();

            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "stop" }));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(
                harness.Logger.Records.Last().EventName,
                Is.EqualTo("battlement.session.failed")
            );

            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    responseSession: session,
                    snapshotSession: new SessionId(Guid.NewGuid())
                )
            );
            harness.Runner.Reconnect();

            Assert.That(harness.Transport.Calls.Count(call => call == "connect"), Is.EqualTo(2));
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        [Test]
        public void ReconnectDiscardsPreviousSessionUntilAnotherExplicitReconnect()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId first = new(Guid.NewGuid());
            SessionId second = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(first));
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(first));
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(second));

            harness.Runner.Connect();
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            harness.Runner.Reconnect();
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(
                harness.Logger.Records.Last().EventName,
                Is.EqualTo("battlement.response.wrong_session")
            );

            harness.Runner.Reconnect();

            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            Assert.That(
                harness.Transport.Calls,
                Is.EqualTo(new[] { "connect", "stop", "connect", "stop", "connect" })
            );
        }

        [Test]
        public void FailureResumeFocusLossAndShutdownApplyLifecycleRules()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Transport.EnqueueConnect(
                new BattlementTransportResult(
                    BattlementTransportStatus.TransportError,
                    diagnostic: "offline"
                )
            );
            harness.Runner.Connect();
            harness.Runner.RunFrame();
            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "stop" }));

            harness.Runner.Reconnect();
            LogAssert.ignoreFailingMessages = true;
            try
            {
                harness.Runner.SendMessage("OnApplicationFocus", false);
                Assert.That(harness.Runner.IsInputAvailable, Is.True);
                Assert.That(
                    harness.Logger.Records.Last().EventName,
                    Is.EqualTo("battlement.input.pointer_presses_cancelled")
                );

                harness.Runner.SendMessage("OnApplicationPause", true);
                harness.Runner.SendMessage("OnApplicationPause", false);
                Assert.That(harness.Runner.IsInputAvailable, Is.False);
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));

                harness.Runner.Reconnect();
                harness.Runner.SendMessage("OnApplicationQuit");
                Assert.That(harness.Runner.IsInputAvailable, Is.False);
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            }
            finally
            {
                LogAssert.ignoreFailingMessages = false;
            }
        }

        private static string Absolute(string path) => Path.GetFullPath(path);

        private static string ExpectedPlatform() =>
            Application.platform switch
            {
                RuntimePlatform.OSXEditor or RuntimePlatform.OSXPlayer => "macOS",
                RuntimePlatform.WindowsEditor or RuntimePlatform.WindowsPlayer => "Windows",
                RuntimePlatform.LinuxEditor or RuntimePlatform.LinuxPlayer => "Linux",
                RuntimePlatform.IPhonePlayer => "iOS",
                RuntimePlatform.Android => "Android",
                _ => Application.platform.ToString(),
            };
    }
}
