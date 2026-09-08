#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.UI
{
    /// <summary>Owns compound subtree masks and the material used to composite a group.</summary>
    internal sealed class BattlementPaintComposition : IDisposable
    {
        private readonly VisualElement target;
        private FilterFunctionDefinition? definition;
        private Material? maskMaterial;
        private Material? blendMaterial;
        private PaintClipPath? path;
        private PaintBlendMode blend;
        private bool requested;
        private StyleList<FilterFunction> originalFilter;
        private StyleMaterialDefinition originalMaterial;
        private bool active;
        private VisualElement? maskTarget;

        public bool IsActive => active;

        public BattlementPaintComposition(VisualElement target) => this.target = target;

        public void Replace(PaintClipPath? nextPath, PaintBlendMode? nextBlend)
        {
            PaintBlendMode mode = nextBlend ?? PaintBlendMode.Normal;
            bool nextRequested = nextPath != null || nextBlend != null;
            if (Equals(path, nextPath) && blend == mode && requested == nextRequested)
                return;
            Dispose();
            path = nextPath;
            blend = mode;
            requested = nextRequested;
            if (!requested)
                return;
            if (mode != PaintBlendMode.Normal && target is not BattlementPaintHost)
                throw new InvalidOperationException(
                    "Subtree blend modes require a decorative View host."
                );
            maskTarget =
                mode == PaintBlendMode.Normal ? target : ((BattlementPaintHost)target).Isolate();
            originalFilter = maskTarget.style.filter;
            originalMaterial = target.style.unityMaterial;
            active = true;
            maskMaterial = Material("BattlementSubtreeMask");
            definition = ScriptableObject.CreateInstance<FilterFunctionDefinition>();
            definition.hideFlags = HideFlags.HideAndDontSave;
            definition.filterName = "Battlement subtree mask";
            definition.passes = new[]
            {
                new PostProcessingPass
                {
                    material = maskMaterial,
                    passIndex = 0,
                    applySettingsCallback = ConfigureMask,
                },
            };
            maskTarget.style.filter = new List<FilterFunction> { new(definition) };
            if (mode != PaintBlendMode.Normal)
            {
                blendMaterial = Material("BattlementComposite");
                blendMaterial.SetFloat(
                    "_DstBlend",
                    mode == PaintBlendMode.Screen
                        ? (float)UnityEngine.Rendering.BlendMode.OneMinusSrcColor
                        : (float)UnityEngine.Rendering.BlendMode.One
                );
                target.style.unityMaterial = blendMaterial;
            }
        }

        public void Dispose()
        {
            if (active)
            {
                if (maskTarget != null)
                    maskTarget.style.filter = originalFilter;
                if (blendMaterial != null)
                    target.style.unityMaterial = originalMaterial;
                if (target is BattlementPaintHost host)
                    host.ReleaseSurface();
            }
            active = false;
            if (definition != null)
                Object.DestroyImmediate(definition);
            if (maskMaterial != null)
                Object.DestroyImmediate(maskMaterial);
            if (blendMaterial != null)
                Object.DestroyImmediate(blendMaterial);
            definition = null;
            maskMaterial = null;
            blendMaterial = null;
        }

        private void ConfigureMask(MaterialPropertyBlock properties, FilterPassContext context)
        {
            var edges = new Vector4[64];
            int count = 0;
            float width = target.layout.width;
            float height = target.layout.height;
            if (path != null && width > 0 && height > 0)
                foreach (IReadOnlyList<IReadOnlyList<UiLength>> contour in path.Contours)
                    for (int i = 0; i < contour.Count; i++)
                    {
                        IReadOnlyList<UiLength> a = contour[i];
                        IReadOnlyList<UiLength> b = contour[(i + 1) % contour.Count];
                        edges[count++] = new Vector4(
                            Resolve(a[0], width),
                            Resolve(a[1], height),
                            Resolve(b[0], width),
                            Resolve(b[1], height)
                        );
                    }
            properties.SetVectorArray("_Edges", edges);
            properties.SetInt("_EdgeCount", count);
            properties.SetInt("_EvenOdd", path?.FillRule == PaintFillRule.EvenOdd ? 1 : 0);
            properties.SetInt("_WritesGamma", context.writesGamma ? 1 : 0);
            properties.SetInt("_ReadsGamma", context.readsGamma ? 1 : 0);
        }

        private static float Resolve(UiLength value, float size) =>
            (float)(value.Pixels / size + value.Percentage / 100);

        private static Material Material(string resource)
        {
            Shader shader = Resources.Load<Shader>(resource);
            if (shader == null || !shader.isSupported)
                throw new InvalidOperationException(
                    $"Required UI composition shader '{resource}' is unavailable."
                );
            return new Material(shader) { hideFlags = HideFlags.HideAndDontSave };
        }
    }
}
