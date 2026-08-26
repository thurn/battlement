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

        private static void Apply(TextFieldState state, UiElement.TextField value)
        {
            RunSuppressed(
                state,
                () =>
                {
                    if (value.Label is not null)
                        state.Target.label = value.Label;
                    if (value.Multiline is bool multiline)
                        state.Target.multiline = multiline;
                    if (value.Password is bool password)
                        state.Target.isPasswordField = password;
                    if (value.ReadOnly is bool readOnly)
                        state.Target.isReadOnly = readOnly;
                    if (value.Placeholder is not null)
                        state.Target.textEdition.placeholder = value.Placeholder;
                    if (value.HidePlaceholderOnFocus is bool hidePlaceholder)
                        state.Target.textEdition.hidePlaceholderOnFocus = hidePlaceholder;
                    if (value.SelectAllOnFocus is bool selectAllOnFocus)
                        state.Target.textSelection.selectAllOnFocus = selectAllOnFocus;
                    if (value.SelectAllOnMouseUp is bool selectAllOnMouseUp)
                        state.Target.textSelection.selectAllOnMouseUp = selectAllOnMouseUp;
                    if (value.Value is not null)
                    {
                        state.Committed = value.Value;
                        state.Target.SetValueWithoutNotify(value.Value);
                    }
                    if (value.CursorIndex is uint cursor)
                    {
                        CheckedIndex(cursor, state.Input.text ?? string.Empty);
                        state.PendingCursorIndex = checked((int)cursor);
                        state.SelectionPending = false;
                    }
                    if (value.SelectIndex is uint select)
                    {
                        CheckedIndex(select, state.Input.text ?? string.Empty);
                        state.PendingSelectIndex = checked((int)select);
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

        private static int CheckedIndex(uint value, string text)
        {
            if (value > text.Length)
                throw new InvalidOperationException("Text selection index is out of range.");
            return checked((int)value);
        }

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
