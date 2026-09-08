#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UColor = UnityEngine.Color;
using UVector3 = UnityEngine.Vector3;

namespace Battlement.UI
{
    internal sealed class BattlementUiParticleStreaks
    {
        private readonly Dictionary<VisualElement, Burst> bursts = new();
        private readonly Func<TimeSpan> now;
        private readonly Func<bool> instant;

        public BattlementUiParticleStreaks(Func<TimeSpan> now, Func<bool> instant) =>
            (this.now, this.instant) = (now, instant);

        public int ActiveCount => bursts.Count;

        public void Restart(VisualElement target, IReadOnlyList<UiParticleStreak> streaks)
        {
            Validate(streaks);
            Remove(target);
            if (streaks.Count == 0 || instant())
                return;
            bursts.Add(target, new Burst(target, streaks, now()));
        }

        public void Advance()
        {
            foreach ((VisualElement target, Burst burst) in bursts.ToArray())
            {
                if (instant() || !burst.Advance(now()))
                    Remove(target);
            }
        }

        public void Remove(VisualElement target)
        {
            if (bursts.Remove(target, out Burst burst))
                burst.Dispose();
        }

        public void Clear()
        {
            foreach (Burst burst in bursts.Values)
                burst.Dispose();
            bursts.Clear();
        }

        private static void Validate(IReadOnlyList<UiParticleStreak> streaks)
        {
            if (streaks.Count > 128)
                throw new BattlementUiException(
                    CoreErrorCode.LimitExceeded,
                    "At most 128 UI streaks are supported."
                );
            foreach (UiParticleStreak streak in streaks)
            {
                Pair(streak.Origin, 1);
                if (streak.Origin.Any(value => value < 0))
                    throw Invalid();
                Pair(streak.Travel, 1024);
                Pair(streak.Size, 1024);
                bool validSize = streak.Size.All(value => value > 0);
                bool validTime = streak.LifetimeMs is > 0 and <= 1000 && streak.DelayMs <= 1000;
                if (!validSize || !validTime || !float.IsFinite(streak.Rotation))
                    throw Invalid();
                double[] color =
                {
                    streak.Color.Red,
                    streak.Color.Green,
                    streak.Color.Blue,
                    streak.Color.Alpha,
                };
                if (color.Any(value => !double.IsFinite(value) || value < 0 || value > 1))
                    throw Invalid();
            }
        }

        private static void Pair(IReadOnlyList<float> values, float limit)
        {
            if (
                values.Count != 2
                || values.Any(value => !float.IsFinite(value) || Math.Abs(value) > limit)
            )
                throw Invalid();
        }

        private static BattlementUiException Invalid() =>
            new(
                CoreErrorCode.InvalidProperty,
                "UI streak geometry, color, and timing must be finite and within their limits."
            );

        private sealed class Burst : IDisposable
        {
            private readonly VisualElement target;
            private readonly VisualElement overlay;
            private readonly UiParticleStreak[] streaks;
            private readonly TimeSpan started;
            private readonly double duration;
            private readonly float padding;
            private readonly GameObject owner;
            private readonly ParticleSystem particles;
            private readonly ParticleSystemRenderer renderer;
            private readonly Camera camera;
            private readonly Mesh mesh = new();
            private readonly List<UVector3> vertices = new();
            private readonly List<Color32> colors = new();
            private readonly List<int> indices = new();
            private float elapsed;

            public Burst(
                VisualElement target,
                IReadOnlyList<UiParticleStreak> streaks,
                TimeSpan started
            )
            {
                this.target = target;
                this.streaks = streaks.OrderBy(value => value.DelayMs).ToArray();
                this.started = started;
                duration = streaks.Max(value => value.DelayMs + value.LifetimeMs) / 1000.0;
                padding = streaks.Max(value => value.Travel.Max(Math.Abs) + value.Size.Max()) + 2;
                owner = new GameObject("Battlement UI particles")
                {
                    hideFlags = HideFlags.HideAndDontSave,
                };
                camera = owner.AddComponent<Camera>();
                camera.enabled = false;
                camera.orthographic = true;
                camera.orthographicSize = 2048;
                particles = owner.AddComponent<ParticleSystem>();
                particles.Stop(true, ParticleSystemStopBehavior.StopEmittingAndClear);
                particles.useAutoRandomSeed = false;
                particles.randomSeed = 1;
                ParticleSystem.MainModule main = particles.main;
                main.playOnAwake = false;
                main.loop = false;
                main.maxParticles = 128;
                main.startSize3D = true;
                main.simulationSpace = ParticleSystemSimulationSpace.Local;
                ParticleSystem.EmissionModule emission = particles.emission;
                emission.enabled = false;
                ParticleSystem.ShapeModule shape = particles.shape;
                shape.enabled = false;
                ParticleSystem.SizeOverLifetimeModule size = particles.sizeOverLifetime;
                size.enabled = true;
                size.separateAxes = true;
                size.x = new ParticleSystem.MinMaxCurve(
                    1,
                    new AnimationCurve(
                        new Keyframe(0, 0.3f),
                        new Keyframe(0.3f, 1),
                        new Keyframe(1, 0.2f)
                    )
                );
                size.y = 1;
                size.z = 1;
                ParticleSystem.ColorOverLifetimeModule color = particles.colorOverLifetime;
                color.enabled = true;
                color.color = new UnityEngine.Gradient
                {
                    colorKeys = new[]
                    {
                        new GradientColorKey(UColor.white, 0),
                        new GradientColorKey(UColor.white, 1),
                    },
                    alphaKeys = new[]
                    {
                        new GradientAlphaKey(0.95f, 0),
                        new GradientAlphaKey(0.9f, 0.3f),
                        new GradientAlphaKey(0, 1),
                    },
                };
                ParticleSystem.VelocityOverLifetimeModule velocity = particles.velocityOverLifetime;
                velocity.enabled = true;
                velocity.speedModifier = new ParticleSystem.MinMaxCurve(
                    1,
                    AnimationCurve.Linear(0, 2, 1, 0)
                );
                renderer = owner.GetComponent<ParticleSystemRenderer>();
                renderer.enabled = false;
                renderer.renderMode = ParticleSystemRenderMode.Billboard;
                renderer.alignment = ParticleSystemRenderSpace.Local;
                overlay = new VisualElement
                {
                    name = "native-particle-streaks",
                    pickingMode = PickingMode.Ignore,
                    usageHints = UsageHints.DynamicTransform,
                };
                overlay.style.position = Position.Absolute;
                overlay.style.left = -padding;
                overlay.style.top = -padding;
                overlay.style.right = -padding;
                overlay.style.bottom = -padding;
                overlay.generateVisualContent += Paint;
                target.hierarchy.Add(overlay);
            }

            public bool Advance(TimeSpan now)
            {
                elapsed = (float)(now - started).TotalSeconds;
                if (target.panel is null || elapsed >= duration)
                    return false;
                overlay.MarkDirtyRepaint();
                return true;
            }

            public void Dispose()
            {
                overlay.generateVisualContent -= Paint;
                overlay.RemoveFromHierarchy();
                Object.DestroyImmediate(mesh);
                Object.DestroyImmediate(owner);
            }

            private void Paint(MeshGenerationContext context)
            {
                // Reconstruct from the controlled clock so repaints at a frozen time cannot drift.
                particles.Stop(true, ParticleSystemStopBehavior.StopEmittingAndClear);
                particles.Simulate(0, false, true, false);
                float sampled = 0;
                foreach (UiParticleStreak streak in streaks)
                {
                    float delay = streak.DelayMs / 1000f;
                    if (delay > elapsed)
                        break;
                    if (delay > sampled)
                        particles.Simulate(delay - sampled, false, false, false);
                    sampled = delay;
                    float lifetime = streak.LifetimeMs / 1000f;
                    particles.Emit(
                        new ParticleSystem.EmitParams
                        {
                            position = new UVector3(
                                streak.Origin[0] * target.layout.width
                                    + streak.Size[0] / 2
                                    + streak.Travel[0] * 0.15f,
                                -streak.Origin[1] * target.layout.height
                                    - streak.Size[1] / 2
                                    - streak.Travel[1] * 0.15f,
                                0
                            ),
                            velocity =
                                new UVector3(streak.Travel[0], -streak.Travel[1], 0)
                                * (0.85f / lifetime),
                            startSize3D = new UVector3(streak.Size[0], streak.Size[1], 1),
                            rotation = streak.Rotation,
                            startLifetime = lifetime,
                            startColor = new UColor(
                                (float)streak.Color.Red,
                                (float)streak.Color.Green,
                                (float)streak.Color.Blue,
                                (float)streak.Color.Alpha
                            ),
                            randomSeed = 1,
                        },
                        1
                    );
                }
                if (elapsed > sampled)
                    particles.Simulate(elapsed - sampled, false, false, false);
                renderer.BakeMesh(mesh, camera, ParticleSystemBakeMeshOptions.Default);
                mesh.GetVertices(vertices);
                if (vertices.Count == 0)
                    return;
                mesh.GetColors(colors);
                mesh.GetTriangles(indices, 0);
                MeshWriteData data = context.Allocate(vertices.Count, indices.Count);
                for (int index = 0; index < vertices.Count; index++)
                {
                    UVector3 point = vertices[index];
                    data.SetNextVertex(
                        new Vertex
                        {
                            position = new UVector3(
                                point.x + padding,
                                -point.y + padding,
                                Vertex.nearZ
                            ),
                            tint = colors[index],
                        }
                    );
                }
                foreach (int index in indices)
                    data.SetNextIndex(checked((ushort)index));
            }
        }
    }
}
