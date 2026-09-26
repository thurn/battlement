#nullable enable

using System;
using System.IO;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementDisplayPreviewTests
    {
        [Test]
        public void DittoDisplaySessionsIgnoreSavedDesktopPreferencesAndPriorScenarios()
        {
            var backend = new Backend();
            var saved = new Store { Record = new DisplayRecoveryRecord(Prior, false) };
            using var runtime = new BattlementConfiguredRuntime(
                new BattlementRunnerOptions(
                    new FakeBattlementTransport(),
                    new FakeBattlementAssetStorage()
                ),
                backend,
                saved
            );
            var portrait = new HostSettings
            {
                Platform = HostPlatform.MacOs,
                Display = SettingAvailability.Available,
                DisplayModes = new[] { DisplayMode.Windowed },
                AppliedDisplay = new DisplayConfiguration(
                    DisplayMode.Windowed,
                    Resolution(720, 1280)
                ),
            };
            runtime.BeginDittoDisplay();
            runtime.Display.Observe(portrait, true);
            Assert.That(
                backend.Requested,
                Is.Null,
                "The declared profile owns the initial framebuffer."
            );
            Assert.That(saved.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, false)));

            CommandId preview = Id();
            runtime.Display.Begin(preview, Prior, portrait);
            var applied = portrait with { AppliedDisplay = Prior };
            runtime.Display.Observe(applied, true);
            runtime.Display.Confirm(Id(), preview, applied);
            Assert.That(runtime.LastSettingResult, Is.Not.Null);
            Assert.That(saved.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, false)));

            backend.Requested = null;
            runtime.BeginDittoDisplay();
            runtime.Display.Observe(portrait, true);
            Assert.That(
                backend.Requested,
                Is.Null,
                "A scenario cannot restore another scenario's choice."
            );
            Assert.That(runtime.LastSettingResult, Is.Null);
            Assert.That(saved.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, false)));
        }

        [Test]
        public void SerializedDisplayOperationsPassNativeAdmissionAndRejectInvalidDimensions()
        {
            var session = new SessionId(Guid.NewGuid());
            CommandId preview = Id();
            foreach (
                DisplayCommand operation in new DisplayCommand[]
                {
                    new DisplayCommand.Preview(Target),
                    new DisplayCommand.Confirm(preview),
                    new DisplayCommand.Cancel(preview),
                    new DisplayCommand.Preview(Target with { Resolution = Resolution(0, 720) }),
                }
            )
            {
                var command = new Command(Id(), new CommandBody.ApplicationDisplay(operation));
                var batch = new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    new[] { new ParallelCommandGroup<Command>(new[] { command }) }
                );
                var bytes = BattlementFlatBufferResponseFixtures.Write(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.BatchMessage(batch),
                        }
                    )
                );
                if (
                    operation is DisplayCommand.Preview
                    {
                        Configuration: { Resolution: { Width: 0 } }
                    }
                )
                {
                    Assert.Throws<InvalidDataException>(() =>
                        new BattlementFlatBufferResponse(bytes, null)
                    );
                    continue;
                }
                using var response = new BattlementFlatBufferResponse(bytes, null);
                Assert.That(
                    response.ReadBatch(0).ReadCommand(0, 0).DirectDisplay,
                    Is.EqualTo(operation)
                );
            }
        }

        [Test]
        public void ReadbackGatesConfirmationAndRealTimeTimeoutRestoresThePriorDisplay()
        {
            var host = new Harness();
            CommandId preview = Id();
            host.Controller.Begin(preview, Target, host.Observation);
            Assert.That(host.Store.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, true)));
            Assert.That(
                host.Controller.Observation!.State,
                Is.EqualTo(DisplayPreviewState.Applying)
            );
            host.Controller.Confirm(Id(), preview, host.Observation);
            Assert.That(host.Store.Record!.PreviewActive, Is.True);
            host.Render();
            Assert.That(
                host.Controller.Observation!.State,
                Is.EqualTo(DisplayPreviewState.Confirmable)
            );
            host.Now = 14.9;
            host.Observe();
            Assert.That(host.Controller.Observation!.RemainingSeconds, Is.EqualTo(1));
            host.Now = 15;
            host.Observe();
            Assert.That(host.Backend.Requested, Is.EqualTo(Prior));
            Assert.That(
                host.Controller.Observation!.State,
                Is.EqualTo(DisplayPreviewState.Reverting)
            );
            host.Render();
            Assert.That(host.Controller.Observation, Is.Null);
            Assert.That(host.Store.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, false)));
            Assert.That(host.Controller.Result!.Error, Does.Contain("timed out"));
        }

        [Test]
        public void ReplacementRejectsStaleConfirmAndCancelWithoutChangingTheNewPreview()
        {
            var host = new Harness();
            CommandId first = Id();
            CommandId second = Id();
            host.Controller.Begin(first, Target, host.Observation);
            host.Render();
            host.Controller.Begin(second, Bigger, host.Observation);
            host.Render();
            host.Controller.Confirm(Id(), first, host.Observation);
            host.Controller.Cancel(Id(), first);
            Assert.That(host.Controller.Observation!.RequestId, Is.EqualTo(second));
            Assert.That(host.Store.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, true)));
            CommandId confirm = Id();
            host.Controller.Confirm(confirm, second, host.Observation);
            Assert.That(host.Controller.Observation, Is.Null);
            Assert.That(host.Store.Record, Is.EqualTo(new DisplayRecoveryRecord(Bigger, false)));
            Assert.That(host.Controller.Result, Is.EqualTo(new HostSettingsResult(confirm)));
            host.Restart(Prior);
            Assert.That(host.Backend.Requested, Is.EqualTo(Bigger));
            host.Render();
            Assert.That(host.Observation.AppliedDisplay, Is.EqualTo(Bigger));
        }

        [TestCase(false)]
        [TestCase(true)]
        public void FailedConfirmationPreservesCrashRecoveryAndNeverConfirmsThePreview(
            bool afterReplace
        )
        {
            var host = new Harness();
            CommandId preview = Id();
            host.Controller.Begin(preview, Target, host.Observation);
            host.Render();
            host.Store.FailNext = true;
            host.Store.FailAfterReplace = afterReplace;
            host.Controller.Confirm(Id(), preview, host.Observation);
            Assert.That(host.Controller.Result!.Error, Does.Contain("Could not keep"));
            Assert.That(host.Store.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, true)));
            // A process exit before Unity renders the rollback leaves the preview on screen.
            host.Restart(Target);
            Assert.That(host.Backend.Requested, Is.EqualTo(Prior));
            host.Render();
            Assert.That(host.Store.Record, Is.EqualTo(new DisplayRecoveryRecord(Prior, false)));
        }

        [Test]
        public void RecoveryMustBeSavedBeforeApplyAndFocusOrOwnerLossReverts()
        {
            var host = new Harness();
            host.Store.FailNext = true;
            host.Controller.Begin(Id(), Target, host.Observation);
            Assert.That(host.Backend.Requested, Is.Null);
            Assert.That(host.Controller.Observation, Is.Null);
            foreach (bool ownerLoss in new[] { false, true })
            {
                host.Controller.Begin(Id(), Target, host.Observation);
                host.Render();
                if (ownerLoss)
                    host.Controller.LoseOwner();
                else
                    host.Controller.Observe(host.Observation, false);
                Assert.That(host.Backend.Requested, Is.EqualTo(Prior));
                host.Render();
                Assert.That(host.Controller.Observation, Is.Null);
            }
        }

        [TestCase(DisplayMode.Fullscreen)]
        [TestCase(DisplayMode.Windowed)]
        public void MonitorChangesAndApplyFailuresUseAWindowThatFitsTheCurrentDisplay(
            DisplayMode mode
        )
        {
            var host = new Harness();
            var oversized = new DisplayConfiguration(mode, Resolution(3840, 2160));
            host.Store.Record = new DisplayRecoveryRecord(oversized, true);
            host.Restart(mode == DisplayMode.Windowed ? oversized : Target);
            Assert.That(host.Backend.Requested, Is.EqualTo(host.Backend.SafeWindow));
            host.Render();
            Assert.That(host.Store.Record!.Confirmed, Is.EqualTo(host.Backend.SafeWindow));
            host.Backend.FailNext = true;
            host.Controller.Begin(Id(), Target, host.Observation);
            Assert.That(host.Backend.Requested, Is.EqualTo(host.Backend.SafeWindow));
            host.Render();
            Assert.That(host.Controller.Result!.Error, Does.Contain("Could not apply"));
        }

        [Test]
        public void NativeRecoveryFileSurvivesReloadAndCorruptionRecoversSafely()
        {
            string directory = Path.Combine(
                Path.GetTempPath(),
                "display-recovery-" + Guid.NewGuid()
            );
            string path = Path.Combine(directory, "display.json");
            try
            {
                var store = new BattlementDisplayRecoveryStore(path);
                store.Save(new DisplayRecoveryRecord(Prior, true));
                Assert.That(
                    new BattlementDisplayRecoveryStore(path).Load(),
                    Is.EqualTo(new DisplayRecoveryRecord(Prior, true))
                );
                var host = new Harness();
                var controller = new BattlementDisplayPreview(host.Backend, store, () => host.Now);
                controller.Observe(host.Observation with { AppliedDisplay = Target }, true);
                Assert.That(host.Backend.Requested, Is.EqualTo(Prior));
                controller.Observe(host.Observation, true);
                Assert.That(store.Load(), Is.EqualTo(new DisplayRecoveryRecord(Prior, false)));
                File.WriteAllText(path, "invalid recovery");
                controller = new BattlementDisplayPreview(host.Backend, store, () => host.Now);
                controller.Observe(host.Observation, true);
                Assert.That(host.Backend.Requested, Is.EqualTo(host.Backend.SafeWindow));
                controller.Observe(
                    host.Observation with
                    {
                        AppliedDisplay = host.Backend.SafeWindow,
                    },
                    true
                );
                Assert.That(controller.RecoveryError, Does.Contain("recovery failed"));
                Assert.That(store.Load()!.Confirmed, Is.EqualTo(host.Backend.SafeWindow));
                Assert.That(Directory.GetFiles(directory, "*.tmp"), Is.Empty);
            }
            finally
            {
                if (Directory.Exists(directory))
                    Directory.Delete(directory, true);
            }
        }

        private static DisplayConfiguration Prior =>
            new(DisplayMode.Windowed, Resolution(1024, 768));
        private static DisplayConfiguration Target =>
            new(DisplayMode.Windowed, Resolution(1280, 720));
        private static DisplayConfiguration Bigger =>
            new(DisplayMode.Borderless, Resolution(1920, 1080));

        private static DisplayResolution Resolution(uint width, uint height) =>
            new(width, height, 60, 1);

        private static CommandId Id() => new(Guid.NewGuid());

        private sealed class Harness
        {
            public readonly Backend Backend = new();
            public readonly Store Store = new();
            public double Now;
            public BattlementDisplayPreview Controller { get; private set; }
            public HostSettings Observation { get; private set; } =
                new()
                {
                    Platform = HostPlatform.MacOs,
                    Display = SettingAvailability.Available,
                    DisplayModes = new[] { DisplayMode.Windowed, DisplayMode.Borderless },
                    Resolutions = new[] { Prior.Resolution, Target.Resolution, Bigger.Resolution },
                    AppliedDisplay = Prior,
                };

            public Harness()
            {
                Controller = new BattlementDisplayPreview(Backend, Store, () => Now);
            }

            public void Observe() => Controller.Observe(Observation, true);

            public void Render()
            {
                Observation = Observation with { AppliedDisplay = Backend.Requested! };
                Observe();
            }

            public void Restart(DisplayConfiguration actual)
            {
                Observation = Observation with
                {
                    AppliedDisplay = actual,
                    Resolutions = Observation
                        .Resolutions.Append(actual.Resolution)
                        .Distinct()
                        .ToArray(),
                };
                Controller = new BattlementDisplayPreview(Backend, Store, () => Now);
                Observe();
            }
        }

        private sealed class Store : IDisplayRecoveryStore
        {
            public DisplayRecoveryRecord? Record;
            public bool FailNext;
            public bool FailAfterReplace;

            public DisplayRecoveryRecord? Load() => Record;

            public void Save(DisplayRecoveryRecord record)
            {
                if (FailNext)
                {
                    FailNext = false;
                    if (FailAfterReplace)
                        Record = record;
                    throw new IOException("Injected durable write failure.");
                }
                Record = record;
            }
        }

        private sealed class Backend : IDisplayBackend
        {
            public DisplayConfiguration? Requested;
            public bool FailNext;
            public DisplayConfiguration SafeWindow =>
                new(DisplayMode.Windowed, Resolution(800, 600));

            public bool WindowFits(DisplayResolution resolution) =>
                resolution.Width <= 1920 && resolution.Height <= 1080;

            public void Apply(DisplayConfiguration configuration)
            {
                if (FailNext)
                {
                    FailNext = false;
                    throw new InvalidOperationException("Injected display failure.");
                }
                Requested = configuration;
            }
        }
    }
}
