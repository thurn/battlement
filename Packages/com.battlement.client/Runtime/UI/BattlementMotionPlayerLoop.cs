#nullable enable

using System;
using Unity.Profiling;
using UnityEngine.LowLevel;
using UnityEngine.PlayerLoop;

namespace Battlement.UI
{
    internal static class BattlementMotionPlayerLoop
    {
        private static BattlementMotionWorld[] worlds = Array.Empty<BattlementMotionWorld>();
        private static PlayerLoopSystem previousLoop;
        private static bool installed;
        private static readonly ProfilerMarker MotionFrameMarker = new("Reactant.MotionFrame");
        private static readonly ProfilerMarker PreLayoutMarker = new("Reactant.MotionPreLayout");
        private static readonly ProfilerMarker PostLayoutMarker = new("Reactant.MotionPostLayout");

        public static void Register(BattlementMotionWorld world)
        {
            for (int index = 0; index < worlds.Length; index++)
                if (ReferenceEquals(worlds[index], world))
                    return;
            if (!installed)
            {
                previousLoop = PlayerLoop.GetCurrentPlayerLoop();
                PlayerLoopSystem loop = RemoveMarkers(previousLoop);
                if (!InsertAroundUiPanels(ref loop))
                    throw new InvalidOperationException(
                        "Unity's UIElementsUpdatePanels PlayerLoop stage was not found."
                    );
                PlayerLoop.SetPlayerLoop(loop);
                installed = true;
            }
            var next = new BattlementMotionWorld[worlds.Length + 1];
            Array.Copy(worlds, next, worlds.Length);
            next[^1] = world;
            worlds = next;
        }

        public static void Unregister(BattlementMotionWorld world)
        {
            int removed = Array.FindIndex(worlds, value => ReferenceEquals(value, world));
            if (removed < 0)
                return;
            if (worlds.Length == 1)
            {
                worlds = Array.Empty<BattlementMotionWorld>();
                if (installed)
                    PlayerLoop.SetPlayerLoop(previousLoop);
                installed = false;
                return;
            }
            var next = new BattlementMotionWorld[worlds.Length - 1];
            if (removed != 0)
                Array.Copy(worlds, 0, next, 0, removed);
            if (removed != next.Length)
                Array.Copy(worlds, removed + 1, next, removed, next.Length - removed);
            worlds = next;
        }

        internal static bool IsInstalled => installed;

        internal static bool HasNormalizedTopology()
        {
            PlayerLoopSystem loop = PlayerLoop.GetCurrentPlayerLoop();
            return Count(loop, typeof(ReactantMotionPreLayout)) == 1
                && Count(loop, typeof(ReactantMotionPostLayout)) == 1
                && HasAdjacentMarkers(loop);
        }

        private static void PreLayout()
        {
            MotionFrameMarker.Begin();
            PreLayoutMarker.Begin();
            BattlementMotionWorld[] snapshot = worlds;
            for (int index = 0; index < snapshot.Length; index++)
                snapshot[index].PreLayout();
            PreLayoutMarker.End();
        }

        private static void PostLayout()
        {
            PostLayoutMarker.Begin();
            BattlementMotionWorld[] snapshot = worlds;
            for (int index = 0; index < snapshot.Length; index++)
                snapshot[index].PostLayout();
            PostLayoutMarker.End();
            MotionFrameMarker.End();
        }

        private static bool InsertAroundUiPanels(ref PlayerLoopSystem parent)
        {
            PlayerLoopSystem[]? children = parent.subSystemList;
            if (children is null)
                return false;
            for (int index = 0; index < children.Length; index++)
            {
                if (children[index].type == typeof(PreLateUpdate.UIElementsUpdatePanels))
                {
                    var replacement = new PlayerLoopSystem[children.Length + 2];
                    Array.Copy(children, 0, replacement, 0, index);
                    replacement[index] = Marker<ReactantMotionPreLayout>(PreLayout);
                    replacement[index + 1] = children[index];
                    replacement[index + 2] = Marker<ReactantMotionPostLayout>(PostLayout);
                    Array.Copy(
                        children,
                        index + 1,
                        replacement,
                        index + 3,
                        children.Length - index - 1
                    );
                    parent.subSystemList = replacement;
                    return true;
                }
                PlayerLoopSystem child = children[index];
                if (!InsertAroundUiPanels(ref child))
                    continue;
                children[index] = child;
                parent.subSystemList = children;
                return true;
            }
            return false;
        }

        private static PlayerLoopSystem RemoveMarkers(PlayerLoopSystem parent)
        {
            PlayerLoopSystem[]? children = parent.subSystemList;
            if (children is null)
                return parent;
            int retained = 0;
            for (int index = 0; index < children.Length; index++)
            {
                Type? type = children[index].type;
                if (
                    type != typeof(ReactantMotionPreLayout)
                    && type != typeof(ReactantMotionPostLayout)
                )
                    retained++;
            }
            var normalized = new PlayerLoopSystem[retained];
            int destination = 0;
            for (int index = 0; index < children.Length; index++)
            {
                Type? type = children[index].type;
                if (
                    type == typeof(ReactantMotionPreLayout)
                    || type == typeof(ReactantMotionPostLayout)
                )
                    continue;
                normalized[destination++] = RemoveMarkers(children[index]);
            }
            parent.subSystemList = normalized;
            return parent;
        }

        private static PlayerLoopSystem Marker<T>(PlayerLoopSystem.UpdateFunction callback) =>
            new() { type = typeof(T), updateDelegate = callback };

        private static int Count(PlayerLoopSystem parent, Type marker)
        {
            int result = parent.type == marker ? 1 : 0;
            foreach (
                PlayerLoopSystem child in parent.subSystemList ?? Array.Empty<PlayerLoopSystem>()
            )
                result += Count(child, marker);
            return result;
        }

        private static bool HasAdjacentMarkers(PlayerLoopSystem parent)
        {
            PlayerLoopSystem[] children = parent.subSystemList ?? Array.Empty<PlayerLoopSystem>();
            for (int index = 1; index + 1 < children.Length; index++)
            {
                if (children[index].type != typeof(PreLateUpdate.UIElementsUpdatePanels))
                    continue;
                return children[index - 1].type == typeof(ReactantMotionPreLayout)
                    && children[index + 1].type == typeof(ReactantMotionPostLayout);
            }
            foreach (PlayerLoopSystem child in children)
                if (HasAdjacentMarkers(child))
                    return true;
            return false;
        }

        private sealed class ReactantMotionPreLayout { }

        private sealed class ReactantMotionPostLayout { }
    }
}
