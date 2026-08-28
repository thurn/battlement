#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiTextFieldControls
    {
        private readonly Dictionary<Guid, TextFieldState> fields = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiTextFieldControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is not UiElement.TextField text)
                return;
            var field = (TextField)target;
            var state = new TextFieldState(
                field,
                field.Q<VisualElement>(TextField.textInputUssName).Q<TextElement>(),
                objectId
            );
            fields.Add(objectId.Value, state);
            state.InputChanged = eventValue => OnInput(state, eventValue);
            state.ValueChanged = eventValue => OnValueChanged(state, eventValue);
            state.KeyDown = eventValue => OnKeyDown(state, eventValue);
            state.NavigationCancel = eventValue => OnNavigationCancel(state, eventValue);
            state.CursorChanged = () => QueueSelection(state);
            state.SelectChanged = () => QueueSelection(state);
            state.Input.RegisterValueChangedCallback(state.InputChanged);
            field.RegisterValueChangedCallback(state.ValueChanged);
            state.Input.RegisterCallback(state.KeyDown, TrickleDown.TrickleDown);
            field.RegisterCallback(state.KeyDown, TrickleDown.TrickleDown);
            state.Input.RegisterCallback(state.NavigationCancel, TrickleDown.TrickleDown);
            field.RegisterCallback(state.NavigationCancel);
            field.textSelection.OnCursorIndexChange += state.CursorChanged;
            field.textSelection.OnSelectIndexChange += state.SelectChanged;
            Apply(state, text);
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.TextField text)
                Apply(fields[objectId.Value], text);
        }

        public void ValidateUpdate(ObjectId objectId, UiElement value)
        {
            if (value is not UiElement.TextField field)
                return;
            TextFieldState state = fields[objectId.Value];
            TextField native = state.Target;
            var defaults = new TextField();
            string text = Resolve(
                field.Value,
                native.value ?? string.Empty,
                defaults.value ?? string.Empty
            );
            int cursor = ResolveIndex(
                field.CursorIndex,
                state.PendingCursorIndex ?? native.cursorIndex,
                defaults.cursorIndex
            );
            int select = ResolveIndex(
                field.SelectIndex,
                state.PendingSelectIndex ?? native.selectIndex,
                defaults.selectIndex
            );
            CheckedIndex(cursor, text);
            CheckedIndex(select, text);
        }

        public void Advance()
        {
            foreach (TextFieldState state in fields.Values)
            {
                ApplyAuthoredSelection(state);
                if (!state.SelectionPending)
                    continue;
                state.SelectionPending = false;
                if (state.Suppressed)
                    continue;
                events.ForwardSelectionChanged(
                    state.ObjectId,
                    state.Target.cursorIndex,
                    state.Target.selectIndex
                );
            }
        }

        public void Remove(Guid objectId)
        {
            if (fields.Remove(objectId, out TextFieldState state))
                state.Dispose();
        }

        public void Clear()
        {
            foreach (TextFieldState state in fields.Values)
                state.Dispose();
            fields.Clear();
        }

        public void CancelAll()
        {
            foreach (TextFieldState state in fields.Values)
                Restore(state);
        }

        private static void Apply(TextFieldState state, UiElement.TextField value)
        {
            var defaults = new TextField();
            RunSuppressed(
                state,
                () =>
                {
                    if (value.Label.IsSet)
                        state.Target.label = value.Label.Value;
                    else if (value.Label.IsReset)
                        state.Target.label = defaults.label;
                    if (value.Multiline.IsSet)
                        state.Target.multiline = value.Multiline.Value;
                    else if (value.Multiline.IsReset)
                        state.Target.multiline = defaults.multiline;
                    if (!value.VerticalScrollerVisibility.IsUnset)
                        state.Target.verticalScrollerVisibility = value
                            .VerticalScrollerVisibility
                            .IsReset
                            ? defaults.verticalScrollerVisibility
                            : value.VerticalScrollerVisibility.Value switch
                            {
                                UiScrollerVisibility.Auto => ScrollerVisibility.Auto,
                                UiScrollerVisibility.AlwaysVisible =>
                                    ScrollerVisibility.AlwaysVisible,
                                UiScrollerVisibility.Hidden => ScrollerVisibility.Hidden,
                                _ => throw new InvalidOperationException(
                                    "Unsupported text-field scroller visibility."
                                ),
                            };
                    Apply(
                        value.Password,
                        next => state.Target.isPasswordField = next,
                        defaults.isPasswordField
                    );
                    Apply(
                        value.ReadOnly,
                        next => state.Target.isReadOnly = next,
                        defaults.isReadOnly
                    );
                    if (value.Placeholder.IsSet)
                        state.Target.textEdition.placeholder = value.Placeholder.Value;
                    else if (value.Placeholder.IsReset)
                        state.Target.textEdition.placeholder = defaults.textEdition.placeholder;
                    Apply(
                        value.HidePlaceholderOnFocus,
                        next => state.Target.textEdition.hidePlaceholderOnFocus = next,
                        defaults.textEdition.hidePlaceholderOnFocus
                    );
                    Apply(
                        value.SelectAllOnFocus,
                        next => state.Target.textSelection.selectAllOnFocus = next,
                        defaults.textSelection.selectAllOnFocus
                    );
                    Apply(
                        value.SelectAllOnMouseUp,
                        next => state.Target.textSelection.selectAllOnMouseUp = next,
                        defaults.textSelection.selectAllOnMouseUp
                    );
                    if (!value.Value.IsUnset)
                    {
                        state.Committed = value.Value.IsReset
                            ? defaults.value ?? string.Empty
                            : value.Value.Value;
                        state.Target.SetValueWithoutNotify(state.Committed);
                    }
                    if (!value.CursorIndex.IsUnset)
                    {
                        state.PendingCursorIndex = value.CursorIndex.IsReset
                            ? defaults.cursorIndex
                            : checked((int)value.CursorIndex.Value);
                        state.SelectionPending = false;
                    }
                    if (!value.SelectIndex.IsUnset)
                    {
                        state.PendingSelectIndex = value.SelectIndex.IsReset
                            ? defaults.selectIndex
                            : checked((int)value.SelectIndex.Value);
                        state.SelectionPending = false;
                    }
                }
            );
        }

        private void OnInput(TextFieldState state, ChangeEvent<string> eventValue)
        {
            if (state.Suppressed || eventValue.target != state.Input)
                return;
            string draft = eventValue.newValue ?? string.Empty;
            events.ForwardInput(state.ObjectId, draft);
        }

        private void OnValueChanged(TextFieldState state, ChangeEvent<string> eventValue)
        {
            if (state.Suppressed || eventValue.target != state.Target)
                return;
            string proposed = eventValue.newValue ?? string.Empty;
            string previous = state.Committed;
            Restore(state);
            if (proposed != previous)
                events.ForwardValueCommitted(state.ObjectId, previous, proposed);
        }

        private void OnKeyDown(TextFieldState state, KeyDownEvent eventValue)
        {
            if (eventValue.keyCode == UnityEngine.KeyCode.Escape)
            {
                Restore(state);
                eventValue.StopImmediatePropagation();
                return;
            }
            if (state.Target.multiline)
                return;
            if (
                eventValue.keyCode != UnityEngine.KeyCode.Return
                && eventValue.character != '\n'
                && eventValue.character != '\r'
            )
                return;
            string proposed = state.Input.text ?? string.Empty;
            string previous = state.Committed;
            Restore(state);
            if (proposed != previous)
                events.ForwardValueCommitted(state.ObjectId, previous, proposed);
            eventValue.StopImmediatePropagation();
        }

        private static void OnNavigationCancel(
            TextFieldState state,
            NavigationCancelEvent eventValue
        )
        {
            Restore(state);
            eventValue.StopImmediatePropagation();
        }

        private static void QueueSelection(TextFieldState state)
        {
            if (!state.Suppressed)
                state.SelectionPending = true;
        }

        private static void ApplyAuthoredSelection(TextFieldState state)
        {
            if (state.PendingCursorIndex is null && state.PendingSelectIndex is null)
                return;
            int cursor = state.PendingCursorIndex ?? state.Target.cursorIndex;
            int select = state.PendingSelectIndex ?? state.Target.selectIndex;
            RunSuppressed(
                state,
                () =>
                {
                    state.Target.cursorIndex = cursor;
                    state.Target.selectIndex = select;
                    state.SelectionPending = false;
                }
            );
            state.PendingCursorIndex = null;
            state.PendingSelectIndex = null;
        }

        private static void Restore(TextFieldState state) =>
            RunSuppressed(
                state,
                () =>
                {
                    state.Target.SetValueWithoutNotify(state.Committed);
                    state.SelectionPending = false;
                }
            );

        private static void CheckedIndex(int value, string text)
        {
            if (value > text.Length)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Text selection index is out of range."
                );
        }

        private static void Apply(Prop<bool> value, Action<bool> assign, bool reset)
        {
            if (value.IsSet)
                assign(value.Value);
            else if (value.IsReset)
                assign(reset);
        }

        private static string Resolve(Prop<string> value, string current, string reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private static int ResolveIndex(Prop<uint> value, int current, int reset) =>
            value.IsSet ? checked((int)value.Value)
            : value.IsReset ? reset
            : current;

        private static void RunSuppressed(TextFieldState state, System.Action action)
        {
            state.SuppressionDepth++;
            try
            {
                action();
            }
            finally
            {
                state.SuppressionDepth--;
            }
        }

        private sealed class TextFieldState : IDisposable
        {
            public TextFieldState(TextField target, TextElement input, ObjectId objectId)
            {
                Target = target;
                Input = input;
                ObjectId = objectId;
                Committed = string.Empty;
                target.isDelayed = true;
            }

            public TextField Target { get; }
            public TextElement Input { get; }
            public ObjectId ObjectId { get; }
            public string Committed { get; set; }
            public bool SelectionPending { get; set; }
            public int? PendingCursorIndex { get; set; }
            public int? PendingSelectIndex { get; set; }
            public int SuppressionDepth { get; set; }
            public bool Suppressed => SuppressionDepth != 0;
            public EventCallback<ChangeEvent<string>> InputChanged { get; set; } = null!;
            public EventCallback<ChangeEvent<string>> ValueChanged { get; set; } = null!;
            public EventCallback<KeyDownEvent> KeyDown { get; set; } = null!;
            public EventCallback<NavigationCancelEvent> NavigationCancel { get; set; } = null!;
            public System.Action CursorChanged { get; set; } = null!;
            public System.Action SelectChanged { get; set; } = null!;

            public void Dispose()
            {
                Input.UnregisterValueChangedCallback(InputChanged);
                Target.UnregisterValueChangedCallback(ValueChanged);
                Input.UnregisterCallback(KeyDown, TrickleDown.TrickleDown);
                Target.UnregisterCallback(KeyDown, TrickleDown.TrickleDown);
                Input.UnregisterCallback(NavigationCancel, TrickleDown.TrickleDown);
                Target.UnregisterCallback(NavigationCancel);
                Target.textSelection.OnCursorIndexChange -= CursorChanged;
                Target.textSelection.OnSelectIndexChange -= SelectChanged;
            }
        }
    }
}
