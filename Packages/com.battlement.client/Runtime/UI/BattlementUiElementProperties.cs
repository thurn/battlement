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
        private readonly Dictionary<Guid, BattlementUiElementDefaults> defaults = new();
        private readonly BattlementUiEventForwarder events;
        private readonly BattlementUiImageProperties images;
        private readonly BattlementUiButtonProperties buttons;
        private readonly BattlementUiStyleBackgroundProperties styleBackgrounds;
        private readonly BattlementUiStyleCursorProperties styleCursors;
        private readonly BattlementUiStyleMaterialProperties styleMaterials;
        private readonly BattlementUiStyleFontProperties styleFonts;

        public BattlementUiElementProperties(
            Func<UiEvent, bool>? emitUiEvent,
            IBattlementUiAssetLookup? assetLookup
        )
        {
            events = new BattlementUiEventForwarder(emitUiEvent);
            images = new BattlementUiImageProperties(assetLookup);
            buttons = new BattlementUiButtonProperties(assetLookup);
            styleBackgrounds = new BattlementUiStyleBackgroundProperties(assetLookup);
            styleCursors = new BattlementUiStyleCursorProperties(assetLookup);
            styleMaterials = new BattlementUiStyleMaterialProperties(assetLookup);
            styleFonts = new BattlementUiStyleFontProperties(assetLookup);
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
                value.Events,
                value.EventSubscriptions
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
                value.Events,
                value.EventSubscriptions
            );
            if (value is UiElement.Image image)
                images.Apply((UnityEngine.UIElements.Image)target, objectId, image);
            if (value is UiElement.Label label)
                BattlementUiTypographyProperties.Apply(
                    (UnityEngine.UIElements.TextElement)target,
                    label
                );
            if (value is UiElement.TextElement text)
                BattlementUiTypographyProperties.Apply(
                    (UnityEngine.UIElements.TextElement)target,
                    text
                );
            if (value is UiElement.Button button)
            {
                IBattlementUiAssetLease? staged = buttons.Stage(button.Icon);
                try
                {
                    buttons.Apply((UnityEngine.UIElements.Button)target, objectId, button, staged);
                    staged = null;
                }
                finally
                {
                    staged?.Dispose();
                }
            }
            if (value is UiElement.Tab tab)
            {
                IBattlementUiAssetLease? staged = buttons.Stage(tab.Icon);
                try
                {
                    buttons.Apply((UnityEngine.UIElements.Tab)target, objectId, tab, staged);
                    staged = null;
                }
                finally
                {
                    staged?.Dispose();
                }
            }
            if (value is UiElement.RepeatButton repeat)
                BattlementUiTypographyProperties.Apply(
                    (UnityEngine.UIElements.TextElement)target,
                    repeat
                );
            BattlementUiContainerProperties.ApplyCreate(target, value);
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
            IBattlementUiAssetLease? stagedIcon =
                value is UiElement.Button buttonValue ? buttons.Stage(buttonValue.Icon)
                : value is UiElement.Tab tabValue ? buttons.Stage(tabValue.Icon)
                : null;
            IBattlementUiAssetLease? stagedBackground = null;
            IBattlementUiAssetLease? stagedCursor = null;
            IBattlementUiAssetLease? stagedMaterial = null;
            BattlementUiStyleFontProperties.FontLeases? stagedFonts = null;
            try
            {
                BattlementUiElementDefaults initial = defaults[objectId.Value];
                stagedBackground = styleBackgrounds.Stage(value.Style);
                stagedCursor = styleCursors.Stage(value.Style);
                stagedMaterial = styleMaterials.Stage(value.Style);
                stagedFonts = styleFonts.Stage(value.Style);
                initial.Apply(
                    target,
                    value.Name,
                    value.Enabled,
                    value.PickingMode,
                    value.LanguageDirection,
                    value.Focusable,
                    value.TabIndex,
                    value.DelegatesFocus
                );
                authoredClasses[objectId.Value] = BattlementUiElementDefaults.ApplyClasses(
                    target,
                    authoredClasses[objectId.Value],
                    value.Classes
                );
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
                    stagedFonts,
                    ToUnityCursor(value.Style, stagedCursor)
                );
                styleBackgrounds.Commit(objectId.Value, value.Style, stagedBackground);
                stagedBackground = null;
                styleMaterials.Commit(objectId.Value, value.Style, stagedMaterial);
                stagedMaterial = null;
                styleCursors.Commit(objectId.Value, value.Style, stagedCursor);
                stagedCursor = null;
                styleFonts.Commit(objectId.Value, value.Style, stagedFonts);
                stagedFonts = null;
                if (!value.Events.IsUnset || !value.EventSubscriptions.IsUnset)
                {
                    events.SetSubscriptions(
                        objectId.Value,
                        BattlementUiElementDefaults.Values(value.Events),
                        BattlementUiElementDefaults.Values(value.EventSubscriptions),
                        sparse: true
                    );
                }
                switch (value)
                {
                    case UiElement.Label label:
                        BattlementUiTypographyProperties.Apply(
                            (UnityEngine.UIElements.TextElement)target,
                            label
                        );
                        break;
                    case UiElement.TextElement text:
                        BattlementUiTypographyProperties.Apply(
                            (UnityEngine.UIElements.TextElement)target,
                            text
                        );
                        break;
                    case UiElement.Button button:
                        buttons.Apply(
                            (UnityEngine.UIElements.Button)target,
                            objectId,
                            button,
                            stagedIcon
                        );
                        stagedIcon = null;
                        break;
                    case UiElement.RepeatButton repeat:
                        BattlementUiTypographyProperties.Apply(
                            (UnityEngine.UIElements.TextElement)target,
                            repeat
                        );
                        if (repeat.Text is string textValue)
                            ((UnityEngine.UIElements.RepeatButton)target).text = textValue;
                        break;
                    case UiElement.Tab tab:
                        buttons.Apply(
                            (UnityEngine.UIElements.Tab)target,
                            objectId,
                            tab,
                            stagedIcon
                        );
                        stagedIcon = null;
                        break;
                    case UiElement.GroupBox:
                    case UiElement.PopupWindow:
                        BattlementUiContainerProperties.ApplyUpdate(target, value);
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
                stagedIcon?.Dispose();
                stagedBackground?.Dispose();
                stagedCursor?.Dispose();
                stagedMaterial?.Dispose();
                stagedFonts?.Dispose();
            }
        }

        public BattlementUiEventForwarder EventForwarder => events;

        public bool IsSubscribed(ObjectId objectId, UiEventKind kind) =>
            events.IsSubscribed(objectId, kind);

        public void Remove(Guid objectId)
        {
            authoredClasses.Remove(objectId);
            defaults.Remove(objectId);
            events.Remove(objectId);
            images.Remove(objectId);
            buttons.Remove(objectId);
            styleBackgrounds.Remove(objectId);
            styleCursors.Remove(objectId);
            styleMaterials.Remove(objectId);
            styleFonts.Remove(objectId);
        }

        public void Clear()
        {
            authoredClasses.Clear();
            defaults.Clear();
            events.Clear();
            images.Clear();
            buttons.Clear();
            styleBackgrounds.Clear();
            styleCursors.Clear();
            styleMaterials.Clear();
            styleFonts.Clear();
        }

        public static void Validate(UiElement element, bool allowUsageHints) =>
            BattlementUiElementValidator.Validate(element, allowUsageHints);

        private void Apply(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            Prop<string> name,
            Prop<bool> enabled,
            Prop<ProtocolPickingMode> pickingMode,
            Prop<ProtocolLanguageDirection> languageDirection,
            Prop<bool> focusable,
            Prop<int> tabIndex,
            Prop<bool> delegatesFocus,
            Prop<IReadOnlyList<string>> classes,
            IReadOnlyList<ProtocolUsageHint>? usageHints,
            UiStyle? style,
            Prop<IReadOnlyList<UiEventKind>> events,
            Prop<IReadOnlyList<UiEventSubscription>> eventSubscriptions
        )
        {
            var initial = new BattlementUiElementDefaults(target);
            defaults[objectId.Value] = initial;
            authoredClasses[objectId.Value] = new HashSet<string>();
            IBattlementUiAssetLease? stagedBackground = styleBackgrounds.Stage(style);
            IBattlementUiAssetLease? stagedCursor = null;
            IBattlementUiAssetLease? stagedMaterial = null;
            BattlementUiStyleFontProperties.FontLeases? stagedFonts = null;
            try
            {
                stagedCursor = styleCursors.Stage(style);
                stagedMaterial = styleMaterials.Stage(style);
                stagedFonts = styleFonts.Stage(style);
                initial.Apply(
                    target,
                    name,
                    enabled,
                    pickingMode,
                    languageDirection,
                    focusable,
                    tabIndex,
                    delegatesFocus
                );
                if (usageHints is not null)
                    target.usageHints = ToUnity(usageHints);
                authoredClasses[objectId.Value] = BattlementUiElementDefaults.ApplyClasses(
                    target,
                    authoredClasses[objectId.Value],
                    classes
                );
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
                    stagedFonts,
                    ToUnityCursor(style, stagedCursor)
                );
                styleBackgrounds.Commit(objectId.Value, style, stagedBackground);
                stagedBackground = null;
                styleMaterials.Commit(objectId.Value, style, stagedMaterial);
                stagedMaterial = null;
                styleCursors.Commit(objectId.Value, style, stagedCursor);
                stagedCursor = null;
                styleFonts.Commit(objectId.Value, style, stagedFonts);
                stagedFonts = null;
                this.events.SetSubscriptions(
                    objectId.Value,
                    BattlementUiElementDefaults.Values(events),
                    BattlementUiElementDefaults.Values(eventSubscriptions),
                    sparse: false
                );
            }
            finally
            {
                stagedBackground?.Dispose();
                stagedCursor?.Dispose();
                stagedMaterial?.Dispose();
                stagedFonts?.Dispose();
            }
        }

        internal static void ApplyStyle(
            VisualElement element,
            UiStyle? value,
            Background? background,
            Material? material,
            BattlementUiStyleFontProperties.FontLeases? fonts,
            UnityEngine.UIElements.Cursor? cursor
        )
        {
            if (value is null)
                return;
            IStyle target = element.style;
            BattlementUiTypographyProperties.ApplyStyle(target, value, fonts);
            Apply(
                value.AlignContent,
                item => target.alignContent = ToUnity(item),
                keyword => target.alignContent = keyword
            );
            Apply(
                value.AlignItems,
                item => target.alignItems = ToUnity(item),
                keyword => target.alignItems = keyword
            );
            Apply(
                value.AlignSelf,
                item => target.alignSelf = ToUnity(item),
                keyword => target.alignSelf = keyword
            );
            Apply(
                value.AspectRatio,
                item => target.aspectRatio = ToUnity(item),
                keyword => target.aspectRatio = keyword
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
                keyword => target.borderBottomWidth = keyword
            );
            Apply(
                value.BorderLeftColor,
                item => target.borderLeftColor = ToUnity(item),
                () => target.borderLeftColor = StyleKeyword.Initial
            );
            Apply(
                value.BorderLeftWidth,
                item => target.borderLeftWidth = item,
                keyword => target.borderLeftWidth = keyword
            );
            Apply(
                value.BorderRightColor,
                item => target.borderRightColor = ToUnity(item),
                () => target.borderRightColor = StyleKeyword.Initial
            );
            Apply(
                value.BorderRightWidth,
                item => target.borderRightWidth = item,
                keyword => target.borderRightWidth = keyword
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
                keyword => target.borderTopWidth = keyword
            );
            Apply(
                value.Bottom,
                item => target.bottom = ToUnity(item),
                keyword => target.bottom = keyword
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
                keyword => target.display = keyword
            );
            Apply(
                value.Filter,
                item => target.filter = BattlementUiStyleTransformProperties.ToUnity(item),
                () => target.filter = StyleKeyword.Initial
            );
            Apply(
                value.FlexBasis,
                item => target.flexBasis = ToUnity(item),
                keyword => target.flexBasis = keyword
            );
            Apply(
                value.FlexDirection,
                item => target.flexDirection = ToUnity(item),
                keyword => target.flexDirection = keyword
            );
            Apply(
                value.FlexGrow,
                item => target.flexGrow = item,
                keyword => target.flexGrow = keyword
            );
            Apply(
                value.FlexShrink,
                item => target.flexShrink = item,
                keyword => target.flexShrink = keyword
            );
            Apply(
                value.FlexWrap,
                item => target.flexWrap = ToUnity(item),
                keyword => target.flexWrap = keyword
            );
            Apply(
                value.Height,
                item => target.height = ToUnity(item),
                keyword => target.height = keyword
            );
            Apply(
                value.JustifyContent,
                item => target.justifyContent = ToUnity(item),
                keyword => target.justifyContent = keyword
            );
            Apply(
                value.Left,
                item => target.left = ToUnity(item),
                keyword => target.left = keyword
            );
            Apply(
                value.MarginBottom,
                item => target.marginBottom = ToUnity(item),
                keyword => target.marginBottom = keyword
            );
            Apply(
                value.MarginLeft,
                item => target.marginLeft = ToUnity(item),
                keyword => target.marginLeft = keyword
            );
            Apply(
                value.MarginRight,
                item => target.marginRight = ToUnity(item),
                keyword => target.marginRight = keyword
            );
            Apply(
                value.MarginTop,
                item => target.marginTop = ToUnity(item),
                keyword => target.marginTop = keyword
            );
            Apply(
                value.MaxHeight,
                item => target.maxHeight = ToUnity(item),
                keyword => target.maxHeight = keyword
            );
            Apply(
                value.MaxWidth,
                item => target.maxWidth = ToUnity(item),
                keyword => target.maxWidth = keyword
            );
            Apply(
                value.MinHeight,
                item => target.minHeight = ToUnity(item),
                keyword => target.minHeight = keyword
            );
            Apply(
                value.MinWidth,
                item => target.minWidth = ToUnity(item),
                keyword => target.minWidth = keyword
            );
            Apply(
                value.Opacity,
                item => target.opacity = item,
                () => target.opacity = StyleKeyword.Initial
            );
            Apply(
                value.Overflow,
                item => target.overflow = ToUnity(item),
                keyword => target.overflow = keyword
            );
            Apply(
                value.PaddingBottom,
                item => target.paddingBottom = ToUnity(item),
                keyword => target.paddingBottom = keyword
            );
            Apply(
                value.PaddingLeft,
                item => target.paddingLeft = ToUnity(item),
                keyword => target.paddingLeft = keyword
            );
            Apply(
                value.PaddingRight,
                item => target.paddingRight = ToUnity(item),
                keyword => target.paddingRight = keyword
            );
            Apply(
                value.PaddingTop,
                item => target.paddingTop = ToUnity(item),
                keyword => target.paddingTop = keyword
            );
            Apply(
                value.Position,
                item => target.position = ToUnity(item),
                keyword => target.position = keyword
            );
            Apply(
                value.Right,
                item => target.right = ToUnity(item),
                keyword => target.right = keyword
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
            Apply(value.Top, item => target.top = ToUnity(item), keyword => target.top = keyword);
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
                keyword => target.width = keyword
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

        private static void Apply<T>(
            Prop<UiStyleValue<T>> value,
            System.Action<T> concrete,
            System.Action<StyleKeyword> keyword
        )
        {
            if (value.IsUnset)
                return;
            if (value.IsReset)
            {
                keyword(StyleKeyword.Null);
                return;
            }
            Apply(value.Value, concrete, () => keyword(StyleKeyword.Initial));
        }

        internal static UnityEngine.UIElements.Cursor? ToUnityCursor(
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

        internal static UnityPickingMode ToUnity(ProtocolPickingMode value) =>
            value == ProtocolPickingMode.Position
                ? UnityPickingMode.Position
                : UnityPickingMode.Ignore;

        internal static UnityLanguageDirection ToUnity(ProtocolLanguageDirection value) =>
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
