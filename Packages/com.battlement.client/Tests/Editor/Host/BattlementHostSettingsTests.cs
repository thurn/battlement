#nullable enable

using System;
using System.Linq;
using Google.FlatBuffers;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.TestTools;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    public sealed class BattlementHostSettingsTests : InputTestFixture
    {
        [Test]
        public void AttachedDevicesPublishWhileInputIsDisabledAndUnchangedSnapshotsStayQuiet()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                readHostSettings: () =>
                    BattlementHostSettings.Read(Array.Empty<string>())
            );
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, inputDisabled: true)
            );
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            HostSettings initial = harness.Transport.ConnectValues.Single().HostSettings;
            Assert.That(initial.KeyboardConnected, Is.False);
            Assert.That(initial.ControllerCount, Is.Zero);
            var keyboard = InputSystem.AddDevice<Keyboard>();
            var gamepad = InputSystem.AddDevice<Gamepad>();
            harness.Runner.RunFrame();
            Assert.That(Observations(harness).Single().KeyboardConnected, Is.True);
            Assert.That(Observations(harness).Single().ControllerCount, Is.EqualTo(1));
            LogAssert.Expect(
                LogType.Assert,
                "Assertion failed on expression: 'ShouldRunBehaviour()'"
            );
            harness.Runner.SendMessage("OnApplicationFocus", true);
            Assert.That(Observations(harness).Length, Is.EqualTo(1));
            InputSystem.RemoveDevice(keyboard);
            InputSystem.RemoveDevice(gamepad);
            LogAssert.Expect(
                LogType.Assert,
                "Assertion failed on expression: 'ShouldRunBehaviour()'"
            );
            harness.Runner.SendMessage("OnApplicationFocus", true);
            Assert.That(Observations(harness).Last().KeyboardConnected, Is.False);
            Assert.That(Observations(harness).Last().ControllerCount, Is.Zero);
            InputSystem.AddDevice<Keyboard>();
            InputSystem.AddDevice<Gamepad>();
            LogAssert.Expect(
                LogType.Assert,
                "Assertion failed on expression: 'ShouldRunBehaviour()'"
            );
            harness.Runner.SendMessage("OnApplicationFocus", true);
            Assert.That(Observations(harness).Last().KeyboardConnected, Is.True);
            Assert.That(Observations(harness).Last().ControllerCount, Is.EqualTo(1));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
        }

        [Test]
        public void NativeObservationAgreesWithUnityDisplayAndPacing()
        {
            HostSettings observed = BattlementHostSettings.Read(Array.Empty<string>());
            Assert.That(
                observed.Platform,
                Is.EqualTo(
                    Application.platform == RuntimePlatform.OSXEditor
                        ? HostPlatform.MacOs
                        : HostPlatform.Windows
                )
            );
            Assert.That(observed.Display, Is.EqualTo(SettingAvailability.Available));
            Assert.That(
                observed.AppliedDisplay!.Resolution.Width,
                Is.EqualTo(Math.Max(1, Screen.width))
            );
            Assert.That(
                observed.AppliedDisplay.Resolution.Height,
                Is.EqualTo(Math.Max(1, Screen.height))
            );
            Assert.That(
                observed.AppliedDisplay.Resolution.RefreshNumerator,
                Is.EqualTo(Screen.currentResolution.refreshRateRatio.numerator)
            );
            Assert.That(observed.AppliedFrameRate, Is.EqualTo(Application.targetFrameRate));
            Assert.That(observed.AppliedVsync, Is.EqualTo(QualitySettings.vSyncCount > 0));
            Assert.That(observed.Diagnostics, Is.EqualTo(SettingAvailability.Unavailable));
            Assert.That(observed.ObservationError, Is.Null);
            Debug.Log(
                $"Host settings snapshot: {observed.Platform}, {observed.AppliedDisplay}, "
                    + $"cap {observed.AppliedFrameRate}, VSync {observed.AppliedVsync}, "
                    + $"{observed.Resolutions.Count} resolutions"
            );
        }

        [Test]
        public void ConnectAndChangedActionWriteVerifiedTypedObservations()
        {
            var value = new HostSettings
            {
                Platform = HostPlatform.Windows,
                Display = SettingAvailability.Available,
                DisplayModes = new[] { DisplayMode.Windowed, DisplayMode.Fullscreen },
                Resolutions = new[] { new DisplayResolution(1920, 1080, 60000, 1001) },
                AppliedDisplay = new DisplayConfiguration(
                    DisplayMode.Windowed,
                    new DisplayResolution(1920, 1080, 60000, 1001)
                ),
                FramePacing = SettingAvailability.Available,
                FrameRates = new uint[] { 30, 60, 144 },
                AppliedFrameRate = 60,
                VsyncAvailable = true,
                AppliedVsync = true,
                KeyboardConnected = true,
                ControllerCount = 2,
                Diagnostics = SettingAvailability.Failed,
                DiagnosticsConfigured = true,
                ObservationError = "vendor unavailable",
                LastResult = new HostSettingsResult(new CommandId(Guid.NewGuid()), "denied"),
            };
            var connect = new Connect(
                "test",
                "test",
                new ScreenSize(1920, 1080),
                Array.Empty<string>()
            )
            {
                HostSettings = value,
            };
            byte[] bytes = new BattlementConnectRequestWriter().Write(connect).ToArray();
            var buffer = new ByteBuffer(bytes);
            Assert.That(
                new Verifier(buffer, new Options()).VerifyBuffer(
                    "BTCO",
                    true,
                    Wire.ConnectRequestVerify.Verify
                ),
                Is.True
            );
            buffer.Position = 4;
            AssertWire(
                Wire.ConnectRequest.GetRootAsConnectRequest(buffer).HostSettings!.Value,
                value
            );
            var writer = new BattlementCoreClientMessageWriter();
            bytes = writer
                .WriteAction(
                    new Action(
                        new ActionId(Guid.NewGuid()),
                        new SessionId(Guid.NewGuid()),
                        new ActionBody.HostSettingsChanged(value)
                    )
                )
                .ToArray();
            buffer = new ByteBuffer(bytes);
            Assert.That(
                new Verifier(buffer, new Options()).VerifyBuffer(
                    "BTCM",
                    true,
                    Wire.CoreClientMessageVerify.Verify
                ),
                Is.True
            );
            buffer.Position = 4;
            Wire.CoreAction action = Wire
                .CoreClientMessage.GetRootAsCoreClientMessage(buffer)
                .BodyAsCoreAction();
            Assert.That(action.Kind, Is.EqualTo(Wire.CoreActionKind.HostSettingsChanged));
            AssertWire(action.BodyAsHostSettingsAction().Value!.Value, value);
            Assert.Throws<System.IO.InvalidDataException>(() =>
                new BattlementConnectRequestWriter().Write(
                    connect with
                    {
                        HostSettings = value with { FrameRates = new uint[] { 60, 30 } },
                    }
                )
            );
        }

        private static HostSettings[] Observations(BattlementTestHarness harness) =>
            harness
                .Transport.Actions.Select(action => action.Body)
                .OfType<ActionBody.HostSettingsChanged>()
                .Select(action => action.Value)
                .ToArray();

        private static void AssertWire(Wire.HostSettings wire, HostSettings value)
        {
            Assert.That(wire.Platform, Is.EqualTo(Wire.HostPlatform.Windows));
            Assert.That(wire.Display, Is.EqualTo(Wire.SettingAvailability.Available));
            Assert.That(wire.DisplayModesLength, Is.EqualTo(2));
            Assert.That(wire.ResolutionsLength, Is.EqualTo(1));
            Assert.That(wire.Resolutions(0)!.Value.RefreshDenominator, Is.EqualTo(1001));
            Assert.That(wire.AppliedDisplay!.Value.Resolution!.Value.Width, Is.EqualTo(1920));
            Assert.That(wire.FramePacing, Is.EqualTo(Wire.SettingAvailability.Available));
            Assert.That(wire.FrameRatesLength, Is.EqualTo(3));
            Assert.That(wire.AppliedFrameRate, Is.EqualTo(60));
            Assert.That(
                wire.VsyncAvailable && wire.AppliedVsync && wire.KeyboardConnected,
                Is.True
            );
            Assert.That(wire.ControllerCount, Is.EqualTo(2));
            Assert.That(wire.Diagnostics, Is.EqualTo(Wire.SettingAvailability.Failed));
            Assert.That(wire.DiagnosticsConfigured, Is.True);
            Assert.That(wire.ObservationError, Is.EqualTo(value.ObservationError));
            Assert.That(wire.LastResult!.Value.Error, Is.EqualTo(value.LastResult!.Error));
        }
    }
}
