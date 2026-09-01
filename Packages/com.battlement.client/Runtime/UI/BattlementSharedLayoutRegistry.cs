#nullable enable

using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal readonly struct BattlementLayoutOrigin
    {
        public BattlementLayoutOrigin(UnityEngine.Rect bounds, IPanel? panel) =>
            (Bounds, Panel) = (bounds, panel);

        public UnityEngine.Rect Bounds { get; }

        public IPanel? Panel { get; }
    }

    internal sealed class BattlementSharedLayoutRegistry
    {
        private readonly Dictionary<string, BattlementLayoutOrigin> departed = new();

        public BattlementLayoutOrigin Origin(
            MotionDescriptor descriptor,
            VisualElement target,
            DescriptorState? previous,
            IEnumerable<DescriptorState> candidates
        )
        {
            if (previous is not null)
                return new BattlementLayoutOrigin(
                    previous.Target.worldBound,
                    previous.Target.panel
                );
            MotionLayoutDescriptor? layout = descriptor.Layout;
            if (layout?.LayoutId is null)
                return new BattlementLayoutOrigin(target.worldBound, target.panel);
            string key = Key(layout);
            foreach (DescriptorState candidate in candidates)
            {
                BattlementLayoutProjection? projection = candidate.LayoutProjection;
                if (projection?.Descriptor.LayoutId is null || Key(projection.Descriptor) != key)
                    continue;
                RequireSamePanel(target.panel, candidate.Target.panel);
                return new BattlementLayoutOrigin(projection.VisibleBounds, candidate.Target.panel);
            }
            if (!departed.TryGetValue(key, out BattlementLayoutOrigin origin))
                return new BattlementLayoutOrigin(target.worldBound, target.panel);
            RequireSamePanel(target.panel, origin.Panel);
            departed.Remove(key);
            return origin;
        }

        public void Remember(DescriptorState descriptor)
        {
            MotionLayoutDescriptor? layout = descriptor.LayoutProjection?.Descriptor;
            if (layout?.LayoutId is not null)
                departed[Key(layout)] = new BattlementLayoutOrigin(
                    descriptor.Target.worldBound,
                    descriptor.Target.panel
                );
        }

        public void Clear() => departed.Clear();

        private static void RequireSamePanel(IPanel? left, IPanel? right)
        {
            if (left is null || right is null || ReferenceEquals(left, right))
                return;
            throw new BattlementUiException(
                CoreErrorCode.InvalidProperty,
                "Shared layout handoffs cannot cross UI panels."
            );
        }

        private static string Key(MotionLayoutDescriptor value) =>
            $"{value.Group.ValueType}:{value.Group.ValueHash}:"
            + $"{value.LayoutId!.ValueType}:{value.LayoutId.ValueHash}";
    }
}
