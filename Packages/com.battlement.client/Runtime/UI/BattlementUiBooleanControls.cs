#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;

namespace Battlement.UI
{
    internal sealed class BattlementUiBooleanControls
    {
        private readonly Dictionary<Guid, BooleanControlState> controls = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiBooleanControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is not (UiElement.Toggle or UiElement.RadioButton))
                return;
            var control = (BaseField<bool>)target;
            var state = new BooleanControlState(control, objectId);
            controls.Add(objectId.Value, state);
            state.ValueChanged = eventValue => OnValueChanged(state, eventValue);
            control.RegisterValueChangedCallback(state.ValueChanged);
            if (value is UiElement.RadioButton)
            {
                state.Clicked = _ => OnRadioClicked(state);
                control.RegisterCallback(state.Clicked);
            }
            Apply(state, value);
            CaptureParts(target, value);
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.Toggle or UiElement.RadioButton)
                Apply(controls[objectId.Value], value);
        }

        public void Remove(Guid objectId)
        {
            if (controls.Remove(objectId, out BooleanControlState state))
                state.Dispose();
        }

        public void Clear()
        {
            foreach (BooleanControlState state in controls.Values)
                state.Dispose();
            controls.Clear();
        }

        private static void Apply(BooleanControlState state, UiElement value)
        {
            string? label;
            string? text;
            bool? authored;
            switch (value)
            {
                case UiElement.Toggle toggle:
                    label = toggle.Label;
                    text = toggle.Text;
                    authored = toggle.Value;
                    if (text is not null)
                        ((Toggle)state.Target).text = text;
                    break;
                case UiElement.RadioButton radio:
                    label = radio.Label;
                    text = radio.Text;
                    authored = radio.Value;
                    if (text is not null)
                        ((RadioButton)state.Target).text = text;
                    break;
                default:
                    throw new InvalidOperationException("Unsupported Boolean control type.");
            }
            if (label is not null)
                state.Target.label = label;
            if (authored is bool committed)
            {
                state.Committed = committed;
                state.Target.SetValueWithoutNotify(committed);
            }
        }

        private void OnValueChanged(BooleanControlState state, ChangeEvent<bool> eventValue)
        {
            if (eventValue.target != state.Target)
                return;
            bool proposed = eventValue.newValue;
            bool previous = state.Committed;
            state.Target.SetValueWithoutNotify(previous);
            if (state.Clicked is not null)
                return;
            if (
                !state.Target.enabledSelf
                || !state.Target.enabledInHierarchy
                || proposed == previous
            )
                return;
            events.ForwardValueCommitted(state.ObjectId, previous, proposed);
        }

        private void OnRadioClicked(BooleanControlState state)
        {
            bool previous = state.Committed;
            state.Target.SetValueWithoutNotify(previous);
            if (!state.Target.enabledSelf || !state.Target.enabledInHierarchy || previous)
                return;
            events.ForwardValueCommitted(state.ObjectId, previous, true);
        }

        private static void CaptureParts(VisualElement target, UiElement value)
        {
            switch (value)
            {
                case UiElement.Toggle toggle:
                    RequireAuthored(target, Toggle.labelUssClassName, toggle.Label);
                    Require(target, Toggle.inputUssClassName);
                    Require(target, Toggle.checkmarkUssClassName);
                    RequireAuthored(target, Toggle.textUssClassName, toggle.Text);
                    break;
                case UiElement.RadioButton radio:
                    RequireAuthored(target, RadioButton.labelUssClassName, radio.Label);
                    Require(target, RadioButton.inputUssClassName);
                    Require(target, RadioButton.checkmarkUssClassName);
                    Require(target, RadioButton.checkmarkBackgroundUssClassName);
                    RequireAuthored(target, RadioButton.textUssClassName, radio.Text);
                    break;
                default:
                    throw new InvalidOperationException("Unsupported Boolean control type.");
            }
        }

        private static VisualElement Require(VisualElement owner, string className) =>
            owner.Q<VisualElement>(className: className)
            ?? throw new InvalidOperationException(
                $"Native Boolean control part .{className} is missing."
            );

        private static void RequireAuthored(VisualElement owner, string className, string? authored)
        {
            if (authored is not null)
                Require(owner, className);
        }

        private sealed class BooleanControlState : IDisposable
        {
            public BooleanControlState(BaseField<bool> target, ObjectId objectId)
            {
                Target = target;
                ObjectId = objectId;
            }

            public BaseField<bool> Target { get; }
            public ObjectId ObjectId { get; }
            public bool Committed { get; set; }
            public EventCallback<ChangeEvent<bool>> ValueChanged { get; set; } = null!;
            public EventCallback<UnityClickEvent>? Clicked { get; set; }

            public void Dispose()
            {
                Target.UnregisterValueChangedCallback(ValueChanged);
                if (Clicked is not null)
                    Target.UnregisterCallback(Clicked);
            }
        }
    }
}
