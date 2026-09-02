#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;

namespace Battlement.UI
{
    /// <summary>Latest allocation-free Motion frame diagnostics.</summary>
    public readonly struct BattlementMotionPerformanceSnapshot
    {
        internal BattlementMotionPerformanceSnapshot(
            ulong frame,
            double motionCpuMilliseconds,
            double frameIntervalMilliseconds,
            int activeTimelines,
            int activeLayoutTracks,
            int graphNodesEvaluated,
            int propertiesApplied,
            int nativeOptimizedTracks,
            long managedAllocationBytes,
            int lifecycleMessages,
            int lifecyclePayloadBytes
        ) =>
            (
                Frame,
                MotionCpuMilliseconds,
                FrameIntervalMilliseconds,
                ActiveTimelines,
                ActiveLayoutTracks,
                GraphNodesEvaluated,
                PropertiesApplied,
                NativeOptimizedTracks,
                ManagedAllocationBytes,
                LifecycleMessages,
                LifecyclePayloadBytes
            ) = (
                frame,
                motionCpuMilliseconds,
                frameIntervalMilliseconds,
                activeTimelines,
                activeLayoutTracks,
                graphNodesEvaluated,
                propertiesApplied,
                nativeOptimizedTracks,
                managedAllocationBytes,
                lifecycleMessages,
                lifecyclePayloadBytes
            );

        public ulong Frame { get; }

        public double MotionCpuMilliseconds { get; }

        public double FrameIntervalMilliseconds { get; }

        public int ActiveTimelines { get; }

        public int ActiveLayoutTracks { get; }

        public int GraphNodesEvaluated { get; }

        public int PropertiesApplied { get; }

        public int NativeOptimizedTracks { get; }

        public long ManagedAllocationBytes { get; }

        public int LifecycleMessages { get; }

        public int LifecyclePayloadBytes { get; }
    }

    internal sealed class BattlementMotionPerformance
    {
        private readonly BattlementMotionPerformanceCapture? capture =
            BattlementMotionPerformanceCapture.Create();
        private long startedAt;
        private long allocatedAt;
        private double previousFrameTime;
        private ulong frame;

        public BattlementMotionPerformanceSnapshot Snapshot { get; private set; }

        public void BeginFrame(double presentationTime)
        {
            startedAt = Stopwatch.GetTimestamp();
            allocatedAt = AllocatedBytes();
            if (previousFrameTime == 0)
                previousFrameTime = presentationTime;
        }

        public void EndFrame(
            double presentationTime,
            Dictionary<Guid, DescriptorState>.ValueCollection descriptors,
            int graphNodesEvaluated
        )
        {
            int activeTimelines = 0;
            int activeLayoutTracks = 0;
            int propertiesApplied = 0;
            int nativeOptimizedTracks = 0;
            foreach (DescriptorState descriptor in descriptors)
            {
                activeTimelines += descriptor.ActiveTimelineCount;
                activeLayoutTracks += descriptor.ActiveLayoutTrackCount;
                propertiesApplied += descriptor.ActivePropertyCount;
                nativeOptimizedTracks += descriptor.NativeOptimizedTrackCount;
            }
            long allocation = Math.Max(0, AllocatedBytes() - allocatedAt);
            double cpu = (Stopwatch.GetTimestamp() - startedAt) * 1000.0 / Stopwatch.Frequency;
            double interval = (presentationTime - previousFrameTime) * 1000.0;
            Snapshot = new BattlementMotionPerformanceSnapshot(
                ++frame,
                cpu,
                interval,
                activeTimelines,
                activeLayoutTracks,
                graphNodesEvaluated,
                propertiesApplied,
                nativeOptimizedTracks,
                allocation,
                0,
                0
            );
            capture?.Observe(Snapshot, presentationTime);
            previousFrameTime = presentationTime;
        }

        public void RecordTraffic(int payloadBytes)
        {
            BattlementMotionPerformanceSnapshot current = Snapshot;
            Snapshot = new BattlementMotionPerformanceSnapshot(
                current.Frame,
                current.MotionCpuMilliseconds,
                current.FrameIntervalMilliseconds,
                current.ActiveTimelines,
                current.ActiveLayoutTracks,
                current.GraphNodesEvaluated,
                current.PropertiesApplied,
                current.NativeOptimizedTracks,
                current.ManagedAllocationBytes,
                1,
                payloadBytes
            );
        }

        public void Reset()
        {
            Snapshot = default;
            previousFrameTime = 0;
            frame = 0;
        }

        private static long AllocatedBytes()
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            return 0;
#else
            return GC.GetAllocatedBytesForCurrentThread();
#endif
        }
    }
}
