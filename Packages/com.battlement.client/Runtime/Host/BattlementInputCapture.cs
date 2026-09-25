#nullable enable

using System;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.Controls;

namespace Battlement
{
    /// <summary>Owns physical capture before keyboard, controller, and UI routing.</summary>
    internal sealed class BattlementInputCapture
    {
        private readonly Action<InputCaptureEvent> publish;
        private InputCaptureRequest? request;
        private InputDevice? device;
        private bool armed;
        private bool terminal;
        private bool quarantine;
        private int neutralUpdates;

        public BattlementInputCapture(Action<InputCaptureEvent> publishResult) =>
            publish = publishResult;

        public bool BlocksInput => request != null || quarantine;

        public void Execute(InputCaptureCommand command)
        {
            switch (command)
            {
                case InputCaptureCommand.Begin begin:
                    InputCaptureRequest? previous = terminal ? null : request;
                    request = begin.Request;
                    device =
                        request.Device == InputCaptureDevice.Keyboard
                            ? Keyboard.current
                            : Gamepad.current;
                    armed = IsNeutral();
                    terminal = false;
                    quarantine = true;
                    neutralUpdates = 0;
                    if (previous != null)
                        publish(
                            new InputCaptureEvent(
                                previous.Id,
                                new InputCaptureResult.Cancelled(
                                    InputCaptureCancellation.Superseded
                                )
                            )
                        );
                    break;
                case InputCaptureCommand.End end:
                    if (request?.Id != end.Id)
                        break;
                    request = null;
                    device = null;
                    quarantine = true;
                    neutralUpdates = 0;
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(command));
            }
        }

        public void Update(bool inputAvailable, bool observePhysicalInput = true)
        {
            if (request == null)
            {
                if (!quarantine)
                    return;
                // Retain the fence through a neutral frame so late UI key-up/navigation
                // events cannot activate the menu after a synchronous capture callback.
                bool neutral = !observePhysicalInput || IsNeutral();
                neutralUpdates = neutral ? Math.Min(neutralUpdates + 1, 2) : 0;
                if (neutralUpdates >= 2)
                    quarantine = false;
                return;
            }
            if (terminal)
                return;
            if (!inputAvailable)
            {
                CancelForFocusLoss();
                return;
            }
            if (!observePhysicalInput)
                return;
            if (device == null || !device.added)
            {
                Complete(
                    new InputCaptureResult.Cancelled(InputCaptureCancellation.DeviceDisconnected)
                );
                return;
            }
            if (!armed)
            {
                armed = IsNeutral();
                return;
            }
            if (device is Keyboard keyboard)
                CaptureKeyboard(keyboard);
            else if (device is Gamepad gamepad)
                CaptureController(gamepad);
        }

        public void CancelForFocusLoss() =>
            Complete(new InputCaptureResult.Cancelled(InputCaptureCancellation.FocusLost));

        public void Reset()
        {
            request = null;
            device = null;
            terminal = false;
            armed = false;
            quarantine = false;
            neutralUpdates = 0;
        }

        private void CaptureKeyboard(Keyboard keyboard)
        {
            KeyControl[] pressed = keyboard.allKeys.Where(key => key.isPressed).ToArray();
            if (pressed.Length == 0)
                return;
            if (pressed.Length != 1 || IsModifier(pressed[0].keyCode))
            {
                armed = false;
                return;
            }
            if (BattlementKeyboardInput.TryPhysical(pressed[0].keyCode, out PhysicalKey physical))
                Complete(new InputCaptureResult.Key(keyboard.deviceId, physical));
        }

        private void CaptureController(Gamepad gamepad)
        {
            ControllerButton[] buttons = Enum.GetValues(typeof(ControllerButton))
                .Cast<ControllerButton>()
                .Where(button => BattlementControllerInput.Control(gamepad, button).isPressed)
                .ToArray();
            Vector2 dpad = gamepad.dpad.ReadValue();
            if (buttons.Length > 1 || (buttons.Length == 1 && dpad != Vector2.zero))
            {
                armed = false;
                return;
            }
            if (buttons.Length == 1)
            {
                Complete(new InputCaptureResult.Button(gamepad.deviceId, buttons[0]));
                return;
            }
            if (dpad == Vector2.zero)
                return;
            ControllerDirection direction =
                Mathf.Abs(dpad.x) > Mathf.Abs(dpad.y)
                    ? (dpad.x < 0 ? ControllerDirection.Left : ControllerDirection.Right)
                    : (dpad.y < 0 ? ControllerDirection.Down : ControllerDirection.Up);
            Complete(
                new InputCaptureResult.Direction(
                    gamepad.deviceId,
                    direction,
                    ControllerNavigationSource.Dpad,
                    false
                )
            );
        }

        private void Complete(InputCaptureResult result)
        {
            if (request == null || terminal)
                return;
            terminal = true;
            publish(new InputCaptureEvent(request.Id, result));
        }

        private static bool IsNeutral()
        {
            bool keyboardReleased = InputSystem
                .devices.OfType<Keyboard>()
                .All(keyboard => !keyboard.anyKey.isPressed);
            return keyboardReleased && Gamepad.all.All(GamepadNeutral);
        }

        private static bool GamepadNeutral(Gamepad gamepad)
        {
            bool buttonsReleased = !Enum.GetValues(typeof(ControllerButton))
                .Cast<ControllerButton>()
                .Any(button => BattlementControllerInput.Control(gamepad, button).isPressed);
            bool directionsReleased =
                gamepad.dpad.ReadValue() == Vector2.zero
                && gamepad.leftStick.ReadValue() == Vector2.zero;
            return buttonsReleased && directionsReleased;
        }

        private static bool IsModifier(Key key) =>
            key
                is Key.LeftShift
                    or Key.RightShift
                    or Key.LeftCtrl
                    or Key.RightCtrl
                    or Key.LeftAlt
                    or Key.RightAlt
                    or Key.LeftMeta
                    or Key.RightMeta;
    }
}
