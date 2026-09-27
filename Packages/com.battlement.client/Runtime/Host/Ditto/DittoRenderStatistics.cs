#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Unity.Profiling;
using UnityEngine;
using UnityEngine.Rendering;

namespace Battlement
{
    /// Retains render counters and scene inventory outside the score timing pass.
    internal sealed class DittoRenderStatistics : IDisposable
    {
        private readonly Dictionary<string, Counter> counters = new()
        {
            ["standard_draw_calls"] = new("Standard Draw Calls Count"),
            ["standard_indirect_draw_calls"] = new("Standard Indirect Draw Calls Count"),
            ["instanced_draw_calls"] = new("Standard Instanced Draw Calls Count"),
            ["srp_batcher_draw_calls"] = new("SRP Batcher Draw Calls Count"),
            ["srp_batcher_instances"] = new("SRP Batcher Instances Count"),
            ["brg_draw_calls"] = new("BRG Draw Calls Count"),
            ["brg_indirect_draw_calls"] = new("BRG Indirect Draw Calls Count"),
            ["procedural_draw_calls"] = new("Null Geometry Draw Calls Count"),
            ["procedural_indirect_draw_calls"] = new("Null Geometry Indirect Draw Calls Count"),
            ["shadow_casters"] = new("Shadow Casters Count"),
            ["set_pass_calls"] = new("SetPass Calls Count"),
            ["triangles"] = new("Triangles Count"),
            ["vertices"] = new("Vertices Count"),
        };

        public void Begin()
        {
            foreach (Counter counter in counters.Values)
                counter.Reset();
        }

        public void Sample()
        {
            foreach (Counter counter in counters.Values)
                counter.Sample();
        }

        public void Emit(string scenarioId, uint? stepIndex)
        {
            var fields = new Dictionary<string, string>
            {
                ["scenario_id"] = scenarioId,
                ["checkpoint"] = stepIndex.HasValue ? "interaction-ended" : "idle",
                ["step_index"] = stepIndex?.ToString(CultureInfo.InvariantCulture) ?? "none",
            };
            foreach ((string name, Counter counter) in counters)
                counter.AddFields(name, fields);
            Renderer[] renderers = UnityEngine
                .Object.FindObjectsByType<Renderer>(FindObjectsInactive.Exclude)
                .Where(renderer => renderer.enabled)
                .ToArray();
            fields["enabled_renderers"] = Number(renderers.Length);
            fields["visible_renderers"] = Number(renderers.Count(renderer => renderer.isVisible));
            fields["shared_materials"] = Number(
                renderers
                    .SelectMany(renderer => renderer.sharedMaterials)
                    .Where(material => material != null)
                    .Distinct()
                    .Count()
            );
            fields["shadow_casting_renderers"] = Number(
                renderers.Count(renderer => renderer.shadowCastingMode != ShadowCastingMode.Off)
            );
            fields["shadow_lights"] = Number(
                UnityEngine
                    .Object.FindObjectsByType<Light>(FindObjectsInactive.Exclude)
                    .Count(light => light.enabled && light.shadows != LightShadows.None)
            );
            ParticleSystem[] particles = UnityEngine.Object.FindObjectsByType<ParticleSystem>(
                FindObjectsInactive.Exclude
            );
            fields["particle_systems"] = Number(particles.Length);
            fields["live_particles"] = Number(particles.Sum(system => system.particleCount));
            BattlementLogStore.Add(
                "ditto-player",
                new BattlementLogRecord(
                    BattlementLogSeverity.Information,
                    "ditto.render_statistics",
                    "Detail-pass render counters and active scene inventory; "
                        + "unavailable counters are explicit.",
                    fields
                )
            );
        }

        public void Dispose()
        {
            foreach (Counter counter in counters.Values)
                counter.Dispose();
        }

        private static string Number(long value) => value.ToString(CultureInfo.InvariantCulture);

        private sealed class Counter : IDisposable
        {
            private ProfilerRecorder recorder;
            private long minimum = long.MaxValue;
            private long maximum;
            private long samples;

            public Counter(string name) =>
                recorder = ProfilerRecorder.StartNew(ProfilerCategory.Render, name);

            public void Reset()
            {
                minimum = long.MaxValue;
                maximum = 0;
                samples = 0;
            }

            public void Sample()
            {
                if (!recorder.Valid || recorder.Count == 0)
                    return;
                long value = recorder.LastValue;
                minimum = Math.Min(minimum, value);
                maximum = Math.Max(maximum, value);
                samples++;
            }

            public void AddFields(string name, Dictionary<string, string> fields)
            {
                fields[name + "_samples"] = Number(samples);
                fields[name + "_min"] = samples == 0 ? "unavailable" : Number(minimum);
                fields[name + "_max"] = samples == 0 ? "unavailable" : Number(maximum);
            }

            public void Dispose() => recorder.Dispose();
        }
    }
}
