#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;
using NativeDropdownField = UnityEngine.UIElements.DropdownField;

namespace Battlement.UI
{
    internal sealed class BattlementUiDropdownControls
    {
        private readonly Dictionary<Guid, DropdownState> controls = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiDropdownControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is not UiElement.DropdownField dropdown)
                return;
            var native = (NativeDropdownField)target;
            var state = new DropdownState(native, objectId);
            controls.Add(objectId.Value, state);
            state.ValueChanged = change => OnValueChanged(state, change);
            native.RegisterValueChangedCallback(state.ValueChanged);
            Apply(state, dropdown);
            CaptureParts(native, dropdown.Label.IsSet);
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.DropdownField dropdown)
                Apply(controls[objectId.Value], dropdown);
        }

        public static void ValidateUpdate(UiElement element, VisualElement current)
        {
            if (element is not UiElement.DropdownField update)
                return;
            var native = (NativeDropdownField)current;
            var defaults = new NativeDropdownField();
            IReadOnlyList<string> choices =
                update.Choices.IsSet ? update.Choices.Value
                : update.Choices.IsReset ? defaults.choices
                : native.choices;
            DropdownChoice selection =
                update.Selection.IsSet ? update.Selection.Value
                : update.Selection.IsReset ? DropdownChoice.None()
                : Selection(native);
            ValidateSelection(selection, choices);
        }

        public static void ValidateNode(UiElement element)
        {
            if (element is not UiElement.DropdownField dropdown)
                return;
            ValidateSelection(
                dropdown.Selection.IsSet ? dropdown.Selection.Value : DropdownChoice.None(),
                dropdown.Choices.IsSet ? dropdown.Choices.Value : Array.Empty<string>()
            );
        }

        public void Remove(Guid objectId)
        {
            if (controls.Remove(objectId, out DropdownState state))
                state.Dispose();
        }

        public void Clear()
        {
            foreach (DropdownState state in controls.Values)
                state.Dispose();
            controls.Clear();
        }

        private static void Apply(DropdownState state, UiElement.DropdownField value)
        {
            var defaults = new NativeDropdownField();
            state.Suppressed = true;
            try
            {
                if (value.Label.IsSet)
                    state.Target.label = value.Label.Value;
                else if (value.Label.IsReset)
                    state.Target.label = defaults.label;
                if (value.ShowMixedValue.IsSet)
                    state.Target.showMixedValue = value.ShowMixedValue.Value;
                else if (value.ShowMixedValue.IsReset)
                    state.Target.showMixedValue = defaults.showMixedValue;
                if (value.Choices.IsSet)
                    state.Target.choices = value.Choices.Value.ToList();
                else if (value.Choices.IsReset)
                    state.Target.choices = defaults.choices.ToList();
                if (value.Selection.IsSet)
                    state.Committed = value.Selection.Value;
                else if (value.Selection.IsReset)
                    state.Committed = DropdownChoice.None();
                state.Target.SetValueWithoutNotify(state.Committed.Value ?? string.Empty);
                SyncLabelColor(state.Target);
            }
            finally
            {
                state.Suppressed = false;
            }
        }

        private void OnValueChanged(DropdownState state, ChangeEvent<string> change)
        {
            if (state.Suppressed)
                return;
            DropdownChoice proposed = Selection(state.Target, change.newValue);
            state.Target.SetValueWithoutNotify(state.Committed.Value ?? string.Empty);
            if (!Available(state.Target) || proposed == state.Committed)
                return;
            events.ForwardValueCommitted(state.ObjectId, state.Committed, proposed);
        }

        private static DropdownChoice Selection(NativeDropdownField target) =>
            Selection(target, target.value);

        private static DropdownChoice Selection(NativeDropdownField target, string value) =>
            target.index < 0
                ? DropdownChoice.None()
                : DropdownChoice.Selected(checked((uint)target.index), value);

        private static void ValidateSelection(
            DropdownChoice selection,
            IReadOnlyList<string> choices
        )
        {
            if (selection.Index is null && selection.Value is null)
                return;
            if (selection.Index is not uint index || selection.Value is not string value)
                throw Failure("Dropdown selection index and value must both be present or absent.");
            if (index >= choices.Count || choices[checked((int)index)] != value)
                throw Failure("Dropdown selection index and value do not match its choices.");
        }

        private static void CaptureParts(NativeDropdownField target, bool hasLabel)
        {
            if (hasLabel)
                Require(target, NativeDropdownField.labelUssClassName);
            Require(target, NativeDropdownField.inputUssClassName);
            Require(target, NativeDropdownField.textUssClassName);
            Require(target, NativeDropdownField.arrowUssClassName);
        }

        private static void SyncLabelColor(NativeDropdownField target)
        {
            VisualElement? label = target.Q<VisualElement>(
                className: NativeDropdownField.labelUssClassName
            );
            if (label is not null)
                label.style.color = target.style.color;
        }

        private static VisualElement Require(VisualElement owner, string className) =>
            owner.Q<VisualElement>(className: className)
            ?? throw new InvalidOperationException(
                $"Native dropdown part .{className} is missing."
            );

        private static bool Available(VisualElement target) =>
            target.enabledSelf && target.enabledInHierarchy;

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class DropdownState : IDisposable
        {
            public DropdownState(NativeDropdownField target, ObjectId objectId)
            {
                Target = target;
                ObjectId = objectId;
            }

            public NativeDropdownField Target { get; }
            public ObjectId ObjectId { get; }
            public DropdownChoice Committed { get; set; } = DropdownChoice.None();
            public bool Suppressed { get; set; }
            public EventCallback<ChangeEvent<string>> ValueChanged { get; set; } = null!;

            public void Dispose() => Target.UnregisterValueChangedCallback(ValueChanged);
        }
    }
}
