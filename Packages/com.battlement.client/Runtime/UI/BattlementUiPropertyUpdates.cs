#nullable enable

using System;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    /// <summary>Admits and applies one sparse UI element property update.</summary>
    internal sealed class BattlementUiPropertyUpdates
    {
        private readonly BattlementUiHierarchy hierarchy;
        private readonly Func<ObjectId, VisualElement> require;
        private readonly BattlementUiPlacementValidator placementValidator;
        private readonly BattlementUiElementProperties properties;
        private readonly BattlementFocusCoordinator focusCoordinator;
        private readonly BattlementOverlayCoordinator overlayCoordinator;
        private readonly BattlementStickyCoordinator stickyCoordinator;
        private readonly BattlementMotionWorld motionWorld;
        private readonly BattlementUiPartProperties partProperties;
        private readonly BattlementUiScrollControls scrollControls;
        private readonly BattlementUiTabControls tabControls;
        private readonly BattlementUiTextFieldControls textFieldControls;
        private readonly BattlementUiBooleanControls booleanControls;
        private readonly BattlementUiChoiceControls choiceControls;
        private readonly BattlementUiDropdownControls dropdownControls;
        private readonly BattlementUiSliderControls sliderControls;
        private readonly BattlementUiRangeControls rangeControls;
        private readonly BattlementUiRepeatControls repeatControls;

        internal BattlementUiPropertyUpdates(
            BattlementUiHierarchy hierarchy,
            Func<ObjectId, VisualElement> require,
            BattlementUiPlacementValidator placementValidator,
            BattlementUiElementProperties properties,
            BattlementFocusCoordinator focusCoordinator,
            BattlementOverlayCoordinator overlayCoordinator,
            BattlementStickyCoordinator stickyCoordinator,
            BattlementMotionWorld motionWorld,
            BattlementUiPartProperties partProperties,
            BattlementUiScrollControls scrollControls,
            BattlementUiTabControls tabControls,
            BattlementUiTextFieldControls textFieldControls,
            BattlementUiBooleanControls booleanControls,
            BattlementUiChoiceControls choiceControls,
            BattlementUiDropdownControls dropdownControls,
            BattlementUiSliderControls sliderControls,
            BattlementUiRangeControls rangeControls,
            BattlementUiRepeatControls repeatControls
        )
        {
            this.hierarchy = hierarchy;
            this.require = require;
            this.placementValidator = placementValidator;
            this.properties = properties;
            this.focusCoordinator = focusCoordinator;
            this.overlayCoordinator = overlayCoordinator;
            this.stickyCoordinator = stickyCoordinator;
            this.motionWorld = motionWorld;
            this.partProperties = partProperties;
            this.scrollControls = scrollControls;
            this.tabControls = tabControls;
            this.textFieldControls = textFieldControls;
            this.booleanControls = booleanControls;
            this.choiceControls = choiceControls;
            this.dropdownControls = dropdownControls;
            this.sliderControls = sliderControls;
            this.rangeControls = rangeControls;
            this.repeatControls = repeatControls;
        }

        internal void Apply(VisualElementUpdate.Properties update)
        {
            VisualElement target = require(update.ObjectId);
            UiElement value = update.Element;
            bool genericRootUpdate =
                hierarchy.IsRoot(update.ObjectId.Value) && value is UiElement.VisualElement;
            if (!genericRootUpdate)
                RequireElementKind(target, value, update.ObjectId);
            ValidateLayoutUpdate(target, value);
            ValidateStickyUpdate(target, update.ObjectId, value);
            ValidateOverlayUpdate(target, update.ObjectId, value);
            BattlementUiElementProperties.Validate(value, allowUsageHints: false);
            BattlementUiChoiceControls.ValidateUpdate(
                value,
                target,
                hierarchy.Children(update.ObjectId.Value).Count
            );
            BattlementUiDropdownControls.ValidateUpdate(value, target);
            BattlementUiSliderControls.ValidateUpdate(value, target);
            BattlementUiRangeControls.ValidateUpdate(value, target);
            BattlementUiScrollControls.ValidateUpdate(target, value);
            BattlementUiTabControls.ValidateUpdate(target, value);
            textFieldControls.ValidateUpdate(update.ObjectId, value);
            focusCoordinator.ValidateUpdate(target, value);

            using BattlementUiPartProperties.PreparedUpdate preparedParts = partProperties.Prepare(
                target,
                update.ObjectId,
                value
            );
            using BattlementPreparedUiPropertyUpdate preparedProperties = properties.PrepareUpdate(
                target,
                update.ObjectId,
                value
            );
            using BattlementPreparedMotionAdmission? preparedMotion = motionWorld.Prepare(
                target,
                update.ObjectId,
                value.Motion,
                value.Paint
            );
            System.Action? commitStyle = motionWorld.PrepareStyle(update.ObjectId, value.Style);
            preparedProperties.Commit();
            commitStyle?.Invoke();
            BattlementPaintProperties.Apply(target, value.Paint);
            if (!value.Paint.IsUnset)
                motionWorld.CommitPaint(update.ObjectId);
            focusCoordinator.ApplyUpdate(target, value);
            BattlementGridItems.Apply(target, value.GridItem);
            BattlementStackItems.Apply(target, value.StackItem);
            BattlementStickyItems.Apply(target, value.Sticky);
            if (target is BattlementLayoutContainer layout && value is UiElement.Flex flex)
                layout.ApplyFlex(flex);
            if (target is BattlementLayoutContainer gridLayout && value is UiElement.Grid grid)
                gridLayout.ApplyGrid(grid);
            if (target is BattlementLayoutContainer stackLayout && value is UiElement.Stack stack)
                stackLayout.ApplyStack(stack);
            RefreshParentLayout(update.ObjectId.Value);
            stickyCoordinator.Apply(target, value.Sticky, hierarchy.SourceOrdinal(target));
            overlayCoordinator.Apply(target, value.OverlayPlacement);
            scrollControls.ApplyUpdate(target, update.ObjectId, value);
            tabControls.ApplyUpdate(target, update.ObjectId, value);
            textFieldControls.ApplyUpdate(target, update.ObjectId, value);
            booleanControls.ApplyUpdate(target, update.ObjectId, value);
            choiceControls.ApplyUpdate(target, update.ObjectId, value);
            dropdownControls.ApplyUpdate(target, update.ObjectId, value);
            sliderControls.ApplyUpdate(target, update.ObjectId, value);
            rangeControls.ApplyUpdate(target, update.ObjectId, value);
            preparedParts.Commit(update.ObjectId.Value);
            preparedMotion?.Commit();
            if (value is UiElement.RepeatButton repeat)
                repeatControls.ApplyUpdate(
                    (UnityEngine.UIElements.RepeatButton)target,
                    update.ObjectId,
                    repeat
                );
            overlayCoordinator.RefreshAll();
            focusCoordinator.Refresh();
        }

        private void ValidateOverlayUpdate(
            VisualElement target,
            ObjectId objectId,
            UiElement element
        )
        {
            OverlayPlacement? current = BattlementOverlayItems.HasAuthored(target)
                ? BattlementOverlayItems.Get(target)
                : null;
            bool modal = element.OverlayPlacement.IsSet
                ? element.OverlayPlacement.Value is OverlayPlacement.Modal
                : element.OverlayPlacement.IsUnset && current is OverlayPlacement.Modal;
            if (modal)
                ValidateModalFocusProperties(target, objectId, element);
            if (!element.OverlayPlacement.IsSet)
            {
                if (element.OverlayPlacement.IsUnset && BattlementOverlayItems.HasAuthored(target))
                    BattlementUiElementValidator.ValidateOverlayStyle(
                        element.Style,
                        BattlementOverlayItems.Get(target)
                    );
                return;
            }
            VisualElement parent =
                target.hierarchy.parent
                ?? throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Overlay wrapper is not attached."
                );
            placementValidator.ValidateOverlayHost(parent);
            overlayCoordinator.Validate(
                objectId,
                element.OverlayPlacement.Value,
                parent,
                hierarchy.IsDescendant
            );
        }

        private void ValidateStickyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            bool remainsSticky =
                value.Sticky.IsSet
                || (value.Sticky.IsUnset && BattlementStickyItems.HasAuthored(target));
            if (!remainsSticky)
                return;
            Guid parentId =
                hierarchy.ParentId(objectId.Value)
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            if (
                !hierarchy.TryGet(new ObjectId(parentId), out VisualElement? parent)
                || parent is null
                || !BattlementUiPlacementValidator.HasScrollAncestor(parent)
            )
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
            bool absolute =
                properties.ResolvePosition(target, objectId, value.Style) == Position.Absolute;
            if (absolute)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires relative positioning."
                );
        }

        private static void ValidateLayoutUpdate(VisualElement target, UiElement value)
        {
            bool parentIsGrid =
                target.parent is BattlementLayoutSlot slot
                && slot.parent
                    is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Grid };
            if (value.GridItem.IsSet && !parentIsGrid)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "GridItem requires a direct Grid placement context."
                );
            if (parentIsGrid)
                BattlementUiPlacementValidator.ValidateLayoutStyle(value.Style, "Grid");
            bool parentIsStack =
                target.parent is BattlementLayoutSlot stackSlot
                && stackSlot.parent
                    is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack };
            if (value.StackItem.IsSet && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "StackItem requires a direct Stack placement context."
                );
            if (parentIsStack)
                BattlementUiPlacementValidator.ValidateLayoutStyle(value.Style, "Stack");
        }

        private void ValidateModalFocusProperties(
            VisualElement target,
            ObjectId objectId,
            UiElement element
        )
        {
            BattlementUiElementDefaults initial = properties.Initial(objectId);
            bool enabled =
                element.Enabled.IsSet ? element.Enabled.Value
                : element.Enabled.IsReset ? initial.Enabled
                : target.enabledSelf;
            (bool focusable, int tabIndex, bool inert) = focusCoordinator.ResolveUpdate(
                target,
                element.Focusable,
                element.TabIndex,
                element.Inert,
                initial.Focusable,
                initial.TabIndex,
                defaultInert: false
            );
            if (!enabled)
                throw Failure(CoreErrorCode.InvalidProperty, "A modal wrapper must be enabled.");
            if (!focusable)
                throw Failure(CoreErrorCode.InvalidProperty, "A modal wrapper must be focusable.");
            if (tabIndex != -1)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "A modal wrapper must use tab index -1."
                );
            if (inert)
                throw Failure(CoreErrorCode.InvalidProperty, "A modal wrapper cannot be inert.");
        }

        private static void RequireElementKind(
            VisualElement target,
            UiElement element,
            ObjectId objectId
        )
        {
            bool matches = element switch
            {
                UiElement.VisualElement => target is BattlementPaintHost
                    || target.GetType() == typeof(VisualElement),
                UiElement.Flex => target is BattlementLayoutContainer layout
                    && layout.Kind == BattlementLayoutContainerKind.Flex,
                UiElement.Grid => target is BattlementLayoutContainer grid
                    && grid.Kind == BattlementLayoutContainerKind.Grid,
                UiElement.Stack => target is BattlementLayoutContainer stack
                    && stack.Kind == BattlementLayoutContainerKind.Stack,
                UiElement.Box => target.GetType() == typeof(Box),
                UiElement.Label => target.GetType() == typeof(Label),
                UiElement.TextElement => target.GetType() == typeof(TextElement),
                UiElement.TextField => target.GetType() == typeof(UnityEngine.UIElements.TextField),
                UiElement.Toggle => target.GetType() == typeof(Toggle),
                UiElement.RadioButton => target.GetType() == typeof(RadioButton),
                UiElement.RadioButtonGroup => target.GetType() == typeof(RadioButtonGroup),
                UiElement.ToggleButtonGroup => target.GetType() == typeof(ToggleButtonGroup),
                UiElement.DropdownField => target.GetType() == typeof(DropdownField),
                UiElement.Slider => target.GetType() == typeof(Slider),
                UiElement.SliderInt => target.GetType() == typeof(SliderInt),
                UiElement.MinMaxSlider => target.GetType() == typeof(MinMaxSlider),
                UiElement.ProgressBar => target.GetType() == typeof(ProgressBar),
                UiElement.Button => target.GetType() == typeof(Button),
                UiElement.RepeatButton => target.GetType() == typeof(RepeatButton),
                UiElement.GroupBox => target.GetType() == typeof(GroupBox),
                UiElement.PopupWindow => target.GetType() == typeof(PopupWindow),
                UiElement.ScrollView => target.GetType() == typeof(ScrollView),
                UiElement.Scroller => target.GetType() == typeof(Scroller),
                UiElement.Tab => target.GetType() == typeof(Tab),
                UiElement.TabView => target.GetType() == typeof(TabView),
                UiElement.Image => target.GetType() == typeof(Image),
                _ => false,
            };
            if (!matches)
                throw new InvalidOperationException(
                    $"UI element {objectId} update has the wrong concrete class."
                );
        }

        private void RefreshParentLayout(Guid objectId)
        {
            if (
                hierarchy.ParentId(objectId) is Guid value
                && hierarchy.TryGet(new ObjectId(value), out VisualElement? parent)
                && parent is BattlementLayoutContainer layout
            )
            {
                layout.FlexLayout?.Refresh();
                layout.GridLayout?.Invalidate();
                layout.StackLayout?.Invalidate();
            }
        }

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }
}
