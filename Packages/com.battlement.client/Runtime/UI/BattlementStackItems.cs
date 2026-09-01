#nullable enable

using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementStackItems
    {
        private static readonly ConditionalWeakTable<VisualElement, State> states = new();

        public static void Apply(VisualElement target, Prop<StackItem> value)
        {
            if (value.IsUnset)
                return;
            State state = states.GetOrCreateValue(target);
            state.Value = value.IsSet ? value.Value : Default();
            state.IsAuthored = value.IsSet;
        }

        public static StackItem Get(VisualElement target) =>
            states.TryGetValue(target, out State state) ? state.Value : Default();

        public static bool HasAuthored(VisualElement target) =>
            states.TryGetValue(target, out State state) && state.IsAuthored;

        private static StackItem Default() =>
            new(0, UiAlign.Auto, UiAlign.Auto, null, null, null, null, true);

        private sealed class State
        {
            public bool IsAuthored { get; set; }

            public StackItem Value { get; set; } = Default();
        }
    }
}
