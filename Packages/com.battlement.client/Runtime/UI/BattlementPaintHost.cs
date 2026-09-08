#nullable enable

using System;
using Unity.Collections;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.UI
{
    /// <summary>A decorative host that can render its children into an isolated surface.</summary>
    internal sealed class BattlementPaintHost : VisualElement
    {
        private GameObject? surfaceObject;
        private PanelSettings? settings;
        private RenderTexture? surface;
        private VisualElement? surfaceRoot;

        public override VisualElement contentContainer => surfaceRoot ?? this;

        public BattlementPaintHost()
        {
            RegisterCallback<GeometryChangedEvent>(_ => ResizeSurface());
            RegisterCallback<AttachToPanelEvent>(_ => ResizeSurface());
            generateVisualContent += DrawSurface;
        }

        public VisualElement Isolate()
        {
            if (surfaceRoot != null)
                return surfaceRoot;
            PanelSettings template = Resources.Load<PanelSettings>(
                "BattlementPanelSettingsTemplate"
            );
            if (template == null)
                throw new InvalidOperationException("The UI panel settings template is missing.");
            settings = Object.Instantiate(template);
            settings.hideFlags = HideFlags.HideAndDontSave;
            settings.scaleMode = UnityEngine.UIElements.PanelScaleMode.ConstantPixelSize;
            settings.clearColor = true;
            settings.colorClearValue = UnityEngine.Color.clear;
            settings.sortingOrder = -10000;
            surface = new RenderTexture(1, 1, 24) { hideFlags = HideFlags.HideAndDontSave };
            surface.Create();
            settings.targetTexture = surface;
            surfaceObject = new GameObject("Battlement decorative surface")
            {
                hideFlags = HideFlags.HideAndDontSave,
            };
            UIDocument document = surfaceObject.AddComponent<UIDocument>();
            document.panelSettings = settings;
            surfaceRoot = new VisualElement();
            document.rootVisualElement.Add(surfaceRoot);
            surfaceRoot.pickingMode = UnityEngine.UIElements.PickingMode.Ignore;
            surfaceRoot.style.overflow = Overflow.Hidden;
            while (hierarchy.childCount > 0)
                surfaceRoot.Add(hierarchy[0]);
            ResizeSurface();
            return surfaceRoot;
        }

        public void ReleaseSurface()
        {
            if (surfaceRoot == null)
                return;
            VisualElement children = surfaceRoot;
            surfaceRoot = null;
            while (children.childCount > 0)
                hierarchy.Add(children[0]);
            if (settings != null)
                settings.targetTexture = null;
            if (surface != null)
            {
                surface.Release();
                Object.DestroyImmediate(surface);
            }
            if (surfaceObject != null)
                Object.DestroyImmediate(surfaceObject);
            if (settings != null)
                Object.DestroyImmediate(settings);
            surface = null;
            settings = null;
            surfaceObject = null;
            MarkDirtyRepaint();
        }

        private void ResizeSurface()
        {
            if (settings == null || surfaceRoot == null)
                return;
            int width = Mathf.CeilToInt(layout.width);
            int height = Mathf.CeilToInt(layout.height);
            if (width <= 0 || height <= 0)
                return;
            if (surface != null && surface.width == width && surface.height == height)
                return;
            if (width > 8192 || height > 8192)
                throw new InvalidOperationException(
                    "A decorative surface cannot exceed 8192 pixels per axis."
                );
            if (surface != null)
            {
                surface.Release();
                Object.DestroyImmediate(surface);
            }
            surface = new RenderTexture(width, height, 24)
            {
                name = "Battlement decorative subtree",
                hideFlags = HideFlags.HideAndDontSave,
            };
            surface.Create();
            settings.targetTexture = surface;
            surfaceRoot.style.width = width;
            surfaceRoot.style.height = height;
            MarkDirtyRepaint();
        }

        private void DrawSurface(MeshGenerationContext context)
        {
            if (surface == null)
                return;
            context.AllocateTempMesh(
                4,
                6,
                out NativeSlice<Vertex> vertices,
                out NativeSlice<ushort> indices
            );
            vertices[0] = new Vertex
            {
                position = new UnityEngine.Vector3(0, 0, Vertex.nearZ),
                tint = UnityEngine.Color.white,
                uv = new Vector2(0, 1),
            };
            vertices[1] = new Vertex
            {
                position = new UnityEngine.Vector3(layout.width, 0, Vertex.nearZ),
                tint = UnityEngine.Color.white,
                uv = new Vector2(1, 1),
            };
            vertices[2] = new Vertex
            {
                position = new UnityEngine.Vector3(layout.width, layout.height, Vertex.nearZ),
                tint = UnityEngine.Color.white,
                uv = new Vector2(1, 0),
            };
            vertices[3] = new Vertex
            {
                position = new UnityEngine.Vector3(0, layout.height, Vertex.nearZ),
                tint = UnityEngine.Color.white,
                uv = new Vector2(0, 0),
            };
            ushort[] order = { 0, 1, 2, 2, 3, 0 };
            for (int i = 0; i < order.Length; i++)
                indices[i] = order[i];
            context.DrawMesh(
                vertices,
                indices,
                surface,
                TextureOptions.PremultipliedAlpha | TextureOptions.SkipDynamicAtlas
            );
        }
    }
}
