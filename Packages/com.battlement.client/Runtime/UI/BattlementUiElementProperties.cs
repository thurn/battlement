#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolFlexDirection = Battlement.UiFlexDirection;
using ProtocolLanguageDirection = Battlement.UiLanguageDirection;
using ProtocolPickingMode = Battlement.UiPickingMode;
using ProtocolUsageHint = Battlement.UiUsageHint;
using UnityAlign = UnityEngine.UIElements.Align;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityFlexDirection = UnityEngine.UIElements.FlexDirection;
using UnityFlexWrap = UnityEngine.UIElements.Wrap;
using UnityJustify = UnityEngine.UIElements.Justify;
using UnityLanguageDirection = UnityEngine.UIElements.LanguageDirection;
using UnityPickingMode = UnityEngine.UIElements.PickingMode;
using UnityPosition = UnityEngine.UIElements.Position;
using UnityUsageHints = UnityEngine.UIElements.UsageHints;

namespace Battlement.UI
{
    internal sealed class BattlementUiElementProperties
    {
        private readonly Dictionary<Guid, HashSet<string>> authoredClasses = new();
        private readonly Dictionary<Guid, HashSet<UiEventKind>> subscriptions = new();
        private readonly BattlementUiImageProperties images;
        private readonly Func<UiEvent, bool>? emit;

        public BattlementUiElementProperties(
            Func<UiEvent, bool>? emitUiEvent,
            IBattlementUiAssetLookup? assetLookup
        )
        {
            emit = emitUiEvent;
            images = new BattlementUiImageProperties(assetLookup);
        }

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
        )
        {
            Apply(
                target,
                objectId,
                value.Name,
                value.Enabled,
                value.PickingMode,
                value.LanguageDirection,
                value.Focusable,
                value.TabIndex,
                value.DelegatesFocus,
                value.Classes,
                value.UsageHints,
                value.Style,
                value.Events
            );
            if (value is UiElement.Image image)
                images.Apply((UnityEngine.UIElements.Image)target, objectId, image);
        }

        public void ApplyUpdate(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiElement value
        )
        {
            Validate(value, allowUsageHints: false);
            IBattlementUiAssetLease? staged = value is UiElement.Image image
                ? images.StageUpdate(objectId, image)
                : null;
            try
            {
                if (value.Name is string name)
                    target.name = name;
                if (value.Enabled is bool enabled)
                    target.SetEnabled(enabled);
                if (value.PickingMode is ProtocolPickingMode pickingMode)
                    target.pickingMode = ToUnity(pickingMode);
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
                ApplyStyle(target, value.Style);
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
                    case UiElement.Image imageValue:
                        images.ApplyUpdate(
                            (UnityEngine.UIElements.Image)target,
                            objectId,
                            imageValue,
                            staged
                        );
                        staged = null;
                        break;
                    default:
                        break;
                }
            }
            finally
            {
                staged?.Dispose();
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
            images.Remove(objectId);
        }

        public void Clear()
        {
            authoredClasses.Clear();
            subscriptions.Clear();
            images.Clear();
        }

        public static void Validate(UiElement element, bool allowUsageHints)
        {
            ValidateString(element.Name, allowEmpty: true, "UI name");
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
            ValidateStyle(element.Style);
            switch (element)
            {
                case UiElement.Label label:
                    ValidateString(label.Text, allowEmpty: true, "label text");
                    break;
                case UiElement.Button button:
                    ValidateString(button.Text, allowEmpty: true, "button text");
                    break;
                case UiElement.Image image:
                    BattlementUiImageProperties.Validate(image);
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
            ApplyStyle(target, style);
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

        private static void ValidateStyle(UiStyle? value) =>
            UiStyleValidator.Validate(
                value,
                message => Failure(CoreErrorCode.InvalidProperty, message)
            );

        private static void ApplyStyle(VisualElement element, UiStyle? value)
        {
            if (value is null)
                return;
            IStyle target = element.style;
            Apply(
                value.AlignContent,
                item => target.alignContent = ToUnity(item),
                () => target.alignContent = StyleKeyword.Initial
            );
            Apply(
                value.AlignItems,
                item => target.alignItems = ToUnity(item),
                () => target.alignItems = StyleKeyword.Initial
            );
            Apply(
                value.AlignSelf,
                item => target.alignSelf = ToUnity(item),
                () => target.alignSelf = StyleKeyword.Initial
            );
            Apply(
                value.AspectRatio,
                item => target.aspectRatio = ToUnity(item),
                () => target.aspectRatio = StyleKeyword.Initial
            );
            if (value.BackgroundColor is Color background)
                target.backgroundColor = ToUnity(background);
            Apply(
                value.Bottom,
                item => target.bottom = ToUnity(item),
                () => target.bottom = StyleKeyword.Initial
            );
            if (value.Color is Color foreground)
                target.color = ToUnity(foreground);
            Apply(
                value.FlexBasis,
                item => target.flexBasis = ToUnity(item),
                () => target.flexBasis = StyleKeyword.Initial
            );
            Apply(
                value.FlexDirection,
                item => target.flexDirection = ToUnity(item),
                () => target.flexDirection = StyleKeyword.Initial
            );
            Apply(
                value.FlexGrow,
                item => target.flexGrow = item,
                () => target.flexGrow = StyleKeyword.Initial
            );
            Apply(
                value.FlexShrink,
                item => target.flexShrink = item,
                () => target.flexShrink = StyleKeyword.Initial
            );
            Apply(
                value.FlexWrap,
                item => target.flexWrap = ToUnity(item),
                () => target.flexWrap = StyleKeyword.Initial
            );
            if (value.FontSize is float fontSize)
                target.fontSize = fontSize;
            Apply(
                value.Height,
                item => target.height = ToUnity(item),
                () => target.height = StyleKeyword.Initial
            );
            Apply(
                value.JustifyContent,
                item => target.justifyContent = ToUnity(item),
                () => target.justifyContent = StyleKeyword.Initial
            );
            Apply(
                value.Left,
                item => target.left = ToUnity(item),
                () => target.left = StyleKeyword.Initial
            );
            Apply(
                value.MarginBottom,
                item => target.marginBottom = ToUnity(item),
                () => target.marginBottom = StyleKeyword.Initial
            );
            Apply(
                value.MarginLeft,
                item => target.marginLeft = ToUnity(item),
                () => target.marginLeft = StyleKeyword.Initial
            );
            Apply(
                value.MarginRight,
                item => target.marginRight = ToUnity(item),
                () => target.marginRight = StyleKeyword.Initial
            );
            Apply(
                value.MarginTop,
                item => target.marginTop = ToUnity(item),
                () => target.marginTop = StyleKeyword.Initial
            );
            Apply(
                value.MaxHeight,
                item => target.maxHeight = ToUnity(item),
                () => target.maxHeight = StyleKeyword.Initial
            );
            Apply(
                value.MaxWidth,
                item => target.maxWidth = ToUnity(item),
                () => target.maxWidth = StyleKeyword.Initial
            );
            Apply(
                value.MinHeight,
                item => target.minHeight = ToUnity(item),
                () => target.minHeight = StyleKeyword.Initial
            );
            Apply(
                value.MinWidth,
                item => target.minWidth = ToUnity(item),
                () => target.minWidth = StyleKeyword.Initial
            );
            Apply(
                value.PaddingBottom,
                item => target.paddingBottom = ToUnity(item),
                () => target.paddingBottom = StyleKeyword.Initial
            );
            Apply(
                value.PaddingLeft,
                item => target.paddingLeft = ToUnity(item),
                () => target.paddingLeft = StyleKeyword.Initial
            );
            Apply(
                value.PaddingRight,
                item => target.paddingRight = ToUnity(item),
                () => target.paddingRight = StyleKeyword.Initial
            );
            Apply(
                value.PaddingTop,
                item => target.paddingTop = ToUnity(item),
                () => target.paddingTop = StyleKeyword.Initial
            );
            Apply(
                value.Position,
                item => target.position = ToUnity(item),
                () => target.position = StyleKeyword.Initial
            );
            Apply(
                value.Right,
                item => target.right = ToUnity(item),
                () => target.right = StyleKeyword.Initial
            );
            Apply(
                value.Top,
                item => target.top = ToUnity(item),
                () => target.top = StyleKeyword.Initial
            );
            Apply(
                value.Width,
                item => target.width = ToUnity(item),
                () => target.width = StyleKeyword.Initial
            );
        }

        private static void Apply<T>(
            UiStyleValue<T>? value,
            System.Action<T> concrete,
            System.Action initial
        )
        {
            if (value is null)
                return;
            if (value.Keyword is UiInlineKeyword.Initial)
                initial();
            else
                concrete(value.Value);
        }

        private static StyleLength ToUnity(UiLength value) =>
            value switch
            {
                UiLength.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLength.Percent item => new Length(item.Value, LengthUnit.Percent),
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI length kind."),
            };

        private static StyleLength ToUnity(UiLengthOrAuto value) =>
            value switch
            {
                UiLengthOrAuto.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLengthOrAuto.Percent item => new Length(item.Value, LengthUnit.Percent),
                UiLengthOrAuto.Auto => StyleKeyword.Auto,
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI length kind."),
            };

        private static StyleRatio ToUnity(UiAspectRatio value) =>
            value switch
            {
                UiAspectRatio.Auto => StyleKeyword.Auto,
                UiAspectRatio.Ratio item => new StyleRatio(item.Width / item.Height),
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI ratio kind."),
            };

        private static UnityAlign ToUnity(UiAlign value) =>
            value switch
            {
                UiAlign.Auto => UnityAlign.Auto,
                UiAlign.FlexStart => UnityAlign.FlexStart,
                UiAlign.Center => UnityAlign.Center,
                UiAlign.FlexEnd => UnityAlign.FlexEnd,
                _ => UnityAlign.Stretch,
            };

        private static UnityFlexDirection ToUnity(ProtocolFlexDirection value) =>
            value switch
            {
                ProtocolFlexDirection.Column => UnityFlexDirection.Column,
                ProtocolFlexDirection.ColumnReverse => UnityFlexDirection.ColumnReverse,
                ProtocolFlexDirection.Row => UnityFlexDirection.Row,
                _ => UnityFlexDirection.RowReverse,
            };

        private static UnityFlexWrap ToUnity(UiFlexWrap value) =>
            value switch
            {
                UiFlexWrap.NoWrap => UnityFlexWrap.NoWrap,
                UiFlexWrap.Wrap => UnityFlexWrap.Wrap,
                _ => UnityFlexWrap.WrapReverse,
            };

        private static UnityJustify ToUnity(UiJustify value) =>
            value switch
            {
                UiJustify.FlexStart => UnityJustify.FlexStart,
                UiJustify.Center => UnityJustify.Center,
                UiJustify.FlexEnd => UnityJustify.FlexEnd,
                UiJustify.SpaceBetween => UnityJustify.SpaceBetween,
                UiJustify.SpaceAround => UnityJustify.SpaceAround,
                _ => UnityJustify.SpaceEvenly,
            };

        private static UnityPosition ToUnity(UiPosition value) =>
            value == UiPosition.Relative ? UnityPosition.Relative : UnityPosition.Absolute;

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
