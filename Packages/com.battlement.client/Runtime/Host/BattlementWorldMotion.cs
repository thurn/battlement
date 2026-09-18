#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using UnityEngine;

namespace Battlement
{
    /// <summary>Connects world object lifetimes to the shared Motion registry.</summary>
    internal sealed class BattlementWorldMotion : IDisposable
    {
        private readonly BattlementWorld world;
        private readonly BattlementWorldMotionGestures gestures;
        private readonly Dictionary<Guid, Target> targets = new();
        private BattlementUiDocuments? documents;
        private BattlementAudioSources? audioSources;
        private int rebuilding;

        public BattlementWorldMotion(BattlementWorld world)
        {
            this.world = world;
            gestures = new BattlementWorldMotionGestures(
                world,
                (host, layer, active) =>
                    documents?.MotionWorld.SetNativeGesture(host, layer, active)
            );
        }

        internal void Handle(UiEvent value) => gestures.Handle(value);

        public void Bind(BattlementUiDocuments value)
        {
            if (ReferenceEquals(documents, value))
                return;
            if (documents is not null)
                throw new InvalidOperationException("A world cannot use two Motion registries.");
            documents = value;
            value.RestoreNativeMotion = Restore;
        }

        public void Bind(BattlementAudioSources value)
        {
            if (ReferenceEquals(audioSources, value))
                return;
            if (audioSources is not null)
                throw new InvalidOperationException("A world cannot use two audio hosts.");
            audioSources = value;
        }

        public IDisposable Rebuild()
        {
            foreach (Target target in targets.Values)
                target.Properties.CapturePresentation();
            rebuilding++;
            return new RebuildScope(this);
        }

        public void Created(ObjectId id)
        {
            if (rebuilding != 0)
                return;
            MotionDescriptor? descriptor = world
                .RequireObject(id)
                .GetComponent<BattlementIdentity>()
                .Motion;
            if (descriptor is not null)
                Install(id, descriptor);
        }

        public IBattlementCommandOperation? Install(
            ObjectId id,
            MotionDescriptor? descriptor,
            bool includeTimelines = false
        )
        {
            BattlementWorldMotionTarget.Validate(id, descriptor);
            BattlementIdentity identity = world
                .RequireObject(id)
                .GetComponent<BattlementIdentity>();
            if (descriptor is null)
            {
                Remove(id);
                identity.Motion = null;
                return null;
            }
            if (documents is null)
                throw new InvalidOperationException(
                    "World Motion requires the shared host registry."
                );
            Transform transform = identity.transform;
            Target target =
                targets.TryGetValue(id.Value, out Target prior)
                && ReferenceEquals(prior.Transform, transform)
                    ? prior
                    : new Target(
                        transform,
                        new BattlementWorldMotionTarget(transform, audioSources, world)
                    );
            target.Properties.Configure(descriptor);
            using BattlementPreparedMotionAdmission? prepared = documents.MotionWorld.Prepare(
                target.Properties,
                id,
                descriptor
            );
            IBattlementCommandOperation? operation = prepared?.Commit(includeTimelines);
            targets[id.Value] = target;
            identity.Motion = descriptor;
            gestures.Restore(id);
            return operation;
        }

        public void Remove(ObjectId id)
        {
            if (rebuilding != 0)
                return;
            gestures.Remove(id);
            if (targets.Remove(id.Value))
                documents?.MotionWorld.RemoveHost(id);
        }

        public void Dispose()
        {
            foreach (Guid id in targets.Keys.ToArray())
                Remove(new ObjectId(id));
            if (documents is not null)
                documents.RestoreNativeMotion = null;
            documents = null;
            audioSources = null;
        }

        private void Restore()
        {
            var live = new HashSet<Guid>();
            foreach (BattlementIdentity identity in world.Identities)
            {
                if (identity.Motion is null)
                    continue;
                live.Add(identity.Id);
                Install(new ObjectId(identity.Id), identity.Motion);
            }
            foreach (Guid id in targets.Keys.Where(id => !live.Contains(id)).ToArray())
                Remove(new ObjectId(id));
        }

        private sealed record Target(Transform Transform, BattlementWorldMotionTarget Properties);

        private sealed class RebuildScope : IDisposable
        {
            private BattlementWorldMotion? owner;

            public RebuildScope(BattlementWorldMotion owner) => this.owner = owner;

            public void Dispose()
            {
                if (owner is null)
                    return;
                owner.rebuilding--;
                owner = null;
            }
        }
    }
}
