#nullable enable

using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementOverlayItems
    {
        private static readonly ConditionalWeakTable<VisualElement, State> states = new();

        public static bool HasAuthored(VisualElement target) => states.TryGetValue(target, out _);

        public static OverlayPlacement Get(VisualElement target) =>
            states.TryGetValue(target, out State state)
                ? state.Value
                : throw new System.InvalidOperationException(
                    "The element has no authored Overlay placement."
                );

        public static void Apply(VisualElement target, Prop<OverlayPlacement> value)
        {
            if (value.IsReset)
            {
                states.Remove(target);
                return;
            }
            if (value.IsSet)
                states.GetOrCreateValue(target).Value = value.Value;
        }

        private sealed class State
        {
            public OverlayPlacement Value { get; set; } = null!;
        }
    }
}
