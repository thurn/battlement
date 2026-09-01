#nullable enable

using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementStickyItems
    {
        private static readonly ConditionalWeakTable<VisualElement, State> states = new();

        public static void Apply(VisualElement target, Prop<Sticky> value)
        {
            if (value.IsUnset)
                return;
            State state = states.GetOrCreateValue(target);
            state.Value = value.IsSet ? value.Value : Default();
            state.IsAuthored = value.IsSet;
        }

        public static Sticky Get(VisualElement target) =>
            states.TryGetValue(target, out State state) ? state.Value : Default();

        public static bool HasAuthored(VisualElement target) =>
            states.TryGetValue(target, out State state) && state.IsAuthored;

        private static Sticky Default() => new(null, null, null, null, 0);

        private sealed class State
        {
            public bool IsAuthored { get; set; }

            public Sticky Value { get; set; } = Default();
        }
    }
}
