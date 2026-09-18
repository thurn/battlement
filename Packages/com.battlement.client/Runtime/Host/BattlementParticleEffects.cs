#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Pool;
using Object = UnityEngine.Object;

namespace Battlement
{
    internal sealed class BattlementParticleEffects : IDisposable
    {
        private readonly BattlementWorld world;
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly DittoMotionClock motionClock;
        private readonly Dictionary<string, EffectPool> pools = new(StringComparer.Ordinal);
        private bool isDisposed;

        public BattlementParticleEffects(
            BattlementWorld world,
            BattlementPreparedAssets preparedAssets,
            DittoMotionClock motionClock
        )
        {
            this.world = world;
            this.preparedAssets = preparedAssets;
            this.motionClock = motionClock;
            Application.lowMemory += HandleLowMemory;
        }

        public IBattlementCommandOperation? Play(BattlementDirectParticlePlay command) =>
            Play(command.ObjectId, command.Restart);

        private IBattlementCommandOperation? Play(ObjectId objectId, bool restart)
        {
            GameObject target = world.RequireObject(objectId);
            ParticleSystem[] systems = RequireSystems(target);
            if (motionClock.IsInstant || motionClock.IsControlled)
            {
                foreach (ParticleSystem system in systems)
                {
                    system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                }
                return null;
            }
            foreach (ParticleSystem system in systems)
            {
                if (restart)
                {
                    system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                }

                system.Play(false);
            }

            return null;
        }

        public IBattlementCommandOperation? Stop(BattlementDirectParticleStop command) =>
            Stop(command.ObjectId, command.Clear);

        private IBattlementCommandOperation? Stop(ObjectId objectId, bool clear)
        {
            ParticleSystemStopBehavior behavior = clear
                ? ParticleSystemStopBehavior.StopEmittingAndClear
                : ParticleSystemStopBehavior.StopEmitting;
            foreach (ParticleSystem system in RequireSystems(world.RequireObject(objectId)))
            {
                system.Stop(false, behavior);
            }

            return null;
        }

        public IBattlementCommandOperation? Spawn(
            CommandId commandId,
            BattlementDirectParticleSpawn command,
            TimeSpan now
        )
        {
            TimeSpan lifetime = BattlementProtocolLimits.RequireDuration(
                TimeSpan.FromMilliseconds(command.LifetimeMilliseconds),
                "A particle effect lifetime",
                allowZero: false
            );
            if (motionClock.IsInstant || motionClock.IsControlled)
                return null;
            UnityEngine.Vector3 position = command.ObjectId is ObjectId objectId
                ? world.RequireObject(objectId).transform.position
                : new UnityEngine.Vector3(
                    RequireFinite(command.X, "World position X"),
                    RequireFinite(command.Y, "World position Y"),
                    RequireFinite(command.Z, "World position Z")
                );
            var asset = new PreparedAsset.ParticleEffect(
                new ParticleEffectAddress(command.Address)
            );
            IBattlementAssetLease lease = preparedAssets.Acquire(asset);
            return Spawn(commandId, command, now, lifetime, position, lease);
        }

        internal IBattlementCommandOperation? Spawn(
            CommandId commandId,
            BattlementDirectParticleSpawn command,
            TimeSpan now,
            IBattlementAssetLease lease
        )
        {
            TimeSpan lifetime = BattlementProtocolLimits.RequireDuration(
                TimeSpan.FromMilliseconds(command.LifetimeMilliseconds),
                "A particle effect lifetime",
                allowZero: false
            );
            if (motionClock.IsInstant || motionClock.IsControlled)
            {
                lease.Dispose();
                return null;
            }
            UnityEngine.Vector3 position = command.ObjectId is ObjectId objectId
                ? world.RequireObject(objectId).transform.position
                : new UnityEngine.Vector3(
                    RequireFinite(command.X, "World position X"),
                    RequireFinite(command.Y, "World position Y"),
                    RequireFinite(command.Z, "World position Z")
                );
            return Spawn(commandId, command, now, lifetime, position, lease);
        }

        private IBattlementCommandOperation Spawn(
            CommandId commandId,
            BattlementDirectParticleSpawn command,
            TimeSpan now,
            TimeSpan lifetime,
            UnityEngine.Vector3 position,
            IBattlementAssetLease lease
        )
        {
            EffectInstance? instance = null;
            try
            {
                if (lease.Value is not GameObject prefab)
                    throw new BattlementCommandException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared particle effect '{command.Address}' is not a GameObject."
                    );
                if (!prefab.TryGetComponent(out BattlementEffectPool marker))
                {
                    instance = EffectInstance.Create(prefab, lease, null);
                    lease = null!;
                }
                else
                {
                    RequirePoolLimit(marker.MaxInactiveCount);
                    EffectPool pool = GetPool(command.Address, prefab, marker.MaxInactiveCount);
                    instance = pool.Get(lease);
                    lease = null!;
                }
                instance.Acquire(commandId.Value, position);
                return new EffectOperation(instance, now + lifetime);
            }
            catch
            {
                instance?.Destroy();
                lease?.Dispose();
                throw;
            }
        }

        public void ClearInactive()
        {
            foreach (EffectPool pool in pools.Values)
            {
                pool.Clear();
            }

            pools.Clear();
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            Application.lowMemory -= HandleLowMemory;
            ClearInactive();
            isDisposed = true;
        }

        private EffectPool GetPool(string address, GameObject prefab, int maxInactiveCount)
        {
            if (pools.TryGetValue(address, out EffectPool existing))
            {
                if (existing.Matches(prefab, maxInactiveCount))
                {
                    return existing;
                }

                existing.Clear();
            }

            var created = new EffectPool(prefab, maxInactiveCount);
            pools[address] = created;
            return created;
        }

        private void HandleLowMemory()
        {
            ClearInactive();
            Resources.UnloadUnusedAssets();
        }

        private static ParticleSystem[] RequireSystems(GameObject target)
        {
            ParticleSystem[] systems = target.GetComponentsInChildren<ParticleSystem>(true);
            if (systems.Length == 0)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.ComponentMissing,
                    $"Object '{target.name}' has no ParticleSystem in its hierarchy."
                );
            }

            return systems;
        }

        private static void RequirePoolLimit(int value)
        {
            if (value is < 1 or > 128)
            {
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    "BattlementEffectPool maxInactiveCount must be between 1 and 128."
                );
            }
        }

        private static float RequireFinite(double value, string name)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || !float.IsFinite(converted))
            {
                throw new BattlementCommandException(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be finite."
                );
            }

            return converted;
        }

        private sealed class EffectPool
        {
            private readonly GameObject prefab;
            private readonly int maxInactiveCount;
            private readonly ObjectPool<EffectInstance> instances;
            private IBattlementAssetLease? creationLease;
            private bool isRetired;

            public EffectPool(GameObject prefab, int maxInactiveCount)
            {
                this.prefab = prefab;
                this.maxInactiveCount = maxInactiveCount;
                instances = new ObjectPool<EffectInstance>(
                    Create,
                    actionOnDestroy: item => item.Destroy(),
                    collectionCheck: true,
                    defaultCapacity: maxInactiveCount,
                    maxSize: maxInactiveCount
                );
            }

            public bool Matches(GameObject value, int limit) =>
                !isRetired && ReferenceEquals(prefab, value) && maxInactiveCount == limit;

            public EffectInstance Get(IBattlementAssetLease lease)
            {
                if (isRetired)
                {
                    throw new InvalidOperationException("A retired effect pool cannot acquire.");
                }

                creationLease = lease;
                try
                {
                    EffectInstance instance = instances.Get();
                    creationLease?.Dispose();
                    creationLease = null;
                    instance.SetPool(this);
                    return instance;
                }
                catch
                {
                    creationLease?.Dispose();
                    creationLease = null;
                    throw;
                }
            }

            public void Release(EffectInstance instance)
            {
                if (isRetired)
                {
                    instance.Destroy();
                }
                else
                {
                    instances.Release(instance);
                }
            }

            public void Clear()
            {
                isRetired = true;
                instances.Clear();
            }

            private EffectInstance Create()
            {
                IBattlementAssetLease lease =
                    creationLease
                    ?? throw new InvalidOperationException("An effect lease was not supplied.");
                creationLease = null;
                return EffectInstance.Create(prefab, lease, this);
            }
        }

        private sealed class EffectInstance
        {
            private readonly GameObject gameObject;
            private readonly IBattlementAssetLease lease;
            private EffectPool? pool;
            private bool isDestroyed;
            private Guid commandId;

            private EffectInstance(
                GameObject gameObject,
                IBattlementAssetLease lease,
                EffectPool? pool
            ) => (this.gameObject, this.lease, this.pool) = (gameObject, lease, pool);

            public static EffectInstance Create(
                GameObject prefab,
                IBattlementAssetLease lease,
                EffectPool? pool
            )
            {
                GameObject? instance = null;
                try
                {
                    instance = Object.Instantiate(prefab);
                    instance.SetActive(false);
                    return new EffectInstance(instance, lease, pool);
                }
                catch
                {
                    lease.Dispose();
                    if (instance != null)
                    {
                        DestroyUnityObject(instance);
                    }

                    throw;
                }
            }

            public void SetPool(EffectPool value) => pool = value;

            public void Acquire(Guid id, UnityEngine.Vector3 position)
            {
                ResetTransform(position);
                ResetParticles();
                commandId = id;
                InvokeResets(acquiring: true);
                gameObject.SetActive(true);
                foreach (ParticleSystem system in RequireSystems(gameObject))
                {
                    system.Play(false);
                }
            }

            public void Release()
            {
                if (isDestroyed || commandId == Guid.Empty)
                {
                    return;
                }

                try
                {
                    InvokeResets(acquiring: false);
                    ResetParticles();
                    gameObject.SetActive(false);
                    ResetTransform(UnityEngine.Vector3.zero);
                    commandId = Guid.Empty;
                    if (pool is EffectPool retainedPool)
                    {
                        retainedPool.Release(this);
                    }
                    else
                    {
                        Destroy();
                    }
                }
                catch
                {
                    Destroy();
                    throw;
                }
            }

            public void Destroy()
            {
                if (isDestroyed)
                {
                    return;
                }

                isDestroyed = true;
                commandId = Guid.Empty;
                lease.Dispose();
                if (gameObject != null)
                {
                    gameObject.SetActive(false);
                    DestroyUnityObject(gameObject);
                }
            }

            private void InvokeResets(bool acquiring)
            {
                foreach (MonoBehaviour component in gameObject.GetComponents<MonoBehaviour>())
                {
                    if (component is not IBattlementPoolReset reset)
                    {
                        continue;
                    }

                    if (acquiring)
                    {
                        reset.OnBattlementAcquire();
                    }
                    else
                    {
                        reset.OnBattlementRelease();
                    }
                }
            }

            private void ResetParticles()
            {
                foreach (
                    ParticleSystem system in gameObject.GetComponentsInChildren<ParticleSystem>(
                        true
                    )
                )
                {
                    system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                }
            }

            private void ResetTransform(UnityEngine.Vector3 position)
            {
                Transform transform = gameObject.transform;
                transform.SetParent(null, false);
                transform.SetPositionAndRotation(position, UnityEngine.Quaternion.identity);
                transform.localScale = UnityEngine.Vector3.one;
            }
        }

        private sealed class EffectOperation : IBattlementCommandOperation
        {
            private readonly EffectInstance instance;
            private readonly TimeSpan completion;
            private bool isComplete;

            public EffectOperation(EffectInstance instance, TimeSpan completion) =>
                (this.instance, this.completion) = (instance, completion);

            public bool IsInfinite => false;

            public bool IsComplete(TimeSpan now)
            {
                if (!isComplete && now >= completion)
                {
                    Complete();
                }

                return isComplete;
            }

            public void Cancel() => Complete();

            private void Complete()
            {
                if (isComplete)
                {
                    return;
                }

                isComplete = true;
                instance.Release();
            }
        }

        private static void DestroyUnityObject(Object value)
        {
            if (Application.isPlaying)
            {
                Object.Destroy(value);
            }
            else
            {
                Object.DestroyImmediate(value);
            }
        }
    }
}
