#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    internal sealed class BattlementGeometryFrames
    {
        private readonly Dictionary<GeometryObservationId, GeometryObservationResult> submitted =
            new();
        private readonly Dictionary<GeometryObservationId, GeometryObservationResult> pending =
            new();
        private GeometryGeneration? generation;

        public void Retire(GeometryObservationUpdate update)
        {
            foreach (GeometryObservationId id in update.Removed)
            {
                submitted.Remove(id);
                pending.Remove(id);
            }
        }

        public void Merge(GeometryObservationBatch batch)
        {
            if (generation is { } previous && batch.Generation.Value <= previous.Value)
                throw new System.ArgumentException(
                    "A geometry generation must increase while frames are pending."
                );

            generation = batch.Generation;
            foreach (GeometryObservationValue changed in batch.Changed)
            {
                if (
                    submitted.TryGetValue(
                        changed.ObservationId,
                        out GeometryObservationResult submittedResult
                    ) && Equals(submittedResult, changed.Result)
                )
                    pending.Remove(changed.ObservationId);
                else
                    pending[changed.ObservationId] = changed.Result;
            }
        }

        public GeometryObservationBatch? Take()
        {
            if (pending.Count == 0)
                return null;
            if (generation is not GeometryGeneration current)
                throw new System.InvalidOperationException(
                    "Pending geometry requires a sampled generation."
                );

            var changed = new List<GeometryObservationValue>(pending.Count);
            foreach (
                KeyValuePair<
                    GeometryObservationId,
                    GeometryObservationResult
                > observation in pending
            )
            {
                changed.Add(new GeometryObservationValue(observation.Key, observation.Value));
                submitted[observation.Key] = observation.Value;
            }

            pending.Clear();
            return new GeometryObservationBatch(current, changed);
        }

        public void Reset()
        {
            submitted.Clear();
            pending.Clear();
            generation = null;
        }
    }
}
