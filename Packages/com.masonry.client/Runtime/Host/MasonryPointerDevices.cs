#nullable enable

using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.Controls;
using InputTouchPhase = UnityEngine.InputSystem.TouchPhase;

namespace Masonry
{
    internal readonly struct MasonryPointerSample
    {
        public MasonryPointerSample(
            Vector2 position,
            HashSet<PointerButton> buttons,
            bool isPresent,
            bool isCancelled
        ) =>
            (Position, Buttons, IsPresent, IsCancelled) = (
                position,
                buttons,
                isPresent,
                isCancelled
            );

        public Vector2 Position { get; }

        public HashSet<PointerButton> Buttons { get; }

        public bool IsPresent { get; }

        public bool IsCancelled { get; }

        public static MasonryPointerSample Absent(Vector2 position) =>
            new(position, new HashSet<PointerButton>(), false, false);
    }

    internal static class MasonryPointerDevices
    {
        public static SortedDictionary<int, MasonryPointerSample> Read(
            IReadOnlyCollection<int> knownPointerIds
        )
        {
            var samples = new SortedDictionary<int, MasonryPointerSample>();
            if (Mouse.current is Mouse mouse)
            {
                var buttons = new HashSet<PointerButton>();
                AddPressed(buttons, PointerButton.Left, mouse.leftButton.isPressed);
                AddPressed(buttons, PointerButton.Middle, mouse.middleButton.isPressed);
                AddPressed(buttons, PointerButton.Right, mouse.rightButton.isPressed);
                samples[0] = new MasonryPointerSample(
                    mouse.position.ReadValue(),
                    buttons,
                    true,
                    false
                );
            }

            foreach (Touchscreen touchscreen in InputSystem.devices.OfType<Touchscreen>())
            {
                AddTouches(samples, knownPointerIds, touchscreen);
            }

            return samples;
        }

        private static void AddTouches(
            IDictionary<int, MasonryPointerSample> samples,
            IReadOnlyCollection<int> knownPointerIds,
            Touchscreen touchscreen
        )
        {
            foreach (TouchControl touch in touchscreen.touches)
            {
                int id = touch.touchId.ReadValue();
                if (id <= 0)
                {
                    continue;
                }

                InputTouchPhase phase = touch.phase.ReadValue();
                if (
                    !touch.press.isPressed
                    && !knownPointerIds.Contains(id)
                    && phase == InputTouchPhase.None
                )
                {
                    continue;
                }

                var buttons = new HashSet<PointerButton>();
                AddPressed(buttons, PointerButton.Left, touch.press.isPressed);
                samples[id] = new MasonryPointerSample(
                    touch.position.ReadValue(),
                    buttons,
                    true,
                    phase == InputTouchPhase.Canceled
                );
            }
        }

        private static void AddPressed(
            ISet<PointerButton> buttons,
            PointerButton button,
            bool isPressed
        )
        {
            if (isPressed)
            {
                buttons.Add(button);
            }
        }
    }
}
