#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;
using NativeRadioButtonGroup = UnityEngine.UIElements.RadioButtonGroup;
using NativeToggleButtonGroup = UnityEngine.UIElements.ToggleButtonGroup;
using NativeToggleButtonGroupState = UnityEngine.UIElements.ToggleButtonGroupState;

namespace Battlement.UI
{
    internal sealed class BattlementUiChoiceControls
    {
        private readonly Dictionary<Guid, RadioState> radios = new();
        private readonly Dictionary<Guid, ToggleState> toggles = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiChoiceControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            switch (value)
            {
                case UiElement.RadioButtonGroup radio:
                    var radioTarget = (NativeRadioButtonGroup)target;
                    var radioState = new RadioState(radioTarget, objectId);
                    radios.Add(objectId.Value, radioState);
                    radioState.ValueChanged = change => OnRadioChanged(radioState, change);
                    radioTarget.RegisterValueChangedCallback(radioState.ValueChanged);
                    Apply(radioState, radio);
                    CaptureRadioParts(
                        radioTarget,
                        radio.Label.IsSet,
                        radio.Choices.IsSet ? radio.Choices.Value.Count : 0
                    );
                    break;
                case UiElement.ToggleButtonGroup toggle:
                    var toggleTarget = (NativeToggleButtonGroup)target;
                    var toggleState = new ToggleState(toggleTarget, objectId);
                    toggles.Add(objectId.Value, toggleState);
                    toggleState.ValueChanged = change => OnToggleChanged(toggleState, change);
                    toggleTarget.RegisterValueChangedCallback(toggleState.ValueChanged);
                    Apply(toggleState, toggle, toggleTarget.contentContainer.childCount);
                    CaptureToggleParts(toggleTarget, toggle.Label.IsSet);
                    break;
                default:
                    return;
            }
        }

        public void InitializeToggle(ObjectId objectId, int childCount)
        {
            ToggleState state = toggles[objectId.Value];
            state.ChildCount = childCount;
            if (!state.HasAuthoredSelection && childCount > 0 && !state.Target.allowEmptySelection)
                state.SelectedIndices = new uint[] { 0 };
            state.Committed = State(state.SelectedIndices, childCount);
            state.Target.SetValueWithoutNotify(state.Committed);
            state.Suppress = false;
        }

        public void BeginHierarchyMutation(ObjectId objectId)
        {
            if (toggles.TryGetValue(objectId.Value, out ToggleState state))
                state.Suppress = true;
        }

        public void Insert(ObjectId objectId, int index, int childCount)
        {
            if (!toggles.TryGetValue(objectId.Value, out ToggleState state))
                return;
            uint position = checked((uint)index);
            Commit(
                state,
                state.SelectedIndices.Select(value => value >= position ? value + 1 : value),
                childCount
            );
        }

        public void Remove(ObjectId objectId, int index, int childCount)
        {
            if (!toggles.TryGetValue(objectId.Value, out ToggleState state))
                return;
            uint position = checked((uint)index);
            Commit(
                state,
                state
                    .SelectedIndices.Where(value => value != position)
                    .Select(value => value > position ? value - 1 : value),
                childCount
            );
        }

        public void Reorder(ObjectId objectId, int previousIndex, int nextIndex)
        {
            if (!toggles.TryGetValue(objectId.Value, out ToggleState state))
                return;
            Commit(
                state,
                state.SelectedIndices.Select(value =>
                    ReorderedIndex(value, previousIndex, nextIndex)
                ),
                state.ChildCount
            );
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            switch (value)
            {
                case UiElement.RadioButtonGroup radio:
                    Apply(radios[objectId.Value], radio);
                    break;
                case UiElement.ToggleButtonGroup toggle:
                    Apply(toggles[objectId.Value], toggle, target.contentContainer.childCount);
                    break;
                default:
                    return;
            }
        }

        public static void ValidateUpdate(UiElement element, VisualElement current, int childCount)
        {
            if (element is UiElement.RadioButtonGroup radioUpdate)
            {
                var radioCurrent = (NativeRadioButtonGroup)current;
                var defaults = new NativeRadioButtonGroup();
                int choiceCount =
                    radioUpdate.Choices.IsSet ? radioUpdate.Choices.Value.Count
                    : radioUpdate.Choices.IsReset ? defaults.choices.Count()
                    : radioCurrent.choices.Count();
                int selected =
                    radioUpdate.SelectedIndex.IsSet ? checked((int)radioUpdate.SelectedIndex.Value)
                    : radioUpdate.SelectedIndex.IsReset ? defaults.value
                    : radioCurrent.value;
                if (selected >= choiceCount)
                    throw Failure("Radio selection is out of range.");
                return;
            }
            if (element is not UiElement.ToggleButtonGroup update)
                return;
            var toggleCurrent = (NativeToggleButtonGroup)current;
            var toggleDefaults = new NativeToggleButtonGroup();
            bool multiple = Resolve(
                update.MultipleSelection,
                toggleCurrent.isMultipleSelection,
                toggleDefaults.isMultipleSelection
            );
            bool allowEmpty = Resolve(
                update.AllowEmptySelection,
                toggleCurrent.allowEmptySelection,
                toggleDefaults.allowEmptySelection
            );
            ValidateToggleSelection(
                update.SelectedIndices.IsSet ? update.SelectedIndices.Value
                    : update.SelectedIndices.IsReset ? DefaultSelection(childCount, allowEmpty)
                    : Indices(toggleCurrent.value),
                childCount,
                multiple,
                allowEmpty
            );
        }

        public static void ValidateNode(UiElement element, int childCount)
        {
            if (element is UiElement.RadioButtonGroup radio)
            {
                int choiceCount = radio.Choices.IsSet ? radio.Choices.Value.Count : 0;
                if (radio.SelectedIndex.IsSet && radio.SelectedIndex.Value >= choiceCount)
                    throw Failure("Radio selection is out of range.");
                return;
            }
            if (element is not UiElement.ToggleButtonGroup toggle)
                return;
            if (childCount > 64)
                throw new BattlementUiException(
                    CoreErrorCode.LimitExceeded,
                    "ToggleButtonGroup accepts 64 buttons."
                );
            bool multiple = toggle.MultipleSelection.IsSet && toggle.MultipleSelection.Value;
            bool allowEmpty = toggle.AllowEmptySelection.IsSet && toggle.AllowEmptySelection.Value;
            IReadOnlyList<uint> selected = toggle.SelectedIndices.IsSet
                ? toggle.SelectedIndices.Value
                : DefaultSelection(childCount, allowEmpty);
            ValidateToggleSelection(selected, childCount, multiple, allowEmpty);
        }

        public void Remove(Guid objectId)
        {
            if (radios.Remove(objectId, out RadioState radio))
                radio.Dispose();
            if (toggles.Remove(objectId, out ToggleState toggle))
                toggle.Dispose();
        }

        public void Clear()
        {
            foreach (RadioState state in radios.Values)
                state.Dispose();
            foreach (ToggleState state in toggles.Values)
                state.Dispose();
            radios.Clear();
            toggles.Clear();
        }

        private static void Apply(RadioState state, UiElement.RadioButtonGroup value)
        {
            var defaults = new NativeRadioButtonGroup();
            if (value.Label.IsSet)
                state.Target.label = value.Label.Value;
            else if (value.Label.IsReset)
                state.Target.label = defaults.label;
            if (value.Choices.IsSet)
                state.Target.choices = value.Choices.Value.ToList();
            else if (value.Choices.IsReset)
                state.Target.choices = defaults.choices.ToList();
            if (!value.SelectedIndex.IsUnset)
            {
                state.Committed = value.SelectedIndex.IsReset
                    ? defaults.value
                    : checked((int)value.SelectedIndex.Value);
                state.Target.SetValueWithoutNotify(state.Committed);
            }
        }

        private static void Apply(
            ToggleState state,
            UiElement.ToggleButtonGroup value,
            int childCount
        )
        {
            var defaults = new NativeToggleButtonGroup();
            if (value.Label.IsSet)
                state.Target.label = value.Label.Value;
            else if (value.Label.IsReset)
                state.Target.label = defaults.label;
            if (!value.MultipleSelection.IsUnset)
                state.Target.isMultipleSelection = value.MultipleSelection.IsReset
                    ? defaults.isMultipleSelection
                    : value.MultipleSelection.Value;
            if (!value.AllowEmptySelection.IsUnset)
                state.Target.allowEmptySelection = value.AllowEmptySelection.IsReset
                    ? defaults.allowEmptySelection
                    : value.AllowEmptySelection.Value;
            if (value.SelectedIndices.IsSet)
            {
                state.SelectedIndices = value.SelectedIndices.Value.ToArray();
                state.HasAuthoredSelection = true;
            }
            else if (value.SelectedIndices.IsReset)
            {
                state.SelectedIndices = DefaultSelection(
                    childCount,
                    state.Target.allowEmptySelection
                );
                state.HasAuthoredSelection = false;
            }
            if (!state.Suppress)
            {
                state.ChildCount = childCount;
                state.Committed = State(state.SelectedIndices, childCount);
                state.Target.SetValueWithoutNotify(state.Committed);
            }
        }

        private void OnRadioChanged(RadioState state, ChangeEvent<int> change)
        {
            state.Target.SetValueWithoutNotify(state.Committed);
            if (!Available(state.Target) || change.newValue == state.Committed)
                return;
            events.ForwardValueCommitted(
                state.ObjectId,
                OptionalIndex(state.Committed),
                OptionalIndex(change.newValue)
            );
        }

        private void OnToggleChanged(
            ToggleState state,
            ChangeEvent<NativeToggleButtonGroupState> change
        )
        {
            if (state.Suppress)
                return;
            state.Target.SetValueWithoutNotify(state.Committed);
            if (!Available(state.Target) || change.newValue == state.Committed)
                return;
            events.ForwardValueCommitted(
                state.ObjectId,
                Indices(state.Committed),
                Indices(change.newValue)
            );
        }

        private static NativeToggleButtonGroupState State(
            IReadOnlyList<uint> selected,
            int childCount
        )
        {
            ulong mask = 0;
            foreach (uint index in selected)
                mask |= 1UL << checked((int)index);
            return new NativeToggleButtonGroupState(mask, childCount);
        }

        private static void Commit(ToggleState state, IEnumerable<uint> selected, int childCount)
        {
            var values = selected.OrderBy(value => value).ToList();
            if (values.Count == 0 && childCount > 0 && !state.Target.allowEmptySelection)
                values.Add(0);
            state.SelectedIndices = values;
            state.ChildCount = childCount;
            state.Committed = State(values, childCount);
            state.Target.SetValueWithoutNotify(state.Committed);
            state.Suppress = false;
        }

        private static uint ReorderedIndex(uint value, int previousIndex, int nextIndex)
        {
            uint previous = checked((uint)previousIndex);
            uint next = checked((uint)nextIndex);
            if (value == previous)
                return next;
            if (previous < next && value > previous && value <= next)
                return value - 1;
            if (previous > next && value >= next && value < previous)
                return value + 1;
            return value;
        }

        private static uint[] Indices(NativeToggleButtonGroupState state)
        {
            var values = new List<uint>();
            for (int index = 0; index < state.length; index++)
            {
                if (state[index])
                    values.Add(checked((uint)index));
            }
            return values.ToArray();
        }

        private static uint[] DefaultSelection(int childCount, bool allowEmpty) =>
            childCount == 0 || allowEmpty ? Array.Empty<uint>() : new uint[] { 0 };

        private static bool Resolve(Prop<bool> value, bool current, bool reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private static void ValidateToggleSelection(
            IReadOnlyList<uint> selected,
            int childCount,
            bool multiple,
            bool allowEmpty
        )
        {
            uint? previous = null;
            foreach (uint index in selected)
            {
                if (index >= childCount || (previous is uint last && last >= index))
                    throw Failure("Toggle selection indices must be unique, sorted, and in range.");
                previous = index;
            }
            if (!multiple && selected.Count > 1)
                throw Failure("Single selection has many indices.");
            if (!allowEmpty && childCount > 0 && selected.Count == 0)
                throw Failure("Toggle selection cannot be empty.");
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static uint? OptionalIndex(int value) => value < 0 ? null : checked((uint)value);

        private static bool Available(VisualElement target) =>
            target.enabledSelf && target.enabledInHierarchy;

        private static void CaptureRadioParts(
            NativeRadioButtonGroup target,
            bool hasLabel,
            int choiceCount
        )
        {
            if (hasLabel)
                Require(target, BaseField<int>.labelUssClassName);
            Require(target, BaseField<int>.inputUssClassName);
            Require(target, NativeRadioButtonGroup.containerUssClassName);
            if (target.contentContainer is null)
                throw new InvalidOperationException(
                    "RadioButtonGroup content container is missing."
                );
            if (target.Query<RadioButton>().ToList().Count != choiceCount)
                throw new InvalidOperationException(
                    "RadioButtonGroup native option count diverged."
                );
        }

        private static void CaptureToggleParts(NativeToggleButtonGroup target, bool hasLabel)
        {
            if (hasLabel)
                Require(target, BaseField<NativeToggleButtonGroupState>.labelUssClassName);
            Require(target, BaseField<NativeToggleButtonGroupState>.inputUssClassName);
        }

        private static VisualElement Require(VisualElement owner, string className) =>
            owner.Q<VisualElement>(className: className)
            ?? throw new InvalidOperationException($"Native choice part .{className} is missing.");

        private sealed class RadioState : IDisposable
        {
            public RadioState(NativeRadioButtonGroup target, ObjectId objectId)
            {
                Target = target;
                ObjectId = objectId;
            }

            public NativeRadioButtonGroup Target { get; }
            public ObjectId ObjectId { get; }
            public int Committed { get; set; } = -1;
            public EventCallback<ChangeEvent<int>> ValueChanged { get; set; } = null!;

            public void Dispose() => Target.UnregisterValueChangedCallback(ValueChanged);
        }

        private sealed class ToggleState : IDisposable
        {
            public ToggleState(NativeToggleButtonGroup target, ObjectId objectId)
            {
                Target = target;
                ObjectId = objectId;
            }

            public NativeToggleButtonGroup Target { get; }
            public ObjectId ObjectId { get; }
            public IReadOnlyList<uint> SelectedIndices { get; set; } = Array.Empty<uint>();
            public NativeToggleButtonGroupState Committed { get; set; }
            public int ChildCount { get; set; }
            public bool HasAuthoredSelection { get; set; }
            public bool Suppress { get; set; } = true;
            public EventCallback<
                ChangeEvent<NativeToggleButtonGroupState>
            > ValueChanged { get; set; } = null!;

            public void Dispose() => Target.UnregisterValueChangedCallback(ValueChanged);
        }
    }
}
