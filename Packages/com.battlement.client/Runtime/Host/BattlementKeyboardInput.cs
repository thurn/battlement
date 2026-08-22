#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.InputSystem;

namespace Battlement
{
    /// <summary>Tracks physical keyboard transitions and emits enabled core actions.</summary>
    internal sealed class BattlementKeyboardInput
    {
        private static readonly IReadOnlyList<KeyMapping> Mappings = CreateMappings();

        private readonly Func<KeyCode, bool> isEnabled;
        private readonly Func<ActionBody, bool> emit;
        private readonly HashSet<KeyCode> held = new();
        private readonly HashSet<KeyCode> suppressed = new();
        private bool needsSynchronization = true;

        public BattlementKeyboardInput(
            Func<KeyCode, bool> keyEnabled,
            Func<ActionBody, bool> emitAction
        )
        {
            isEnabled = keyEnabled;
            emit = emitAction;
        }

        public void Update(bool isInputAvailable)
        {
            Keyboard? keyboard = Keyboard.current;
            if (keyboard == null)
            {
                Reset();
                return;
            }

            if (!isInputAvailable || needsSynchronization)
            {
                Synchronize(keyboard);
                return;
            }

            foreach (KeyMapping mapping in Mappings)
            {
                bool isPressed = keyboard[mapping.Key].isPressed;
                if (suppressed.Contains(mapping.Code))
                {
                    if (!isPressed)
                    {
                        suppressed.Remove(mapping.Code);
                    }
                    continue;
                }

                bool wasPressed = held.Contains(mapping.Code);
                if (wasPressed == isPressed)
                {
                    continue;
                }

                SetHeld(mapping.Code, isPressed);
                if (!isEnabled(mapping.Code))
                {
                    continue;
                }

                if (
                    !emit(
                        isPressed
                            ? new ActionBody.KeyDown(mapping.Code)
                            : new ActionBody.KeyUp(mapping.Code)
                    )
                )
                {
                    Reset();
                    return;
                }
            }
        }

        public void Reset()
        {
            held.Clear();
            suppressed.Clear();
            needsSynchronization = true;
        }

        private void Synchronize(Keyboard keyboard)
        {
            held.Clear();
            suppressed.Clear();
            foreach (KeyMapping mapping in Mappings)
            {
                if (keyboard[mapping.Key].isPressed)
                {
                    suppressed.Add(mapping.Code);
                }
            }

            needsSynchronization = false;
        }

        private void SetHeld(KeyCode code, bool isPressed)
        {
            if (isPressed)
            {
                held.Add(code);
            }
            else
            {
                held.Remove(code);
            }
        }

        private static IReadOnlyList<KeyMapping> CreateMappings()
        {
            var inputNames = new Dictionary<KeyCode, string>
            {
                [KeyCode.Equal] = nameof(Key.Equals),
                [KeyCode.BracketLeft] = nameof(Key.LeftBracket),
                [KeyCode.BracketRight] = nameof(Key.RightBracket),
                [KeyCode.ShiftLeft] = nameof(Key.LeftShift),
                [KeyCode.ShiftRight] = nameof(Key.RightShift),
                [KeyCode.ControlLeft] = nameof(Key.LeftCtrl),
                [KeyCode.ControlRight] = nameof(Key.RightCtrl),
                [KeyCode.AltLeft] = nameof(Key.LeftAlt),
                [KeyCode.AltRight] = nameof(Key.RightAlt),
                [KeyCode.MetaLeft] = nameof(Key.LeftMeta),
                [KeyCode.MetaRight] = nameof(Key.RightMeta),
                [KeyCode.ArrowLeft] = nameof(Key.LeftArrow),
                [KeyCode.ArrowRight] = nameof(Key.RightArrow),
                [KeyCode.ArrowUp] = nameof(Key.UpArrow),
                [KeyCode.ArrowDown] = nameof(Key.DownArrow),
                [KeyCode.NumpadDecimal] = nameof(Key.NumpadPeriod),
                [KeyCode.NumpadAdd] = nameof(Key.NumpadPlus),
                [KeyCode.NumpadSubtract] = nameof(Key.NumpadMinus),
            };
            var mappings = new List<KeyMapping>();
            foreach (KeyCode code in Enum.GetValues(typeof(KeyCode)))
            {
                string inputName = inputNames.TryGetValue(code, out string value)
                    ? value
                    : InputName(code);
                if (!Enum.TryParse(inputName, out Key key) || key == Key.None)
                {
                    throw new InvalidOperationException($"Key {code} has no Input System mapping.");
                }

                mappings.Add(new KeyMapping(code, key));
            }

            return mappings;
        }

        private static string InputName(KeyCode code)
        {
            string name = code.ToString();
            return name.StartsWith("Key", StringComparison.Ordinal) && name.Length == 4
                ? name.Substring(3)
                : name;
        }

        private readonly struct KeyMapping
        {
            public KeyMapping(KeyCode code, Key key) => (Code, Key) = (code, key);

            public KeyCode Code { get; }

            public Key Key { get; }
        }
    }
}
