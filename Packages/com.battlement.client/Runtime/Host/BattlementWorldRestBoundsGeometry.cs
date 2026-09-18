#nullable enable

using System;
using UnityEngine;
using UnityBounds = UnityEngine.Bounds;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement
{
    internal static class BattlementWorldRestBoundsGeometry
    {
        public static GeometryObservationResult Sample(GameObject target)
        {
            Matrix4x4 worldToTarget = target.transform.worldToLocalMatrix;
            bool found = false;
            double minX = double.PositiveInfinity;
            double minY = double.PositiveInfinity;
            double maxX = double.NegativeInfinity;
            double maxY = double.NegativeInfinity;
            foreach (Renderer renderer in target.GetComponentsInChildren<Renderer>(true))
            {
                if (!Qualifies(renderer))
                    continue;
                UnityBounds bounds = renderer.localBounds;
                if (!Finite(bounds.min) || !Finite(bounds.max))
                    throw Invalid($"Renderer '{renderer.name}' has nonfinite local bounds.");
                Matrix4x4 localToTarget = worldToTarget * renderer.localToWorldMatrix;
                foreach (UnityVector3 corner in Corners(bounds))
                {
                    UnityVector3 point = localToTarget.MultiplyPoint3x4(corner);
                    if (!Finite(point))
                        throw Invalid(
                            $"Renderer '{renderer.name}' produced nonfinite rest bounds."
                        );
                    minX = Math.Min(minX, point.x);
                    minY = Math.Min(minY, point.y);
                    maxX = Math.Max(maxX, point.x);
                    maxY = Math.Max(maxY, point.y);
                }
                found = true;
            }
            if (!found)
                return Unavailable(GeometryUnavailable.NoRenderers);
            double width = maxX - minX;
            double height = maxY - minY;
            if (width <= 0 || height <= 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            return Current(new WorldRestBoundsGeometry(new Rect(minX, minY, width, height)));
        }

        private static bool Qualifies(Renderer renderer)
        {
            if (!renderer.enabled || !renderer.gameObject.activeInHierarchy)
                return false;
            return renderer is not ParticleSystemRenderer
                && renderer is not TrailRenderer
                && renderer is not LineRenderer;
        }

        private static UnityVector3[] Corners(UnityBounds bounds)
        {
            UnityVector3 min = bounds.min;
            UnityVector3 max = bounds.max;
            return new[]
            {
                new UnityVector3(min.x, min.y, min.z),
                new UnityVector3(max.x, min.y, min.z),
                new UnityVector3(min.x, max.y, min.z),
                new UnityVector3(max.x, max.y, min.z),
                new UnityVector3(min.x, min.y, max.z),
                new UnityVector3(max.x, min.y, max.z),
                new UnityVector3(min.x, max.y, max.z),
                new UnityVector3(max.x, max.y, max.z),
            };
        }

        private static bool Finite(UnityVector3 value) =>
            float.IsFinite(value.x) && float.IsFinite(value.y) && float.IsFinite(value.z);

        private static InvalidOperationException Invalid(string message) => new(message);

        private static GeometryObservationResult Current(WorldRestBoundsGeometry value) =>
            new GeometryObservationResult.Current(new GeometryValue.WorldRestBounds(value));

        private static GeometryObservationResult Unavailable(GeometryUnavailable reason) =>
            new GeometryObservationResult.Unavailable(reason);
    }
}
