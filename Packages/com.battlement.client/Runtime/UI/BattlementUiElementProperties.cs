#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolFlexDirection = Battlement.UiFlexDirection;
using ProtocolLanguageDirection = Battlement.UiLanguageDirection;
using ProtocolPickingMode = Battlement.UiPickingMode;
using ProtocolUsageHint = Battlement.UiUsageHint;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityFlexDirection = UnityEngine.UIElements.FlexDirection;
using UnityLanguageDirection = UnityEngine.UIElements.LanguageDirection;
using UnityPickingMode = UnityEngine.UIElements.PickingMode;
using UnityUsageHints = UnityEngine.UIElements.UsageHints;

namespace Battlement.UI
{
    internal sealed class BattlementUiElementProperties
    {
        private readonly Dictionary<Guid, HashSet<string>> authoredClasses = new();
        private readonly Dictionary<Guid, HashSet<UiEventKind>> subscriptions = new();
        private readonly Func<UiEvent, bool>? emit;

        public BattlementUiElementProperties(Func<UiEvent, bool>? emitUiEvent) =>
            emit = emitUiEvent;

        public void ApplyRoot(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiDocument value
        ) =>
            Apply(
                target,
                objectId,
                value.Name,
                value.Enabled,
                value.PickingMode,
                value.Tooltip,
                value.LanguageDirection,
                value.Focusable,
                value.TabIndex,
                value.DelegatesFocus,
                value.Classes,
                null,
                value.Style,
                value.Events
            );

        public void ApplyElement(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiElement value
        ) =>
            Apply(
                target,
                objectId,
                value.Name,
                value.Enabled,
                value.PickingMode,
                value.Tooltip,
                value.LanguageDirection,
                value.Focusable,
                value.TabIndex,
                value.DelegatesFocus,
                value.Classes,
                value.UsageHints,
                value.Style,
                value.Events
            );

        public void ApplyUpdate(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiElement value
        )
        {
            Validate(value, allowUsageHints: false);
            if (value.Name is string name)
                target.name = name;
            if (value.Enabled is bool enabled)
                target.SetEnabled(enabled);
            if (value.PickingMode is ProtocolPickingMode pickingMode)
                target.pickingMode = ToUnity(pickingMode);
            if (value.Tooltip is string tooltip)
                target.tooltip = tooltip;
            if (value.LanguageDirection is ProtocolLanguageDirection languageDirection)
                target.languageDirection = ToUnity(languageDirection);
            if (value.Focusable is bool focusable)
                target.focusable = focusable;
            if (value.TabIndex is int tabIndex)
                target.tabIndex = tabIndex;
            if (value.DelegatesFocus is bool delegatesFocus)
                target.delegatesFocus = delegatesFocus;
            if (value.Classes is IReadOnlyList<string> classes)
            {
                foreach (string className in authoredClasses[objectId.Value])
                    target.RemoveFromClassList(className);
                var replacements = new HashSet<string>();
                foreach (string className in classes)
                {
                    target.AddToClassList(className);
                    replacements.Add(className);
                }
                authoredClasses[objectId.Value] = replacements;
            }
            ApplyStyle(target.style, value.Style);
            if (value.Events is IReadOnlyList<UiEventKind> events)
                subscriptions[objectId.Value] = new HashSet<UiEventKind>(events);
            switch (value)
            {
                case UiElement.Label label when label.Text is string text:
                    ((UnityEngine.UIElements.Label)target).text = text;
                    break;
                case UiElement.Button button when button.Text is string text:
                    ((UnityEngine.UIElements.Button)target).text = text;
                    break;
                default:
                    break;
            }
        }

        public void ForwardClick(ObjectId objectId, UnityClickEvent eventValue)
        {
            if (emit is null)
                return;
            if (!subscriptions.TryGetValue(objectId.Value, out HashSet<UiEventKind> values))
                return;
            if (!values.Contains(UiEventKind.Click))
                return;
            emit(
                new UiEvent(
                    objectId,
                    new UiEventBody.Click(
                        new Battlement.ClickEvent.Pointer(
                            new PanelPoint(eventValue.position.x, eventValue.position.y),
                            checked((uint)Math.Max(1, eventValue.clickCount)),
                            eventValue.pointerId,
                            ToPointerButton(eventValue.button),
                            ToModifiers(eventValue.modifiers)
                        )
                    )
                )
            );
        }

        public void Remove(Guid objectId)
        {
            authoredClasses.Remove(objectId);
            subscriptions.Remove(objectId);
        }

        public void Clear()
        {
            authoredClasses.Clear();
            subscriptions.Clear();
        }

        public static void Validate(UiElement element, bool allowUsageHints)
        {
            ValidateString(element.Name, allowEmpty: true, "UI name");
            ValidateString(element.Tooltip, allowEmpty: true, "UI tooltip");
            var classes = new HashSet<string>(StringComparer.Ordinal);
            foreach (string className in element.Classes ?? Array.Empty<string>())
            {
                ValidateString(className, allowEmpty: false, "UI class");
                if (!classes.Add(className))
                    throw Failure(CoreErrorCode.InvalidProperty, "UI classes must be unique.");
            }
            ValidateUnique(element.Events, "UI event subscriptions must be unique.");
            if (!allowUsageHints && element.UsageHints is not null)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "UI usage hints can only be assigned during creation."
                );
            ValidateUnique(element.UsageHints, "UI usage hints must be unique.");
            switch (element)
            {
                case UiElement.Label label:
                    ValidateString(label.Text, allowEmpty: true, "label text");
                    break;
                case UiElement.Button button:
                    ValidateString(button.Text, allowEmpty: true, "button text");
                    break;
                default:
                    break;
            }
        }

        private void Apply(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            string? name,
            bool? enabled,
            ProtocolPickingMode? pickingMode,
            string? tooltip,
            ProtocolLanguageDirection? languageDirection,
            bool? focusable,
            int? tabIndex,
            bool? delegatesFocus,
            IReadOnlyList<string>? classes,
            IReadOnlyList<ProtocolUsageHint>? usageHints,
            UiStyle? style,
            IReadOnlyList<UiEventKind>? events
        )
        {
            if (name is not null)
                target.name = name;
            if (enabled is bool enabledValue)
                target.SetEnabled(enabledValue);
            if (pickingMode is ProtocolPickingMode picking)
                target.pickingMode = ToUnity(picking);
            if (tooltip is not null)
                target.tooltip = tooltip;
            if (languageDirection is ProtocolLanguageDirection direction)
                target.languageDirection = ToUnity(direction);
            if (focusable is bool receivesFocus)
                target.focusable = receivesFocus;
            if (tabIndex is int focusIndex)
                target.tabIndex = focusIndex;
            if (delegatesFocus is bool transfersFocus)
                target.delegatesFocus = transfersFocus;
            if (usageHints is not null)
                target.usageHints = ToUnity(usageHints);
            var classSet = new HashSet<string>();
            foreach (string className in classes ?? Array.Empty<string>())
            {
                target.AddToClassList(className);
                classSet.Add(className);
            }
            authoredClasses[objectId.Value] = classSet;
            ApplyStyle(target.style, style);
            subscriptions[objectId.Value] = new HashSet<UiEventKind>(
                events ?? Array.Empty<UiEventKind>()
            );
        }

        private static void ValidateUnique<T>(IReadOnlyList<T>? values, string message)
        {
            IReadOnlyList<T> items = values ?? Array.Empty<T>();
            if (items.Count != new HashSet<T>(items).Count)
                throw Failure(CoreErrorCode.InvalidProperty, message);
        }

        private static void ValidateString(string? value, bool allowEmpty, string description)
        {
            if (value is null)
                return;
            if (!allowEmpty && value.Length == 0)
                throw Failure(CoreErrorCode.InvalidProperty, $"{description} cannot be empty.");
            if (System.Text.Encoding.UTF8.GetByteCount(value) > 65_536)
                throw Failure(CoreErrorCode.LimitExceeded, $"{description} is too long.");
        }

        private static void ApplyStyle(IStyle target, UiStyle? value)
        {
            if (value is null)
                return;
            if (value.BackgroundColor is Color background)
                target.backgroundColor = ToUnity(background);
            if (value.Color is Color foreground)
                target.color = ToUnity(foreground);
            if (value.Width is float width)
                target.width = width;
            if (value.Height is float height)
                target.height = height;
            if (value.FlexGrow is float flexGrow)
                target.flexGrow = flexGrow;
            if (value.FlexDirection is ProtocolFlexDirection direction)
                target.flexDirection =
                    direction == ProtocolFlexDirection.Row
                        ? UnityFlexDirection.Row
                        : UnityFlexDirection.Column;
            if (value.Padding is float padding)
            {
                target.paddingTop = padding;
                target.paddingRight = padding;
                target.paddingBottom = padding;
                target.paddingLeft = padding;
            }
            if (value.Margin is float margin)
            {
                target.marginTop = margin;
                target.marginRight = margin;
                target.marginBottom = margin;
                target.marginLeft = margin;
            }
            if (value.FontSize is float fontSize)
                target.fontSize = fontSize;
        }

        private static PointerButton ToPointerButton(int value) =>
            value switch
            {
                1 => PointerButton.Right,
                2 => PointerButton.Middle,
                _ => PointerButton.Left,
            };

        private static IReadOnlyList<KeyModifier> ToModifiers(EventModifiers values)
        {
            var result = new List<KeyModifier>();
            if ((values & EventModifiers.Alt) != 0)
                result.Add(KeyModifier.Alt);
            if ((values & EventModifiers.Control) != 0)
                result.Add(KeyModifier.Control);
            if ((values & EventModifiers.Command) != 0)
                result.Add(KeyModifier.Command);
            if ((values & EventModifiers.Shift) != 0)
                result.Add(KeyModifier.Shift);
            return result;
        }

        private static UnityPickingMode ToUnity(ProtocolPickingMode value) =>
            value == ProtocolPickingMode.Position
                ? UnityPickingMode.Position
                : UnityPickingMode.Ignore;

        private static UnityLanguageDirection ToUnity(ProtocolLanguageDirection value) =>
            value switch
            {
                ProtocolLanguageDirection.Inherit => UnityLanguageDirection.Inherit,
                ProtocolLanguageDirection.Ltr => UnityLanguageDirection.LTR,
                _ => UnityLanguageDirection.RTL,
            };

        private static UnityUsageHints ToUnity(IReadOnlyList<ProtocolUsageHint> values)
        {
            UnityUsageHints result = UnityUsageHints.None;
            foreach (ProtocolUsageHint value in values)
            {
                result |= value switch
                {
                    ProtocolUsageHint.DynamicTransform => UnityUsageHints.DynamicTransform,
                    ProtocolUsageHint.GroupTransform => UnityUsageHints.GroupTransform,
                    ProtocolUsageHint.MaskContainer => UnityUsageHints.MaskContainer,
                    ProtocolUsageHint.DynamicColor => UnityUsageHints.DynamicColor,
                    ProtocolUsageHint.DynamicPostProcessing =>
                        UnityUsageHints.DynamicPostProcessing,
                    _ => UnityUsageHints.LargePixelCoverage,
                };
            }
            return result;
        }

        private static UnityEngine.Color ToUnity(Color value) =>
            new((float)value.Red, (float)value.Green, (float)value.Blue, (float)value.Alpha);

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }
}
