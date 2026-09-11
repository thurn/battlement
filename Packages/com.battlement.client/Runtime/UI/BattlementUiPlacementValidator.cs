#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiPlacementValidator
    {
        private readonly BattlementUiHierarchy hierarchy;

        public BattlementUiPlacementValidator(BattlementUiHierarchy hierarchy) =>
            this.hierarchy = hierarchy;

        internal void ValidateOverlayHost(VisualElement physicalParent)
        {
            BattlementLayoutContainer host = physicalParent switch
            {
                BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack } direct =>
                    direct,
                BattlementLayoutSlot
                {
                    ContainingBlock: BattlementLayoutContainer
                    {
                        Kind: BattlementLayoutContainerKind.Stack
                    } slotted
                } => slotted,
                _ => throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Overlay placement requires a direct OverlayHost Stack target."
                ),
            };
            if (!hierarchy.TryGetId(host, out Guid hostId))
                throw Failure(CoreErrorCode.InvalidHierarchy, "OverlayHost is not registered.");
            Guid rootStackId =
                hierarchy.ParentId(hostId)
                ?? throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "OverlayHost requires a document-root Stack."
                );
            IReadOnlyList<Guid> rootChildren = hierarchy.Children(rootStackId);
            bool finalChild = rootChildren.Count != 0 && rootChildren[^1] == hostId;
            bool rootStack =
                hierarchy.TryGet(new ObjectId(rootStackId), out VisualElement? rootStackElement)
                && rootStackElement
                    is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack }
                && hierarchy.ParentId(rootStackId) is Guid documentRoot
                && hierarchy.IsRoot(documentRoot);
            StackItem item = BattlementStackItems.Get(host);
            bool configured =
                BattlementStackItems.HasAuthored(host)
                && item.Order == int.MaxValue
                && !item.ContributesToSize
                && host.pickingMode == PickingMode.Ignore
                && host.style.overflow.value == Overflow.Visible;
            if (!rootStack || !finalChild || !configured)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "OverlayHost must be the configured final child of a document-root Stack."
                );
        }

        internal static bool HasScrollAncestor(VisualElement value)
        {
            for (VisualElement? current = value; current is not null; current = current.parent)
                if (current is ScrollView)
                    return true;
            return false;
        }

        internal static void ValidateLayoutStyle(UiStyle? style, string layoutKind)
        {
            if (style is null)
                return;
            bool absolute =
                style.Position.IsSet
                && style.Position.Value.Keyword is null
                && style.Position.Value.Value == UiPosition.Absolute;
            bool offsetsAreAutomatic = new[]
            {
                style.Top,
                style.Right,
                style.Bottom,
                style.Left,
            }.All(LayoutOffsetIsAutomatic);
            if (absolute || !offsetsAreAutomatic)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    $"{layoutKind} placement children require relative position "
                        + "and automatic offsets."
                );
        }

        internal static void ValidateNativeLayoutStyle(VisualElement child, string container)
        {
            bool offsetsAreAutomatic = new[]
            {
                child.style.top,
                child.style.right,
                child.style.bottom,
                child.style.left,
            }.All(value => value.keyword == StyleKeyword.Auto);
            if (child.style.position.value == Position.Absolute || !offsetsAreAutomatic)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    $"{container} placement children require relative position "
                        + "and automatic offsets."
                );
        }

        private static bool LayoutOffsetIsAutomatic(Prop<UiStyleValue<UiLengthOrAuto>> value) =>
            !value.IsSet
            || value.Value.Keyword is not null
            || value.Value.Value is UiLengthOrAuto.Auto;

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }
}
