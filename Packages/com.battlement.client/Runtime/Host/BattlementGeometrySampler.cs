#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using UnityEngine;
using UnityEngine.UIElements;
using ProtocolRect = Battlement.Rect;
using UnityPanelRenderMode = UnityEngine.UIElements.PanelRenderMode;

namespace Battlement
{
    internal readonly struct BattlementDisplayGeometry
    {
        public BattlementDisplayGeometry(
            double width,
            double height,
            UnityEngine.Rect safeArea,
            double scale,
            double? dpi,
            DisplayOrientation orientation
        )
        {
            Width = width;
            Height = height;
            SafeArea = safeArea;
            Scale = scale;
            Dpi = dpi;
            Orientation = orientation;
        }

        public double Width { get; }

        public double Height { get; }

        public UnityEngine.Rect SafeArea { get; }

        public double Scale { get; }

        public double? Dpi { get; }

        public DisplayOrientation Orientation { get; }
    }

    internal interface IBattlementGeometryDisplaySource
    {
        bool TryGet(DisplayId id, out BattlementDisplayGeometry geometry);
    }

    internal sealed class BattlementGeometrySampler
    {
        private readonly BattlementUiDocuments documents;
        private readonly IBattlementGeometryDisplaySource displays;
        private readonly GeometryRegistry registry = new();
        private readonly Dictionary<GeometryObservationId, GeometryObservationResult> latest =
            new();
        private ulong generation;

        public BattlementGeometrySampler(
            BattlementUiDocuments documents,
            IBattlementGeometryDisplaySource? displays = null
        )
        {
            this.documents = documents;
            this.displays = displays ?? new UnityBattlementGeometryDisplaySource();
        }

        public void Apply(GeometryObservationUpdate update)
        {
            registry.Apply(update);
            foreach (GeometryObservationId id in update.Removed)
                latest.Remove(id);
        }

        public GeometryObservationBatch? Sample()
        {
            if (registry.Targets.Count == 0)
                return null;

            generation = checked(generation + 1);
            var changed = new List<GeometryObservationValue>();
            foreach (
                KeyValuePair<
                    GeometryObservationId,
                    GeometryObservationTarget
                > observation in registry.Targets
            )
            {
                GeometryObservationResult result = Sample(observation.Value);
                if (!latest.TryGetValue(observation.Key, out GeometryObservationResult previous))
                    changed.Add(new GeometryObservationValue(observation.Key, result));
                else if (!Equals(previous, result))
                    changed.Add(new GeometryObservationValue(observation.Key, result));
                latest[observation.Key] = result;
            }

            return changed.Count == 0
                ? null
                : new GeometryObservationBatch(new GeometryGeneration(generation), changed);
        }

        private GeometryObservationResult Sample(GeometryObservationTarget target) =>
            target switch
            {
                GeometryObservationTarget.UiElement element => SampleElement(element.ObjectId),
                GeometryObservationTarget.Viewport viewport => SampleViewport(viewport.DisplayId),
                _ => throw new InvalidOperationException(
                    $"Geometry target {target.GetType().Name} is not supported by this sampler."
                ),
            };

        private GeometryObservationResult SampleElement(ObjectId id)
        {
            if (
                !documents.TryGetGeometryTarget(
                    id,
                    out VisualElement element,
                    out ObjectId panelId,
                    out UIDocument document
                )
                || document == null
                || element.panel == null
            )
                return Unavailable(GeometryUnavailable.Detached);
            if (IsHidden(element))
                return Unavailable(GeometryUnavailable.Hidden);

            PanelSettings panel = document.panelSettings;
            var displayId = new DisplayId(checked((uint)panel.targetDisplay));
            if (!displays.TryGet(displayId, out _))
                return Unavailable(GeometryUnavailable.DisplayUnavailable);
            if (panel.targetTexture != null)
                return Unavailable(GeometryUnavailable.NoViewportMapping);
            if (panel.renderMode == UnityPanelRenderMode.WorldSpace)
                throw new InvalidOperationException(
                    "World-space UI sampling is not available yet."
                );

            double scale = element.panel.scaledPixelsPerPoint;
            UnityEngine.Rect layout = element.layout;
            UnityEngine.Rect bound = element.worldBound;
            if (!Finite(scale, layout, bound) || scale <= 0)
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);

            Matrix4x4 parent = element.parent?.worldTransform ?? Matrix4x4.identity;
            if (
                !TryProjective(element.worldTransform, scale, out Projective2 localTransform)
                || !TryProjective(parent, scale, out Projective2 parentTransform)
            )
                return Unavailable(GeometryUnavailable.ProjectionUnavailable);
            return Current(
                new ElementGeometry(
                    new ProtocolRect(layout.x, layout.y, layout.width, layout.height),
                    Viewport(bound, scale, displayId),
                    localTransform,
                    parentTransform,
                    panelId
                )
            );
        }

        private GeometryObservationResult SampleViewport(DisplayId id)
        {
            if (!displays.TryGet(id, out BattlementDisplayGeometry display))
                return Unavailable(GeometryUnavailable.DisplayUnavailable);

            ViewportRect viewport = new(0, 0, display.Width, display.Height, id);
            UnityEngine.Rect safe = display.SafeArea;
            ViewportRect safeArea = new(
                safe.x,
                display.Height - safe.yMax,
                safe.width,
                safe.height,
                id
            );
            return Current(
                new ViewportGeometry(
                    viewport,
                    safeArea,
                    display.Scale,
                    display.Dpi,
                    display.Orientation
                )
            );
        }

        private static GeometryObservationResult Current(ElementGeometry value) =>
            new GeometryObservationResult.Current(new GeometryValue.Element(value));

        private static GeometryObservationResult Current(ViewportGeometry value) =>
            new GeometryObservationResult.Current(new GeometryValue.Viewport(value));

        private static GeometryObservationResult Unavailable(GeometryUnavailable reason) =>
            new GeometryObservationResult.Unavailable(reason);

        private static ViewportRect Viewport(
            UnityEngine.Rect value,
            double scale,
            DisplayId displayId
        ) =>
            new(
                value.x * scale,
                value.y * scale,
                value.width * scale,
                value.height * scale,
                displayId
            );

        private static bool TryProjective(Matrix4x4 value, double scale, out Projective2 result)
        {
            result = new Projective2(
                value.m00 * scale,
                value.m01 * scale,
                value.m03 * scale,
                value.m10 * scale,
                value.m11 * scale,
                value.m13 * scale,
                value.m30,
                value.m31,
                value.m33
            );
            double[] components =
            {
                result.M11,
                result.M12,
                result.M13,
                result.M21,
                result.M22,
                result.M23,
                result.M31,
                result.M32,
                result.M33,
            };
            foreach (double component in components)
                if (!double.IsFinite(component))
                    return false;
            double determinant =
                result.M11 * (result.M22 * result.M33 - result.M23 * result.M32)
                - result.M12 * (result.M21 * result.M33 - result.M23 * result.M31)
                + result.M13 * (result.M21 * result.M32 - result.M22 * result.M31);
            return double.IsFinite(determinant) && determinant != 0;
        }

        private static bool IsHidden(VisualElement element)
        {
            for (VisualElement? current = element; current != null; current = current.parent)
                if (current.resolvedStyle.display == DisplayStyle.None)
                    return true;
            return false;
        }

        private static bool Finite(double scale, UnityEngine.Rect layout, UnityEngine.Rect bound) =>
            double.IsFinite(scale) && Finite(layout) && Finite(bound);

        private static bool Finite(UnityEngine.Rect value)
        {
            float[] fields = { value.x, value.y, value.width, value.height };
            foreach (float field in fields)
                if (!float.IsFinite(field))
                    return false;
            return true;
        }
    }

    internal sealed class UnityBattlementGeometryDisplaySource : IBattlementGeometryDisplaySource
    {
        public bool TryGet(DisplayId id, out BattlementDisplayGeometry geometry)
        {
            Display[] available = Display.displays;
            if (id.Value >= available.Length)
            {
                geometry = default;
                return false;
            }

            Display display = available[checked((int)id.Value)];
            if (id.Value != 0 && !display.active)
            {
                geometry = default;
                return false;
            }

            double width = display.renderingWidth;
            double height = display.renderingHeight;
            if (id.Value == 0)
            {
                width = width > 0 ? width : Screen.width;
                height = height > 0 ? height : Screen.height;
            }
            if (width <= 0 || height <= 0)
            {
                geometry = default;
                return false;
            }
            UnityEngine.Rect safeArea =
                id.Value == 0
                    ? Screen.safeArea
                    : new UnityEngine.Rect(0, 0, (float)width, (float)height);
            double scale =
                Screen.fullScreen && display.systemWidth > 0 ? width / display.systemWidth : 1;
            double? dpi = id.Value == 0 && Screen.dpi > 0 ? Screen.dpi : null;
            geometry = new BattlementDisplayGeometry(
                width,
                height,
                safeArea,
                scale,
                dpi,
                Orientation(id, width, height)
            );
            return true;
        }

        private static DisplayOrientation Orientation(DisplayId id, double width, double height)
        {
            bool isLandscape = width >= height;
            if (id.Value != 0)
                return isLandscape ? DisplayOrientation.Landscape : DisplayOrientation.Portrait;
            return Screen.orientation switch
            {
                ScreenOrientation.LandscapeLeft when isLandscape => DisplayOrientation.Landscape,
                ScreenOrientation.LandscapeRight when isLandscape =>
                    DisplayOrientation.LandscapeFlipped,
                ScreenOrientation.PortraitUpsideDown when !isLandscape =>
                    DisplayOrientation.PortraitFlipped,
                ScreenOrientation.Portrait when !isLandscape => DisplayOrientation.Portrait,
                _ => isLandscape ? DisplayOrientation.Landscape : DisplayOrientation.Portrait,
            };
        }
    }
}
