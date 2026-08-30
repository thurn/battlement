# Reactant Asset Generator Technical Design

The Reactant asset generator turns static, CSS-style Rust declarations into
lossless PNG textures before Unity builds or runs a game. It fills deliberate
gaps in Unity UI Toolkit's paint model while leaving layout, interaction, and
styles that UI Toolkit already supports to native Battlement UI.

The public declaration is the source of truth. A procedural macro assigns the
declaration a stable Addressables key and registers a typed handle. A separate
CLI discovers those declarations, renders missing or stale PNGs in one system
Chrome or Chromium session, and writes the complete generated asset set for
Unity to import. Reactant automatically prepares every linked generated asset
in the initial snapshot.

This is an implementation contract for Battlement maintainers. The exploratory
Python/Pillow generator and arcade mockups informed the paint coverage and
examples, but they are not runtime dependencies or normative specifications.

## Related information

- [Battlement Reactant technical design](reactant-technical-design.md) defines
  component rendering, sessions, and the snapshot conversion extended here.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  `TextureAddress`, `PreparedAsset`, `Image`, `Style`, nine-slice properties,
  asset validation, and asset leases.
- [UI Toolkit visual compatibility audit][visual-audit] records the mockup
  treatments that cannot be represented by ordinary USS.
- [Addressable asset constants](../address-code-generation.md) defines the
  existing Unity-driven address catalog. Generated assets use the same typed
  protocol values but do not require that command to discover their addresses.
- [CSS Backgrounds and Borders][css-backgrounds],
  [CSS Images][css-images], [CSS Masking][css-masking],
  [Filter Effects][filter-effects], [CSS Transforms][css-transforms], and
  [Compositing and Blending][compositing], and [CSS Values][css-values] define
  the syntax and browser behavior used by the supported property catalog.
- [Unity UI Toolkit USS properties][unity-uss] is the native-support baseline.
- [Unity UI Toolkit 9-slicing][unity-slicing] defines how Unity consumes the
  generated slice insets.

[css-backgrounds]: https://www.w3.org/TR/css-backgrounds-3/
[css-images]: https://www.w3.org/TR/css-images-4/
[css-masking]: https://www.w3.org/TR/css-masking-1/
[css-values]: https://www.w3.org/TR/css-values-4/
[filter-effects]: https://www.w3.org/TR/filter-effects-1/
[css-transforms]: https://www.w3.org/TR/css-transforms-1/
[compositing]: https://www.w3.org/TR/compositing-1/
[visual-audit]: ../ui-toolkit-visual-compatibility-audit.md
[unity-uss]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-USS-SupportedProperties.html
[unity-slicing]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-9-slice-images-with-ui-toolkit.html

## Design contract

The generator is a static paint compiler, not a replacement UI renderer.

- Rust and UI Toolkit continue to own layout, hit testing, accessibility,
  focus, state, input, animation, and ordinary supported styling.
- Each request paints exactly one box or one text run into an explicit canvas.
- Every runtime variant is a separate named declaration. Requests have no
  runtime parameters.
- Every output is an Addressable `Texture2D`. The generator never imports an
  output as a Sprite.
- Generated textures may be used as `Image` content or element backgrounds.
- A request is valid only when its final appearance needs at least one
  generator-only feature. A request expressible entirely with the current
  Battlement UI and UI Toolkit surface is a build error.
- The catalog is closed and typed. Declarations use supported CSS syntax, not
  arbitrary stylesheet text. There is no HTML, JavaScript, shader, SVG
  document, Canvas, or custom-pixel callback escape hatch.
- Browser raster output is accepted as implementation-defined across browser
  versions. The generator records the browser identity and invalidates its
  cache when that identity changes.

The native-support decision targets Unity 6000.5.8f1 and the Battlement UI
surface documented in the linked design. It changes with that surface. Removing
a generator-only distinction after Battlement gains equivalent native support
is allowed; backwards compatibility and asset-generator versioning are not
requirements.

## Authoring API

Ordinary authoring imports `battlement_reactant::asset_generator`. The
`generate!` function-like procedural macro is publicly reexported from that
module so a rules crate does not depend directly on its procedural-macro
implementation.

A declaration is written in a dedicated macro grammar and produces one named
module-level `pub static`:

```rust
use battlement_reactant::asset_generator;

asset_generator::generate! {
  @nine-slice ACTION_BUTTON {
    @canvas 760px 140px;
    @slices 24px 26px 24px 26px;
    @allow-clipping top right bottom left;

    clip-path: polygon(
      2.4% 0%,
      97.6% 0%,
      100% 12%,
      100% 88%,
      97.6% 100%,
      2.4% 100%,
      0% 88%,
      0% 12%
    );
    background: linear-gradient(110deg, #b9fbff 0%, #ff4bd1 100%);
  }
}
```

The values are illustrative; the polygon itself is a complete closed shape.

The body uses CSS property names, units, functions, lists, and shorthands.
Custom at-rules carry generator metadata that has no CSS equivalent. The macro
parses and validates its input, emits a public `NineSliceAsset` static named
`ACTION_BUTTON`, and emits one linked catalog registration. Request values
exist only in the macro's typed syntax tree. Expansion does not reproduce the
declaration as a Rust expression, and the crate exposes no request-builder or
intermediate paint-value API. Rust type-checks only the generated handle and
registration. The macro never reads files, walks the project, launches a
process, or performs rendering.

There are three declaration kinds and three corresponding handle types:

| Declaration at-rule | Handle | Intended use |
| --- | --- | --- |
| `@background` | `BackgroundAsset` | One fixed decorative or content texture |
| `@nine-slice` | `NineSliceAsset` | Resizable background with fixed edges |
| `@text-image` | `TextImageAsset` | Pre-rendered text with advanced paint |

All three handles are `Copy`, have no allocation, and expose:

```rust
pub const fn texture_address(self) -> TextureAddress;
pub const fn canvas_size(self) -> LogicalSize;
pub const fn subject_bounds(self) -> LogicalRect;
pub fn image_source(self) -> ImageSource;
pub fn image(self) -> Image;
```

`image()` creates an `Image` with only the generated texture source. It does not
assign dimensions or fit behavior. Callers use the canvas and subject metadata
to author layout through ordinary native properties.

`BackgroundAsset` and `TextImageAsset` also expose:

```rust
pub fn background_style(self) -> Style;
```

The method sets `background_image`, an upper-left origin, no-repeat paint, and
an explicit background size equal to the logical canvas. Those are paint
properties; it does not assign element width, height, or other layout.

`NineSliceAsset` exposes:

```rust
pub const fn slice_insets(self) -> LogicalInsets;
pub fn background_style(self) -> Style;
```

Its style sets the generated texture, four source-pixel slice values, and
`unity_slice_scale` so the visible fixed edges equal the authored logical
insets at the effective raster scale. It leaves the element's size and other
layout properties untouched.

`subject_bounds` is the smallest declared rectangle expected to contain the
painted subject before effects. It is expressed in logical canvas coordinates.
It lets callers align content independently of transparent shadow or transform
padding. It is metadata only and never changes UI Toolkit layout implicitly.

### Static declaration subset

The macro and CLI accept the same deliberately restricted CSS token grammar:

- Exactly one named asset declaration inside each module-level
  `asset_generator::generate!` invocation.
- One of the three declaration at-rules followed by a valid Rust identifier and
  a braced body.
- Generator metadata at-rules followed by CSS declarations from the closed
  catalog below.
- CSS numbers, dimensions, percentages, hashes, strings, functions,
  comma-separated lists, and space-separated lists accepted by that catalog.

The subset rejects selectors, nested style rules, custom properties, `var()`,
`env()`, `url()`, imports, namespaces, media and container queries, keyframes,
transitions, animations, inheritance, `!important`, unknown properties, and
unknown at-rules. It also rejects Rust expressions, paths, helper calls,
closures, nested macro invocations, and declarations produced by another
macro. The CLI does not run game code to evaluate a request.

The invocation path is exactly `asset_generator::generate!`. Declaration and
metadata at-rules, property names, function names, and keywords have one
ASCII-case-insensitive CSS spelling. The canonicalizer lowercases those names.
They cannot be aliased, reexported, qualified, or supplied through Rust
imports. The generated static identifier remains case-sensitive because it is
a Rust symbol.

The source must first tokenize as a Rust macro token tree. Every normative CSS
form below does. CSS escapes in identifiers, unquoted URLs, and tokenizer error
recovery are outside the grammar. Authors use ordinary CSS block comments;
comments are removed as lexical trivia before macro parsing.

A procedural macro cannot observe the path used to invoke it. The CLI therefore
examines every macro invocation before filtering by path. Any invocation body
that begins with an asset declaration at-rule under an alias, reexport, or
other path is an error naming the required path. The scanner also descends into
macro definitions and nested token trees; an asset-generator invocation there
is an error rather than a discoverable declaration. This makes the direct
module-level spelling an enforced source contract instead of relying on the
procedural macro to enforce information it cannot see.

`cfg`, `cfg_attr`, Cargo-feature gating, and placement inside a conditionally
compiled parent module are errors. Every linked request must exist in every
target build so the generated catalog is identical for native and WebAssembly
rules engines.

The shared parser and canonicalizer live in an ordinary library consumed by
both the procedural macro and CLI. The macro passes its input token stream to
that parser. The CLI locates exact invocations and passes the delimited source
tokens to the same parser. Parser conformance fixtures prove semantic parity
for declaration bodies. Separate discovery fixtures prove path and placement
diagnostics, which are necessarily owned by the source scanner.

### Normative macro grammar

The declaration at-rule determines the generated Rust type:

| At-rule | Generated handle type |
| --- | --- |
| `@background NAME { ... }` | `BackgroundAsset` |
| `@nine-slice NAME { ... }` | `NineSliceAsset` |
| `@text-image NAME { ... }` | `TextImageAsset` |

`NAME` must be a non-raw Rust identifier. The macro emits it as a module-level
`pub static`. Request and paint values are typed grammar nodes internal to the
shared parser. They are not Rust types and cannot appear outside the macro.

Each body contains these required statements:

| Declaration | Required statements |
| --- | --- |
| `@background` | `@canvas` |
| `@nine-slice` | `@canvas`, `@slices` |
| `@text-image` | `@canvas`, `@font-file`, `content`, `font-size` |

Statement order is insignificant. Order remains significant inside CSS lists
where it controls layer, shadow, filter, transform, stop, point, or path order.
Every property and metadata at-rule may appear at most once unless its grammar
explicitly contains a list.

### Generator metadata at-rules

The metadata grammar is:

```css
@canvas <px> <px>;
@subject <px> <px> <px> <px>;
@slices <px> <px> <px> <px>;
@allow-clipping <clip-edge>+;
@raster-scale <integer>;
@filter-mode bilinear | nearest;
@wrap-mode clamp | repeat;
@compression lossless | lossy-low | lossy-normal | lossy-high;
@font-file cargo("<path>") | unity("<path>");
```

`@canvas` is width then height. `@subject` is x, y, width, then height.
`@slices` follows CSS order: top, right, bottom, left. These geometry values
must be direct finite `px` dimensions, not zero without a unit, calculations,
or other CSS units. Canvas dimensions are positive; subject values are
nonnegative and remain inside the canvas.

`@slices` is valid only for `@nine-slice`. `@font-file` is valid and
required only for `@text-image`. Clipping edges are unique and must appear in
top, right, bottom, left order. Raster scale is an integer from 1 through 8.

The defaults are the complete canvas for `@subject`, no permitted clipping,
the project raster scale of 2, bilinear filtering, clamp wrapping, and lossless
compression. The lossy compression cases map to Unity's low-, normal-, and
high-quality compressed importer settings. Explicitly writing a metadata value
equal to its default is rejected as redundant.

### CSS declaration catalog

`@background` and `@nine-slice` accept these CSS properties:

| Property | Accepted CSS behavior |
| --- | --- |
| `background` | Color and ordered image layers, including position, size, repeat, origin, and clip |
| `border`, `border-width`, `border-style`, `border-color`, `border-top`, `border-right`, `border-bottom`, `border-left` | CSS shorthand and per-edge width, style, and color longhands |
| `border-radius` | One-to-four horizontal radii and optional slash-separated vertical radii |
| `box-shadow` | A nonempty comma-separated list of outer or inset shadows |
| `clip-path` | `inset`, `circle`, `ellipse`, `polygon`, or typed `path` |
| `mask` | A nonempty comma-separated list of image or gradient mask layers |
| `opacity` | Number from 0 through 1 |
| `background-blend-mode` | One mode or one mode per background layer |
| `isolation` | `isolate` only; the omitted default is `auto` |
| `filter` | A nonempty ordered supported filter-function list |
| `transform` | A nonempty ordered supported 2D transform-function list |
| `transform-origin` | A standard two-dimensional CSS position |

`@text-image` accepts these CSS properties:

| Property | Accepted CSS behavior |
| --- | --- |
| `content` | Exactly one string and no generated-content functions |
| `font-size` | One positive length |
| `font-style` | `normal`, `italic`, or `oblique` with an optional angle |
| `font-weight` | Integer from 1 through 1000 |
| `font-stretch` | Percentage from 50% through 200% |
| `line-height` | `normal` or a nonnegative length |
| `letter-spacing`, `word-spacing` | `normal` or one length |
| `text-align` | `start`, `center`, `end`, `justify`, `left`, or `right` |
| `white-space` | `normal`, `pre`, `pre-wrap`, or `nowrap` |
| `color` | One CSS color |
| `background` | One color, gradient, or local image used for advanced text fill |
| `background-clip` | `text`, required when `background` supplies text fill |
| `-webkit-text-stroke` | One nonnegative width and one color |
| `text-shadow` | A nonempty comma-separated shadow list |
| `opacity`, `filter`, `transform`, `transform-origin` | The same grammar as box paint |

The font file selects one non-variable face. Omitted style, weight, and stretch
come from its metadata. When any is authored explicitly, it must equal that
metadata; the browser may not synthesize a different face.

Advanced text fill uses the normal CSS composition:

```css
color: transparent;
background: linear-gradient(174deg, #fff 0%, #ff6dda 100%);
background-clip: text;
```

Those three declarations canonicalize as one text-fill value. `background`
and `background-clip` are invalid independently in `@text-image`. The color
must be `transparent` when advanced fill is present. Without advanced fill,
`color` defaults to opaque black.

### CSS value grammar

Lengths accept `px`, `%`, `em`, `rem`, `vw`, `vh`, `vmin`, and
`vmax` where the corresponding property permits them. Angles accept `deg`,
`grad`, `rad`, and `turn`. `calc()`, `min()`, `max()`, and
`clamp()` produce typed expression trees and use the linked CSS Values grammar.
Division by zero, non-finite results, and invalid dimensional arithmetic are
errors.

Colors accept CSS three-, four-, six-, and eight-digit hex notation,
`rgb()`/`rgba()`, `hsl()`/`hsla()`, the closed CSS named-color table,
and `transparent`. `currentColor`, system colors, profiles, and
device-dependent color spaces are rejected. All colors canonicalize to explicit
sRGB RGBA.

Local files use custom URL functions only where the property accepts an image:

```css
background: cargo-url("images/glow.png") center / cover no-repeat;
mask: unity-url("Assets/Masks/cutout.png") alpha;
```

`cargo-url()` resolves relative to the declaring Cargo package;
`unity-url()` resolves relative to the selected Unity project. Ordinary
`url()`, data URLs, network URLs, and unquoted paths are rejected.

Gradient images use the standard `linear-gradient`, `radial-gradient`,
`conic-gradient`, and repeating forms. Stops use standard CSS color-stop and
transition-hint syntax. At least two color stops are required. Explicit radial
shape and size must agree.

Background layers use the CSS shorthand grammar. Each layer may contain one
gradient or local image, a position with optional slash-separated size, repeat
keywords, and origin/clip boxes. A final color may follow the last layer.
Unsupported or redundant shorthand spellings are errors rather than browser
recovery cases. Explicit default subvalues are rejected.

Borders accept `none`, `solid`, `dashed`, `dotted`, and `double`.
Widths are nonnegative. Shadows use CSS offset-x, offset-y, optional blur and
spread, optional `inset`, and color grammar. Blur radii are nonnegative.

Clips use standard basic-shape syntax. Polygons require at least three points
and nonzero signed area after unit resolution. Adjacent duplicate points are
rejected. `path()` accepts a quoted SVG-compatible path containing only
absolute move, line, horizontal, vertical, cubic, quadratic, arc, and close
commands. It must begin with a move and contain a drawing command. Relative
commands are rejected.

Mask layers follow the CSS `mask` shorthand with position, size, repeat,
origin, clip, `alpha` or `luminance` mode, and `add`, `subtract`,
`intersect`, or `exclude` composition. A mask source must be one gradient or
local image. Every layer is internal to the request.

Filters accept `blur`, `brightness`, `contrast`, `drop-shadow`,
`grayscale`, `hue-rotate`, `invert`, `opacity`, `saturate`, and
`sepia`. Transforms accept `translate`, `rotate`, `scale`, `skew`,
`skewX`, `skewY`, and `matrix`. Perspective and 3D forms are errors.
Empty, `none`, or explicitly default lists are rejected as redundant.

Box paint defaults to transparent with no border, radius, shadow, clip, mask,
filter, transform, or blend. Text spacing and line height default to `normal`;
alignment defaults to `start`; white space defaults to `normal`; and stroke,
shadows, filters, and transforms default to absent. Omitted values do not
appear in canonical bytes. Explicit values equal to these defaults are rejected
so one visual configuration has one canonical form.

A complete background declaration looks like:

```rust
asset_generator::generate! {
  @background PANEL_GLOW {
    @canvas 640px 360px;
    @subject 16px 16px 608px 328px;
    @allow-clipping top right bottom left;

    background: radial-gradient(
      circle at 50% 65%,
      #153d75 0%,
      #020611 100%
    );
    box-shadow: 0 0 16px 0 #368dff80;
  }
}
```

A complete text declaration looks like:

```rust
asset_generator::generate! {
  @text-image PLAY_LABEL {
    @canvas 480px 146px;
    @subject 12px 16px 456px 114px;
    @font-file cargo("fonts/barlow-condensed-800-italic.ttf");

    content: "PLAY";
    font-size: 91px;
    color: transparent;
    background: linear-gradient(174deg, #fff 0%, #ff6dda 100%);
    background-clip: text;
    transform: skewX(-5deg);
  }
}
```

The public handle metadata types are independent of CSS value syntax:

```rust
pub struct LogicalSize { pub width: f32, pub height: f32 }
pub struct LogicalRect { pub x: f32, pub y: f32,
                         pub width: f32, pub height: f32 }
pub struct LogicalInsets { pub top: f32, pub right: f32,
                           pub bottom: f32, pub left: f32 }
```

Their values are finite logical pixels. The macro embeds them in the handle;
reading them performs no registry search or allocation.

### Canvas and bounds

Every request declares:

- A positive logical canvas width and height.
- A subject rectangle wholly inside that canvas.
- Whether paint may be intentionally clipped at each canvas edge.
- An optional raster-scale override.

The default subject rectangle is the complete canvas. Effects, transforms,
strokes, and glyph pixels may extend beyond it, but their resulting paint must
remain inside the canvas. Generation computes alpha bounds after rendering. If
nonzero paint touches an edge without an explicit clip permission for that
edge, generation fails with the measured edge and source declaration.

Explicit clipping means the edge contact is intentional. It does not suppress
errors for an invalid subject rectangle, missing glyphs, or nine-slice regions
that exceed the canvas.

Nine-slice requests always declare logical insets in CSS order: top, right,
bottom, left. Insets are never inferred from alpha, colors, edge differences,
or stretch previews. The top plus bottom must be less than the logical height,
and the left plus right must be less than the logical width.

For effective raster scale `s`, generation requires each of these products to
be an exact nonnegative integer:

```text
canvas width * s, canvas height * s
slice top * s, slice right * s, slice bottom * s, slice left * s
```

The two canvas products become PNG dimensions. The four slice products become
the integer `unity_slice_top`, `unity_slice_right`, `unity_slice_bottom`, and
`unity_slice_left` values. `NineSliceAsset::background_style` sets
`unity_slice_scale` to exactly `1 / s`, so each fixed region occupies its
authored logical inset. There is no rounding tolerance or implicit snapping.
The handle embeds the shared project default or its request override, so style
construction needs no manifest or filesystem lookup.

### Local dependencies

Requests may depend on local fonts and local image layers. Each reference is
one of two distinct typed forms:

- `cargo("...")` in `@font-file` and `cargo-url("...")` in image values
  resolve relative to the declaring Cargo package.
- `unity("...")` in `@font-file` and `unity-url("...")` in image values
  resolve relative to the selected Unity project.

Absolute paths, parent traversal outside the selected root, symlink escape,
URLs, data URLs, and system font-family lookup are rejected. The resolved file
must be a regular file. Dependency bytes participate in cache identity, while
the normalized declared reference participates in the public asset identity.

The initial dependency formats are PNG for raster image layers and OpenType
TTF, OTF, or WOFF2 for fonts. Extension and decoded content must agree. Animated
images, color-managed image profiles other than sRGB, SVG files, font
collections, variable fonts, and browser-selected font formats are rejected.
PNG decoding ignores textual metadata, timestamps, EXIF orientation, and other
ancillary chunks. It honors only dimensions, sRGB pixel samples, alpha, and the
standard sRGB rendering intent. Intrinsic CSS size is the decoded pixel width
and height.

Text requests must name explicit local font files for every face they use.
The browser receives only those faces through generated `@font-face` rules.
Fallback to installed fonts or network fonts is disabled. Generation waits for
all declared faces through `document.fonts.ready`, checks every requested face,
and fails if the browser reports a missing face. Before launch, the CLI checks
the selected font's Unicode character map and fails when a required scalar has
no glyph. Variation selectors, joiners, and combining behavior are validated
as part of a browser shaping probe for the complete text run. That probe
compares glyph runs against a deliberately unavailable control family and
rejects any fallback or `.notdef` glyph before screenshot capture.

## Request value model

Lengths are typed syntax-tree values, not strings. The initial unit catalog is:

- `px`, interpreted as logical Unity pixels.
- `%`, relative to the property-defined reference box.
- `em`, relative to the request's declared font size.
- `rem`, relative to the generator's fixed 16 px root font size.
- `vw`, `vh`, `vmin`, and `vmax`, relative to the explicit canvas.
- Unitless numbers where the corresponding CSS grammar permits them.
- `deg`, `grad`, `rad`, and `turn` for angles.
- Typed `calc`, `min`, `max`, and `clamp` expression trees.

Environment-dependent units and values are rejected, including dynamic
viewport units, physical units, container-query units, `env`, `var`, inherited
values, current viewport state, and current system theme.

All color inputs normalize to explicit sRGB RGBA. Accepted syntax includes CSS
hex, `rgb`/`rgba`, `hsl`/`hsla`, and the closed CSS named-color table.
`currentColor`, system colors, color profiles, and device-dependent color
spaces are rejected.

The renderer initializes these browser defaults explicitly:

- Transparent canvas and element background.
- `box-sizing: border-box`.
- Zero margin, border, padding, and text decoration.
- Left-to-right horizontal writing.
- No inherited page styles.
- No default user-agent focus, form, or link appearance.

## Supported paint catalog

The catalog models CSS computed values closely enough that generated HTML uses
standard CSS declarations rather than reimplementing their pixel algorithms.
The authored CSS spelling is the behavioral contract.

### Box paint

| CSS property | Behavior |
| --- | --- |
| `background` | Ordered color and image layers with CSS layer controls |
| `border`, width/style/color longhands, and four edge shorthands | Per-edge width, style, and color |
| `border-radius` | Elliptical per-corner radii |
| `box-shadow` | Ordered outer and inset CSS shadows |
| `opacity` | Group opacity for the generated subject |

Background image layers initially include:

- Linear, radial, and conic gradients.
- Repeating linear, radial, and conic gradients.
- Local raster images.
- Multiple ordered combinations of those layers.

Gradient stops support typed positions, two-position color stops, and
transition hints where browsers support the corresponding standard syntax.
Radial gradients support explicit shape, size, and position. Conic gradients
support start angle and position.

### Clipping, masking, and compositing

| CSS property | Behavior |
| --- | --- |
| `clip-path` | Basic shapes or typed SVG-compatible path data |
| `mask` | Ordered local image or gradient masks and standard mask positioning |
| `background-blend-mode` | Blend background layers within the request |
| `isolation` | Isolated internal stacking context |

Path data uses a typed, closed sequence of move, line, cubic, quadratic, arc,
and close commands. It is serialized as a CSS `path()` value. The API does not
accept an arbitrary SVG document.

`mix-blend-mode` and `backdrop-filter` are rejected. Their result depends on
pixels outside the generated request and therefore cannot be baked correctly
into a standalone transparent texture. Blending is supported only among layers
owned by the same request.

### Filters and transforms

The `filter` list supports standard browser functions:

- `blur`
- `brightness`
- `contrast`
- `drop-shadow`
- `grayscale`
- `hue-rotate`
- `invert`
- `opacity`
- `saturate`
- `sepia`

The `transform` list supports 2D `translate`, `rotate`, `scale`, `skew`,
`skewX`, `skewY`, and `matrix`, plus an explicit `transform-origin`.
Perspective and 3D transforms are rejected.

### Text paint

A text request contains exactly one Unicode string and one text run. It may
contain newline characters but cannot mix spans with different styles.

| CSS property | Behavior |
| --- | --- |
| `@font-file`, `font-*` | Explicit local face, size, style, weight, stretch, and line height |
| `letter-spacing` | CSS letter spacing |
| `word-spacing` | CSS word spacing |
| `text-align` | Horizontal alignment within the subject box |
| `white-space` | Closed `normal`, `pre`, `pre-wrap`, or `nowrap` behavior |
| `color` | Solid glyph fill |
| `background` plus `background-clip: text` | Gradient or local-image fill clipped to glyphs |
| `-webkit-text-stroke` | Browser text-stroke width and color |
| `text-shadow` | Ordered CSS text shadows |
| `filter` | Filter list applied to the complete text run |
| `transform` | 2D transform applied to the complete text run |

Gradient text uses browser background clipping and transparent glyph fill.
Stroke support uses the browser's prefixed property named by the grammar. That
browser-dependent raster result is covered by browser identity in the cache
key.

### Native properties in composites

A request may include native-capable paint such as a solid background, border,
border radius, opacity, or simple transform only when the complete result also
contains a generator-only feature. This permits one browser compositing pass
for a clipped gradient with shadows, for example.

The canonicalizer classifies the whole request against a shared native-support
table. If every authored property and combination has a faithful Battlement UI
representation, both the macro and CLI reject it with an error naming the
native `Style` or `Image` APIs to use.

The native-support table is closed for the Unity 6000.5.8f1 baseline:

| Authored paint | Native Battlement replacement |
| --- | --- |
| One solid box fill | `Style::background_color` |
| One local raster image | Import it normally and use `Style::background_image` or `Image::source` |
| Image position, repeat, size, or tint | Corresponding `Style::background_*` property |
| Solid per-edge border and elliptical radii | `Style::border_*` properties |
| Group opacity | `Style::opacity` |
| Translate, rotate, scale, or transform origin | Corresponding `Style` transform property |
| Filter opacity, invert, grayscale, sepia, blur, contrast, or hue rotation | `Style::filter` |
| Plain text color, local Unity font, size, style, weight, spacing, alignment, or white space | Corresponding text `Style` property |
| One solid text shadow | `Style::text_shadow` |
| One uniform solid text outline | `Style::unity_text_outline_*` |
| Existing raster nine-slicing | `Style::unity_slice_*` with a normal texture |

These cases make a request generator-required:

| Authored paint | Missing native capability |
| --- | --- |
| Any linear, radial, conic, or repeating gradient | UI Toolkit has no gradient background or text fill |
| More than one background image layer | UI Toolkit exposes one background source |
| Outer or inset `box-shadow` | UI Toolkit has no box-shadow property |
| Dashed, dotted, or double border paint | UI Toolkit exposes border width and color, not CSS border style |
| Circle, ellipse, polygon, path, or rounded-inset clip | UI Toolkit exposes only rectangular overflow clipping |
| Any mask layer | UI Toolkit has no mask-image style |
| Internal background blending | UI Toolkit has no background blend mode |
| Brightness, drop-shadow, or saturate filter | Absent from Battlement's native filter union |
| Skew, skew-axis, or matrix transform | Absent from Battlement's transform properties |
| Gradient or image text fill | UI Toolkit text color is solid |
| Multiple text shadows or CSS stroke combined with advanced text paint | UI Toolkit cannot reproduce the browser composition |

One generator-required row is sufficient. Any number of native rows may join
it in the same request so Chrome performs one composition. Multiple values that
overflow a native single-value slot, such as two otherwise native background
images or two text shadows, use the generator-required row for that
combination.

A composition containing only native rows is rejected even when it combines
several properties. Classification is semantic and table-driven; it does not
render both engines and compare pixels. A property in the external-context or
absent-dynamic categories remains unsupported rather than becoming
generator-required.

CSS Grid, sticky positioning, flex layout, ordinary size and spacing, and all
other layout-only CSS are absent from the catalog. Pseudo-classes, transitions,
animations, keyframes, media queries, container queries, and dynamic CSS
variables are also absent. Hover, pressed, disabled, selected, and animated
appearances are separate static declarations selected by Rust state.

Procedural imagery that is not expressible through the catalog is rejected.
The mockup prototype's starfield, perspective grid, particles, and similar
recipes therefore require ordinary checked-in art, a shader, or a future
separately designed rendering facility.

## Discovery and canonical identity

The CLI starts from the selected rules package's Cargo manifest and resolves
the package through Cargo metadata. It scans that package and its linked local
Rust dependencies for exact `asset_generator::generate!` invocations. Registry
dependencies are scanned only when Cargo resolves them into the selected rules
build.

Discovery resolves two Cargo graphs with the same selected features: the host
target triple and `wasm32-unknown-unknown`. CLI flags matching Cargo's
`--features`, `--all-features`, and `--no-default-features` choose that common
feature set. The sorted canonical declaration sets and local package identities
must match across both graphs. A target-only package matters only when it
contains a generated declaration. A target-only declaration or a difference in
one declaration's local dependency resolution is an error that prints both
graph origins. Generation never emits a target-specific catalog.

Discovery follows Rust modules from the crate root and accepts the static subset
defined above. It reports unsupported or ambiguous syntax instead of silently
omitting a request. Source locations use a stable Cargo package coordinate,
package-root-relative source path, line, column, and static symbol for
diagnostics only. Absolute paths may appear in terminal diagnostics but never
in canonical requests or manifests.

At each module item, discovery recursively inspects macro-invocation token
trees for an `@background`, `@nine-slice`, or `@text-image` declaration prefix.
A body with that prefix is either one direct canonical
`asset_generator::generate!` invocation or a fatal unsupported-indirection
diagnostic. The scanner never ignores a declaration-shaped body merely because
the invocation path is unfamiliar. It similarly rejects canonical invocations
inside declarative macro definitions instead of treating their templates as
real declarations.

Package coordinates have one of these forms:

- `workspace:<path>:<name>@<version>` for a workspace member, where `path` is
  relative to the selected rules workspace root and uses forward slashes.
- `registry:<index-url>:<name>@<version>` for a registry package.
- `git:<repository-url>#<commit>:<name>@<version>` for a Git package, with the
  exact resolved 40-character commit.

Registry and Git source files are relative to Cargo's resolved package root.
A path dependency outside the selected workspace may not contain generated
declarations because it has no machine-independent package coordinate. The CLI
reports that package and asks the author to make it a workspace member or a Git
or registry dependency.

The canonical request is a deterministic tagged value tree. Canonicalization:

- Preserves order only inside CSS lists where order changes paint.
- Sorts unordered map-like values by their stable tag.
- Normalizes equivalent color spellings to sRGB RGBA.
- Normalizes negative zero and rejects non-finite numbers.
- Preserves exact typed length and calculation structure.
- Uses normalized forward-slash local references, never absolute paths.
- Includes the request kind, canvas, subject, clipping permissions, property
  tree, and declared scale override.
- Excludes the Rust symbol, source location, resolved dependency bytes,
  project-default scale, browser identity, and generator build identity.

Every accepted number is parsed as IEEE-754 binary64. Canonical bytes contain
its big-endian bits after negative zero becomes positive zero. CSS serialization
uses the shortest decimal that round-trips to those bits. Thus `1`, `1.0`, and
`1e0` have one identity without introducing decimal rounding choices.

The public Addressables key is:

```text
battlement-reactant/generated/<lowercase SHA-256 of canonical request>.png
```

Identical canonical declarations deduplicate to one generated texture even when
they have different symbols or source locations. A collision between different
canonical byte streams is a fatal diagnostic that prints both declarations.
If identical canonical declarations resolve a local reference to different
bytes, generation also fails and prints every declaring and resolved location.
There is no ordering rule that chooses one dependency over another.

The output cache key is a separate SHA-256 over:

- The canonical request bytes.
- The resolved bytes and declared reference of every local dependency.
- The effective raster scale.
- The browser executable identity and full reported version.
- The renderer and generator build identity.

Dependency changes, project-default scale changes, browser changes, and
renderer changes regenerate bytes at the same public address. That keeps Rust
addresses stable while ensuring a build never reuses pixels produced from stale
inputs.

## Command-line contract

The CLI namespace is:

```text
cargo battlement reactant assets generate
cargo battlement reactant assets check
cargo battlement reactant assets preview
```

All three commands accept a Unity project and rules manifest selection matching
`cargo battlement author`. They also accept an explicit browser executable.
When it is omitted, the CLI searches supported system Chrome and Chromium
installations in documented platform order. Failure to find a browser reports
the searched candidates and the explicit override syntax.

Browser selection is deterministic:

1. Use the executable named by `--browser` when present.
2. On macOS, try the stable Google Chrome application, then stable Chromium.
3. On Windows, query registered stable application paths for Google Chrome,
   then Chromium, considering the current user before the local machine.
4. On Linux, search `PATH` for `google-chrome-stable`, `google-chrome`, then
   `chromium`.

Beta, Dev, Canary, and ungoogled variants are used only through `--browser`.
The selected executable must report a Chrome or Chromium product through the
browser protocol.

The project-wide raster scale defaults to 2. A positive per-request override
wins. The effective pixel canvas equals logical canvas dimensions multiplied by
that scale. Fractional results are errors; the generator never silently rounds
the requested output dimensions.

The project default is a library constant shared by the macro, CLI, and runtime
handle implementation, not a project configuration file. Changing that default
is a renderer change and invalidates the output cache. A request override is
part of its canonical declaration and is known to its runtime handle.

An empty declaration set has no browser identity. `generate` does not locate or
launch Chrome; after successful discovery it removes any previous generated
root and sibling metadata through the normal transactional cleanup and writes
no manifest. `check` succeeds only when both are absent. `preview` opens an
empty explanatory gallery through the operating system's normal URL opener but
starts no rendering session. Author and sample hooks skip generated
Addressables registration when the manifest is absent.

### `generate`

`generate` discovers all requests, validates the complete set, compares cache
records, and writes the exact generated output set. Its phases are:

1. Resolve the Cargo graph and discover declarations.
2. Parse, type-check, canonicalize, and hash each declaration.
3. Resolve local dependencies and reject canonical duplicates whose resolved
   dependency sets differ.
4. Deduplicate the remaining declarations and compute cache keys.
5. Compare expected outputs, manifest records, import metadata, and cached
   dependency state without launching a browser.
6. If every output is current, exit successfully without starting a browser or
   rewriting any file.
7. If any output is stale or missing, launch one isolated browser session and
   render all misses through that session.
8. Validate every rendered result and stage the complete output set.
9. Atomically replace generated files and remove stale generated files only
   after every request succeeds.

The command must not require Unity. It may run before Unity is installed as long
as the selected project and referenced files are present.

Phase five never invokes or rereads the complete browser executable, even with
a `--version` flag. It compares path, byte length, modification timestamp, and
operating-system file ID with the last manifest. If all match, the recorded
executable hash, protocol product, and version remain valid. If any differs,
`check` reports stale and `generate` proceeds to the one real browser session,
where it hashes the executable and records the new protocol identity.

### `check`

`check` performs phases one through five and is read-only. It succeeds only when
the generated output set, manifest, cache identity, PNG metadata, and Unity
import metadata exactly match the discovered declarations. It never launches a
browser, repairs files, removes stale files, or starts Unity.

Diagnostics tell the caller to run `cargo battlement reactant assets generate`
and list added, changed, missing, corrupt, and stale assets separately.

### `preview`

`preview` first performs generation, then opens a local HTML gallery. It may
launch the system browser for both rendering and viewing. The gallery contains:

- A checkerboard behind transparent output.
- Logical and raster dimensions, effective scale, and asset kind.
- Public address, canonical hash, browser identity, and source locations.
- Subject bounds and canvas-edge paint diagnostics.
- Local dependency references and content hashes.
- The authored property summary.
- A resizable live nine-slice preview for every nine-slice request.

The nine-slice preview implements Unity's stretched slice behavior with the
declared logical insets. It offers width and height controls and highlights the
four slice boundaries. Preview is a development aid; its HTML and temporary
server are not bundled into the game.

## Browser rendering

The generator uses an installed stable Chrome or Chromium executable. It does
not download, bundle, pin, or update a browser. The executable's resolved path,
file identity, product name, and full version string are recorded in the
manifest and cache key.

One clean browser context renders every cache miss in a `generate` invocation.
The context has:

- Per-request device metrics whose device scale factor equals the effective
  raster scale.
- A transparent page and request canvas.
- Fixed locale, time zone, color scheme, reduced-motion preference, and default
  root font size.
- Disabled browser extensions, cache, service workers, and persistent profile.
- Network requests blocked at the protocol layer.
- Only generated data documents and validated local dependency bytes.
- Animation time fixed before capture.

The renderer keeps the element and viewport dimensions in logical CSS pixels.
It does not multiply serialized CSS values. Immediately before each request it
sets the browser's emulated device scale factor to that request's scale and
forces a fresh layout and paint. A 760 by 140 CSS-pixel canvas at scale 2 thus
captures exactly 1520 by 280 device pixels.

This one mechanism scales CSS pixels, font rasterization, `em` and `rem`
results, viewport units, shadows, blur radii, strokes, transform geometry, and
gradient sampling together. Percentages continue to resolve against their CSS
reference boxes. A local PNG's intrinsic dimensions remain its decoded pixel
dimensions in CSS pixels unless the authored background size overrides them;
device scaling changes raster density, not CSS intrinsic size.

Each request is rendered in an isolated container within the same page. The
renderer waits for fonts, two animation frames, and stable layout before
capture. It verifies the captured device dimensions against the exact products
defined under Canvas and bounds. It captures the raster canvas as an RGBA PNG
and strips
nonessential, nondeterministic metadata. The CLI decodes the capture and
re-encodes it with one fixed Rust PNG encoder configuration, so browser PNG
chunk ordering and compression choices never enter output identity.

The CLI batches requests to amortize process startup but preserves isolation:
one request cannot define CSS, DOM IDs, fonts, or state visible to another.
Failure of one request aborts the complete staged set.

Browser version changes may change antialiasing, gradient interpolation,
filters, or text metrics. Such changes are expected to invalidate the cache;
the design does not promise pixel identity between browser versions or
operating systems.

## Generated output and Unity import

Generated PNGs, their manifest, cache records, and Unity `.meta` files are
ignored build products in one generator-owned subtree of the selected Unity
project. Users do not edit or commit them.

The generated root is the project-relative Unity asset path
`Assets/Generated/BattlementReactant`. Its `manifest.json` is the sole cache and
discovery record. Texture files are named `textures/<request hash>.png`; their
file names therefore agree with the final segment of the public Addressables
key. The CLI and Unity editor package share these literal names. The repository
ignore rule covers that generated root and its sibling directory `.meta` file.

Each `.meta` file uses a deterministic Unity GUID derived from the public
address with a separate domain tag. The generator checks the full derivation
input for collisions before truncating it to Unity's GUID width. Regeneration,
cache invalidation, and clean clones therefore retain the same GUID for the same
public address.

The manifest is canonical UTF-8 JSON with two-space indentation, a trailing
newline, lexicographically sorted object keys, and assets sorted by address. It
rejects missing and unknown fields. It has this logical schema:

```json
{
  "assets": [{
    "address": "battlement-reactant/generated/<hash>.png",
    "cacheKey": "<sha256>",
    "canonicalRequestSha256": "<sha256>",
    "dependencies": [],
    "import": {},
    "kind": "background",
    "logicalCanvas": {"height": 140.0, "width": 760.0},
    "png": "textures/<hash>.png",
    "pngSha256": "<sha256>",
    "rasterScale": 2,
    "rasterSize": {"height": 280, "width": 1520},
    "sliceInsets": null,
    "sourceLocations": [],
    "subjectBounds": {},
    "unityGuid": "<32 lowercase hex>",
    "unityGuidDerivationSha256": "<sha256>"
  }],
  "browser": {},
  "rendererIdentity": "<build identity>"
}
```

Empty objects above stand for required records whose fields are the metadata
listed below; they are not permitted to remain empty in a real manifest. No
schema version is stored. The renderer identity changes whenever the parser,
canonical form, renderer, PNG encoder, manifest shape, or import template
changes, and an unrecognized manifest shape is stale rather than migrated.

Nested records have these exact fields:

- `browser` has `executableFileIdentity`, `executablePath`,
  `executableSha256`, `product`, and `version`. The file identity has
  `byteLength`, `fileId`, and `modifiedNanoseconds`.
- A dependency has `contentSha256`, `kind`, `package`, and `path`.
- `import` has `alphaIsTransparency`, `compression`, `filterMode`, `mipmaps`,
  `sRgb`, `textureType`, and `wrapMode`.
- A source location has `column`, `line`, `package`, `path`, and `symbol`.
- A logical size has `height` and `width`; a raster size uses unsigned integer
  values with the same names.
- Subject bounds have `height`, `width`, `x`, and `y`.
- Slice insets are either null or have `bottom`, `left`, `right`, and `top`.

JSON values use these exact encodings:

- SHA-256 values are 64 lowercase hexadecimal characters. A Unity GUID is 32.
- Asset `kind` is `"background"`, `"nineSlice"`, or `"textImage"`.
- Dependency `kind` is `"cargoFont"`, `"cargoImage"`, `"unityFont"`, or
  `"unityImage"`. Cargo dependencies store the stable package coordinate;
  Unity dependencies store null. `path` is root-relative UTF-8 with forward
  slashes, no empty segment, dot segment, or parent segment.
- `compression` is `"lossless"`, `"lossyLow"`, `"lossyNormal"`, or
  `"lossyHigh"`; `filterMode` is `"bilinear"` or `"nearest"`; `wrapMode` is
  `"clamp"` or `"repeat"`; and `textureType` is always `"default"`.
- `alphaIsTransparency` and `sRgb` are true. `mipmaps` is false.
- Logical geometry and slice fields are finite JSON numbers without negative
  zero. Raster dimensions, line, and column are positive unsigned integers.
  Raster scale is an integer from 1 through 8.
- `executablePath`, `product`, and `version` are nonempty UTF-8 strings.
  `byteLength` and `modifiedNanoseconds` are unsigned JSON integers; the latter
  is UTC nanoseconds since the Unix epoch.
- On macOS and Linux, `fileId` is
  `unix:<device-lowercase-hex>:<inode-lowercase-hex>`. On Windows it is
  `windows:<volume-serial-lowercase-hex>:<file-id-lowercase-hex>`.
- Source `package`, `path`, and `symbol` are nonempty strings. Line and column
  are one-based. Source locations are sorted by package, path, line, column,
  then symbol.
- `png` is exactly `textures/<canonicalRequestSha256>.png`. Address, PNG path,
  request hash, and deterministic GUID derivation must agree.

`rendererIdentity` is a lowercase SHA-256 embedded when the CLI is built. Its
input is the canonicalizer, renderer document and script, browser protocol
adapter, PNG encoder configuration, manifest serializer, and Unity import
template bytes. It is a build identity, not a compatibility version.

Each asset record contains:

- Canonical request hash and cache key.
- Request kind and every source location deduplicated to that address.
- Logical canvas, raster size, subject bounds, scale, and slice metadata.
- Normalized dependency references and content hashes.
- PNG content hash and expected import metadata.
- Deterministic Unity GUID and its complete derivation hash.

Each generated PNG and JSON file receives a deterministic `.meta`. Directory
metadata uses domain-separated hashes of the project-relative directory path.
PNG metadata uses Unity 6000.5.8f1 text serialization for a `TextureImporter`
and contains only the GUID plus the import fields named below. Manifest metadata
imports it as a non-Addressable `TextAsset`. `check` parses semantic importer
fields rather than relying on YAML key order, while rejecting extra platform
overrides or labels.

Every PNG is sRGB RGBA with transparency. Default Unity import settings are:

- Texture type `Default`, resolving to `Texture2D`.
- sRGB color texture enabled.
- Alpha imported as transparency without premultiplication.
- Clamp wrap mode.
- Bilinear filtering.
- Mipmaps disabled.
- Lossless, uncompressed texture storage.

A request may override filtering with nearest-neighbor, wrapping with repeat,
or texture compression with an explicit supported lossless or lossy choice.
Overrides are part of the canonical request. The generator rejects repeat when
the request relies on transparent edge padding that would visibly tile.

The generator owns the entire generated set. It first writes and verifies a
complete sibling staging set, flushes its manifest, and then swaps the generated
root into place with same-volume renames. The stable sibling directory `.meta`
is written only after its deterministic bytes are validated. If a prior root
exists, it is renamed to a backup until the staged root is installed; startup
recovers the last manifest-complete root after an interrupted swap. Stale files
disappear only through that successful replacement. A parse, dependency,
browser, paint-bound, PNG, or import-metadata failure leaves the previous
successful set unchanged.

## Addressables and build integration

Explicit asset commands never launch Unity. Unity-facing authoring and build
commands run generation before they build the rules plugin or open/build the
Unity project:

- `cargo battlement author`
- `cargo battlement sample build <name>`
- `cargo battlement sample run <name>`

The sample hook is generic and applies to every sample whose rules dependency
graph contains generated declarations. A fixture with a non-Reactant sample
name proves there is no chess-specific path or allowlist.

During authoring or player builds, Battlement's Unity editor hook temporarily
registers every generated texture with the manifest's public Addressables key.
It snapshots any user-owned Addressables settings it touches, performs the
authoring launch or build, and restores those settings on success,
interruption, or failure. Generated registration follows the same
capture/restore ownership pattern as Battlement's generated Opus audio assets.

The hook verifies that every registered object imports as exactly
`UnityEngine.Texture2D`. A missing PNG, wrong imported type, mismatched address,
or conflicting user-owned Addressables key fails before play mode or build.
Generated registration never overwrites or permanently adopts a user-owned
Addressables entry.

## Linked runtime catalog

Every `asset_generator::generate!` invocation emits an `AssetRegistration`
into a linker-collected Reactant registry. The registration contains only
const runtime metadata:

- Public texture address.
- Logical canvas and subject bounds.
- Optional nine-slice metadata.
- Source symbol for panic diagnostics.

The Unity authoring hook compares the complete linked registration-address set
with the generated manifest before play mode or build. An extra linked
registration is a fatal non-discoverable-declaration error naming its source
symbol and instructing the author to place the invocation directly at module
scope. This catches declarations emitted by an opaque procedural macro, whose
expanded tokens cannot be recovered by source scanning. A missing linked
registration is also fatal. The comparison is exact after address
deduplication.

The registry implementation must support both native and WebAssembly rules
artifacts. Target-specific tests link two declarations from separate local
crates into native and `wasm32-unknown-unknown` fixtures, then assert that both
registrations are enumerated exactly once. A registry mechanism is acceptable
only after passing those fixtures.

When `SessionUi::into_parts` or `SessionUi::into_response` receives the initial
snapshot, Reactant merges every registered generated texture into
`Snapshot::prepared_assets` as `PreparedAsset::Texture`. It sorts by address
and deduplicates against both generated and caller-authored entries before core
snapshot validation.

Deduplication compares the complete prepared-asset case. Repeating the same
generated `PreparedAsset::Texture` or an identical caller-authored Texture case
produces one entry. If a caller supplies any other `PreparedAsset` case with the
same address string, snapshot conversion panics with the generated symbol,
address, caller case, and required `Texture2D` case. Reactant never drops its
Texture registration in favor of a wrong-kind entry and never forwards two
kinds at one address to core validation.

The merge happens for the initial authoritative snapshot, not only for assets
visible in the first render. A button's pressed image, a later route's panel,
and every other linked state variant are therefore prepared before any command
can refer to them. Reactant emits no command to add generated assets later.

Subsequent snapshot conversions perform the same deterministic union so an
authoritative replacement cannot accidentally omit linked generated assets.
Caller entries retain their normal behavior; Reactant adds only missing
generated `Texture` cases.

Using a handle through `image`, `image_source`, or `background_style` produces
ordinary Battlement UI values. Asset lease acquisition, replacement,
retirement, and teardown continue to follow the Battlement UI asset contract.
There is no asset-generator-specific runtime loading path.

If Unity cannot resolve the generated address or resolves it to anything other
than `Texture2D`, snapshot preparation fails the session. Reactant never sends a
placeholder, silently drops the style, or retries generation at runtime.

## Diagnostics and failure behavior

Declaration errors are compiler errors when the macro can determine them and
CLI errors otherwise. Every diagnostic includes the static symbol and source
location. Deduplicated request errors list all declaring locations.

Required fatal diagnostics include:

- Unsupported declaration syntax or conditional compilation.
- Unknown property, value, unit, enum case, or property combination.
- A request representable entirely by native Battlement UI.
- Canvas, subject, or slice dimensions that are invalid.
- Unpermitted clipped paint on any canvas edge.
- External-context paint such as backdrop filtering or external blending.
- Missing, escaped, unreadable, or unsupported local dependency files.
- Network access attempted by rendered content.
- Unresolved font face or missing glyph.
- No supported browser or browser launch/crash/protocol failure.
- Invalid, missing, corrupt, or nondeterministic PNG output.
- Address collision or conflict with user-owned Addressables state.
- Generated import resolving to the wrong Unity type.

Warnings are reserved for valid output that deserves review, such as a very
large raster allocation, a lossy compression override on translucent art, or
paint within one raster pixel of a permitted clip edge. A warning never stands
in for the native-only error or an ambiguous rendering result.

The CLI prints one final count of discovered, deduplicated, current, rendered,
and stale requests. On a no-op run it explicitly reports that the browser was
not started.

## Performance contract

Macro expansion is pure parsing, validation, canonicalization, and token
generation. It performs no filesystem access, Cargo invocation, or subprocess
work. Expansion emits only the handle static and its fixed-shape registration;
it never expands the request into Rust constructors or a hidden expression.
Consequently, rustc does not resolve, type-check, const-evaluate, or generate
code for the request value tree. Compile-time performance has no numeric
benchmark requirement.

The common unchanged `generate` path performs source discovery, hashing, small
manifest reads, and file metadata/content validation. It must not launch a
browser, start Unity, rewrite generated files, or touch their modification
times.

A clean or partially stale generation launches at most one browser process and
one browser context. It renders only cache misses and preserves deterministic
manifest order independent of render completion order.

The runtime cost of a generated handle is the same ordinary texture address,
prepared-asset entry, style value, and lease used by a hand-authored texture.
Registry enumeration and prepared-set merging occur during snapshot conversion,
never during component reconciliation for each element.

## Automated validation

Black-box tests exercise the public authoring and CLI contracts:

- Compile-pass fixtures cover one declaration of each kind, CSS dimensions,
  percentages, hash colors, hyphenated names, shorthands, keyword values,
  duplicate declarations, native and WebAssembly linkage, and separate local
  dependency forms.
- Compile-fail fixtures cover conditional declarations, Rust expressions,
  nested macros, unsupported properties, invalid units, invalid slices,
  external blending, and native-only requests.
- Parser parity tests feed the same fixture corpus to the macro and CLI parser
  and compare canonical bytes or diagnostic categories.
- End-to-end discovery fixtures invoke the procedural macro through an import
  alias and reexport, wrap a canonical invocation in `macro_rules!`, and emit an
  invocation from an opaque procedural macro. Alias, reexport, and declarative
  wrapper cases must fail during CLI discovery; the opaque expansion must fail
  linked-manifest parity before play mode or build.
- Golden canonicalization tests prove equivalent color spellings deduplicate,
  source locations do not change addresses, only ordered lists preserve source
  order, and dependency-byte changes do not change public addresses.
- Cache tests prove unchanged generation never starts the fake browser, while
  dependency, scale, browser, and renderer identity changes render only the
  affected requests.
- Transaction tests inject a failure during every generation phase and prove
  the previous successful set remains byte-for-byte intact.
- Browser integration tests render representative gradients, clips, masks,
  inset and outer shadows, filters, transforms, and advanced text, then inspect
  dimensions, alpha bounds, and stable browser-local reference PNGs.
- Unity editor tests verify exact import settings, `Texture2D` type, temporary
  Addressables registration, conflict failure, and restoration after success,
  interruption, and build failure.
- Snapshot tests prove automatic sorted deduplication, inclusion of unused
  variants, preservation of caller assets, and failure for missing or
  wrong-type generated assets.
- Command tests prove `check` is read-only, `generate` is a no-op when current,
  `preview` shows all metadata, and generic sample build/run invokes generation.

The existing Reactant sample contains a compact asset gallery derived from the
mockup needs: advanced gradient text, one clipped layered background, one
explicit nine-slice control, and representative gradient, clip, shadow, mask,
filter, and skew cases. Interactive size controls demonstrate nine-slice
stretching while the surrounding layout remains native Reactant UI.

Acceptance requires the sample to run in native and WebAssembly players with
the same linked catalog and public addresses. Pixel output may differ across
system browser versions, but each build must use current cache identities and
its generated files must match its manifest.

## Manual QA

1. Install a supported system Chrome or Chromium and start from a clean clone
   with no generated asset outputs.
2. Run `cargo battlement reactant assets generate` for the Reactant sample.
   Confirm one browser session renders the gallery and Unity is not launched.
3. Run the same command again. Confirm it reports every request current, does
   not launch a browser, and does not change generated modification times.
4. Run `cargo battlement reactant assets check`. Confirm it is read-only and
   succeeds.
5. Run `cargo battlement reactant assets preview`. Inspect transparency,
   metadata, subject bounds, advanced text, gradients, clipping, shadows,
   masks, filters, and skew. Resize every nine-slice preview well above and
   below its authored size and confirm its corners and logical edge widths stay
   fixed.
6. Change a referenced font or local image byte. Confirm `check` reports only
   affected requests stale, the public address stays unchanged, and `generate`
   rerenders only those requests.
7. Change the system browser version or explicit browser executable. Confirm
   the cache invalidates and the manifest records the new identity.
8. Author paint that touches an undeclared canvas edge. Confirm generation
   fails with the edge and source location, then opt into intentional clipping
   and confirm preview exposes the permitted contact.
9. Author a solid rounded rectangle that Battlement UI can render natively.
   Confirm compilation or generation rejects it and names the native style
   path. Add a gradient or unsupported clip and confirm the composite succeeds.
10. Run `cargo battlement sample run reactant` natively and for WebAssembly.
    Exercise later UI states and confirm assets absent from the first screen
    were nevertheless prepared and display without a runtime load command.
11. Interrupt an authoring launch and force a Unity build failure after
    generated Addressables registration. Confirm user-owned Addressables state
    is restored in both cases.
12. Delete or change the type of one registered generated texture and start a
    session. Confirm the session fails before rendering and no placeholder is
    displayed.
