#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;

namespace Masonry.Tests
{
    public sealed class MasonryKeyboardInputTests : InputTestFixture
    {
        private readonly HashSet<Key> pressed = new();
        private Keyboard? keyboard;

        public static IEnumerable<TestCaseData> SupportedMappings =>
            Enum.GetValues(typeof(KeyCode))
                .Cast<KeyCode>()
                .Select(code => new TestCaseData(code, InputKey(code)).SetName($"Physical_{code}"));

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            keyboard = InputSystem.AddDevice<Keyboard>("Masonry Test Keyboard");
        }

        [TearDown]
        public override void TearDown()
        {
            pressed.Clear();
            keyboard = null;
            base.TearDown();
        }

        [TestCaseSource(nameof(SupportedMappings))]
        public void EveryPhysicalCodeEmitsOneDownAndUpWithSessionIdentity(KeyCode code, Key key)
        {
            using MasonryTestHarness harness = Connect(code);
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
            using MasonryTestHarness harness = Connect(KeyCode.KeyA);

            Transition(harness, Key.A, true);
            harness.Runner.RunFrame();
            SetGlobalKeys(harness, KeyCode.KeyB);
            Transition(harness, Key.A, false);
            Transition(harness, Key.B, true);
            harness.Runner.RunFrame();
            SetGlobalKeys(harness, KeyCode.KeyA);
            Transition(harness, Key.B, false);
            Transition(harness, Key.A, true);

            Assert.That(
                Actions(harness).Select(action => action.Body),
                Is.EqualTo(
                    new ActionBody[]
                    {
                        new ActionBody.KeyDown(KeyCode.KeyA),
                        new ActionBody.KeyDown(KeyCode.KeyB),
                        new ActionBody.KeyDown(KeyCode.KeyA),
                    }
                )
            );
        }

        [Test]
        public void InputGateSuppressesTransitionsWithoutSynthesizingAKeyDown()
        {
            using MasonryTestHarness harness = Connect(KeyCode.KeyA, inputDisabled: true);

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
            using MasonryTestHarness harness = Connect(KeyCode.KeyA);
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
                        new ActionBody.KeyDown(KeyCode.KeyA),
                        new ActionBody.KeyDown(KeyCode.KeyA),
                        new ActionBody.KeyDown(KeyCode.KeyA),
                        new ActionBody.KeyDown(KeyCode.KeyA),
                    }
                )
            );
            Assert.That(actions.Take(3).All(action => action.SessionId == firstSession), Is.True);
            Assert.That(actions.Last().SessionId, Is.EqualTo(secondSession));
        }

        private SessionId harnessSession;

        private MasonryTestHarness Connect(KeyCode code, bool inputDisabled = false)
        {
            MasonryTestHarness harness = MasonryTestHarness.Create();
            harnessSession = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    harnessSession,
                    inputDisabled: inputDisabled,
                    globalKeys: new[] { code }
                )
            );
            harness.Transport.DefaultSubmitResult = FakeMasonryTransport.ResponseResult(
                new Response(harnessSession, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.Connect();
            harness.Runner.RunFrame();
            return harness;
        }

        private SessionId Reconnect(MasonryTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, globalKeys: new[] { KeyCode.KeyA })
            );
            harness.Runner.Reconnect();
            harness.Transport.DefaultSubmitResult = FakeMasonryTransport.ResponseResult(
                new Response(session, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.RunFrame();
            return session;
        }

        private void ReplaceSnapshot(MasonryTestHarness harness, SessionId session)
        {
            harness.Transport.EnqueuePoll(
                FakeMasonryTransport.SnapshotResponse(session, globalKeys: new[] { KeyCode.KeyA })
            );
            harness.Runner.RunFrame();
            harness.Runner.RunFrame();
        }

        private void Transition(MasonryTestHarness harness, Key key, bool isPressed)
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

        private void SetGlobalKeys(MasonryTestHarness harness, params KeyCode[] keys) =>
            SubmitCommand(harness, new CommandBody.Input.SetGlobalKeys(keys));

        private void SetInputEnabled(MasonryTestHarness harness, bool enabled) =>
            SubmitCommand(harness, new CommandBody.Input.SetEnabled(enabled));

        private void SubmitCommand(MasonryTestHarness harness, CommandBody body)
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
                FakeMasonryTransport.ResponseResult(
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

        private static void SendFocus(MasonryTestHarness harness, bool focused)
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

        private static Action[] Actions(MasonryTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Select(TryDeserializeAction)
                .OfType<Action>()
                .ToArray();

        private static Action? TryDeserializeAction(byte[] bytes)
        {
            try
            {
                ClientMessage<CoreErrorCode, byte> message =
                    MasonryMessagePack.DeserializeClientMessage(
                        bytes,
                        new CoreErrorFormatter(),
                        new UnusedPayloadFormatter()
                    );
                return message is ClientMessage<CoreErrorCode, byte>.ActionMessage action
                    ? action.Action
                    : null;
            }
            catch (MessagePackSerializationException)
            {
                return null;
            }
        }

        private static Key InputKey(KeyCode code)
        {
            string name = code switch
            {
                KeyCode.Equal => nameof(Key.Equals),
                KeyCode.BracketLeft => nameof(Key.LeftBracket),
                KeyCode.BracketRight => nameof(Key.RightBracket),
                KeyCode.ShiftLeft => nameof(Key.LeftShift),
                KeyCode.ShiftRight => nameof(Key.RightShift),
                KeyCode.ControlLeft => nameof(Key.LeftCtrl),
                KeyCode.ControlRight => nameof(Key.RightCtrl),
                KeyCode.AltLeft => nameof(Key.LeftAlt),
                KeyCode.AltRight => nameof(Key.RightAlt),
                KeyCode.MetaLeft => nameof(Key.LeftMeta),
                KeyCode.MetaRight => nameof(Key.RightMeta),
                KeyCode.ArrowLeft => nameof(Key.LeftArrow),
                KeyCode.ArrowRight => nameof(Key.RightArrow),
                KeyCode.ArrowUp => nameof(Key.UpArrow),
                KeyCode.ArrowDown => nameof(Key.DownArrow),
                KeyCode.NumpadDecimal => nameof(Key.NumpadPeriod),
                KeyCode.NumpadAdd => nameof(Key.NumpadPlus),
                KeyCode.NumpadSubtract => nameof(Key.NumpadMinus),
                _ => InputName(code),
            };
            return Enum.Parse<Key>(name);
        }

        private static string InputName(KeyCode code)
        {
            string name = code.ToString();
            return name.StartsWith("Key", StringComparison.Ordinal) && name.Length == 4
                ? name.Substring(3)
                : name;
        }

        private sealed class CoreErrorFormatter : IMessagePackFormatter<CoreErrorCode>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                CoreErrorCode value,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();

            public CoreErrorCode Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }

        private sealed class UnusedPayloadFormatter : IMessagePackFormatter<byte>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                byte value,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();

            public byte Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }
    }
}
