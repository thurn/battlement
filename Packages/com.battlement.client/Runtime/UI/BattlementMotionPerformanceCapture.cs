#nullable enable

using System;
using System.IO;
using UnityEngine;

namespace Battlement.UI
{
    internal sealed class BattlementMotionPerformanceCapture
    {
        private const int SampleCapacity = 8192;
        private const double WarmupSeconds = 5;
        private const double CaptureSeconds = 30;

        private readonly string scenario;
        private readonly string? outputPath;
        private readonly double[] cpuMilliseconds = new double[SampleCapacity];
        private readonly double[] frameIntervals = new double[SampleCapacity];
        private double activeSince = -1;
        private double captureStarted = -1;
        private int sampleCount;
        private long maximumAllocation;
        private int maximumActiveTimelines;
        private int maximumActiveLayoutTracks;
        private int maximumGraphNodes;
        private int maximumProperties;
        private int maximumNativeOptimizedTracks;
        private int lifecycleMessages;
        private int lifecyclePayloadBytes;
        private bool completed;

        private BattlementMotionPerformanceCapture(string scenario, string? outputPath)
        {
            this.scenario = scenario;
            this.outputPath = outputPath;
            QualitySettings.vSyncCount = 0;
            Application.targetFrameRate = 64;
        }

        public static BattlementMotionPerformanceCapture? Create()
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            const string query = "battlement-motion-profile=";
            int start = Application.absoluteURL.IndexOf(query, StringComparison.Ordinal);
            if (start < 0)
                return null;
            start += query.Length;
            int end = Application.absoluteURL.IndexOf('&', start);
            string scenario = Uri.UnescapeDataString(
                Application.absoluteURL.Substring(
                    start,
                    end < 0 ? Application.absoluteURL.Length - start : end - start
                )
            );
            return ValidScenario(scenario)
                ? new BattlementMotionPerformanceCapture(scenario, null)
                : null;
#else
            string? scenario = Argument("--battlement-motion-scenario=");
            string? output = Argument("--battlement-motion-profile=");
            return scenario != null && output != null && ValidScenario(scenario)
                ? new BattlementMotionPerformanceCapture(scenario, output)
                : null;
#endif
        }

        public void Observe(BattlementMotionPerformanceSnapshot snapshot, double presentationTime)
        {
            if (completed)
                return;
            if (activeSince < 0)
            {
                bool ready = snapshot.ActiveTimelines > 0;
                ready |= snapshot.PropertiesApplied > 0;
                if (ready)
                    activeSince = presentationTime;
                return;
            }
            if (presentationTime - activeSince < WarmupSeconds)
                return;
            if (captureStarted < 0)
                captureStarted = presentationTime;
            Record(snapshot);
            if (presentationTime - captureStarted < CaptureSeconds)
                return;
            completed = true;
            Publish(Result(presentationTime - captureStarted));
        }

        private void Record(BattlementMotionPerformanceSnapshot snapshot)
        {
            if (sampleCount >= SampleCapacity)
                return;
            cpuMilliseconds[sampleCount] = snapshot.MotionCpuMilliseconds;
            frameIntervals[sampleCount] = snapshot.FrameIntervalMilliseconds;
            sampleCount += 1;
            maximumAllocation = Math.Max(maximumAllocation, snapshot.ManagedAllocationBytes);
            maximumActiveTimelines = Math.Max(maximumActiveTimelines, snapshot.ActiveTimelines);
            maximumActiveLayoutTracks = Math.Max(
                maximumActiveLayoutTracks,
                snapshot.ActiveLayoutTracks
            );
            maximumGraphNodes = Math.Max(maximumGraphNodes, snapshot.GraphNodesEvaluated);
            maximumProperties = Math.Max(maximumProperties, snapshot.PropertiesApplied);
            maximumNativeOptimizedTracks = Math.Max(
                maximumNativeOptimizedTracks,
                snapshot.NativeOptimizedTracks
            );
            lifecycleMessages += snapshot.LifecycleMessages;
            lifecyclePayloadBytes += snapshot.LifecyclePayloadBytes;
        }

        private MotionPerformanceProfile Result(double durationSeconds)
        {
            Array.Sort(cpuMilliseconds, 0, sampleCount);
            Array.Sort(frameIntervals, 0, sampleCount);
            double p95Cpu = Percentile(cpuMilliseconds, 0.95);
            double p99Interval = Percentile(frameIntervals, 0.99);
            double maximumInterval = frameIntervals[sampleCount - 1];
            double averageFramesPerSecond = Math.Max(0, sampleCount - 1) / durationSeconds;
            return new MotionPerformanceProfile
            {
                schemaVersion = 1,
                scenario = scenario,
                platform = Application.platform.ToString(),
                unityVersion = Application.unityVersion,
                targetFrameRate = Application.targetFrameRate,
                vSyncCount = QualitySettings.vSyncCount,
                release = !Debug.isDebugBuild,
                warmupSeconds = WarmupSeconds,
                capturedSeconds = durationSeconds,
                samples = sampleCount,
                motionCpuP95Milliseconds = p95Cpu,
                averageFramesPerSecond = averageFramesPerSecond,
                frameIntervalP99Milliseconds = p99Interval,
                maximumFrameIntervalMilliseconds = maximumInterval,
                maximumManagedAllocationBytes = maximumAllocation,
                maximumActiveTimelines = maximumActiveTimelines,
                maximumActiveLayoutTracks = maximumActiveLayoutTracks,
                maximumGraphNodesEvaluated = maximumGraphNodes,
                maximumPropertiesApplied = maximumProperties,
                maximumNativeOptimizedTracks = maximumNativeOptimizedTracks,
                lifecycleMessages = lifecycleMessages,
                lifecyclePayloadBytes = lifecyclePayloadBytes,
                passed =
                    p95Cpu < 4
                    && averageFramesPerSecond >= 59
                    && p99Interval <= 18.337
                    && maximumInterval <= 33.34
                    && maximumAllocation == 0,
            };
        }

        private double Percentile(double[] samples, double percentile) =>
            samples[Math.Min(sampleCount - 1, (int)Math.Ceiling(sampleCount * percentile) - 1)];

        private void Publish(MotionPerformanceProfile result)
        {
            string json = JsonUtility.ToJson(result, true);
#if UNITY_WEBGL && !UNITY_EDITOR
            BattlementMotionProfilePublish(json);
#else
            string directory = Path.GetDirectoryName(outputPath!)!;
            if (directory.Length > 0)
                Directory.CreateDirectory(directory);
            File.WriteAllText(outputPath!, json + Environment.NewLine);
#endif
        }

        private static string? Argument(string prefix)
        {
            foreach (string argument in Environment.GetCommandLineArgs())
            {
                if (argument.StartsWith(prefix, StringComparison.Ordinal))
                    return argument.Substring(prefix.Length);
            }
            return null;
        }

        private static bool ValidScenario(string value) =>
            value is "transform-200" or "mixed-200" or "mixed-interaction";

#if UNITY_WEBGL && !UNITY_EDITOR
        [System.Runtime.InteropServices.DllImport("__Internal")]
        private static extern void BattlementMotionProfilePublish(string json);
#endif
    }

    [Serializable]
    internal sealed class MotionPerformanceProfile
    {
        public int schemaVersion;
        public string scenario = "";
        public string platform = "";
        public string unityVersion = "";
        public int targetFrameRate;
        public int vSyncCount;
        public bool release;
        public double warmupSeconds;
        public double capturedSeconds;
        public int samples;
        public double motionCpuP95Milliseconds;
        public double averageFramesPerSecond;
        public double frameIntervalP99Milliseconds;
        public double maximumFrameIntervalMilliseconds;
        public long maximumManagedAllocationBytes;
        public int maximumActiveTimelines;
        public int maximumActiveLayoutTracks;
        public int maximumGraphNodesEvaluated;
        public int maximumPropertiesApplied;
        public int maximumNativeOptimizedTracks;
        public int lifecycleMessages;
        public int lifecyclePayloadBytes;
        public bool passed;
    }
}
