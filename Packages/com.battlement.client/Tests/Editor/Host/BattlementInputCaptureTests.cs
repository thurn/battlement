#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Google.FlatBuffers;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    public sealed class BattlementInputCaptureTests : InputTestFixture
    {
        private Keyboard keyboard = null!;
        private Gamepad gamepad = null!;
        private readonly List<InputCaptureEvent> results = new();

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            keyboard = InputSystem.AddDevice<Keyboard>();
            gamepad = InputSystem.AddDevice<Gamepad>();
            results.Clear();
        }

        [Test]
        public void HeldOpenerMustReleaseBeforeOneSourceAwareDpadCapture()
        {
            var capture = new BattlementInputCapture(results.Add);
            var id = new ObjectId(Guid.NewGuid());
            Press(gamepad.buttonSouth);
            capture.Execute(new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Controller)));
            capture.Update(true);
            Set(gamepad.dpad, Vector2.left);
            capture.Update(true);
            Assert.That(results, Is.Empty);
            Release(gamepad.buttonSouth);
            Set(gamepad.dpad, Vector2.zero);
            capture.Update(true);
            Set(gamepad.leftStick, Vector2.right);
            capture.Update(true);
            Assert.That(results, Is.Empty, "stick input is consumed but is not a binding");
            Set(gamepad.leftStick, Vector2.zero);
            Set(gamepad.dpad, Vector2.left);
            capture.Update(true);
            capture.Update(true);
            Assert.That(
                results,
                Is.EqualTo(
                    new[]
                    {
                        new InputCaptureEvent(
                            id,
                            new InputCaptureResult.Direction(
                                gamepad.deviceId,
                                ControllerDirection.Left,
                                ControllerNavigationSource.Dpad,
                                false
                            )
                        ),
                    }
                )
            );
        }

        [Test]
        public void ClosingFromResultKeepsInputBlockedThroughRelease()
        {
            var id = new ObjectId(Guid.NewGuid());
            BattlementInputCapture capture = null!;
            capture = new BattlementInputCapture(value =>
            {
                results.Add(value);
                capture.Execute(new InputCaptureCommand.End(id));
            });
            capture.Execute(new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Keyboard)));
            Press(keyboard.kKey);
            capture.Update(true);
            Assert.That(capture.BlocksInput, Is.True);
            capture.Update(true);
            Assert.That(
                results,
                Is.EqualTo(
                    new[]
                    {
                        new InputCaptureEvent(
                            id,
                            new InputCaptureResult.Key(keyboard.deviceId, PhysicalKey.KeyK)
                        ),
                    }
                )
            );
            Release(keyboard.kKey);
            capture.Update(true);
            Assert.That(capture.BlocksInput, Is.True);
            capture.Update(true);
            Assert.That(capture.BlocksInput, Is.False);
        }

        [Test]
        public void ChordsCannotBecomeBindingsWhenOneKeyIsReleased()
        {
            var capture = new BattlementInputCapture(results.Add);
            var id = new ObjectId(Guid.NewGuid());
            capture.Execute(new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Keyboard)));
            Press(keyboard.leftCtrlKey);
            Press(keyboard.kKey);
            capture.Update(true);
            Release(keyboard.leftCtrlKey);
            capture.Update(true);
            Assert.That(results, Is.Empty);
            Release(keyboard.kKey);
            capture.Update(true);
            Press(keyboard.jKey);
            capture.Update(true);
            Assert.That(
                results[0].Result,
                Is.EqualTo(new InputCaptureResult.Key(keyboard.deviceId, PhysicalKey.KeyJ))
            );
        }

        [TestCase(true)]
        [TestCase(false)]
        public void FocusLossOrDeviceRemovalPublishesOneCancellation(bool focusLost)
        {
            var capture = new BattlementInputCapture(results.Add);
            var id = new ObjectId(Guid.NewGuid());
            capture.Execute(new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Controller)));
            if (!focusLost)
                InputSystem.RemoveDevice(gamepad);
            capture.Update(!focusLost);
            capture.Update(!focusLost);
            Assert.That(
                results,
                Is.EqualTo(
                    new[]
                    {
                        new InputCaptureEvent(
                            id,
                            new InputCaptureResult.Cancelled(
                                focusLost
                                    ? InputCaptureCancellation.FocusLost
                                    : InputCaptureCancellation.DeviceDisconnected
                            )
                        ),
                    }
                )
            );
            capture.Execute(new InputCaptureCommand.End(id));
            capture.Update(true);
            capture.Update(true);
            Assert.That(capture.BlocksInput, Is.False);
        }

        [Test]
        public void RunnerCapturesUnsubscribedButtonBeforeGlobalInputAndWritesDeviceMetadata()
        {
            using var harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    controllerInput: new ControllerInputSettings(new[] { ControllerButton.South })
                )
            );
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            harness.Runner.RunFrame();
            var id = new ObjectId(Guid.NewGuid());
            SendCapture(
                harness,
                session,
                new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Controller))
            );
            harness.Transport.Actions.Clear();
            Press(gamepad.buttonNorth);
            harness.Runner.RunFrame();
            harness.Runner.RunFrame();
            ActionBody.InputCaptured captured = harness
                .Transport.Actions.Select(action => action.Body)
                .OfType<ActionBody.InputCaptured>()
                .Single();
            Assert.That(
                captured.Value,
                Is.EqualTo(
                    new InputCaptureEvent(
                        id,
                        new InputCaptureResult.Button(gamepad.deviceId, ControllerButton.North)
                    )
                )
            );
            Assert.That(
                harness
                    .Transport.Actions.Select(action => action.Body)
                    .OfType<ActionBody.ControllerButtonDown>(),
                Is.Empty
            );
            var buffer = harness
                .Transport.SubmitMessages.Where(bytes => bytes.Length > 4)
                .Select(bytes => new ByteBuffer(bytes))
                .Single(candidate =>
                {
                    candidate.Position = 4;
                    Wire.CoreClientMessage message =
                        Wire.CoreClientMessage.GetRootAsCoreClientMessage(candidate);
                    return message.BodyType == Wire.CoreClientMessageBody.CoreAction
                        && message.BodyAsCoreAction().Kind == Wire.CoreActionKind.InputCaptured;
                });
            buffer.Position = 0;
            Assert.That(
                new Verifier(buffer, new Options()).VerifyBuffer(
                    "BTCM",
                    true,
                    Wire.CoreClientMessageVerify.Verify
                ),
                Is.True
            );
            buffer.Position = 4;
            Wire.InputCaptureAction wire = Wire
                .CoreClientMessage.GetRootAsCoreClientMessage(buffer)
                .BodyAsCoreAction()
                .BodyAsInputCaptureAction();
            Assert.That(wire.DeviceId, Is.EqualTo(gamepad.deviceId));
            Assert.That(wire.Button, Is.EqualTo(Wire.ControllerButton.North));
            SendCapture(harness, session, new InputCaptureCommand.End(id));
            Release(gamepad.buttonNorth);
            harness.Runner.RunFrame();
            harness.Runner.RunFrame();
            Press(gamepad.buttonSouth);
            harness.Runner.RunFrame();
            Assert.That(
                harness
                    .Transport.Actions.Select(action => action.Body)
                    .OfType<ActionBody.ControllerButtonDown>()
                    .Single()
                    .Button,
                Is.EqualTo(ControllerButton.South)
            );
        }

        [Test]
        public void ControlledInputKeepsCaptureOpenWithoutReadingPhysicalDevices()
        {
            var capture = new BattlementInputCapture(results.Add);
            var id = new ObjectId(Guid.NewGuid());
            capture.Execute(new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Keyboard)));
            Press(keyboard.kKey);
            capture.Update(true, observePhysicalInput: false);
            InputSystem.RemoveDevice(keyboard);
            capture.Update(true, observePhysicalInput: false);
            Assert.That(results, Is.Empty);
            Assert.That(capture.BlocksInput, Is.True);
            capture.Execute(new InputCaptureCommand.End(id));
            Press(gamepad.buttonSouth);
            capture.Update(true, observePhysicalInput: false);
            Assert.That(capture.BlocksInput, Is.True);
            capture.Update(true, observePhysicalInput: false);
            Assert.That(capture.BlocksInput, Is.False);
        }

        [TestCase("OnApplicationFocus", false)]
        [TestCase("OnApplicationPause", true)]
        public void LifecycleCallbackCancelsBeforeAnotherFrame(string message, bool unavailable)
        {
            using var harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            var id = new ObjectId(Guid.NewGuid());
            SendCapture(
                harness,
                session,
                new InputCaptureCommand.Begin(new(id, InputCaptureDevice.Controller))
            );
            harness.Transport.Actions.Clear();
            MethodInfo callback = typeof(BattlementRunner).GetMethod(
                message,
                BindingFlags.Instance | BindingFlags.NonPublic
            )!;
            callback.Invoke(harness.Runner, new object[] { unavailable });
            callback.Invoke(harness.Runner, new object[] { !unavailable });
            Assert.That(
                harness
                    .Transport.Actions.Select(action => action.Body)
                    .OfType<ActionBody.InputCaptured>()
                    .Select(action => action.Value),
                Is.EqualTo(
                    new[]
                    {
                        new InputCaptureEvent(
                            id,
                            new InputCaptureResult.Cancelled(InputCaptureCancellation.FocusLost)
                        ),
                    }
                )
            );
            Press(gamepad.buttonNorth);
            harness.Runner.RunFrame();
            Assert.That(
                harness
                    .Transport.Actions.Select(action => action.Body)
                    .OfType<ActionBody.InputCaptured>()
                    .Count(),
                Is.EqualTo(1)
            );
        }

        private static void SendCapture(
            BattlementTestHarness harness,
            SessionId session,
            InputCaptureCommand capture
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    new ParallelCommandGroup<Command>(
                        new[]
                        {
                            new Command(
                                new CommandId(Guid.NewGuid()),
                                new CommandBody.Input.Capture(capture)
                            ),
                        }
                    ),
                },
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
        }

        [Test]
        public void StaleOwnerCannotReleaseReplacementCapture()
        {
            var capture = new BattlementInputCapture(results.Add);
            var first = new ObjectId(Guid.NewGuid());
            var second = new ObjectId(Guid.NewGuid());
            capture.Execute(new InputCaptureCommand.Begin(new(first, InputCaptureDevice.Keyboard)));
            capture.Execute(
                new InputCaptureCommand.Begin(new(second, InputCaptureDevice.Controller))
            );
            capture.Execute(new InputCaptureCommand.End(first));
            Assert.That(capture.BlocksInput, Is.True);
            Press(gamepad.buttonWest);
            capture.Update(true);
            Assert.That(
                results,
                Is.EqualTo(
                    new[]
                    {
                        new InputCaptureEvent(
                            first,
                            new InputCaptureResult.Cancelled(InputCaptureCancellation.Superseded)
                        ),
                        new InputCaptureEvent(
                            second,
                            new InputCaptureResult.Button(gamepad.deviceId, ControllerButton.West)
                        ),
                    }
                )
            );
        }
    }
}
