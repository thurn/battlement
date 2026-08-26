#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolDisplay = Battlement.UiDisplay;
using ProtocolFlexDirection = Battlement.UiFlexDirection;
using ProtocolLanguageDirection = Battlement.UiLanguageDirection;
using ProtocolOverflow = Battlement.UiOverflow;
using ProtocolOverflowClipBox = Battlement.UiOverflowClipBox;
using ProtocolPickingMode = Battlement.UiPickingMode;
using ProtocolSliceType = Battlement.UiSliceType;
using ProtocolUsageHint = Battlement.UiUsageHint;
using ProtocolVisibility = Battlement.UiVisibility;
using UnityAlign = UnityEngine.UIElements.Align;
using UnityBackgroundPositionKeyword = UnityEngine.UIElements.BackgroundPositionKeyword;
using UnityBackgroundRepeat = UnityEngine.UIElements.BackgroundRepeat;
using UnityBackgroundRepeatMode = UnityEngine.UIElements.Repeat;
using UnityBackgroundSize = UnityEngine.UIElements.BackgroundSize;
using UnityBackgroundSizeType = UnityEngine.UIElements.BackgroundSizeType;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityDisplayStyle = UnityEngine.UIElements.DisplayStyle;
using UnityFlexDirection = UnityEngine.UIElements.FlexDirection;
using UnityFlexWrap = UnityEngine.UIElements.Wrap;
using UnityJustify = UnityEngine.UIElements.Justify;
using UnityLanguageDirection = UnityEngine.UIElements.LanguageDirection;
using UnityOverflow = UnityEngine.UIElements.Overflow;
using UnityOverflowClipBox = UnityEngine.UIElements.OverflowClipBox;
using UnityPickingMode = UnityEngine.UIElements.PickingMode;
using UnityPosition = UnityEngine.UIElements.Position;
using UnitySliceType = UnityEngine.UIElements.SliceType;
using UnityUsageHints = UnityEngine.UIElements.UsageHints;
using UnityVisibility = UnityEngine.UIElements.Visibility;

namespace Battlement.UI
{
    internal sealed class BattlementUiElementProperties
    {
        private readonly Dictionary<Guid, HashSet<string>> authoredClasses = new();
        private readonly Dictionary<Guid, HashSet<UiEventKind>> subscriptions = new();
        private readonly BattlementUiImageProperties images;
        private readonly BattlementUiStyleBackgroundProperties styleBackgrounds;
        private readonly BattlementUiStyleCursorProperties styleCursors;
        private readonly BattlementUiStyleMaterialProperties styleMaterials;
        private readonly Func<UiEvent, bool>? emit;

        public BattlementUiElementProperties(
            Func<UiEvent, bool>? emitUiEvent,
            IBattlementUiAssetLookup? assetLookup
        )
        {
            emit = emitUiEvent;
            images = new BattlementUiImageProperties(assetLookup);
            styleBackgrounds = new BattlementUiStyleBackgroundProperties(assetLookup);
            styleCursors = new BattlementUiStyleCursorProperties(assetLookup);
            styleMaterials = new BattlementUiStyleMaterialProperties(assetLookup);
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
            IBattlementUiAssetLease? stagedBackground = null;
            IBattlementUiAssetLease? stagedCursor = null;
            IBattlementUiAssetLease? stagedMaterial = null;
            try
            {
                stagedBackground = styleBackgrounds.Stage(value.Style);
                stagedCursor = styleCursors.Stage(value.Style);
                stagedMaterial = styleMaterials.Stage(value.Style);
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
                ApplyStyle(
                    target,
                    value.Style,
                    stagedBackground is null
                        ? null
                        : BattlementUiStyleBackgroundProperties.ToUnity(
                            value.Style!.BackgroundImage!.Value,
                            stagedBackground.Value
                        ),
                    stagedMaterial?.Value as Material,
                    ToUnityCursor(value.Style, stagedCursor)
                );
                styleBackgrounds.Commit(objectId.Value, value.Style, stagedBackground);
                stagedBackground = null;
                styleMaterials.Commit(objectId.Value, value.Style, stagedMaterial);
                stagedMaterial = null;
                styleCursors.Commit(objectId.Value, value.Style, stagedCursor);
                stagedCursor = null;
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
                stagedBackground?.Dispose();
                stagedCursor?.Dispose();
                stagedMaterial?.Dispose();
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

        public void ForwardTransition(
            ObjectId objectId,
            UiEventKind kind,
            IEnumerable<StylePropertyName> propertyNames,
            double elapsedSeconds
        )
        {
            if (emit is null)
                return;
            if (!subscriptions.TryGetValue(objectId.Value, out HashSet<UiEventKind> values))
                return;
            if (!values.Contains(kind))
                return;
            var properties = new List<UiTransitionProperty>();
            foreach (StylePropertyName propertyName in propertyNames)
            {
                if (
                    BattlementUiStyleTransformProperties.TryFromUnity(
                        propertyName,
                        out UiTransitionProperty property
                    )
                )
                {
                    properties.Add(property);
                }
            }
            float elapsedMs = (float)(elapsedSeconds * 1_000.0);
            if (properties.Count == 0 || !float.IsFinite(elapsedMs))
                return;
            var transition = new TransitionEvent(properties, elapsedMs);
            UiEventBody body = kind switch
            {
                UiEventKind.TransitionStart => new UiEventBody.TransitionStart(transition),
                UiEventKind.TransitionEnd => new UiEventBody.TransitionEnd(transition),
                UiEventKind.TransitionCancel => new UiEventBody.TransitionCancel(transition),
                _ => throw new InvalidOperationException("Unknown transition event kind."),
            };
            emit(new UiEvent(objectId, body));
        }

        public void Remove(Guid objectId)
        {
            authoredClasses.Remove(objectId);
            subscriptions.Remove(objectId);
            images.Remove(objectId);
            styleBackgrounds.Remove(objectId);
            styleCursors.Remove(objectId);
            styleMaterials.Remove(objectId);
        }

        public void Clear()
        {
            authoredClasses.Clear();
            subscriptions.Clear();
            images.Clear();
            styleBackgrounds.Clear();
            styleCursors.Clear();
            styleMaterials.Clear();
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
            IBattlementUiAssetLease? stagedBackground = styleBackgrounds.Stage(style);
            IBattlementUiAssetLease? stagedCursor = null;
            IBattlementUiAssetLease? stagedMaterial = null;
            try
            {
                stagedCursor = styleCursors.Stage(style);
                stagedMaterial = styleMaterials.Stage(style);
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
                ApplyStyle(
                    target,
                    style,
                    stagedBackground is null
                        ? null
                        : BattlementUiStyleBackgroundProperties.ToUnity(
                            style!.BackgroundImage!.Value,
                            stagedBackground.Value
                        ),
                    stagedMaterial?.Value as Material,
                    ToUnityCursor(style, stagedCursor)
                );
                styleBackgrounds.Commit(objectId.Value, style, stagedBackground);
                stagedBackground = null;
                styleMaterials.Commit(objectId.Value, style, stagedMaterial);
                stagedMaterial = null;
                styleCursors.Commit(objectId.Value, style, stagedCursor);
                stagedCursor = null;
                subscriptions[objectId.Value] = new HashSet<UiEventKind>(
                    events ?? Array.Empty<UiEventKind>()
                );
            }
            finally
            {
                stagedBackground?.Dispose();
                stagedCursor?.Dispose();
                stagedMaterial?.Dispose();
            }
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

        private static void ApplyStyle(
            VisualElement element,
            UiStyle? value,
            Background? background,
            Material? material,
            UnityEngine.UIElements.Cursor? cursor
        )
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
            Apply(
                value.BackgroundColor,
                item => target.backgroundColor = ToUnity(item),
                () => target.backgroundColor = StyleKeyword.Initial
            );
            Apply(
                value.BackgroundImage,
                _ => target.backgroundImage = new StyleBackground(background!.Value),
                () => target.backgroundImage = StyleKeyword.Initial
            );
            Apply(
                value.BackgroundPositionX,
                item => target.backgroundPositionX = ToUnity(item),
                () => target.backgroundPositionX = StyleKeyword.Initial
            );
            Apply(
                value.BackgroundPositionY,
                item => target.backgroundPositionY = ToUnity(item),
                () => target.backgroundPositionY = StyleKeyword.Initial
            );
            Apply(
                value.BackgroundRepeat,
                item => target.backgroundRepeat = ToUnity(item),
                () => target.backgroundRepeat = StyleKeyword.Initial
            );
            Apply(
                value.BackgroundSize,
                item => target.backgroundSize = ToUnity(item),
                () => target.backgroundSize = StyleKeyword.Initial
            );
            Apply(
                value.BorderBottomColor,
                item => target.borderBottomColor = ToUnity(item),
                () => target.borderBottomColor = StyleKeyword.Initial
            );
            Apply(
                value.BorderBottomLeftRadius,
                item => target.borderBottomLeftRadius = ToUnity(item),
                () => target.borderBottomLeftRadius = StyleKeyword.Initial
            );
            Apply(
                value.BorderBottomRightRadius,
                item => target.borderBottomRightRadius = ToUnity(item),
                () => target.borderBottomRightRadius = StyleKeyword.Initial
            );
            Apply(
                value.BorderBottomWidth,
                item => target.borderBottomWidth = item,
                () => target.borderBottomWidth = StyleKeyword.Initial
            );
            Apply(
                value.BorderLeftColor,
                item => target.borderLeftColor = ToUnity(item),
                () => target.borderLeftColor = StyleKeyword.Initial
            );
            Apply(
                value.BorderLeftWidth,
                item => target.borderLeftWidth = item,
                () => target.borderLeftWidth = StyleKeyword.Initial
            );
            Apply(
                value.BorderRightColor,
                item => target.borderRightColor = ToUnity(item),
                () => target.borderRightColor = StyleKeyword.Initial
            );
            Apply(
                value.BorderRightWidth,
                item => target.borderRightWidth = item,
                () => target.borderRightWidth = StyleKeyword.Initial
            );
            Apply(
                value.BorderTopColor,
                item => target.borderTopColor = ToUnity(item),
                () => target.borderTopColor = StyleKeyword.Initial
            );
            Apply(
                value.BorderTopLeftRadius,
                item => target.borderTopLeftRadius = ToUnity(item),
                () => target.borderTopLeftRadius = StyleKeyword.Initial
            );
            Apply(
                value.BorderTopRightRadius,
                item => target.borderTopRightRadius = ToUnity(item),
                () => target.borderTopRightRadius = StyleKeyword.Initial
            );
            Apply(
                value.BorderTopWidth,
                item => target.borderTopWidth = item,
                () => target.borderTopWidth = StyleKeyword.Initial
            );
            Apply(
                value.Bottom,
                item => target.bottom = ToUnity(item),
                () => target.bottom = StyleKeyword.Initial
            );
            Apply(
                value.Color,
                item => target.color = ToUnity(item),
                () => target.color = StyleKeyword.Initial
            );
            Apply(
                value.Cursor,
                _ => target.cursor = cursor!.Value,
                () => target.cursor = StyleKeyword.Initial
            );
            Apply(
                value.Display,
                item => target.display = ToUnity(item),
                () => target.display = StyleKeyword.Initial
            );
            Apply(
                value.Filter,
                item => target.filter = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.filter = StyleKeyword.Initial
            );
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
                value.Opacity,
                item => target.opacity = item,
                () => target.opacity = StyleKeyword.Initial
            );
            Apply(
                value.Overflow,
                item => target.overflow = ToUnity(item),
                () => target.overflow = StyleKeyword.Initial
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
                value.Rotate,
                item => target.rotate = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.rotate = StyleKeyword.Initial
            );
            Apply(
                value.Scale,
                item => target.scale = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.scale = StyleKeyword.Initial
            );
            Apply(
                value.Top,
                item => target.top = ToUnity(item),
                () => target.top = StyleKeyword.Initial
            );
            Apply(
                value.TransformOrigin,
                item => target.transformOrigin = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.transformOrigin = StyleKeyword.Initial
            );
            Apply(
                value.TransitionDelay,
                item =>
                    target.transitionDelay = BattlementUiStyleTransformProperties.ToUnityTimes(
                        item
                    ),
                () => target.transitionDelay = StyleKeyword.Initial
            );
            Apply(
                value.TransitionDuration,
                item =>
                    target.transitionDuration = BattlementUiStyleTransformProperties.ToUnityTimes(
                        item
                    ),
                () => target.transitionDuration = StyleKeyword.Initial
            );
            Apply(
                value.TransitionProperty,
                item =>
                    target.transitionProperty = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.transitionProperty = StyleKeyword.Initial
            );
            Apply(
                value.TransitionTimingFunction,
                item =>
                    target.transitionTimingFunction = BattlementUiStyleTransformProperties.ToUnity(
                        item
                    ),
                () => target.transitionTimingFunction = StyleKeyword.Initial
            );
            Apply(
                value.Translate,
                item => target.translate = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.translate = StyleKeyword.Initial
            );
            Apply(
                value.UnityBackgroundImageTintColor,
                item => target.unityBackgroundImageTintColor = ToUnity(item),
                () => target.unityBackgroundImageTintColor = StyleKeyword.Initial
            );
            Apply(
                value.UnityMaterial,
                _ => target.unityMaterial = material!,
                () => target.unityMaterial = StyleKeyword.Initial
            );
            Apply(
                value.UnityOverflowClipBox,
                item => target.unityOverflowClipBox = ToUnity(item),
                () => target.unityOverflowClipBox = StyleKeyword.Initial
            );
            Apply(
                value.UnitySliceBottom,
                item => target.unitySliceBottom = item,
                () => target.unitySliceBottom = StyleKeyword.Initial
            );
            Apply(
                value.UnitySliceLeft,
                item => target.unitySliceLeft = item,
                () => target.unitySliceLeft = StyleKeyword.Initial
            );
            Apply(
                value.UnitySliceRight,
                item => target.unitySliceRight = item,
                () => target.unitySliceRight = StyleKeyword.Initial
            );
            Apply(
                value.UnitySliceScale,
                item => target.unitySliceScale = item,
                () => target.unitySliceScale = StyleKeyword.Initial
            );
            Apply(
                value.UnitySliceTop,
                item => target.unitySliceTop = item,
                () => target.unitySliceTop = StyleKeyword.Initial
            );
            Apply(
                value.UnitySliceType,
                item => target.unitySliceType = ToUnity(item),
                () => target.unitySliceType = StyleKeyword.Initial
            );
            Apply(
                value.Visibility,
                item => target.visibility = ToUnity(item),
                () => target.visibility = StyleKeyword.Initial
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

        private static UnityEngine.UIElements.Cursor? ToUnityCursor(
            UiStyle? style,
            IBattlementUiAssetLease? lease
        )
        {
            UiStyleValue<UiCursor>? property = style?.Cursor;
            if (property is null || property.Keyword is UiInlineKeyword.Initial)
                return null;
            return BattlementUiStyleCursorProperties.ToUnity(property.Value, lease?.Value);
        }

        private static StyleLength ToUnity(UiLength value) =>
            value switch
            {
                UiLength.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLength.Percent item => new Length(item.Value, LengthUnit.Percent),
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI length kind."),
            };

        private static StyleBackgroundPosition ToUnity(UiBackgroundPosition value) =>
            new(
                new BackgroundPosition(
                    value.Keyword switch
                    {
                        UiBackgroundPositionKeyword.Left => UnityBackgroundPositionKeyword.Left,
                        UiBackgroundPositionKeyword.Right => UnityBackgroundPositionKeyword.Right,
                        UiBackgroundPositionKeyword.Top => UnityBackgroundPositionKeyword.Top,
                        UiBackgroundPositionKeyword.Bottom => UnityBackgroundPositionKeyword.Bottom,
                        _ => UnityBackgroundPositionKeyword.Center,
                    },
                    ToUnityLength(value.Offset)
                )
            );

        private static StyleBackgroundRepeat ToUnity(UiBackgroundRepeat value) =>
            new(new UnityBackgroundRepeat(ToUnity(value.X), ToUnity(value.Y)));

        private static UnityBackgroundRepeatMode ToUnity(UiBackgroundRepeatMode value) =>
            value switch
            {
                UiBackgroundRepeatMode.NoRepeat => UnityBackgroundRepeatMode.NoRepeat,
                UiBackgroundRepeatMode.Repeat => UnityBackgroundRepeatMode.Repeat,
                UiBackgroundRepeatMode.Round => UnityBackgroundRepeatMode.Round,
                _ => UnityBackgroundRepeatMode.Space,
            };

        private static StyleBackgroundSize ToUnity(UiBackgroundSize value) =>
            value switch
            {
                UiBackgroundSize.Auto => new UnityBackgroundSize(Length.Auto(), Length.Auto()),
                UiBackgroundSize.Cover => new UnityBackgroundSize(UnityBackgroundSizeType.Cover),
                UiBackgroundSize.Contain => new UnityBackgroundSize(
                    UnityBackgroundSizeType.Contain
                ),
                UiBackgroundSize.Axes axes => new UnityBackgroundSize(
                    ToUnityLength(axes.X),
                    ToUnityLength(axes.Y)
                ),
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI background size."),
            };

        private static Length ToUnityLength(UiLength value) =>
            value switch
            {
                UiLength.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLength.Percent item => new Length(item.Value, LengthUnit.Percent),
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI length kind."),
            };

        private static Length ToUnityLength(UiLengthOrAuto value) =>
            value switch
            {
                UiLengthOrAuto.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLengthOrAuto.Percent item => new Length(item.Value, LengthUnit.Percent),
                UiLengthOrAuto.Auto => Length.Auto(),
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

        private static UnityDisplayStyle ToUnity(ProtocolDisplay value) =>
            value == ProtocolDisplay.Flex ? UnityDisplayStyle.Flex : UnityDisplayStyle.None;

        private static UnityOverflow ToUnity(ProtocolOverflow value) =>
            value == ProtocolOverflow.Visible ? UnityOverflow.Visible : UnityOverflow.Hidden;

        private static UnityOverflowClipBox ToUnity(ProtocolOverflowClipBox value) =>
            value == ProtocolOverflowClipBox.PaddingBox
                ? UnityOverflowClipBox.PaddingBox
                : UnityOverflowClipBox.ContentBox;

        private static UnitySliceType ToUnity(ProtocolSliceType value) =>
            value == ProtocolSliceType.Sliced ? UnitySliceType.Sliced : UnitySliceType.Tiled;

        private static UnityVisibility ToUnity(ProtocolVisibility value) =>
            value == ProtocolVisibility.Visible ? UnityVisibility.Visible : UnityVisibility.Hidden;

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
