#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using InputTouchPhase = UnityEngine.InputSystem.TouchPhase;

namespace Battlement.Tests
{
    public sealed class DittoVirtualInputTests : InputTestFixture
    {
        [Test]
        public void DesktopSequencesExposeExactFramesWithoutChangingHostDevices()
        {
            Mouse hostMouse = InputSystem.AddDevice<Mouse>("Host Fixture Mouse");
            Keyboard hostKeyboard = InputSystem.AddDevice<Keyboard>("Host Fixture Keyboard");
            InputSystem.QueueStateEvent(
                hostMouse,
                new MouseState { position = new Vector2(7, 9) }.WithButton(MouseButton.Right, true)
            );
            InputSystem.QueueStateEvent(hostKeyboard, new KeyboardState(Key.Space));
            InputSystem.Update();
            using var input = new DittoVirtualInput(DittoPlatform.Macos, 101, 101);
            var journal = new List<DesktopFrame>();

            Assert.That(Mouse.current, Is.Not.SameAs(hostMouse));
            Assert.That(Keyboard.current, Is.Not.SameAs(hostKeyboard));

            input.Click(new Vector2(25, 30));
            DrainDesktop(input, hostMouse, hostKeyboard, journal);
            int segments = input.Drag(new Vector2(10, 20), new Vector2(20, 20));
            DrainDesktop(input, hostMouse, hostKeyboard, journal);
            input.Key("Enter", DittoKeyAction.Tap);
            DrainDesktop(input, hostMouse, hostKeyboard, journal);

            Assert.That(segments, Is.EqualTo(2));
            Assert.That(
                journal.Select(frame => frame.Kind),
                Is.EqualTo(
                    new[]
                    {
                        DittoInputFrameKind.Move,
                        DittoInputFrameKind.Press,
                        DittoInputFrameKind.Release,
                        DittoInputFrameKind.Move,
                        DittoInputFrameKind.Press,
                        DittoInputFrameKind.Move,
                        DittoInputFrameKind.Move,
                        DittoInputFrameKind.Release,
                        DittoInputFrameKind.KeyDown,
                        DittoInputFrameKind.KeyUp,
                    }
                )
            );
            Assert.That(
                journal.Take(3).Select(frame => frame.Position),
                Is.EqualTo(Enumerable.Repeat(new Vector2(25, 70), 3))
            );
            Assert.That(
                journal.Skip(3).Take(5).Select(frame => frame.Position),
                Is.EqualTo(
                    new[]
                    {
                        new Vector2(10, 80),
                        new Vector2(10, 80),
                        new Vector2(15, 80),
                        new Vector2(20, 80),
                        new Vector2(20, 80),
                    }
                )
            );
            Assert.That(
                journal.Select(frame => frame.LeftPressed),
                Is.EqualTo(
                    new[] { false, true, false, false, true, true, true, false, false, false }
                )
            );
            Assert.That(journal[^2].EnterPressed, Is.True);
            Assert.That(journal[^1].EnterPressed, Is.False);
            Assert.That(journal.All(frame => frame.HostPosition == new Vector2(7, 9)), Is.True);
            Assert.That(journal.All(frame => frame.HostRightPressed), Is.True);
            Assert.That(journal.All(frame => frame.HostSpacePressed), Is.True);
            Assert.That(input.HasHeldInput, Is.False);
            TestContext.Progress.WriteLine(string.Join(Environment.NewLine, journal));
        }

        [Test]
        public void IosUsesOneFingerSequencesAndRejectsHover()
        {
            using var input = new DittoVirtualInput(DittoPlatform.IosSimulator, 101, 201);
            var journal = new List<TouchFrame>();

            Assert.That(input.Hover(new Vector2(1, 2)), Is.False);
            Assert.That(input.PendingFrameCount, Is.Zero);
            input.Click(new Vector2(20, 40));
            DrainTouch(input, journal);
            int segments = input.Drag(new Vector2(20, 40), new Vector2(20, 60));
            DrainTouch(input, journal);

            Assert.That(segments, Is.EqualTo(2));
            Assert.That(
                journal.Select(frame => frame.Kind),
                Is.EqualTo(
                    new[]
                    {
                        DittoInputFrameKind.TouchBegin,
                        DittoInputFrameKind.TouchEnd,
                        DittoInputFrameKind.TouchBegin,
                        DittoInputFrameKind.TouchMove,
                        DittoInputFrameKind.TouchMove,
                        DittoInputFrameKind.TouchEnd,
                    }
                )
            );
            Assert.That(
                journal.Select(frame => frame.Phase),
                Is.EqualTo(
                    new[]
                    {
                        InputTouchPhase.Began,
                        InputTouchPhase.Ended,
                        InputTouchPhase.Began,
                        InputTouchPhase.Moved,
                        InputTouchPhase.Moved,
                        InputTouchPhase.Ended,
                    }
                )
            );
            Assert.That(journal[0].Position, Is.EqualTo(new Vector2(20, 160)));
            Assert.That(journal[^1].Position, Is.EqualTo(new Vector2(20, 140)));
            Assert.That(input.HasHeldInput, Is.False);
        }

        [Test]
        public void DesktopPointerReaderPrefersTheDittoMouseOverHostInput()
        {
            Mouse hostMouse = InputSystem.AddDevice<Mouse>("Host Fixture Mouse");
            InputSystem.QueueStateEvent(hostMouse, new MouseState { position = new Vector2(7, 9) });
            InputSystem.Update();
            using var input = new DittoVirtualInput(DittoPlatform.Macos, 101, 101);
            input.Click(new Vector2(25, 30));
            Advance(input);
            hostMouse.MakeCurrent();

            BattlementPointerSample sample = BattlementPointerDevices.Read(Array.Empty<int>())[0];

            Assert.That(sample.Position, Is.EqualTo(new Vector2(25, 70)));
        }

        [Test]
        public void InterruptedPointerAndKeySequencesReportHeldInput()
        {
            using var pointer = new DittoVirtualInput(DittoPlatform.Macos, 100, 100);
            pointer.Click(new Vector2(50, 50));
            Advance(pointer);
            Advance(pointer);

            Assert.That(pointer.HeldInputDiagnostic(), Does.Contain("pointer button"));

            using var keyboard = new DittoVirtualInput(DittoPlatform.Macos, 100, 100);
            keyboard.Key("A", DittoKeyAction.Down);
            Advance(keyboard);

            Assert.That(keyboard.HeldInputDiagnostic(), Does.Contain("A"));
        }

        [Test]
        public void FirstAuthoredKeyTapReachesBattlementKeyboardInput()
        {
            var actions = new List<ActionBody>();
            Keyboard hostKeyboard = InputSystem.AddDevice<Keyboard>("Host Fixture Keyboard");
            InputSystem.QueueStateEvent(hostKeyboard, new KeyboardState());
            InputSystem.Update();
            using var input = new DittoVirtualInput(DittoPlatform.Macos, 100, 100);
            var keyboard = new BattlementKeyboardInput(
                _ => true,
                action =>
                {
                    actions.Add(action);
                    return true;
                }
            );
            keyboard.Update(true);

            input.Key("Enter", DittoKeyAction.Tap);
            Advance(input);
            keyboard.Update(true);
            Advance(input);
            keyboard.Update(true);

            Assert.That(
                actions,
                Is.EqualTo(
                    new ActionBody[]
                    {
                        new ActionBody.KeyDown(PhysicalKey.Enter),
                        new ActionBody.KeyUp(PhysicalKey.Enter),
                    }
                )
            );
        }

        [Test]
        public void PrintableKeyTapAlsoQueuesTextInput()
        {
            using var input = new DittoVirtualInput(DittoPlatform.Macos, 100, 100);
            Keyboard virtualKeyboard = Keyboard.current;
            InputSystem.AddDevice<Keyboard>("Host Fixture Keyboard").MakeCurrent();
            var text = new List<char>();
            virtualKeyboard.onTextInput += text.Add;

            input.Key("A", DittoKeyAction.Tap);
            Advance(input);
            Advance(input);

            Assert.That(text, Is.EqualTo(new[] { 'a' }));
        }

        [Test]
        public void PlayerResetRejectsHeldAuthoredInput()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            using var input = new DittoVirtualInput(DittoPlatform.Macos, 100, 100);
            input.Key("A", DittoKeyAction.Down);
            Advance(input);
            var reset = new DittoPlayerStateReset(
                harness.Runner,
                null,
                () => harness.Clock.Elapsed,
                input
            );

            reset.Begin();

            Assert.That(reset.IsComplete, Is.True);
            Assert.That(reset.IsReusable, Is.False);
            Assert.That(reset.Failure!.Stage, Is.EqualTo(DittoBoundaryStage.Reset));
            Assert.That(reset.Failure.Diagnostic, Does.Contain("A"));
        }

        private static void DrainDesktop(
            DittoVirtualInput input,
            Mouse hostMouse,
            Keyboard hostKeyboard,
            ICollection<DesktopFrame> journal
        )
        {
            while (input.PendingFrameCount > 0)
            {
                DittoInputFrame frame = input.QueueNextFrame();
                InputSystem.Update();
                Mouse virtualMouse = InputSystem
                    .devices.OfType<Mouse>()
                    .Single(value => value != hostMouse);
                Keyboard virtualKeyboard = InputSystem
                    .devices.OfType<Keyboard>()
                    .SingleOrDefault(value => value != hostKeyboard)!;
                journal.Add(
                    new DesktopFrame(
                        frame.Kind,
                        virtualMouse?.position.ReadValue() ?? default,
                        virtualMouse?.leftButton.isPressed ?? false,
                        virtualKeyboard?.enterKey.isPressed ?? false,
                        hostMouse.position.ReadValue(),
                        hostMouse.rightButton.isPressed,
                        hostKeyboard.spaceKey.isPressed
                    )
                );
            }
        }

        private static void DrainTouch(DittoVirtualInput input, ICollection<TouchFrame> journal)
        {
            while (input.PendingFrameCount > 0)
            {
                DittoInputFrame frame = input.QueueNextFrame();
                InputSystem.Update();
                Touchscreen screen = InputSystem.devices.OfType<Touchscreen>().Single();
                journal.Add(
                    new TouchFrame(
                        frame.Kind,
                        screen.primaryTouch.phase.ReadValue(),
                        screen.primaryTouch.position.ReadValue()
                    )
                );
            }
        }

        private static void Advance(DittoVirtualInput input)
        {
            input.QueueNextFrame();
            InputSystem.Update();
        }

        private sealed record DesktopFrame(
            DittoInputFrameKind Kind,
            Vector2 Position,
            bool LeftPressed,
            bool EnterPressed,
            Vector2 HostPosition,
            bool HostRightPressed,
            bool HostSpacePressed
        );

        private sealed record TouchFrame(
            DittoInputFrameKind Kind,
            InputTouchPhase Phase,
            Vector2 Position
        );
    }
}
