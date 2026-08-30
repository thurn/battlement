# UI Toolkit Visual Compatibility Audit

This record covers the static base UI in the arcade-style mockups. Animation,
press and hover states, accessibility, and other interaction behavior are out of
scope. The audit targets Unity 6000.5.8f1.

## Visuals requiring assets or custom rendering

The following visual features are not reproducible with ordinary UXML and USS
styling alone. They require image assets, custom `VisualElement` rendering, or
shader-backed rendering.

| Mockup feature | Recommended treatment |
| --- | --- |
| Large asymmetric cyan and pink metal cabinet frame | Use a full-resolution transparent overlay, SVG, or carefully segmented 9-slice. |
| Angular or octagonal buttons | Use a reusable 9-slice PNG or SVG. |
| Settings panel and tab silhouettes with cut corners | Create separate 9-slice assets for the panel, inactive tab, and selected tab. |
| Dropdown and key-cap silhouettes | Share a small-control 9-slice. |
| Multicolor gradient outlines | Bake the gradient into the chrome asset or draw it with a custom mesh. |
| Dark blue beveled or gradient interiors | Include the treatment in the 9-slice center, or use a small repeatable gradient texture. |
| Outer neon glows | Bake transparent padding into the source asset, use a separate glow sprite, or use a custom material. |
| Inset shadows and inner glows | Bake these effects into the 9-slice border and center. |
| Radial screen illumination | Use a full-screen background texture or a shader-backed custom element. |
| Starfield and perspective grid | Use a background texture or custom-generated geometry. |
| Blue and red diagonal header stripes | Use a small repeatable texture or SVG. |
| Exact title and button-label rendering | TextCore can approximate the treatment; bake wordmarks when pixel matching is important. |
| Exact skew applied to text | Bake the text or manipulate its generated geometry. |
| Angular checkbox outline and luminous check mark | Use small checkbox-frame and check-mark sprites or SVGs. |
| Slider gradient track, angular handle, and glows | Use separate track, fill, and handle assets while controlling the handle position natively. |
| D-pad and controller icons with bevels and glows | Use an icon atlas, individual SVGs, or layered sprites. |
| Dropdown caret with exact styling | Use a small SVG, PNG, or suitable font glyph. |
| Red **Erase Saved Data** chrome | Use a dedicated 9-slice or a tinted variant when tinting preserves the intended result. |

UI Toolkit supports 9-slicing for textures, sprites, render textures, and vector
images. It also exposes `generateVisualContent`, `Painter2D`, and mesh generation
for procedural alternatives to image assets.

For the fixed 1024 x 1536 reference design, prefer:

- One full-screen frame and background composition. The outer frame is
  asymmetric, and the entire canvas scales uniformly.
- Reusable 9-slices for action buttons, tabs, dropdowns, key caps, checkboxes,
  and slider parts.
- TextCore for ordinary settings text.
- TextCore or baked wordmarks for **Settings**, the game logo, and large action
  button labels, depending on the required fidelity.
- A small SVG or PNG icon atlas for arrows, check marks, controller buttons, and
  the information badge.

References:

- [9-slice images with UI Toolkit](https://docs.unity3d.com/cn/2022.3/Manual/UIE-9-slice-images-with-ui-toolkit.html)
- [`MeshGenerationContext` API](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/UIElements.MeshGenerationContext.html)

## Unsupported web CSS features

No static base visual in the mockups is categorically impossible in UI Toolkit.
However, the following web CSS techniques used by the mockups do not have
equivalent USS properties:

- `clip-path: polygon(...)` and `clip-path: path(...)`
- CSS `linear-gradient`, `radial-gradient`, and
  `repeating-linear-gradient` backgrounds
- `box-shadow`, including inset shadows
- CSS `filter` effects such as `drop-shadow`, `blur`, `brightness`, `contrast`,
  and `saturate`
- `mask-image`
- `mix-blend-mode: screen`
- `background-clip: text`
- `skewX`
- CSS Grid
- `position: sticky`

These limitations prevent a direct CSS-to-USS translation, but do not prevent
the final static appearance. Image assets, layered elements, TextCore effects,
or custom mesh rendering can reproduce the result.

The outer cabinet, reusable angular control chrome, neon glows, and exact
wordmark styling are the most asset-dependent portions of the design. The
underlying layout and control structure remain within UI Toolkit's capabilities.
