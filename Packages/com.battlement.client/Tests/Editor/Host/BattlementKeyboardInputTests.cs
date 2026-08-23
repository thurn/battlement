#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json;
using NUnit.Framework;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;

namespace Battlement.Tests
{
    public sealed class BattlementKeyboardInputTests : InputTestFixture
    {
        private readonly HashSet<Key> pressed = new();
        private Keyboard? keyboard;

        public static IEnumerable<TestCaseData> SupportedMappings =>
            Enum.GetValues(typeof(PhysicalKey))
                .Cast<PhysicalKey>()
                .Select(code => new TestCaseData(code, InputKey(code)).SetName($"Physical_{code}"));

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            keyboard = InputSystem.AddDevice<Keyboard>("Battlement Test Keyboard");
        }

        [TearDown]
        public override void TearDown()
        {
            pressed.Clear();
            keyboard = null;
            base.TearDown();
        }

        [TestCaseSource(nameof(SupportedMappings))]
        public void EveryPhysicalCodeEmitsOneDownAndUpWithSessionIdentity(PhysicalKey code, Key key)
        {
            using BattlementTestHarness harness = Connect(code);
            SessionId session = harnessSession;

            Transition(harness, key, true);
            harness.Runner.RunFrame();
            Transition(harness, key, false);

            Action[] actions = Actions(harness);
            Assert.That(
                actions.Select(action => action.Body),
                Is.EqualTo(
                    new ActionBody[] { new ActionBody.KeyDown(code), new ActionBody.KeyUp(code) }
                )
            );
            Assert.That(actions.All(action => action.SessionId == session), Is.True);
            Assert.That(actions.Select(action => action.Id).Distinct().Count(), Is.EqualTo(2));
        }

        [Test]
        public void KeySelectionChangesDoNotCreateTransitionsOrRepeatHeldKeys()
        {
            using BattlementTestHarness harness = Connect(PhysicalKey.KeyA);

            Transition(harness, Key.A, true);
            harness.Runner.RunFrame();
            SetGlobalKeys(harness, PhysicalKey.KeyB);
            Transition(harness, Key.A, false);
            Transition(harness, Key.B, true);
            harness.Runner.RunFrame();
            SetGlobalKeys(harness, PhysicalKey.KeyA);
            Transition(harness, Key.B, false);
            Transition(harness, Key.A, true);

            Assert.That(
                Actions(harness).Select(action => action.Body),
                Is.EqualTo(
                    new ActionBody[]
                    {
                        new ActionBody.KeyDown(PhysicalKey.KeyA),
                        new ActionBody.KeyDown(PhysicalKey.KeyB),
                        new ActionBody.KeyDown(PhysicalKey.KeyA),
                    }
                )
            );
        }

        [Test]
        public void InputGateSuppressesTransitionsWithoutSynthesizingAKeyDown()
        {
            using BattlementTestHarness harness = Connect(PhysicalKey.KeyA, inputDisabled: true);

            Transition(harness, Key.A, true);
            SetInputEnabled(harness, true);
            harness.Runner.RunFrame();
            Transition(harness, Key.A, false);
            SetInputEnabled(harness, false);
            Transition(harness, Key.A, true);
            Transition(harness, Key.A, false);

            Assert.That(Actions(harness), Is.Empty);
        }

        [Test]
        public void FocusSnapshotAndReconnectClearHeldTrackingWithoutSyntheticUp()
        {
            using BattlementTestHarness harness = Connect(PhysicalKey.KeyA);
            SessionId firstSession = harnessSession;

            Transition(harness, Key.A, true);
            SendFocus(harness, false);
            Transition(harness, Key.A, false);
            SendFocus(harness, true);
            Transition(harness, Key.A, true);
            ReplaceSnapshot(harness, firstSession);
            Transition(harness, Key.A, false);
            Transition(harness, Key.A, true);
            SessionId secondSession = Reconnect(harness);
            Transition(harness, Key.A, false);
            Transition(harness, Key.A, true);

            Action[] actions = Actions(harness);
            Assert.That(
                actions.Select(action => action.Body),
                Is.EqualTo(
                    new ActionBody[]
                    {
                        new ActionBody.KeyDown(PhysicalKey.KeyA),
                        new ActionBody.KeyDown(PhysicalKey.KeyA),
                        new ActionBody.KeyDown(PhysicalKey.KeyA),
                        new ActionBody.KeyDown(PhysicalKey.KeyA),
                    }
                )
            );
            Assert.That(actions.Take(3).All(action => action.SessionId == firstSession), Is.True);
            Assert.That(actions.Last().SessionId, Is.EqualTo(secondSession));
        }

        private SessionId harnessSession;

        private BattlementTestHarness Connect(PhysicalKey code, bool inputDisabled = false)
        {
            BattlementTestHarness harness = BattlementTestHarness.Create();
            harnessSession = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    harnessSession,
                    inputDisabled: inputDisabled,
                    globalKeys: new[] { code }
                )
            );
            harness.Transport.DefaultSubmitResult = FakeBattlementTransport.ResponseResult(
                new Response(harnessSession, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.Connect();
            harness.Runner.RunFrame();
            return harness;
        }

        private SessionId Reconnect(BattlementTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    globalKeys: new[] { PhysicalKey.KeyA }
                )
            );
            harness.Runner.Reconnect();
            harness.Transport.DefaultSubmitResult = FakeBattlementTransport.ResponseResult(
                new Response(session, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.RunFrame();
            return session;
        }

        private void ReplaceSnapshot(BattlementTestHarness harness, SessionId session)
        {
            harness.Transport.EnqueuePoll(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    globalKeys: new[] { PhysicalKey.KeyA }
                )
            );
            harness.Runner.RunFrame();
            harness.Runner.RunFrame();
        }

        private void Transition(BattlementTestHarness harness, Key key, bool isPressed)
        {
            if (isPressed)
            {
                pressed.Add(key);
            }
            else
            {
                pressed.Remove(key);
            }

            InputSystem.QueueStateEvent(keyboard!, new KeyboardState(pressed.ToArray()));
            InputSystem.Update();
            harness.Runner.RunFrame();
        }

        private void SetGlobalKeys(BattlementTestHarness harness, params PhysicalKey[] keys) =>
            SubmitCommand(harness, new CommandBody.Input.SetGlobalKeys(keys));

        private void SetInputEnabled(BattlementTestHarness harness, bool enabled) =>
            SubmitCommand(harness, new CommandBody.Input.SetEnabled(enabled));

        private void SubmitCommand(BattlementTestHarness harness, CommandBody body)
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                harnessSession,
                new[]
                {
                    new ParallelCommandGroup<Command>(
                        new[] { new Command(new CommandId(Guid.NewGuid()), body) }
                    ),
                },
                Start: BatchStart.Now
            );
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        harnessSession,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.BatchMessage(batch),
                        }
                    )
                )
            );
            harness.Runner.Submit(new byte[] { 1 });
            harness.Runner.RunFrame();
        }

        private static void SendFocus(BattlementTestHarness harness, bool focused)
        {
            LogAssert.ignoreFailingMessages = true;
            try
            {
                harness.Runner.SendMessage("OnApplicationFocus", focused);
            }
            finally
            {
                LogAssert.ignoreFailingMessages = false;
            }
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

        private static Key InputKey(PhysicalKey code)
        {
            string name = code switch
            {
                PhysicalKey.Equal => nameof(Key.Equals),
                PhysicalKey.BracketLeft => nameof(Key.LeftBracket),
                PhysicalKey.BracketRight => nameof(Key.RightBracket),
                PhysicalKey.ShiftLeft => nameof(Key.LeftShift),
                PhysicalKey.ShiftRight => nameof(Key.RightShift),
                PhysicalKey.ControlLeft => nameof(Key.LeftCtrl),
                PhysicalKey.ControlRight => nameof(Key.RightCtrl),
                PhysicalKey.AltLeft => nameof(Key.LeftAlt),
                PhysicalKey.AltRight => nameof(Key.RightAlt),
                PhysicalKey.MetaLeft => nameof(Key.LeftMeta),
                PhysicalKey.MetaRight => nameof(Key.RightMeta),
                PhysicalKey.ArrowLeft => nameof(Key.LeftArrow),
                PhysicalKey.ArrowRight => nameof(Key.RightArrow),
                PhysicalKey.ArrowUp => nameof(Key.UpArrow),
                PhysicalKey.ArrowDown => nameof(Key.DownArrow),
                PhysicalKey.NumpadDecimal => nameof(Key.NumpadPeriod),
                PhysicalKey.NumpadAdd => nameof(Key.NumpadPlus),
                PhysicalKey.NumpadSubtract => nameof(Key.NumpadMinus),
                _ => InputName(code),
            };
            return Enum.Parse<Key>(name);
        }

        private static string InputName(PhysicalKey code)
        {
            string name = code.ToString();
            return name.StartsWith("Key", StringComparison.Ordinal) && name.Length == 4
                ? name.Substring(3)
                : name;
        }
    }
}
