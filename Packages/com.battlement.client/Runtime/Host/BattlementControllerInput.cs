#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.Controls;

namespace Battlement
{
    /// <summary>Emits selected controller buttons and cardinal navigation actions.</summary>
    internal sealed class BattlementControllerInput : IDisposable
    {
        private static readonly IReadOnlyList<ControllerButton> Buttons = (ControllerButton[])
            Enum.GetValues(typeof(ControllerButton));

        private readonly Func<ControllerInputSettings?> settings;
        private readonly Func<(TimeSpan Delay, TimeSpan Interval)> defaultNavigationTiming;
        private readonly Func<ActionBody, bool> emit;
        private readonly HashSet<ControllerButton> held = new();
        private readonly HashSet<ControllerButton> suppressed = new();
        private Gamepad? active;
        private NavigationState? navigation;
        private NavigationState? suppressedDpad;
        private NavigationState? suppressedStick;
        private TimeSpan vibrationEnds;
        private bool needsSynchronization = true;

        public BattlementControllerInput(
            Func<ControllerInputSettings?> inputSettings,
            Func<(TimeSpan Delay, TimeSpan Interval)> unityNavigationTiming,
            Func<ActionBody, bool> emitAction
        )
        {
            settings = inputSettings;
            defaultNavigationTiming = unityNavigationTiming;
            emit = emitAction;
        }

        public void Update(bool isInputAvailable, TimeSpan now)
        {
            UpdateVibration(now);
            Gamepad? gamepad = Gamepad.current;
            ControllerInputSettings? current = settings();
            if (gamepad == null || current == null)
            {
                Reset();
                return;
            }

            if (!ReferenceEquals(active, gamepad))
            {
                Reset();
                active = gamepad;
            }
            if (!isInputAvailable || needsSynchronization)
            {
                Synchronize(gamepad, current);
                return;
            }

            EmitButtonTransitions(gamepad, current);
            if (current.NavigationEnabled)
            {
                EmitNavigation(gamepad, current, now);
            }
            else
            {
                navigation = null;
                suppressedDpad = null;
                suppressedStick = null;
            }
        }

        public IBattlementCommandOperation? Vibrate(
            CommandBody.Controller.Vibrate command,
            TimeSpan now
        )
        {
            Gamepad? gamepad = Gamepad.current;
            if (gamepad == null)
            {
                return null;
            }
            gamepad.SetMotorSpeeds((float)command.LowFrequency, (float)command.HighFrequency);
            vibrationEnds = now + command.Duration;
            return null;
        }

        public void Reset()
        {
            held.Clear();
            suppressed.Clear();
            navigation = null;
            suppressedDpad = null;
            suppressedStick = null;
            needsSynchronization = true;
        }

        public void StopHaptics()
        {
            active?.ResetHaptics();
            Gamepad.current?.ResetHaptics();
            vibrationEnds = TimeSpan.Zero;
        }

        public void Dispose() => StopHaptics();

        private void EmitButtonTransitions(Gamepad gamepad, ControllerInputSettings current)
        {
            foreach (ControllerButton button in Buttons)
            {
                bool isPressed = Control(gamepad, button).isPressed;
                if (suppressed.Contains(button))
                {
                    if (!isPressed)
                    {
                        suppressed.Remove(button);
                    }
                    continue;
                }

                bool wasPressed = held.Contains(button);
                if (wasPressed == isPressed)
                {
                    continue;
                }
                SetHeld(button, isPressed);
                if (!current.Buttons.Contains(button))
                {
                    continue;
                }
                ActionBody body = isPressed
                    ? new ActionBody.ControllerButtonDown(gamepad.deviceId, button)
                    : new ActionBody.ControllerButtonUp(gamepad.deviceId, button);
                if (!emit(body))
                {
                    Reset();
                    return;
                }
            }
        }

        private void EmitNavigation(Gamepad gamepad, ControllerInputSettings current, TimeSpan now)
        {
            NavigationState? dpad = ReadDpad(gamepad);
            NavigationState? stick = ReadStick(gamepad, current.StickDeadZone);
            suppressedDpad = RetainMatchingSuppression(suppressedDpad, dpad);
            suppressedStick = RetainMatchingSuppression(suppressedStick, stick);
            NavigationState? next = dpad ?? stick;
            if (next == null)
            {
                navigation = null;
                return;
            }
            if (IsSuppressed(next.Value))
            {
                navigation = null;
                return;
            }

            if (navigation == null || !navigation.Value.SameControl(next.Value))
            {
                (TimeSpan delay, _) = defaultNavigationTiming();
                navigation = next.Value.WithNextRepeat(now + (current.RepeatDelay ?? delay));
                EmitNavigationAction(gamepad, next.Value, false);
                return;
            }
            if (now < navigation.Value.NextRepeat)
            {
                return;
            }

            (_, TimeSpan interval) = defaultNavigationTiming();
            navigation = navigation.Value.WithNextRepeat(
                now + (current.RepeatInterval ?? interval)
            );
            EmitNavigationAction(gamepad, navigation.Value, true);
        }

        private void EmitNavigationAction(Gamepad gamepad, NavigationState state, bool repeat)
        {
            if (
                !emit(
                    new ActionBody.ControllerNavigate(
                        gamepad.deviceId,
                        state.Direction,
                        state.Source,
                        repeat
                    )
                )
            )
            {
                Reset();
            }
        }

        private void Synchronize(Gamepad gamepad, ControllerInputSettings current)
        {
            held.Clear();
            suppressed.Clear();
            foreach (ControllerButton button in Buttons)
            {
                if (Control(gamepad, button).isPressed)
                {
                    suppressed.Add(button);
                }
            }
            navigation = null;
            suppressedDpad = current.NavigationEnabled ? ReadDpad(gamepad) : null;
            suppressedStick = current.NavigationEnabled
                ? ReadStick(gamepad, current.StickDeadZone)
                : null;
            needsSynchronization = false;
        }

        private void UpdateVibration(TimeSpan now)
        {
            if (vibrationEnds != TimeSpan.Zero && now >= vibrationEnds)
            {
                StopHaptics();
            }
        }

        private static NavigationState? ReadDpad(Gamepad gamepad)
        {
            Vector2 dpad = gamepad.dpad.ReadValue();
            return dpad == Vector2.zero
                ? null
                : StateForVector(dpad, ControllerNavigationSource.Dpad);
        }

        private static NavigationState? ReadStick(Gamepad gamepad, double? deadZone)
        {
            Vector2 stick = deadZone.HasValue
                ? gamepad.leftStick.ReadUnprocessedValue()
                : gamepad.leftStick.ReadValue();
            double threshold = deadZone ?? 0;
            bool belowOverride = Mathf.Max(Mathf.Abs(stick.x), Mathf.Abs(stick.y)) < threshold;
            if (belowOverride || stick == Vector2.zero)
            {
                return null;
            }
            return StateForVector(stick, ControllerNavigationSource.LeftStick);
        }

        private bool IsSuppressed(NavigationState state) =>
            state.Source switch
            {
                ControllerNavigationSource.Dpad => suppressedDpad.HasValue,
                ControllerNavigationSource.LeftStick => suppressedStick.HasValue,
                _ => false,
            };

        private static NavigationState? RetainMatchingSuppression(
            NavigationState? suppressedState,
            NavigationState? currentState
        ) =>
            suppressedState.HasValue
            && currentState.HasValue
            && suppressedState.Value.SameControl(currentState.Value)
                ? suppressedState
                : null;

        private static NavigationState StateForVector(
            Vector2 value,
            ControllerNavigationSource source
        )
        {
            ControllerDirection direction;
            if (Mathf.Abs(value.x) > Mathf.Abs(value.y))
            {
                direction = value.x < 0 ? ControllerDirection.Left : ControllerDirection.Right;
            }
            else
            {
                direction = value.y < 0 ? ControllerDirection.Down : ControllerDirection.Up;
            }
            return new NavigationState(direction, source, TimeSpan.Zero);
        }

        private static ButtonControl Control(Gamepad gamepad, ControllerButton button) =>
            button switch
            {
                ControllerButton.South => gamepad.buttonSouth,
                ControllerButton.East => gamepad.buttonEast,
                ControllerButton.West => gamepad.buttonWest,
                ControllerButton.North => gamepad.buttonNorth,
                ControllerButton.LeftShoulder => gamepad.leftShoulder,
                ControllerButton.RightShoulder => gamepad.rightShoulder,
                ControllerButton.LeftStickButton => gamepad.leftStickButton,
                ControllerButton.RightStickButton => gamepad.rightStickButton,
                ControllerButton.Start => gamepad.startButton,
                ControllerButton.Select => gamepad.selectButton,
                _ => throw new ArgumentOutOfRangeException(nameof(button)),
            };

        private void SetHeld(ControllerButton button, bool isPressed)
        {
            if (isPressed)
            {
                held.Add(button);
            }
            else
            {
                held.Remove(button);
            }
        }

        private readonly struct NavigationState
        {
            public NavigationState(
                ControllerDirection direction,
                ControllerNavigationSource source,
                TimeSpan nextRepeat
            ) => (Direction, Source, NextRepeat) = (direction, source, nextRepeat);

            public ControllerDirection Direction { get; }

            public ControllerNavigationSource Source { get; }

            public TimeSpan NextRepeat { get; }

            public bool SameControl(NavigationState other) =>
                Direction == other.Direction && Source == other.Source;

            public NavigationState WithNextRepeat(TimeSpan value) => new(Direction, Source, value);
        }
    }
}
