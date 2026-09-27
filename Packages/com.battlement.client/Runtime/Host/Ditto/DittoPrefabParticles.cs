#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Samples owned particles on the deterministic Ditto clock.</summary>
    internal sealed class DittoPrefabParticles
    {
        private readonly DittoMotionClock? clock;
        private readonly Dictionary<ParticleSystem, Emitter> emitters = new();
        private readonly List<ParticleSystem> destroyed = new();

        public DittoPrefabParticles(DittoMotionClock? clock) => this.clock = clock;

        public bool IsEnabled => clock is not null && (clock.IsControlled || clock.IsInstant);
        public int FiniteCount { get; private set; }
        public int InfiniteCount { get; private set; }
        public int HeldCount { get; private set; }

        public void Track(GameObject prefab)
        {
            if (!IsEnabled)
                return;
            ParticleSystem[] systems = prefab
                .GetComponentsInChildren<ParticleSystem>(true)
                .Where(system => system.main.playOnAwake)
                .ToArray();
            for (int index = 0; index < systems.Length; index++)
            {
                ParticleSystem system = systems[index];
                if (emitters.ContainsKey(system))
                    continue;
                uint seed = system.useAutoRandomSeed ? checked((uint)index + 1) : system.randomSeed;
                var emitter = new Emitter(system);
                emitters.Add(system, emitter);
                emitter.Play(seed, restart: true, clock!.Elapsed, clock.IsInstant);
            }
        }

        public void Play(ParticleSystem system, uint seed, bool restart)
        {
            if (!emitters.TryGetValue(system, out Emitter emitter))
            {
                emitter = new Emitter(system);
                emitters.Add(system, emitter);
            }
            emitter.Play(seed, restart, clock!.Elapsed, clock.IsInstant);
        }

        public void Stop(ParticleSystem system, bool clear)
        {
            if (emitters.TryGetValue(system, out Emitter emitter))
                emitter.Stop(clear);
        }

        public bool Pause(ParticleSystem system)
        {
            if (!emitters.TryGetValue(system, out Emitter emitter) || !emitter.Running)
                return false;
            if (emitter.Held)
                return false;
            emitter.Held = true;
            return true;
        }

        public void Resume(ParticleSystem system)
        {
            if (emitters.TryGetValue(system, out Emitter emitter))
                emitter.Resume(clock!.Elapsed);
        }

        public void Sample()
        {
            FiniteCount = 0;
            InfiniteCount = 0;
            HeldCount = 0;
            foreach (var pair in emitters)
            {
                Emitter emitter = pair.Value;
                if (pair.Key == null)
                {
                    destroyed.Add(pair.Key!);
                    continue;
                }
                if (!emitter.Sample(clock!.Elapsed, clock.IsInstant))
                    continue;
                if (emitter.Held)
                    HeldCount++;
                else if (emitter.Looping)
                    InfiniteCount++;
                else
                    FiniteCount++;
            }
            foreach (ParticleSystem system in destroyed)
                emitters.Remove(system);
            destroyed.Clear();
        }

        public void Clear()
        {
            emitters.Clear();
            destroyed.Clear();
            FiniteCount = 0;
            InfiniteCount = 0;
            HeldCount = 0;
        }

        private sealed class Emitter
        {
            private readonly ParticleSystem system;
            private readonly bool authoredEmission;
            private TimeSpan? sampledAt;
            private bool emitting;
            public bool Running { get; private set; }
            public bool Held { get; set; }
            public bool Looping => emitting && system.main.loop;

            public Emitter(ParticleSystem system)
            {
                this.system = system;
                authoredEmission = system.emission.enabled;
                ParticleSystem.MainModule main = system.main;
                main.playOnAwake = false;
            }

            public void Play(uint seed, bool restart, TimeSpan now, bool instant)
            {
                if (Running && !restart)
                {
                    emitting = true;
                    SetEmission(authoredEmission);
                    Resume(now);
                    return;
                }
                system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                system.useAutoRandomSeed = false;
                system.randomSeed = seed;
                SetEmission(authoredEmission);
                emitting = true;
                Running = true;
                Held = false;
                sampledAt = null;
                Sample(now, instant);
            }

            public void Stop(bool clear)
            {
                emitting = false;
                SetEmission(false);
                if (clear)
                {
                    system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                    Running = false;
                    Held = false;
                    sampledAt = null;
                }
            }

            public void Resume(TimeSpan now)
            {
                Held = false;
                sampledAt = now;
            }

            public bool Sample(TimeSpan now, bool instant)
            {
                if (!Running)
                    return false;
                if (!system.gameObject.activeInHierarchy)
                {
                    system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                    sampledAt = null;
                    return false;
                }
                if (Held)
                    return true;
                if (sampledAt is not TimeSpan previous)
                {
                    ParticleSystem.MainModule main = system.main;
                    // Instant snapshots use a canonical looping phase, independent of
                    // finite work and asynchronous rules publication latency.
                    float initial = main.loop && (main.prewarm || instant) ? main.duration : 0;
                    system.Simulate(initial, false, true, true);
                }
                else if (now > previous && !(instant && Looping))
                {
                    system.Simulate((float)(now - previous).TotalSeconds, false, false, true);
                }
                system.Pause(false);
                sampledAt = now;
                if (Looping || (emitting ? system.IsAlive(false) : system.particleCount > 0))
                    return true;
                Running = false;
                return false;
            }

            private void SetEmission(bool enabled)
            {
                ParticleSystem.EmissionModule emission = system.emission;
                emission.enabled = enabled;
            }
        }
    }
}
