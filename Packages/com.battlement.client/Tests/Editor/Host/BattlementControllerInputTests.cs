#nullable enable

using System;
using System.Linq;
using Newtonsoft.Json;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;

namespace Battlement.Tests
{
    public sealed class BattlementControllerInputTests : InputTestFixture
    {
        private Gamepad? gamepad;
        private SessionId session;

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            gamepad = InputSystem.AddDevice<Gamepad>("Battlement Test Controller");
        }

        [TearDown]
        public override void TearDown()
        {
            gamepad = null;
            base.TearDown();
        }

        [Test]
        public void SelectedButtonEmitsDownAndUp()
        {
            using BattlementTestHarness harness = Connect();

            Press(gamepad!.buttonSouth);
            harness.Runner.RunFrame();
            Release(gamepad.buttonSouth);
            harness.Runner.RunFrame();

            Assert.That(
                Actions(harness).Select(action => action.Body),
                Is.EqualTo(
                    new ActionBody[]
                    {
                        new ActionBody.ControllerButtonDown(
                            gamepad.deviceId,
                            ControllerButton.South
                        ),
                        new ActionBody.ControllerButtonUp(gamepad.deviceId, ControllerButton.South),
                    }
                )
            );
        }

        [Test]
        public void StickUsesDominantAxisDeadZoneAndConfiguredRepeatTiming()
        {
            using BattlementTestHarness harness = Connect(
                settings: new ControllerInputSettings(
                    new[] { ControllerButton.South },
                    StickDeadZone: 0.35,
                    RepeatDelay: TimeSpan.FromMilliseconds(275),
                    RepeatInterval: TimeSpan.FromMilliseconds(125)
                )
            );

            Set(gamepad!.leftStick, new Vector2(0.7f, 0.45f));
            harness.Runner.RunFrame();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(274));
            harness.Runner.RunFrame();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(1));
            harness.Runner.RunFrame();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(125));
            harness.Runner.RunFrame();

            ActionBody.ControllerNavigate[] navigation = Actions(harness)
                .Select(action => action.Body)
                .OfType<ActionBody.ControllerNavigate>()
                .ToArray();
            Assert.That(
                navigation.Select(action => action.Direction),
                Is.All.EqualTo(ControllerDirection.Right)
            );
            Assert.That(
                navigation.Select(action => action.Source),
                Is.All.EqualTo(ControllerNavigationSource.LeftStick)
            );
            Assert.That(
                navigation.Select(action => action.Repeat),
                Is.EqualTo(new[] { false, true, true })
            );
        }

        [Test]
        public void OmittedOverridesUseUnityStickProcessingAndUiRepeatTiming()
        {
            using BattlementTestHarness harness = Connect();

            Set(gamepad!.leftStick, new Vector2(0.15f, 0));
            harness.Runner.RunFrame();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(499));
            harness.Runner.RunFrame();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(1));
            harness.Runner.RunFrame();

            ActionBody.ControllerNavigate[] navigation = Actions(harness)
                .Select(action => action.Body)
                .OfType<ActionBody.ControllerNavigate>()
                .ToArray();
            Assert.That(
                navigation.Select(action => action.Repeat),
                Is.EqualTo(new[] { false, true })
            );
        }

        [Test]
        public void HeldInputAcrossTheGateDoesNotSuppressUnrelatedControls()
        {
            using BattlementTestHarness harness = Connect(inputDisabled: true);

            Set(gamepad!.leftStick, Vector2.right);
            Press(gamepad.dpad.up);
            harness.Runner.RunFrame();
            SetInputEnabled(harness, true);
            Press(gamepad.buttonSouth);
            harness.Runner.RunFrame();

            Assert.That(
                Actions(harness).Select(action => action.Body),
                Is.EqualTo(
                    new ActionBody[]
                    {
                        new ActionBody.ControllerButtonDown(
                            gamepad.deviceId,
                            ControllerButton.South
                        ),
                    }
                )
            );

            Release(gamepad.dpad.up);
            Set(gamepad.leftStick, Vector2.up);
            harness.Runner.RunFrame();
            Assert.That(
                Actions(harness).Last().Body,
                Is.EqualTo(
                    new ActionBody.ControllerNavigate(
                        gamepad.deviceId,
                        ControllerDirection.Up,
                        ControllerNavigationSource.LeftStick,
                        false
                    )
                )
            );
        }

        private BattlementTestHarness Connect(
            bool inputDisabled = false,
            ControllerInputSettings? settings = null
        )
        {
            BattlementTestHarness harness = BattlementTestHarness.Create();
            session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    inputDisabled: inputDisabled,
                    controllerInput: settings
                        ?? new ControllerInputSettings(new[] { ControllerButton.South })
                )
            );
            harness.Transport.DefaultSubmitResult = FakeBattlementTransport.ResponseResult(
                new Response(session, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.Connect();
            harness.Runner.RunFrame();
            return harness;
        }

        private void SetInputEnabled(BattlementTestHarness harness, bool enabled)
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
                                new CommandBody.Input.SetEnabled(enabled)
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

        private static Action[] Actions(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Select(TryDeserializeAction)
                .OfType<Action>()
                .ToArray();

        private static Action? TryDeserializeAction(byte[] bytes)
        {
            try
            {
                ClientMessage<CoreErrorCode, byte> message =
                    BattlementJson.DeserializeClientMessage<CoreErrorCode, byte>(bytes);
                return message is ClientMessage<CoreErrorCode, byte>.ActionMessage action
                    ? action.Action
                    : null;
            }
            catch (JsonSerializationException)
            {
                return null;
            }
        }
    }
}
