#nullable enable

using System;
using System.Linq;
using System.Reflection;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class BattlementFramePacingTests
    {
        [Test]
        public void DesktopRetainsItsCapAcrossSynchronizationAndCapabilityChanges()
        {
            var backend = new Backend();
            var pacing = new BattlementFramePacing(backend);
            HostSettings host = Host(HostPlatform.MacOs, 60, 120, 144, 240);
            pacing.Apply(new CommandId(Guid.NewGuid()), new FramePacing(144, true), host);
            Assert.That((backend.SyncCount, backend.TargetRate), Is.EqualTo((1, -1)));
            pacing.Apply(new CommandId(Guid.NewGuid()), new FramePacing(144, false), host);
            Assert.That((backend.SyncCount, backend.TargetRate), Is.EqualTo((0, 144)));
            pacing.Reconcile(host with { FrameRates = new uint[] { 60, 120 } });
            Assert.That(backend.TargetRate, Is.EqualTo(120));
            pacing.Reconcile(host);
            Assert.That(backend.TargetRate, Is.EqualTo(144));
        }

        [Test]
        public void UnavailableAndPartialFailureReportActualState()
        {
            var backend = new Backend { TargetRate = 60 };
            var pacing = new BattlementFramePacing(backend);
            var id = new CommandId(Guid.NewGuid());
            Assert.Throws<BattlementCommandException>(() =>
                pacing.Apply(id, new FramePacing(144, true), new HostSettings())
            );
            Assert.That((backend.SyncCount, backend.TargetRate), Is.EqualTo((0, 60)));
            backend.FailTargetWrite = true;
            HostSettings host = Host(HostPlatform.Windows, 60, 144);
            Assert.Throws<BattlementCommandException>(() =>
                pacing.Apply(id, new FramePacing(144, true), host)
            );
            HostSettings failed = pacing.Reconcile(host);
            Assert.That(failed.LastResult!.RequestId, Is.EqualTo(id));
            Assert.That(failed.LastResult.Error, Is.Not.Null);
            Assert.That(failed.AppliedVsync, Is.True);
            Assert.That(failed.AppliedFrameRate, Is.EqualTo(60));
            backend.FailTargetWrite = false;
            Assert.That(pacing.Reconcile(host).LastResult!.Error, Is.Null);
            Assert.That(backend.TargetRate, Is.EqualTo(-1));
        }

        [TestCase(60U, new uint[] { 30, 60 })]
        [TestCase(120U, new uint[] { 30, 60, 120 })]
        [TestCase(90U, new uint[] { 30 })]
        [TestCase(0U, new uint[] { })]
        public void IosOffersOnlyDivisionsOfTheReportedConfiguredRefresh(
            uint refresh,
            uint[] expected
        )
        {
            uint[] rates = BattlementFramePacing.Rates(
                HostPlatform.Ios,
                new RefreshRate { numerator = refresh, denominator = 1 }
            );
            Assert.That(rates, Is.EqualTo(expected));
            if (rates.Length == 0)
                return;
            var backend = new Backend();
            var pacing = new BattlementFramePacing(backend);
            pacing.Apply(
                new CommandId(Guid.NewGuid()),
                new FramePacing(144, true),
                Host(HostPlatform.Ios, rates)
            );
            Assert.That(backend.SyncCount, Is.Zero);
            Assert.That(backend.TargetRate, Is.EqualTo(expected.Max()));
        }

        [Test]
        public void BrowserCapDisablesInheritedSynchronizationAndMapsUnsupportedIntent()
        {
            var backend = new Backend { SyncCount = 1 };
            var pacing = new BattlementFramePacing(backend);
            var host = Host(HostPlatform.Web, 30, 60, 120, 144, 240);
            pacing.Apply(new CommandId(Guid.NewGuid()), new FramePacing(100, true), host);
            Assert.That((backend.SyncCount, backend.TargetRate), Is.EqualTo((0, 60)));
            pacing.Apply(new CommandId(Guid.NewGuid()), new FramePacing(1, false), host);
            Assert.That(backend.TargetRate, Is.EqualTo(30));
        }

        [Test]
        public void SerializedCommandAppliesAndReportsNativeUnityPacingWithInputDisabled()
        {
            int priorSync = QualitySettings.vSyncCount;
            int priorRate = Application.targetFrameRate;
            try
            {
                using var harness = BattlementTestHarness.Create(readHostSettings: () =>
                    Host(HostPlatform.MacOs, 60, 120, 144, 240)
                );
                var session = new SessionId(Guid.NewGuid());
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(session, inputDisabled: true)
                );
                harness.Transport.DefaultSubmitResult = () =>
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    );
                harness.Runner.Connect();
                foreach (bool sync in new[] { true, false })
                {
                    var command = new Command(
                        new CommandId(Guid.NewGuid()),
                        new CommandBody.ApplicationSetFramePacing(new FramePacing(120, sync))
                    );
                    var batch = new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        new[] { new ParallelCommandGroup<Command>(new[] { command }) },
                        Start: BatchStart.Now
                    );
                    harness.Transport.EnqueueSubmit(
                        FakeBattlementTransport.ResponseResult(
                            new Response(
                                session,
                                new ResponseMessage<Command>[]
                                {
                                    new ResponseMessage<Command>.BatchMessage(batch),
                                }
                            )
                        )
                    );
                    harness.Runner.Submit(new byte[] { 1 });
                    harness.Runner.RunFrame();
                    typeof(BattlementRunner)
                        .GetMethod(
                            "PublishHostSettings",
                            BindingFlags.Instance | BindingFlags.NonPublic
                        )!
                        .Invoke(harness.Runner, new object[] { true });
                    HostSettings observed = harness
                        .Transport.Actions.Select(action => action.Body)
                        .OfType<ActionBody.HostSettingsChanged>()
                        .Last()
                        .Value;
                    Assert.That(
                        observed.LastResult,
                        Is.EqualTo(new HostSettingsResult(command.Id))
                    );
                    Assert.That(observed.AppliedVsync, Is.EqualTo(sync));
                    Assert.That(observed.AppliedFrameRate, Is.EqualTo(sync ? -1 : 120));
                    Assert.That(QualitySettings.vSyncCount, Is.EqualTo(sync ? 1 : 0));
                    Assert.That(Application.targetFrameRate, Is.EqualTo(sync ? -1 : 120));
                    Debug.Log($"Pacing: {observed.AppliedVsync}/{observed.AppliedFrameRate}");
                }
            }
            finally
            {
                QualitySettings.vSyncCount = priorSync;
                Application.targetFrameRate = priorRate;
            }
        }

        private static HostSettings Host(HostPlatform platform, params uint[] rates) =>
            new()
            {
                Platform = platform,
                FramePacing = SettingAvailability.Available,
                FrameRates = rates,
                VsyncAvailable = platform is HostPlatform.MacOs or HostPlatform.Windows,
            };

        private sealed class Backend : IFramePacingBackend
        {
            private int targetRate;
            public bool FailTargetWrite { get; set; }
            public int SyncCount { get; set; }
            public int TargetRate
            {
                get => targetRate;
                set
                {
                    if (FailTargetWrite)
                        throw new InvalidOperationException("Injected pacing failure.");
                    targetRate = value;
                }
            }
        }
    }
}
