#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Samples authored prefab particles on the deterministic Ditto clock.</summary>
    internal sealed class DittoPrefabParticles
    {
        private readonly DittoMotionClock? clock;
        private readonly List<Emitter> emitters = new();

        public DittoPrefabParticles(DittoMotionClock? clock) => this.clock = clock;

        public int FiniteCount { get; private set; }
        public int InfiniteCount { get; private set; }

        public void Track(GameObject prefab)
        {
            if (clock is null || (!clock.IsControlled && !clock.IsInstant))
                return;
            ParticleSystem[] systems = prefab
                .GetComponentsInChildren<ParticleSystem>(true)
                .Where(system => system.main.playOnAwake)
                .ToArray();
            for (int index = 0; index < systems.Length; index++)
            {
                ParticleSystem system = systems[index];
                ParticleSystem.MainModule main = system.main;
                system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                main.playOnAwake = false;
                if (system.useAutoRandomSeed)
                {
                    system.useAutoRandomSeed = false;
                    system.randomSeed = checked((uint)index + 1);
                }
                emitters.Add(new Emitter(system));
            }
        }

        public void Sample()
        {
            FiniteCount = 0;
            InfiniteCount = 0;
            for (int index = emitters.Count - 1; index >= 0; index--)
            {
                Emitter emitter = emitters[index];
                if (emitter.System == null)
                {
                    emitters.RemoveAt(index);
                    continue;
                }
                if (!emitter.Sample(clock!.Elapsed, clock.IsInstant))
                    continue;
                if (emitter.System.main.loop)
                    InfiniteCount++;
                else if (!clock.IsInstant)
                    FiniteCount++;
            }
        }

        public void Clear()
        {
            emitters.Clear();
            FiniteCount = 0;
            InfiniteCount = 0;
        }

        private sealed class Emitter
        {
            public readonly ParticleSystem System;
            private TimeSpan? sampledAt;

            public Emitter(ParticleSystem system) => System = system;

            public bool Sample(TimeSpan now, bool instant)
            {
                if (!System.gameObject.activeInHierarchy)
                {
                    System.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                    sampledAt = null;
                    return false;
                }
                if (sampledAt is not TimeSpan previous)
                {
                    ParticleSystem.MainModule main = System.main;
                    // Prewarmed looping prefabs must remain visible at their initial phase.
                    float initial = main.prewarm && main.loop ? main.duration : 0;
                    System.Simulate(initial, false, true, true);
                }
                else if (System.isStopped)
                {
                    return false;
                }
                else if (!instant && now > previous)
                {
                    System.Simulate((float)(now - previous).TotalSeconds, false, false, true);
                }
                System.Pause(false);
                sampledAt = now;
                return System.main.loop || System.IsAlive(false);
            }
        }
    }
}
