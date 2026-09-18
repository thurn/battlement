#nullable enable

using System.Collections.Generic;

namespace Battlement.UI
{
    internal readonly struct BattlementLayoutOrigin
    {
        public BattlementLayoutOrigin(ViewportRect bounds, BattlementLayoutDomain domain) =>
            (Bounds, Domain) = (bounds, domain);

        public ViewportRect Bounds { get; }

        public BattlementLayoutDomain Domain { get; }
    }

    internal enum BattlementLayoutDomain
    {
        Ui,
        World,
    }

    internal readonly struct BattlementUiProjectionSpace
    {
        public BattlementUiProjectionSpace(DisplayId displayId, double pixelsPerPoint) =>
            (DisplayId, PixelsPerPoint) = (displayId, pixelsPerPoint);

        public DisplayId DisplayId { get; }

        public double PixelsPerPoint { get; }

        public ViewportRect ToViewport(UnityEngine.Rect value) =>
            new(
                value.x * PixelsPerPoint,
                value.y * PixelsPerPoint,
                value.width * PixelsPerPoint,
                value.height * PixelsPerPoint,
                DisplayId
            );

        public UnityEngine.Rect FromViewport(ViewportRect value)
        {
            if (!value.DisplayId.Equals(DisplayId))
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Shared layout handoffs cannot cross physical displays."
                );
            return new UnityEngine.Rect(
                checked((float)(value.X / PixelsPerPoint)),
                checked((float)(value.Y / PixelsPerPoint)),
                checked((float)(value.Width / PixelsPerPoint)),
                checked((float)(value.Height / PixelsPerPoint))
            );
        }
    }

    internal sealed class BattlementSharedLayoutRegistry
    {
        private readonly Dictionary<string, BattlementLayoutOrigin> departed = new();

        public BattlementLayoutOrigin Origin(
            MotionDescriptor descriptor,
            IBattlementLayoutProjectionTarget target,
            DescriptorState? previous,
            IEnumerable<DescriptorState> candidates
        )
        {
            if (previous?.LayoutProjection is IBattlementLayoutProjection previousProjection)
            {
                RequireProjection(descriptor.Layout!, target.Domain, previousProjection);
                return new BattlementLayoutOrigin(
                    previousProjection.VisibleBounds,
                    previousProjection.Domain
                );
            }
            MotionLayoutDescriptor? layout = descriptor.Layout;
            if (layout?.LayoutId is null)
                return new BattlementLayoutOrigin(target.VisibleBounds(layout!), target.Domain);
            string key = Key(layout);
            foreach (DescriptorState candidate in candidates)
            {
                IBattlementLayoutProjection? projection = candidate.LayoutProjection;
                if (projection?.Descriptor.LayoutId is null || Key(projection.Descriptor) != key)
                    continue;
                RequireProjection(layout, target.Domain, projection);
                return new BattlementLayoutOrigin(projection.VisibleBounds, projection.Domain);
            }
            if (!departed.TryGetValue(key, out BattlementLayoutOrigin origin))
                return new BattlementLayoutOrigin(target.VisibleBounds(layout), target.Domain);
            RequireProjection(layout, target.Domain, origin.Domain);
            departed.Remove(key);
            return origin;
        }

        public void Remember(DescriptorState descriptor)
        {
            MotionLayoutDescriptor? layout = descriptor.LayoutProjection?.Descriptor;
            if (layout?.LayoutId is not null)
                departed[Key(layout)] = new BattlementLayoutOrigin(
                    descriptor.LayoutProjection!.VisibleBounds,
                    descriptor.LayoutProjection.Domain
                );
        }

        public void Clear() => departed.Clear();

        private static void RequireProjection(
            MotionLayoutDescriptor destination,
            BattlementLayoutDomain destinationDomain,
            IBattlementLayoutProjection source
        ) => RequireProjection(destination, destinationDomain, source.Domain);

        private static void RequireProjection(
            MotionLayoutDescriptor destination,
            BattlementLayoutDomain destinationDomain,
            BattlementLayoutDomain sourceDomain
        )
        {
            if (destinationDomain == sourceDomain)
                return;
            if (destination.Projection is null)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "UI/world shared layout handoffs require an explicit projection."
                );
        }

        private static string Key(MotionLayoutDescriptor value) =>
            $"{value.Group.ValueType}:{value.Group.ValueHash}:"
            + $"{value.LayoutId!.ValueType}:{value.LayoutId.ValueHash}";
    }
}
