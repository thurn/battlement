#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;
using NativeDropdownField = UnityEngine.UIElements.DropdownField;
using NativeGroupBox = UnityEngine.UIElements.GroupBox;
using NativePopupWindow = UnityEngine.UIElements.PopupWindow;
using NativeProgressBar = UnityEngine.UIElements.ProgressBar;
using NativeRadioButton = UnityEngine.UIElements.RadioButton;
using NativeToggle = UnityEngine.UIElements.Toggle;

namespace Battlement.UI
{
    internal sealed class BattlementUiPartProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<UiPart, PartAssets> partAssets = new();

        public BattlementUiPartProperties(IBattlementUiAssetLookup? assetLookup) =>
            assets = assetLookup;

        public void Apply(VisualElement owner, ObjectId objectId, UiElement value)
        {
            using PreparedUpdate prepared = Prepare(owner, value);
            prepared.Commit(objectId.Value);
        }

        public PreparedUpdate Prepare(VisualElement owner, UiElement value)
        {
            IReadOnlyList<UiPartStyle>? declarations = Parts(value);
            var staged = new List<StagedPart>(declarations?.Count ?? 0);
            try
            {
                foreach (UiPartStyle declaration in declarations ?? Array.Empty<UiPartStyle>())
                {
                    PartAssets retained = AssetsFor(declaration.Part);
                    staged.Add(
                        new StagedPart(
                            Resolve(owner, declaration.Part),
                            declaration,
                            retained,
                            retained.Stage(declaration.Style)
                        )
                    );
                }
                return new PreparedUpdate(this, staged, RemovedConditionalPart(value));
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
            foreach (PartAssets value in partAssets.Values)
                value.Remove(objectId);
        }

        public void Clear()
        {
            foreach (PartAssets value in partAssets.Values)
                value.Clear();
            partAssets.Clear();
        }

        private PartAssets AssetsFor(UiPart part)
        {
            if (!partAssets.TryGetValue(part, out PartAssets value))
            {
                value = new PartAssets(assets);
                partAssets.Add(part, value);
            }
            return value;
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
                _ => null,
            };

        private static UiPart? RemovedConditionalPart(UiElement value) =>
            value is UiElement.GroupBox { Text: "" } ? UiPart.GroupBoxTitle : null;

        private static VisualElement Resolve(VisualElement owner, UiPart part) =>
            part switch
            {
                UiPart.ButtonIcon => Require(owner, NativeButton.iconUssClassName),
                UiPart.GroupBoxTitle => Require(owner, NativeGroupBox.labelUssClassName),
                UiPart.PopupWindowContentContainer => ((NativePopupWindow)owner).contentContainer,
                UiPart.ToggleLabel => Require(owner, NativeToggle.labelUssClassName),
                UiPart.ToggleInput => Require(owner, NativeToggle.inputUssClassName),
                UiPart.ToggleCheckmark => Require(owner, NativeToggle.checkmarkUssClassName),
                UiPart.ToggleText => Require(owner, NativeToggle.textUssClassName),
                UiPart.RadioButtonLabel => Require(owner, NativeRadioButton.labelUssClassName),
                UiPart.RadioButtonInput => Require(owner, NativeRadioButton.inputUssClassName),
                UiPart.RadioButtonCheckmarkBackground => Require(
                    owner,
                    NativeRadioButton.checkmarkBackgroundUssClassName
                ),
                UiPart.RadioButtonCheckmark => Require(
                    owner,
                    NativeRadioButton.checkmarkUssClassName
                ),
                UiPart.RadioButtonText => Require(owner, NativeRadioButton.textUssClassName),
                UiPart.DropdownFieldLabel => Require(owner, NativeDropdownField.labelUssClassName),
                UiPart.DropdownFieldInput => Require(owner, NativeDropdownField.inputUssClassName),
                UiPart.DropdownFieldText => Require(owner, NativeDropdownField.textUssClassName),
                UiPart.DropdownFieldArrow => Require(owner, NativeDropdownField.arrowUssClassName),
                UiPart.ProgressBarContainer => Require(
                    owner,
                    NativeProgressBar.containerUssClassName
                ),
                UiPart.ProgressBarBackground => Require(
                    owner,
                    NativeProgressBar.backgroundUssClassName
                ),
                UiPart.ProgressBarProgress => Require(
                    owner,
                    NativeProgressBar.progressUssClassName
                ),
                UiPart.ProgressBarTitleContainer => Require(
                    owner,
                    NativeProgressBar.titleContainerUssClassName
                ),
                UiPart.ProgressBarTitle => Require(owner, NativeProgressBar.titleUssClassName),
                _ => throw Failure($"Unsupported UI part {part}."),
            };

        private static VisualElement Require(VisualElement owner, string className)
        {
            List<VisualElement> matches = owner.Query<VisualElement>(className: className).ToList();
            if (matches.Count != 1)
            {
                throw new UnityException(
                    $"Native part .{className} matched {matches.Count} elements "
                        + $"beneath {owner.GetType().Name}."
                );
            }
            return matches[0];
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

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
            private readonly UiPart? removedPart;
            private bool committed;

            internal PreparedUpdate(
                BattlementUiPartProperties owner,
                List<StagedPart> staged,
                UiPart? removedPart
            )
            {
                this.owner = owner;
                this.staged = staged;
                this.removedPart = removedPart;
            }

            public void Commit(Guid objectId)
            {
                foreach (StagedPart part in staged)
                    part.Commit(objectId);
                if (removedPart is UiPart removed)
                    owner.AssetsFor(removed).Remove(objectId);
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
            private readonly VisualElement target;
            private readonly UiPartStyle declaration;
            private readonly PartAssets retained;
            private StagedAssets? staged;

            public StagedPart(
                VisualElement target,
                UiPartStyle declaration,
                PartAssets retained,
                StagedAssets staged
            )
            {
                this.target = target;
                this.declaration = declaration;
                this.retained = retained;
                this.staged = staged;
            }

            public void Commit(Guid objectId)
            {
                StagedAssets value = staged!;
                BattlementUiElementProperties.ApplyStyle(
                    target,
                    declaration.Style,
                    value.Background is null
                        ? null
                        : BattlementUiStyleBackgroundProperties.ToUnity(
                            declaration.Style.BackgroundImage!.Value,
                            value.Background.Value
                        ),
                    value.Material?.Value as Material,
                    value.Fonts,
                    BattlementUiElementProperties.ToUnityCursor(declaration.Style, value.Cursor)
                );
                retained.Commit(objectId, declaration.Style, value);
                staged = null;
            }

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
