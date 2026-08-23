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

        private readonly Func<PhysicalKey, bool> isEnabled;
        private readonly Func<ActionBody, bool> emit;
        private readonly HashSet<PhysicalKey> held = new();
        private readonly HashSet<PhysicalKey> suppressed = new();
        private bool needsSynchronization = true;

        public BattlementKeyboardInput(
            Func<PhysicalKey, bool> keyEnabled,
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

        private void SetHeld(PhysicalKey code, bool isPressed)
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
            var inputNames = new Dictionary<PhysicalKey, string>
            {
                [PhysicalKey.Equal] = nameof(Key.Equals),
                [PhysicalKey.BracketLeft] = nameof(Key.LeftBracket),
                [PhysicalKey.BracketRight] = nameof(Key.RightBracket),
                [PhysicalKey.ShiftLeft] = nameof(Key.LeftShift),
                [PhysicalKey.ShiftRight] = nameof(Key.RightShift),
                [PhysicalKey.ControlLeft] = nameof(Key.LeftCtrl),
                [PhysicalKey.ControlRight] = nameof(Key.RightCtrl),
                [PhysicalKey.AltLeft] = nameof(Key.LeftAlt),
                [PhysicalKey.AltRight] = nameof(Key.RightAlt),
                [PhysicalKey.MetaLeft] = nameof(Key.LeftMeta),
                [PhysicalKey.MetaRight] = nameof(Key.RightMeta),
                [PhysicalKey.ArrowLeft] = nameof(Key.LeftArrow),
                [PhysicalKey.ArrowRight] = nameof(Key.RightArrow),
                [PhysicalKey.ArrowUp] = nameof(Key.UpArrow),
                [PhysicalKey.ArrowDown] = nameof(Key.DownArrow),
                [PhysicalKey.NumpadDecimal] = nameof(Key.NumpadPeriod),
                [PhysicalKey.NumpadAdd] = nameof(Key.NumpadPlus),
                [PhysicalKey.NumpadSubtract] = nameof(Key.NumpadMinus),
            };
            var mappings = new List<KeyMapping>();
            foreach (PhysicalKey code in Enum.GetValues(typeof(PhysicalKey)))
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

        private static string InputName(PhysicalKey code)
        {
            string name = code.ToString();
            return name.StartsWith("Key", StringComparison.Ordinal) && name.Length == 4
                ? name.Substring(3)
                : name;
        }

        private readonly struct KeyMapping
        {
            public KeyMapping(PhysicalKey code, Key key) => (Code, Key) = (code, key);

            public PhysicalKey Code { get; }

            public Key Key { get; }
        }
    }
}
