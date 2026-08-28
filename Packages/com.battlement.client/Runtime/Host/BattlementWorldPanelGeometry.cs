#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolRect = Battlement.Rect;
using UnityVector2 = UnityEngine.Vector2;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement
{
    internal static class BattlementWorldPanelGeometry
    {
        public static GeometryObservationResult Sample(
            VisualElement element,
            ObjectId panelId,
            UIDocument document,
            Camera? camera,
            IBattlementGeometryDisplaySource displays
        )
        {
            if (camera == null || !camera.isActiveAndEnabled)
                return Unavailable(GeometryUnavailable.CameraDisabled);
            if (camera.targetTexture != null)
                return Unavailable(GeometryUnavailable.NoViewportMapping);

            var displayId = new DisplayId(checked((uint)camera.targetDisplay));
            if (!displays.TryGet(displayId, out BattlementDisplayGeometry display))
                return Unavailable(GeometryUnavailable.DisplayUnavailable);
            UnityEngine.Rect layout = element.layout;
            if (!Finite(layout))
                throw Invalid("UI layout contains nonfinite geometry.");
            Matrix4x4 localPlane = Plane(document, element);
            if (element.parent == null)
                return Unavailable(GeometryUnavailable.Detached);
            Matrix4x4 parentPlane = Plane(document, element.parent);
            if (
                !TryProjective(camera, display, localPlane, out Projective2 localTransform)
                || !TryProjective(camera, display, parentPlane, out Projective2 parentTransform)
            )
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);

            GeometryUnavailable? unavailable = TryBound(
                camera,
                localPlane,
                localTransform,
                layout,
                out ViewportRect bound,
                displayId
            );
            return unavailable is GeometryUnavailable reason
                ? Unavailable(reason)
                : Current(
                    new ElementGeometry(
                        new ProtocolRect(layout.x, layout.y, layout.width, layout.height),
                        bound,
                        localTransform,
                        parentTransform,
                        panelId
                    )
                );
        }

        private static GeometryUnavailable? TryBound(
            Camera camera,
            Matrix4x4 plane,
            Projective2 projective,
            UnityEngine.Rect layout,
            out ViewportRect bound,
            DisplayId displayId
        )
        {
            var vertices = new List<Vertex>(4);
            UnityVector2[] corners =
            {
                UnityVector2.zero,
                new(layout.width, 0),
                new(layout.width, layout.height),
                new(0, layout.height),
            };
            double nearest = double.PositiveInfinity;
            double farthest = double.NegativeInfinity;
            foreach (UnityVector2 corner in corners)
            {
                double depth = Depth(camera, plane.MultiplyPoint3x4(corner));
                if (!double.IsFinite(depth))
                    throw Invalid("World-panel camera depth is nonfinite.");
                nearest = Math.Min(nearest, depth);
                farthest = Math.Max(farthest, depth);
                vertices.Add(new Vertex(corner, depth));
            }

            double near = camera.nearClipPlane;
            if (!double.IsFinite(near))
                throw Invalid("World-panel camera near clipping is nonfinite.");
            if (near <= 0)
            {
                bound = null!;
                return GeometryUnavailable.ProjectionUnavailable;
            }
            if (farthest < near)
            {
                bound = null!;
                return GeometryUnavailable.BehindCamera;
            }
            if (nearest <= 0)
            {
                bound = null!;
                return GeometryUnavailable.ProjectionUnavailable;
            }

            vertices = Clip(vertices, near);
            if (CrossesHorizon(vertices, projective))
            {
                bound = null!;
                return GeometryUnavailable.ProjectionUnavailable;
            }
            double minX = double.PositiveInfinity;
            double minY = double.PositiveInfinity;
            double maxX = double.NegativeInfinity;
            double maxY = double.NegativeInfinity;
            foreach (Vertex vertex in vertices)
            {
                if (!TryMap(projective, vertex.Point, out UnityVector2 mapped))
                {
                    bound = null!;
                    return GeometryUnavailable.ProjectionUnavailable;
                }
                minX = Math.Min(minX, mapped.x);
                minY = Math.Min(minY, mapped.y);
                maxX = Math.Max(maxX, mapped.x);
                maxY = Math.Max(maxY, mapped.y);
            }
            bound = new ViewportRect(minX, minY, maxX - minX, maxY - minY, displayId);
            return null;
        }

        private static List<Vertex> Clip(IReadOnlyList<Vertex> input, double near)
        {
            var result = new List<Vertex>(input.Count + 2);
            Vertex previous = input[^1];
            bool previousInside = previous.Depth >= near;
            foreach (Vertex current in input)
            {
                bool currentInside = current.Depth >= near;
                if (currentInside != previousInside)
                {
                    double amount = (near - previous.Depth) / (current.Depth - previous.Depth);
                    result.Add(
                        new Vertex(
                            UnityVector2.LerpUnclamped(
                                previous.Point,
                                current.Point,
                                (float)amount
                            ),
                            near
                        )
                    );
                }
                if (currentInside)
                    result.Add(current);
                previous = current;
                previousInside = currentInside;
            }
            return result;
        }

        private static bool CrossesHorizon(IReadOnlyList<Vertex> vertices, Projective2 projective)
        {
            int sign = 0;
            foreach (Vertex vertex in vertices)
            {
                double divisor = Divisor(projective, vertex.Point);
                if (!double.IsFinite(divisor) || divisor == 0)
                    return true;
                int current = divisor > 0 ? 1 : -1;
                if (sign != 0 && sign != current)
                    return true;
                sign = current;
            }
            return false;
        }

        private static Matrix4x4 Plane(UIDocument document, VisualElement element)
        {
            UnityVector3 origin = document.transform.TransformPoint(
                element.worldTransform.MultiplyPoint3x4(UnityVector3.zero)
            );
            UnityVector3 right = document.transform.TransformPoint(
                element.worldTransform.MultiplyPoint3x4(UnityVector3.right)
            );
            UnityVector3 up = document.transform.TransformPoint(
                element.worldTransform.MultiplyPoint3x4(UnityVector3.up)
            );
            if (!Finite(origin) || !Finite(right) || !Finite(up))
                throw Invalid("World-panel transform contains nonfinite geometry.");

            Matrix4x4 plane = Matrix4x4.identity;
            plane.SetColumn(0, right - origin);
            plane.SetColumn(1, up - origin);
            plane.SetColumn(3, new Vector4(origin.x, origin.y, origin.z, 1));
            return plane;
        }

        private static bool TryProjective(
            Camera camera,
            BattlementDisplayGeometry display,
            Matrix4x4 plane,
            out Projective2 result
        )
        {
            UnityEngine.Rect viewport = camera.rect;
            RequireFinite(camera.projectionMatrix, "World-panel camera projection");
            RequireFinite(camera.worldToCameraMatrix, "World-panel camera transform");
            RequireFinite(plane, "World-panel element transform");
            if (!Finite(viewport))
                throw Invalid("World-panel camera viewport contains nonfinite geometry.");
            if (!Finite(display.Width, display.Height))
                throw Invalid("World-panel display contains nonfinite geometry.");
            double width = viewport.width * display.Width;
            double height = viewport.height * display.Height;
            double centerX = viewport.x * display.Width + width / 2;
            double centerY = display.Height - viewport.y * display.Height - height / 2;
            Matrix4x4 clip = camera.projectionMatrix * camera.worldToCameraMatrix * plane;
            result = new Projective2(
                width / 2 * clip.m00 + centerX * clip.m30,
                width / 2 * clip.m01 + centerX * clip.m31,
                width / 2 * clip.m03 + centerX * clip.m33,
                -height / 2 * clip.m10 + centerY * clip.m30,
                -height / 2 * clip.m11 + centerY * clip.m31,
                -height / 2 * clip.m13 + centerY * clip.m33,
                clip.m30,
                clip.m31,
                clip.m33
            );
            double determinant =
                result.M11 * (result.M22 * result.M33 - result.M23 * result.M32)
                - result.M12 * (result.M21 * result.M33 - result.M23 * result.M31)
                + result.M13 * (result.M21 * result.M32 - result.M22 * result.M31);
            if (width <= 0 || height <= 0)
                return false;
            if (!Finite(result) || !double.IsFinite(determinant))
                return false;
            return determinant != 0;
        }

        private static bool TryMap(Projective2 value, UnityVector2 point, out UnityVector2 mapped)
        {
            double divisor = Divisor(value, point);
            if (!double.IsFinite(divisor) || divisor == 0)
            {
                mapped = default;
                return false;
            }
            double x = (value.M11 * point.x + value.M12 * point.y + value.M13) / divisor;
            double y = (value.M21 * point.x + value.M22 * point.y + value.M23) / divisor;
            mapped = new UnityVector2((float)x, (float)y);
            return double.IsFinite(x) && double.IsFinite(y);
        }

        private static double Divisor(Projective2 value, UnityVector2 point) =>
            value.M31 * point.x + value.M32 * point.y + value.M33;

        private static double Depth(Camera camera, UnityVector3 world) =>
            UnityVector3.Dot(camera.transform.forward, world - camera.transform.position);

        private static bool Finite(Projective2 value)
        {
            double[] components =
            {
                value.M11,
                value.M12,
                value.M13,
                value.M21,
                value.M22,
                value.M23,
                value.M31,
                value.M32,
                value.M33,
            };
            foreach (double component in components)
                if (!double.IsFinite(component))
                    return false;
            return true;
        }

        private static bool Finite(UnityEngine.Rect value) =>
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

        private static GeometryObservationResult Current(ElementGeometry value) =>
            new GeometryObservationResult.Current(new GeometryValue.Element(value));

        private static GeometryObservationResult Unavailable(GeometryUnavailable reason) =>
            new GeometryObservationResult.Unavailable(reason);

        private readonly struct Vertex
        {
            public Vertex(UnityVector2 point, double depth)
            {
                Point = point;
                Depth = depth;
            }

            public UnityVector2 Point { get; }

            public double Depth { get; }
        }
    }
}
