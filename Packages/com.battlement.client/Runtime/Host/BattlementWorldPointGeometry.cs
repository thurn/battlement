#nullable enable

using System;
using UnityEngine;
using UnityRect = UnityEngine.Rect;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement
{
    internal static class BattlementWorldPointGeometry
    {
        public static Transform FindAnchor(GameObject target, AnchorName name)
        {
            if (!target.TryGetComponent(out BattlementGeometryAnchorMap map))
                throw Invalid($"Object '{target.name}' has no prepared geometry anchor metadata.");
            return map.Resolve(name);
        }

        public static GeometryObservationResult Sample(
            Transform point,
            Camera camera,
            IBattlementGeometryDisplaySource displays
        )
        {
            if (!camera.isActiveAndEnabled)
                return Unavailable(GeometryUnavailable.CameraDisabled);
            if (camera.targetTexture != null)
                return Unavailable(GeometryUnavailable.NoViewportMapping);

            var displayId = new DisplayId(checked((uint)camera.targetDisplay));
            if (!displays.TryGet(displayId, out BattlementDisplayGeometry display))
                return Unavailable(GeometryUnavailable.DisplayUnavailable);
            UnityVector3 world = point.position;
            if (!Finite(world))
                throw Invalid("World geometry contains a nonfinite position.");
            RequireFinite(camera.worldToCameraMatrix, "World camera transform");
            RequireFinite(camera.projectionMatrix, "World camera projection");
            UnityRect viewportRect = camera.rect;
            if (!Finite(viewportRect) || !Finite(display.Width, display.Height))
                throw Invalid("World camera viewport contains nonfinite geometry.");
            if (viewportRect.width <= 0 || viewportRect.height <= 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            if (camera.projectionMatrix.determinant == 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);

            double depth = UnityVector3.Dot(
                camera.transform.forward,
                world - camera.transform.position
            );
            double near = camera.nearClipPlane;
            if (!double.IsFinite(depth) || !double.IsFinite(near))
                throw Invalid("World camera depth contains nonfinite geometry.");
            if (near <= 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            if (depth < near)
                return Unavailable(GeometryUnavailable.BehindCamera);

            UnityVector3 viewport = camera.WorldToViewportPoint(world);
            if (!Finite(viewport))
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            double x = (viewportRect.x + viewport.x * viewportRect.width) * display.Width;
            double y =
                display.Height
                - (viewportRect.y + viewport.y * viewportRect.height) * display.Height;
            bool insideX = viewport.x >= 0 && viewport.x < 1;
            bool insideY = viewport.y >= 0 && viewport.y < 1;
            return Current(
                new WorldPointGeometry(
                    new ViewportPoint(x, y, displayId),
                    depth,
                    insideX && insideY
                )
            );
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

        private static GeometryObservationResult Current(WorldPointGeometry value) =>
            new GeometryObservationResult.Current(new GeometryValue.WorldPoint(value));

        private static GeometryObservationResult Unavailable(GeometryUnavailable reason) =>
            new GeometryObservationResult.Unavailable(reason);
    }
}
