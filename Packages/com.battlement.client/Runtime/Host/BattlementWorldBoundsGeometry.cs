#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityBounds = UnityEngine.Bounds;
using UnityRect = UnityEngine.Rect;
using UnityVector3 = UnityEngine.Vector3;
using UnityVector4 = UnityEngine.Vector4;

namespace Battlement
{
    internal static class BattlementWorldBoundsGeometry
    {
        private static readonly (int Left, int Right)[] Edges =
        {
            (0, 1),
            (0, 2),
            (0, 4),
            (1, 3),
            (1, 5),
            (2, 3),
            (2, 6),
            (3, 7),
            (4, 5),
            (4, 6),
            (5, 7),
            (6, 7),
        };

        public static GeometryObservationResult Sample(
            GameObject target,
            Camera camera,
            IBattlementGeometryDisplaySource displays
        )
        {
            if (!camera.isActiveAndEnabled)
                return Unavailable(GeometryUnavailable.CameraDisabled);
            if (!TryCombinedBounds(target, out UnityBounds bounds))
                return Unavailable(GeometryUnavailable.NoRenderers);

            RequireFinite(camera.worldToCameraMatrix, "World camera transform");
            RequireFinite(camera.projectionMatrix, "World camera projection");
            double near = camera.nearClipPlane;
            if (!double.IsFinite(near))
                throw Invalid("World camera near plane is nonfinite.");
            if (near <= 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);

            UnityVector3[] corners = Corners(bounds);
            double[] depths = Depths(camera, corners);
            var clipped = new List<UnityVector3>(20);
            for (int index = 0; index < corners.Length; index++)
                if (depths[index] >= near)
                    clipped.Add(corners[index]);
            foreach ((int leftIndex, int rightIndex) in Edges)
            {
                bool leftFront = depths[leftIndex] >= near;
                bool rightFront = depths[rightIndex] >= near;
                if (leftFront == rightFront)
                    continue;
                double amount =
                    (near - depths[leftIndex]) / (depths[rightIndex] - depths[leftIndex]);
                clipped.Add(
                    UnityVector3.LerpUnclamped(
                        corners[leftIndex],
                        corners[rightIndex],
                        (float)amount
                    )
                );
            }
            if (clipped.Count == 0)
                return Unavailable(GeometryUnavailable.BehindCamera);
            if (camera.targetTexture != null)
                return Unavailable(GeometryUnavailable.NoViewportMapping);

            var displayId = new DisplayId(checked((uint)camera.targetDisplay));
            if (!displays.TryGet(displayId, out BattlementDisplayGeometry display))
                return Unavailable(GeometryUnavailable.DisplayUnavailable);
            UnityRect viewportRect = camera.rect;
            if (!Finite(viewportRect) || !Finite(display.Width, display.Height))
                throw Invalid("World camera viewport contains nonfinite geometry.");
            if (viewportRect.width <= 0 || viewportRect.height <= 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            float determinant = camera.projectionMatrix.determinant;
            if (!float.IsFinite(determinant) || determinant == 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            if (CrossesProjectionHorizon(camera, clipped))
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);

            double leftViewport = double.PositiveInfinity;
            double rightViewport = double.NegativeInfinity;
            double bottomViewport = double.PositiveInfinity;
            double topViewport = double.NegativeInfinity;
            double nearest = double.PositiveInfinity;
            double farthest = double.NegativeInfinity;
            foreach (UnityVector3 point in clipped)
            {
                UnityVector3 viewport = camera.WorldToViewportPoint(point);
                if (!Finite(viewport))
                    return Unavailable(GeometryUnavailable.ProjectionUnavailable);
                double depth = Depth(camera, point);
                leftViewport = Math.Min(leftViewport, viewport.x);
                rightViewport = Math.Max(rightViewport, viewport.x);
                bottomViewport = Math.Min(bottomViewport, viewport.y);
                topViewport = Math.Max(topViewport, viewport.y);
                nearest = Math.Min(nearest, depth);
                farthest = Math.Max(farthest, depth);
            }

            double left = (viewportRect.x + leftViewport * viewportRect.width) * display.Width;
            double right = (viewportRect.x + rightViewport * viewportRect.width) * display.Width;
            double top =
                display.Height
                - (viewportRect.y + topViewport * viewportRect.height) * display.Height;
            double bottom =
                display.Height
                - (viewportRect.y + bottomViewport * viewportRect.height) * display.Height;
            double cameraLeft = viewportRect.x * display.Width;
            double cameraRight = (viewportRect.x + viewportRect.width) * display.Width;
            double cameraTop = display.Height - viewportRect.yMax * display.Height;
            double cameraBottom = display.Height - viewportRect.y * display.Height;
            bool insideX = right >= cameraLeft && left < cameraRight;
            bool insideY = bottom >= cameraTop && top < cameraBottom;
            return Current(
                new WorldBoundsGeometry(
                    new ViewportRect(left, top, right - left, bottom - top, displayId),
                    nearest,
                    farthest,
                    insideX && insideY
                )
            );
        }

        private static bool TryCombinedBounds(GameObject target, out UnityBounds combined)
        {
            combined = default;
            bool found = false;
            foreach (Renderer renderer in target.GetComponentsInChildren<Renderer>(true))
            {
                if (!Qualifies(renderer))
                    continue;
                UnityBounds bounds = renderer.bounds;
                if (!Finite(bounds.min) || !Finite(bounds.max))
                    throw Invalid($"Renderer '{renderer.name}' has nonfinite world bounds.");
                if (!found)
                {
                    combined = bounds;
                    found = true;
                }
                else
                {
                    combined.Encapsulate(bounds.min);
                    combined.Encapsulate(bounds.max);
                }
            }
            return found;
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

        private static double[] Depths(Camera camera, IReadOnlyList<UnityVector3> points)
        {
            var depths = new double[points.Count];
            for (int index = 0; index < points.Count; index++)
                depths[index] = Depth(camera, points[index]);
            return depths;
        }

        private static bool CrossesProjectionHorizon(
            Camera camera,
            IReadOnlyList<UnityVector3> points
        )
        {
            Matrix4x4 projection = camera.projectionMatrix * camera.worldToCameraMatrix;
            int sign = 0;
            foreach (UnityVector3 point in points)
            {
                UnityVector4 clip = projection * new UnityVector4(point.x, point.y, point.z, 1);
                if (!float.IsFinite(clip.w) || clip.w == 0)
                    return true;
                int pointSign = Math.Sign(clip.w);
                if (sign != 0 && pointSign != sign)
                    return true;
                sign = pointSign;
            }
            return false;
        }

        private static double Depth(Camera camera, UnityVector3 point)
        {
            double depth = UnityVector3.Dot(
                camera.transform.forward,
                point - camera.transform.position
            );
            if (!double.IsFinite(depth))
                throw Invalid("World renderer bounds contain nonfinite camera depth.");
            return depth;
        }

        private static bool Finite(UnityRect value) =>
            Finite(value.x, value.y) && Finite(value.width, value.height);

        private static bool Finite(UnityVector3 value) =>
            Finite(value.x, value.y) && float.IsFinite(value.z);

        private static bool Finite(float left, float right) =>
            float.IsFinite(left) && float.IsFinite(right);

        private static bool Finite(double left, double right) =>
            double.IsFinite(left) && double.IsFinite(right);

        private static void RequireFinite(Matrix4x4 value, string name)
        {
            for (int row = 0; row < 4; row++)
            for (int column = 0; column < 4; column++)
                if (!float.IsFinite(value[row, column]))
                    throw Invalid($"{name} contains nonfinite geometry.");
        }

        private static InvalidOperationException Invalid(string message) => new(message);

        private static GeometryObservationResult Current(WorldBoundsGeometry value) =>
            new GeometryObservationResult.Current(new GeometryValue.WorldBounds(value));

        private static GeometryObservationResult Unavailable(GeometryUnavailable reason) =>
            new GeometryObservationResult.Unavailable(reason);
    }
}
