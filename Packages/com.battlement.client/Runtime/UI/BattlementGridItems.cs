#nullable enable

using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementGridItems
    {
        private static readonly ConditionalWeakTable<VisualElement, State> states = new();

        public static void Apply(VisualElement target, Prop<GridItem> value)
        {
            if (value.IsUnset)
                return;
            State state = states.GetOrCreateValue(target);
            state.Value = value.IsSet ? value.Value : Default();
            state.IsAuthored = value.IsSet;
        }

        public static GridItem Get(VisualElement target) =>
            states.TryGetValue(target, out State state) ? state.Value : Default();

        public static bool HasAuthored(VisualElement target) =>
            states.TryGetValue(target, out State state) && state.IsAuthored;

        private static GridItem Default() => new(null, null, 1, 1, UiAlign.Auto, UiAlign.Auto);

        private sealed class State
        {
            public bool IsAuthored { get; set; }

            public GridItem Value { get; set; } = Default();
        }
    }
}
