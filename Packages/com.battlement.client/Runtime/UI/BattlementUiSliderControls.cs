#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;
using NativeSlider = UnityEngine.UIElements.Slider;
using NativeSliderInt = UnityEngine.UIElements.SliderInt;
using ProtocolDirection = Battlement.UiSliderDirection;
using UnityColor = UnityEngine.Color;
using UnityDirection = UnityEngine.UIElements.SliderDirection;

namespace Battlement.UI
{
    internal sealed class BattlementUiSliderControls
    {
        private static readonly UnityColor InputBackground = new(0.04f, 0.105f, 0.125f, 1);
        private static readonly UnityColor InputText = new(0.94f, 0.98f, 0.99f, 1);
        private const float InputWidth = 64;

        private readonly Dictionary<Guid, FloatState> floats = new();
        private readonly Dictionary<Guid, IntState> integers = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiSliderControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.Slider slider)
            {
                var native = (NativeSlider)target;
                native.fill = false;
                Apply(native, slider);
                FloatState state = CreateState(native, objectId);
                floats.Add(objectId.Value, state);
                CaptureParts(native, slider.Label.IsSet, native.fill, native.showInputField);
            }
            if (value is UiElement.SliderInt sliderInt)
            {
                var native = (NativeSliderInt)target;
                native.fill = false;
                Apply(native, sliderInt);
                IntState state = CreateState(native, objectId);
                integers.Add(objectId.Value, state);
                CaptureParts(native, sliderInt.Label.IsSet, native.fill, native.showInputField);
            }
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.Slider slider)
            {
                FloatState state = floats[objectId.Value];
                state.Cancel();
                state.CommandOrigin = true;
                try
                {
                    Apply(state.Target, slider);
                }
                finally
                {
                    state.CommandOrigin = false;
                }
                if (TouchesValue(slider))
                    state.Committed = state.Target.value;
                CaptureParts(
                    state.Target,
                    state.Target.label.Length > 0,
                    state.Target.fill,
                    state.Target.showInputField
                );
            }
            if (value is UiElement.SliderInt sliderInt)
            {
                IntState state = integers[objectId.Value];
                state.Cancel();
                state.CommandOrigin = true;
                try
                {
                    Apply(state.Target, sliderInt);
                }
                finally
                {
                    state.CommandOrigin = false;
                }
                if (TouchesValue(sliderInt))
                    state.Committed = state.Target.value;
                CaptureParts(
                    state.Target,
                    state.Target.label.Length > 0,
                    state.Target.fill,
                    state.Target.showInputField
                );
            }
        }

        public static void ValidateUpdate(UiElement element, VisualElement current)
        {
            switch (element)
            {
                case UiElement.Slider update:
                    NativeSlider floatTarget = (NativeSlider)current;
                    Validate(
                        Resolve(update.LowValue, floatTarget.lowValue, new NativeSlider().lowValue),
                        Resolve(
                            update.HighValue,
                            floatTarget.highValue,
                            new NativeSlider().highValue
                        ),
                        Resolve(update.Value, floatTarget.value, new NativeSlider().value),
                        Resolve(update.PageSize, floatTarget.pageSize, new NativeSlider().pageSize)
                    );
                    break;
                case UiElement.SliderInt update:
                    NativeSliderInt intTarget = (NativeSliderInt)current;
                    Validate(
                        Resolve(
                            update.LowValue,
                            intTarget.lowValue,
                            new NativeSliderInt().lowValue
                        ),
                        Resolve(
                            update.HighValue,
                            intTarget.highValue,
                            new NativeSliderInt().highValue
                        ),
                        Resolve(update.Value, intTarget.value, new NativeSliderInt().value),
                        Resolve(update.PageSize, intTarget.pageSize, new NativeSliderInt().pageSize)
                    );
                    break;
                default:
                    break;
            }
        }

        public static void ValidateNode(UiElement element)
        {
            switch (element)
            {
                case UiElement.Slider slider:
                    Validate(
                        Resolve(slider.LowValue, 0, 0),
                        Resolve(slider.HighValue, 10, 10),
                        Resolve(slider.Value, 0, 0),
                        Resolve(slider.PageSize, 0, 0)
                    );
                    break;
                case UiElement.SliderInt slider:
                    Validate(
                        Resolve(slider.LowValue, 0, 0),
                        Resolve(slider.HighValue, 10, 10),
                        Resolve(slider.Value, 0, 0),
                        Resolve(slider.PageSize, 0, 0)
                    );
                    break;
                default:
                    break;
            }
        }

        public void Remove(Guid objectId)
        {
            if (floats.Remove(objectId, out FloatState state))
                state.Dispose();
            if (integers.Remove(objectId, out IntState intState))
                intState.Dispose();
        }

        public void Clear()
        {
            foreach (FloatState state in floats.Values)
                state.Dispose();
            foreach (IntState state in integers.Values)
                state.Dispose();
            floats.Clear();
            integers.Clear();
        }

        public void CancelAll()
        {
            foreach (FloatState state in floats.Values)
            {
                state.Cancel();
                state.PendingCommits.Clear();
                ReleaseCaptures(state.Captures);
            }
            foreach (IntState state in integers.Values)
            {
                state.Cancel();
                state.PendingCommits.Clear();
                ReleaseCaptures(state.Captures);
            }
        }

        public void Advance()
        {
            foreach (FloatState state in floats.Values)
                TryForwardCommit(state);
            foreach (IntState state in integers.Values)
                TryForwardCommit(state);
        }

        private FloatState CreateState(NativeSlider target, ObjectId objectId)
        {
            var state = new FloatState(target, objectId);
            state.Owner = this;
            state.ValueChanged = change =>
            {
                if (state.CommandOrigin || !target.enabledInHierarchy)
                    return;
                bool interacting = state.Interacting;
                events.ForwardValueChanging(objectId, change.newValue);
                if (!interacting)
                    Commit(state, change.newValue);
            };
            Register(state);
            return state;
        }

        private IntState CreateState(NativeSliderInt target, ObjectId objectId)
        {
            var state = new IntState(target, objectId);
            state.Owner = this;
            state.ValueChanged = change =>
            {
                if (state.CommandOrigin || !target.enabledInHierarchy)
                    return;
                bool interacting = state.Interacting;
                events.ForwardValueChanging(objectId, change.newValue);
                if (!interacting)
                    Commit(state, change.newValue);
            };
            Register(state);
            return state;
        }

        private static void Register(FloatState state)
        {
            state.PointerDown = _ => state.Interacting = state.Target.enabledInHierarchy;
            state.Capture = eventValue =>
            {
                state.Interacting = state.Target.enabledInHierarchy;
                if (state.Interacting && eventValue.target is VisualElement owner)
                    state.Captures[eventValue.pointerId] = owner;
            };
            state.PointerUp = _ => Commit(state, state.Target.value);
            state.PointerCancel = _ => Restore(state);
            state.CaptureOut = eventValue =>
            {
                if (!state.Captures.Remove(eventValue.pointerId))
                    return;
                Commit(state, state.Target.value);
            };
            state.Detach = _ => state.Cancel();
            state.Target.RegisterValueChangedCallback(state.ValueChanged);
            RegisterPointerCallbacks(state.Target, state);
        }

        private static void Register(IntState state)
        {
            state.PointerDown = _ => state.Interacting = state.Target.enabledInHierarchy;
            state.Capture = eventValue =>
            {
                state.Interacting = state.Target.enabledInHierarchy;
                if (state.Interacting && eventValue.target is VisualElement owner)
                    state.Captures[eventValue.pointerId] = owner;
            };
            state.PointerUp = _ => Commit(state, state.Target.value);
            state.PointerCancel = _ => Restore(state);
            state.CaptureOut = eventValue =>
            {
                if (!state.Captures.Remove(eventValue.pointerId))
                    return;
                Commit(state, state.Target.value);
            };
            state.Detach = _ => state.Cancel();
            state.Target.RegisterValueChangedCallback(state.ValueChanged);
            RegisterPointerCallbacks(state.Target, state);
        }

        private static void RegisterPointerCallbacks(VisualElement target, FloatState state)
        {
            target.RegisterCallback(state.PointerDown, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Capture, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerUp, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerCancel, TrickleDown.TrickleDown);
            target.RegisterCallback(state.CaptureOut, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Detach);
            VisualElement interaction = InteractionTarget(target);
            interaction.RegisterCallback(state.PointerDown);
            interaction.RegisterCallback(state.Capture);
            interaction.RegisterCallback(state.PointerUp);
            interaction.RegisterCallback(state.PointerCancel);
            interaction.RegisterCallback(state.CaptureOut);
        }

        private static void RegisterPointerCallbacks(VisualElement target, IntState state)
        {
            target.RegisterCallback(state.PointerDown, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Capture, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerUp, TrickleDown.TrickleDown);
            target.RegisterCallback(state.PointerCancel, TrickleDown.TrickleDown);
            target.RegisterCallback(state.CaptureOut, TrickleDown.TrickleDown);
            target.RegisterCallback(state.Detach);
            VisualElement interaction = InteractionTarget(target);
            interaction.RegisterCallback(state.PointerDown);
            interaction.RegisterCallback(state.Capture);
            interaction.RegisterCallback(state.PointerUp);
            interaction.RegisterCallback(state.PointerCancel);
            interaction.RegisterCallback(state.CaptureOut);
        }

        private static void Commit(FloatState state, float proposed)
        {
            if (!state.Interacting && proposed == state.Committed)
                return;
            float previous = state.Committed;
            state.Interacting = false;
            RestoreValue(state);
            state.Owner.ForwardCommit(state, previous, proposed);
        }

        private static void Commit(IntState state, int proposed)
        {
            if (!state.Interacting && proposed == state.Committed)
                return;
            int previous = state.Committed;
            state.Interacting = false;
            RestoreValue(state);
            state.Owner.ForwardCommit(state, previous, proposed);
        }

        private void ForwardCommit(FloatState state, float previous, float proposed)
        {
            if (!events.CanForward(state.ObjectId, UiEventKind.ValueCommitted))
                return;
            state.PendingCommits.Enqueue((previous, proposed));
            TryForwardCommit(state);
        }

        private void ForwardCommit(IntState state, int previous, int proposed)
        {
            if (!events.CanForward(state.ObjectId, UiEventKind.ValueCommitted))
                return;
            state.PendingCommits.Enqueue((previous, proposed));
            TryForwardCommit(state);
        }

        private bool TryForwardCommit(FloatState state)
        {
            if (!events.CanForward(state.ObjectId, UiEventKind.ValueCommitted))
            {
                state.PendingCommits.Clear();
                return true;
            }
            while (state.PendingCommits.TryPeek(out (float Previous, float Proposed) pending))
            {
                if (
                    !events.ForwardValueCommitted(
                        state.ObjectId,
                        pending.Previous,
                        pending.Proposed
                    )
                )
                    return false;
                state.PendingCommits.Dequeue();
            }
            return true;
        }

        private bool TryForwardCommit(IntState state)
        {
            if (!events.CanForward(state.ObjectId, UiEventKind.ValueCommitted))
            {
                state.PendingCommits.Clear();
                return true;
            }
            while (state.PendingCommits.TryPeek(out (int Previous, int Proposed) pending))
            {
                if (
                    !events.ForwardValueCommitted(
                        state.ObjectId,
                        pending.Previous,
                        pending.Proposed
                    )
                )
                    return false;
                state.PendingCommits.Dequeue();
            }
            return true;
        }

        private static void Restore(FloatState state)
        {
            state.Interacting = false;
            RestoreValue(state);
        }

        private static void Restore(IntState state)
        {
            state.Interacting = false;
            RestoreValue(state);
        }

        private static void RestoreValue(FloatState state)
        {
            state.CommandOrigin = true;
            try
            {
                state.Target.SetValueWithoutNotify(state.Committed);
            }
            finally
            {
                state.CommandOrigin = false;
            }
        }

        private static void RestoreValue(IntState state)
        {
            state.CommandOrigin = true;
            try
            {
                state.Target.SetValueWithoutNotify(state.Committed);
            }
            finally
            {
                state.CommandOrigin = false;
            }
        }

        private static void Apply(NativeSlider target, UiElement.Slider value)
        {
            var defaults = new NativeSlider();
            Apply(value.Label, item => target.label = item, defaults.label);
            Apply(value.LowValue, item => target.lowValue = item, defaults.lowValue);
            Apply(value.HighValue, item => target.highValue = item, defaults.highValue);
            Apply(value.Fill, item => target.fill = item, defaults.fill);
            Apply(value.PageSize, item => target.pageSize = item, defaults.pageSize);
            Apply(
                value.ShowInputField,
                item => target.showInputField = item,
                defaults.showInputField
            );
            Apply(
                value.Direction,
                item => target.direction = ToUnity(item),
                ProtocolDirection.Horizontal
            );
            Apply(value.Inverted, item => target.inverted = item, defaults.inverted);
            if (!value.Value.IsUnset)
                target.SetValueWithoutNotify(
                    value.Value.IsReset ? defaults.value : value.Value.Value
                );
        }

        private static void Apply(NativeSliderInt target, UiElement.SliderInt value)
        {
            var defaults = new NativeSliderInt();
            Apply(value.Label, item => target.label = item, defaults.label);
            Apply(value.LowValue, item => target.lowValue = item, defaults.lowValue);
            Apply(value.HighValue, item => target.highValue = item, defaults.highValue);
            Apply(value.Fill, item => target.fill = item, defaults.fill);
            Apply(value.PageSize, item => target.pageSize = item, defaults.pageSize);
            Apply(
                value.ShowInputField,
                item => target.showInputField = item,
                defaults.showInputField
            );
            Apply(
                value.Direction,
                item => target.direction = ToUnity(item),
                ProtocolDirection.Horizontal
            );
            Apply(value.Inverted, item => target.inverted = item, defaults.inverted);
            if (!value.Value.IsUnset)
                target.SetValueWithoutNotify(
                    value.Value.IsReset ? defaults.value : value.Value.Value
                );
        }

        private static void CaptureParts(
            VisualElement target,
            bool hasLabel,
            bool hasFill,
            bool hasTextInput
        )
        {
            if (hasLabel)
                Require(target, BaseSlider<float>.labelUssClassName);
            Require(target, BaseSlider<float>.inputUssClassName);
            Require(target, BaseSlider<float>.trackerUssClassName);
            VisualElement dragger = Require(target, BaseSlider<float>.draggerUssClassName);
            Require(target, BaseSlider<float>.draggerBorderUssClassName);
            if (
                hasFill
                && target.Q<VisualElement>(className: BaseSlider<float>.fillUssClassName) == null
            )
            {
                EventCallback<GeometryChangedEvent> materialized = null!;
                materialized = _ =>
                {
                    if (HasFill(target))
                        Require(target, BaseSlider<float>.fillUssClassName);
                    dragger.UnregisterCallback(materialized);
                };
                dragger.RegisterCallback(materialized);
            }
            if (hasTextInput)
            {
                VisualElement textField = Require(target, TextField.ussClassName);
                textField.style.backgroundColor = InputBackground;
                textField.style.color = InputText;
                textField.style.minWidth = InputWidth;
                textField.style.width = InputWidth;
                VisualElement? input = textField.Q<VisualElement>(
                    className: TextField.inputUssClassName
                );
                if (input is not null)
                    input.style.backgroundColor = InputBackground;
                TextElement? text = textField.Q<TextElement>();
                if (text is not null)
                    text.style.color = InputText;
            }
        }

        private static VisualElement Require(VisualElement owner, string className) =>
            owner.Q<VisualElement>(className: className)
            ?? throw new InvalidOperationException($"Native slider part .{className} is missing.");

        private static VisualElement InteractionTarget(VisualElement target) =>
            Require(target, BaseSlider<float>.dragContainerUssClassName);

        private static void ReleaseCaptures(Dictionary<int, VisualElement> captures)
        {
            var owned = new Dictionary<int, VisualElement>(captures);
            captures.Clear();
            foreach ((int pointerId, VisualElement owner) in owned)
            {
                if (owner.HasPointerCapture(pointerId))
                    owner.ReleasePointer(pointerId);
            }
        }

        private static bool HasFill(VisualElement target) =>
            target is NativeSlider slider ? slider.fill : ((NativeSliderInt)target).fill;

        private static UnityDirection ToUnity(ProtocolDirection value) =>
            value == ProtocolDirection.Horizontal
                ? UnityDirection.Horizontal
                : UnityDirection.Vertical;

        private static bool TouchesValue(UiElement.Slider value) =>
            !value.LowValue.IsUnset || !value.HighValue.IsUnset || !value.Value.IsUnset;

        private static bool TouchesValue(UiElement.SliderInt value) =>
            !value.LowValue.IsUnset || !value.HighValue.IsUnset || !value.Value.IsUnset;

        private static T Resolve<T>(Prop<T> value, T current, T reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private static void Apply<T>(Prop<T> value, System.Action<T> assign, T reset)
        {
            if (value.IsSet)
                assign(value.Value);
            else if (value.IsReset)
                assign(reset);
        }

        private static void Validate(float low, float high, float selected, float pageSize)
        {
            if (!float.IsFinite(low) || !float.IsFinite(high) || !float.IsFinite(selected))
                throw Failure("Slider values must be finite.");
            if (!float.IsFinite(pageSize) || pageSize < 0 || low > high)
                throw Failure("Slider range or page size is invalid.");
            if (selected < low || selected > high)
                throw Failure("Slider value is outside its range.");
        }

        private static void Validate(int low, int high, int selected, float pageSize)
        {
            if (!float.IsFinite(pageSize) || pageSize < 0 || low > high)
                throw Failure("Slider range or page size is invalid.");
            if (selected < low || selected > high)
                throw Failure("Slider value is outside its range.");
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class FloatState : IDisposable
        {
            public FloatState(NativeSlider target, ObjectId objectId)
            {
                Target = target;
                ObjectId = objectId;
                Committed = target.value;
                Owner = null!;
            }

            public BattlementUiSliderControls Owner { get; set; } = null!;
            public NativeSlider Target { get; }
            public ObjectId ObjectId { get; }
            public float Committed { get; set; }
            public bool CommandOrigin { get; set; }
            public bool Interacting { get; set; }
            public Dictionary<int, VisualElement> Captures { get; } = new();
            public Queue<(float Previous, float Proposed)> PendingCommits { get; } = new();
            public EventCallback<ChangeEvent<float>> ValueChanged { get; set; } = null!;
            public EventCallback<PointerDownEvent> PointerDown { get; set; } = null!;
            public EventCallback<PointerCaptureEvent> Capture { get; set; } = null!;
            public EventCallback<PointerUpEvent> PointerUp { get; set; } = null!;
            public EventCallback<PointerCancelEvent> PointerCancel { get; set; } = null!;
            public EventCallback<PointerCaptureOutEvent> CaptureOut { get; set; } = null!;
            public EventCallback<DetachFromPanelEvent> Detach { get; set; } = null!;

            public void Cancel()
            {
                Restore(this);
            }

            public void Dispose()
            {
                Cancel();
                ReleaseCaptures(Captures);
                Target.UnregisterValueChangedCallback(ValueChanged);
                Target.UnregisterCallback(PointerDown, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Capture, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerUp, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerCancel, TrickleDown.TrickleDown);
                Target.UnregisterCallback(CaptureOut, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Detach);
                VisualElement interaction = InteractionTarget(Target);
                interaction.UnregisterCallback(PointerDown);
                interaction.UnregisterCallback(Capture);
                interaction.UnregisterCallback(PointerUp);
                interaction.UnregisterCallback(PointerCancel);
                interaction.UnregisterCallback(CaptureOut);
            }
        }

        private sealed class IntState : IDisposable
        {
            public IntState(NativeSliderInt target, ObjectId objectId)
            {
                Target = target;
                ObjectId = objectId;
                Committed = target.value;
                Owner = null!;
            }

            public BattlementUiSliderControls Owner { get; set; } = null!;
            public NativeSliderInt Target { get; }
            public ObjectId ObjectId { get; }
            public int Committed { get; set; }
            public bool CommandOrigin { get; set; }
            public bool Interacting { get; set; }
            public Dictionary<int, VisualElement> Captures { get; } = new();
            public Queue<(int Previous, int Proposed)> PendingCommits { get; } = new();
            public EventCallback<ChangeEvent<int>> ValueChanged { get; set; } = null!;
            public EventCallback<PointerDownEvent> PointerDown { get; set; } = null!;
            public EventCallback<PointerCaptureEvent> Capture { get; set; } = null!;
            public EventCallback<PointerUpEvent> PointerUp { get; set; } = null!;
            public EventCallback<PointerCancelEvent> PointerCancel { get; set; } = null!;
            public EventCallback<PointerCaptureOutEvent> CaptureOut { get; set; } = null!;
            public EventCallback<DetachFromPanelEvent> Detach { get; set; } = null!;

            public void Cancel()
            {
                Restore(this);
            }

            public void Dispose()
            {
                Cancel();
                ReleaseCaptures(Captures);
                Target.UnregisterValueChangedCallback(ValueChanged);
                Target.UnregisterCallback(PointerDown, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Capture, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerUp, TrickleDown.TrickleDown);
                Target.UnregisterCallback(PointerCancel, TrickleDown.TrickleDown);
                Target.UnregisterCallback(CaptureOut, TrickleDown.TrickleDown);
                Target.UnregisterCallback(Detach);
                VisualElement interaction = InteractionTarget(Target);
                interaction.UnregisterCallback(PointerDown);
                interaction.UnregisterCallback(Capture);
                interaction.UnregisterCallback(PointerUp);
                interaction.UnregisterCallback(PointerCancel);
                interaction.UnregisterCallback(CaptureOut);
            }
        }
    }
}
