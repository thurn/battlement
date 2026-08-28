#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiPartProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<PartKey, PartAssets> partAssets = new();
        private readonly BattlementUiPartStyleState styleState = new();
        private readonly Dictionary<(Guid, PartKey), System.Action> deferredCancellations = new();

        public BattlementUiPartProperties(IBattlementUiAssetLookup? assetLookup) =>
            assets = assetLookup;

        public void Apply(VisualElement owner, ObjectId objectId, UiElement value)
        {
            using PreparedUpdate prepared = Prepare(owner, objectId, value);
            prepared.Commit(objectId.Value);
        }

        public PreparedUpdate Prepare(VisualElement owner, ObjectId objectId, UiElement value)
        {
            IReadOnlyList<UiPartStyle>? declarations = Parts(value);
            List<UiPartStyle> effective = styleState.EffectiveDeclarations(
                objectId.Value,
                value,
                declarations ?? Array.Empty<UiPartStyle>()
            );
            var staged = new List<StagedPart>(effective.Count);
            try
            {
                foreach (UiPartStyle declaration in effective)
                {
                    PartAssets retained = AssetsFor(
                        new PartKey(declaration.Part, declaration.Index)
                    );
                    staged.Add(
                        new StagedPart(
                            this,
                            owner,
                            declaration,
                            MayMaterialize(value, declaration.Part)
                                ? null
                                : BattlementUiPartCatalog.Resolve(
                                    owner,
                                    declaration.Part,
                                    declaration.Index
                                ),
                            retained,
                            retained.Stage(declaration.Style)
                        )
                    );
                }
                return new PreparedUpdate(
                    this,
                    staged,
                    styleState.RemovedParts(objectId.Value, value, RemovedConditionalParts(value))
                );
            }
            catch
            {
                foreach (StagedPart part in staged)
                    part.Dispose();
                throw;
            }
        }

        public void Remove(Guid objectId)
        {
            CancelDeferred(objectId);
            foreach (PartAssets value in partAssets.Values)
                value.Remove(objectId);
            styleState.Remove(objectId);
        }

        public void Clear()
        {
            foreach (System.Action cancel in deferredCancellations.Values)
                cancel();
            deferredCancellations.Clear();
            foreach (PartAssets value in partAssets.Values)
                value.Clear();
            partAssets.Clear();
            styleState.Clear();
        }

        private PartAssets AssetsFor(PartKey part)
        {
            if (!partAssets.TryGetValue(part, out PartAssets value))
            {
                value = new PartAssets(assets);
                partAssets.Add(part, value);
            }
            return value;
        }

        private void ReplaceDeferred(Guid objectId, PartKey key, System.Action cancel)
        {
            if (deferredCancellations.Remove((objectId, key), out System.Action previous))
                previous();
            deferredCancellations.Add((objectId, key), cancel);
        }

        private void CompleteDeferred(Guid objectId, PartKey key, System.Action cancel)
        {
            if (
                deferredCancellations.TryGetValue((objectId, key), out System.Action current)
                && current == cancel
            )
                deferredCancellations.Remove((objectId, key));
        }

        private void CancelDeferred(Guid objectId)
        {
            var keys = new List<(Guid, PartKey)>();
            foreach ((Guid, PartKey) key in deferredCancellations.Keys)
                if (key.Item1 == objectId)
                    keys.Add(key);
            foreach ((Guid, PartKey) key in keys)
            {
                deferredCancellations[key]();
                deferredCancellations.Remove(key);
            }
        }

        private void CancelDeferred(Guid objectId, PartKey key)
        {
            if (deferredCancellations.Remove((objectId, key), out System.Action cancel))
                cancel();
        }

        private void RemovePart(Guid objectId, PartKey key)
        {
            CancelDeferred(objectId, key);
            if (partAssets.TryGetValue(key, out PartAssets retained))
                retained.Remove(objectId);
            styleState.Remove(objectId, key);
        }

        private static IReadOnlyList<UiPartStyle>? Parts(UiElement value) =>
            value switch
            {
                UiElement.Button item => item.Parts,
                UiElement.GroupBox item => item.Parts,
                UiElement.PopupWindow item => item.Parts,
                UiElement.Toggle item => item.Parts,
                UiElement.RadioButton item => item.Parts,
                UiElement.DropdownField item => item.Parts,
                UiElement.ProgressBar item => item.Parts,
                UiElement.ScrollView item => item.Parts,
                UiElement.Scroller item => item.Parts,
                UiElement.Tab item => item.Parts,
                UiElement.TabView item => item.Parts,
                UiElement.TextField item => item.Parts,
                UiElement.RadioButtonGroup item => item.Parts,
                UiElement.ToggleButtonGroup item => item.Parts,
                UiElement.Slider item => item.Parts,
                UiElement.SliderInt item => item.Parts,
                UiElement.MinMaxSlider item => item.Parts,
                _ => null,
            };

        private static IReadOnlyList<PartKey> RemovedConditionalParts(UiElement value)
        {
            var removed = new List<PartKey>();
            if (value is UiElement.GroupBox { Text: "" })
                removed.Add(new PartKey(UiPart.GroupBoxTitle, null));
            if (value is UiElement.Tab { Closeable: false })
                removed.Add(new PartKey(UiPart.TabCloseButton, null));
            if (value is UiElement.TextField { Multiline: false })
                AddTextFieldMultilineParts(removed);
            if (value is UiElement.Slider { Fill: false })
                removed.Add(new PartKey(UiPart.SliderFill, null));
            if (value is UiElement.Slider { ShowInputField: false })
                removed.Add(new PartKey(UiPart.SliderTextInput, null));
            if (value is UiElement.SliderInt { Fill: false })
                removed.Add(new PartKey(UiPart.SliderIntFill, null));
            if (value is UiElement.SliderInt { ShowInputField: false })
                removed.Add(new PartKey(UiPart.SliderIntTextInput, null));
            return removed;
        }

        private static bool MayMaterialize(UiElement value, UiPart part) =>
            (value, part) switch
            {
                (UiElement.GroupBox { Text: not null and not "" }, UiPart.GroupBoxTitle) => true,
                (UiElement.Button { Icon: not null }, UiPart.ButtonIcon) => true,
                (UiElement.Toggle { Label: not null }, UiPart.ToggleLabel) => true,
                (UiElement.Toggle { Text: not null }, UiPart.ToggleText) => true,
                (UiElement.RadioButton { Label: not null }, UiPart.RadioButtonLabel) => true,
                (UiElement.RadioButton { Text: not null }, UiPart.RadioButtonText) => true,
                (UiElement.DropdownField { Label: not null }, UiPart.DropdownFieldLabel) => true,
                (UiElement.Tab { Icon: not null }, UiPart.TabIcon) => true,
                (UiElement.Tab { Closeable: true }, UiPart.TabCloseButton) => true,
                (UiElement.TextField { Label: not null }, UiPart.TextFieldLabel) => true,
                (
                    UiElement.TextField { Multiline: true },
                    UiPart.TextFieldMultilineScrollView
                        or UiPart.TextFieldVerticalScroller
                        or UiPart.TextFieldVerticalSlider
                        or UiPart.TextFieldVerticalLowButton
                        or UiPart.TextFieldVerticalHighButton
                        or UiPart.TextFieldVerticalTrack
                        or UiPart.TextFieldVerticalDragger
                        or UiPart.TextFieldVerticalDraggerBorder
                ) => true,
                (UiElement.RadioButtonGroup { Label: not null }, UiPart.RadioButtonGroupLabel) =>
                    true,
                (
                    UiElement.RadioButtonGroup { Choices: not null },
                    UiPart.RadioButtonGroupAllOptions
                        or UiPart.RadioButtonGroupOption
                        or UiPart.RadioButtonGroupOptionCheckmarkBackground
                        or UiPart.RadioButtonGroupOptionCheckmark
                        or UiPart.RadioButtonGroupOptionText
                ) => true,
                (UiElement.ToggleButtonGroup { Label: not null }, UiPart.ToggleButtonGroupLabel) =>
                    true,
                (UiElement.Slider { Label: not null }, UiPart.SliderLabel) => true,
                (UiElement.Slider { Fill: true }, UiPart.SliderFill) => true,
                (UiElement.Slider { ShowInputField: true }, UiPart.SliderTextInput) => true,
                (UiElement.SliderInt { Label: not null }, UiPart.SliderIntLabel) => true,
                (UiElement.SliderInt { Fill: true }, UiPart.SliderIntFill) => true,
                (UiElement.SliderInt { ShowInputField: true }, UiPart.SliderIntTextInput) => true,
                (UiElement.MinMaxSlider { Label: not null }, UiPart.MinMaxSliderLabel) => true,
                _ => false,
            };

        private static void AddTextFieldMultilineParts(List<PartKey> removed)
        {
            UiPart[] parts =
            {
                UiPart.TextFieldMultilineScrollView,
                UiPart.TextFieldVerticalScroller,
                UiPart.TextFieldVerticalSlider,
                UiPart.TextFieldVerticalLowButton,
                UiPart.TextFieldVerticalHighButton,
                UiPart.TextFieldVerticalTrack,
                UiPart.TextFieldVerticalDragger,
                UiPart.TextFieldVerticalDraggerBorder,
            };
            foreach (UiPart part in parts)
                removed.Add(new PartKey(part, null));
        }

        internal sealed record PartKey(UiPart Part, uint? Index);

        internal sealed class PartAssets
        {
            private readonly BattlementUiStyleBackgroundProperties backgrounds;
            private readonly BattlementUiStyleCursorProperties cursors;
            private readonly BattlementUiStyleMaterialProperties materials;
            private readonly BattlementUiStyleFontProperties fonts;

            public PartAssets(IBattlementUiAssetLookup? assets)
            {
                backgrounds = new BattlementUiStyleBackgroundProperties(assets);
                cursors = new BattlementUiStyleCursorProperties(assets);
                materials = new BattlementUiStyleMaterialProperties(assets);
                fonts = new BattlementUiStyleFontProperties(assets);
            }

            public StagedAssets Stage(UiStyle style)
            {
                IBattlementUiAssetLease? background = null;
                IBattlementUiAssetLease? cursor = null;
                IBattlementUiAssetLease? material = null;
                BattlementUiStyleFontProperties.FontLeases? font = null;
                try
                {
                    background = backgrounds.Stage(style);
                    cursor = cursors.Stage(style);
                    material = materials.Stage(style);
                    font = fonts.Stage(style);
                    var staged = new StagedAssets(background, cursor, material, font);
                    background = null;
                    cursor = null;
                    material = null;
                    font = null;
                    return staged;
                }
                finally
                {
                    background?.Dispose();
                    cursor?.Dispose();
                    material?.Dispose();
                    font?.Dispose();
                }
            }

            public void Commit(Guid objectId, UiStyle style, StagedAssets staged)
            {
                backgrounds.Commit(objectId, style, staged.Background);
                staged.Background = null;
                cursors.Commit(objectId, style, staged.Cursor);
                staged.Cursor = null;
                materials.Commit(objectId, style, staged.Material);
                staged.Material = null;
                fonts.Commit(objectId, style, staged.Fonts!);
                staged.Fonts = null;
            }

            public void Remove(Guid objectId)
            {
                backgrounds.Remove(objectId);
                cursors.Remove(objectId);
                materials.Remove(objectId);
                fonts.Remove(objectId);
            }

            public void Clear()
            {
                backgrounds.Clear();
                cursors.Clear();
                materials.Clear();
                fonts.Clear();
            }
        }

        internal sealed class PreparedUpdate : IDisposable
        {
            private readonly BattlementUiPartProperties owner;
            private readonly List<StagedPart> staged;
            private readonly IReadOnlyList<PartKey> removedParts;
            private bool committed;

            internal PreparedUpdate(
                BattlementUiPartProperties owner,
                List<StagedPart> staged,
                IReadOnlyList<PartKey> removedParts
            )
            {
                this.owner = owner;
                this.staged = staged;
                this.removedParts = removedParts;
            }

            public void Commit(Guid objectId)
            {
                foreach (StagedPart part in staged)
                    part.Commit(objectId);
                foreach (PartKey removed in removedParts)
                    owner.RemovePart(objectId, removed);
                committed = true;
            }

            public void Dispose()
            {
                if (committed)
                    return;
                foreach (StagedPart part in staged)
                    part.Dispose();
            }
        }

        internal sealed class StagedPart : IDisposable
        {
            private readonly BattlementUiPartProperties properties;
            private readonly VisualElement owner;
            private readonly UiPartStyle declaration;
            private readonly IReadOnlyList<VisualElement>? preparedTargets;
            private readonly PartAssets retained;
            private StagedAssets? staged;

            public StagedPart(
                BattlementUiPartProperties properties,
                VisualElement owner,
                UiPartStyle declaration,
                IReadOnlyList<VisualElement>? preparedTargets,
                PartAssets retained,
                StagedAssets staged
            )
            {
                this.properties = properties;
                this.owner = owner;
                this.declaration = declaration;
                this.preparedTargets = preparedTargets;
                this.retained = retained;
                this.staged = staged;
            }

            public void Commit(Guid objectId)
            {
                StagedAssets value = staged!;
                Background? background = value.Background is null
                    ? null
                    : BattlementUiStyleBackgroundProperties.ToUnity(
                        declaration.Style.BackgroundImage.Value!.Value,
                        value.Background.Value
                    );
                Material? material = value.Material?.Value as Material;
                BattlementUiStyleFontProperties.FontLeases fonts = value.Fonts!;
                UnityEngine.UIElements.Cursor? cursor = BattlementUiElementProperties.ToUnityCursor(
                    declaration.Style,
                    value.Cursor
                );
                if (IsDeferredFill(declaration.Part) && preparedTargets is null)
                    ApplyDeferredFill(objectId, background, material, fonts, cursor);
                else
                    foreach (
                        VisualElement target in preparedTargets
                            ?? BattlementUiPartCatalog.Resolve(
                                owner,
                                declaration.Part,
                                declaration.Index
                            )
                    )
                        Apply(target, background, material, fonts, cursor);
                retained.Commit(objectId, declaration.Style, value);
                properties.styleState.Record(
                    objectId,
                    new PartKey(declaration.Part, declaration.Index),
                    declaration.Style
                );
                staged = null;
            }

            private void ApplyDeferredFill(
                Guid objectId,
                Background? background,
                Material? material,
                BattlementUiStyleFontProperties.FontLeases fonts,
                UnityEngine.UIElements.Cursor? cursor
            )
            {
                UiPart draggerPart =
                    declaration.Part == UiPart.SliderFill
                        ? UiPart.SliderDragger
                        : UiPart.SliderIntDragger;
                VisualElement dragger = BattlementUiPartCatalog.Resolve(owner, draggerPart, null)[
                    0
                ];
                var key = new PartKey(declaration.Part, declaration.Index);
                EventCallback<GeometryChangedEvent> materialized = null!;
                System.Action cancel = () => dragger.UnregisterCallback(materialized);
                materialized = _ =>
                {
                    bool enabled = owner is UnityEngine.UIElements.Slider slider
                        ? slider.fill
                        : ((UnityEngine.UIElements.SliderInt)owner).fill;
                    if (!enabled)
                    {
                        cancel();
                        properties.CompleteDeferred(objectId, key, cancel);
                        return;
                    }
                    List<VisualElement> targets = owner
                        .Query<VisualElement>(
                            className: UnityEngine.UIElements.Slider.fillUssClassName
                        )
                        .ToList();
                    if (targets.Count == 0)
                        return;
                    if (targets.Count != 1)
                        throw new UnityException(
                            $"Native slider fill matched {targets.Count} elements."
                        );
                    cancel();
                    properties.CompleteDeferred(objectId, key, cancel);
                    Apply(targets[0], background, material, fonts, cursor);
                };
                dragger.RegisterCallback(materialized);
                properties.ReplaceDeferred(objectId, key, cancel);
            }

            private void Apply(
                VisualElement target,
                Background? background,
                Material? material,
                BattlementUiStyleFontProperties.FontLeases fonts,
                UnityEngine.UIElements.Cursor? cursor
            ) =>
                BattlementUiElementProperties.ApplyStyle(
                    target,
                    declaration.Style,
                    background,
                    material,
                    fonts,
                    cursor
                );

            private static bool IsDeferredFill(UiPart part) =>
                part is UiPart.SliderFill or UiPart.SliderIntFill;

            public void Dispose() => staged?.Dispose();
        }

        internal sealed class StagedAssets : IDisposable
        {
            public StagedAssets(
                IBattlementUiAssetLease? background,
                IBattlementUiAssetLease? cursor,
                IBattlementUiAssetLease? material,
                BattlementUiStyleFontProperties.FontLeases fonts
            )
            {
                Background = background;
                Cursor = cursor;
                Material = material;
                Fonts = fonts;
            }

            public IBattlementUiAssetLease? Background { get; set; }
            public IBattlementUiAssetLease? Cursor { get; set; }
            public IBattlementUiAssetLease? Material { get; set; }
            public BattlementUiStyleFontProperties.FontLeases? Fonts { get; set; }

            public void Dispose()
            {
                Background?.Dispose();
                Cursor?.Dispose();
                Material?.Dispose();
                Fonts?.Dispose();
            }
        }
    }
}
