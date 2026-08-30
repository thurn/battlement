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
  [Compositing and Blending][compositing] define the browser behavior copied by
  the supported property catalog.
- [Unity UI Toolkit USS properties][unity-uss] is the native-support baseline.
- [Unity UI Toolkit 9-slicing][unity-slicing] defines how Unity consumes the
  generated slice insets.

[css-backgrounds]: https://www.w3.org/TR/css-backgrounds-3/
[css-images]: https://www.w3.org/TR/css-images-4/
[css-masking]: https://www.w3.org/TR/css-masking-1/
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
- The catalog is closed and typed. There is no raw CSS, HTML, JavaScript,
  shader, SVG, Canvas, or custom-pixel callback escape hatch.
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
`generate` attribute macro is publicly reexported from that module so a rules
crate does not depend directly on its procedural-macro implementation.

A declaration is a named module-level `pub static` with a concrete generated
asset type. Its initializer is a const request builder:

```rust
use battlement_reactant::asset_generator::{
  self, BackgroundAsset, ClipEdge, NineSliceAsset, TextImageAsset,
};

#[asset_generator::generate]
pub static ACTION_BUTTON: NineSliceAsset = asset_generator::nine_slice(
  asset_generator::canvas(
    asset_generator::px(760),
    asset_generator::px(140),
  ),
  asset_generator::slices(
    asset_generator::px(24),
    asset_generator::px(26),
    asset_generator::px(24),
    asset_generator::px(26),
  ),
)
.clip_path(asset_generator::polygon_percent(&[
  (2.4, 0.0),
  (97.6, 0.0),
  (100.0, 12.0),
  (100.0, 88.0),
  (97.6, 100.0),
  (2.4, 100.0),
  (0.0, 88.0),
  (0.0, 12.0),
]))
.allow_clipping(&[
  ClipEdge::Top,
  ClipEdge::Right,
  ClipEdge::Bottom,
  ClipEdge::Left,
])
.background(asset_generator::linear_gradient(
  asset_generator::deg(110.0),
  &[
    asset_generator::stop(0.0, asset_generator::hex("b9fbff")),
    asset_generator::stop(100.0, asset_generator::hex("ff4bd1")),
  ],
));
```

The values are illustrative; the polygon itself is a complete closed shape.

The attribute rewrites the initializer into a `NineSliceAsset` handle and
emits one linked catalog registration. It also emits a hidden const expression
containing the request, so Rust continues to type-check the complete builder
expression. The macro never reads files, walks the project, launches a process,
or performs rendering.

There are three request constructors and three corresponding handle types:

| Constructor | Handle | Intended use |
| --- | --- | --- |
| `background` | `BackgroundAsset` | One fixed decorative or content texture |
| `nine_slice` | `NineSliceAsset` | Resizable background with fixed edges |
| `text_image` | `TextImageAsset` | Pre-rendered text with advanced paint |

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

The macro and CLI accept the same deliberately restricted Rust syntax:

- A named, module-level `pub static` with the `generate` attribute.
- One of the three exact generated handle types.
- One constructor call followed by literal const builder calls.
- Numeric, Boolean, string, color, enum, array, tuple, and local-file-reference
  literals accepted by the asset-generator types.
- Closed asset-generator enum values and built-in constants named explicitly by
  the parser.

The subset rejects arbitrary helper function calls, control flow, closures,
computed collections, user-defined const references, environment reads, and
proc-macro-generated declarations. The CLI does not run game code to evaluate
a request. Inline literal arrays express gradient stops, polygons, shadows,
filters, transforms, and other ordered lists.

Accepted Rust paths have one spelling. The attribute is exactly
`#[asset_generator::generate]`. A static uses one bare handle type named above.
Every free constructor is `asset_generator::<name>`, every builder is a method
call, and every fieldless enum value is `<Type>::<Variant>` with its type
imported under the original name. Aliases, reexports, wildcard imports, leading
`::`, `crate`, `self`, `super`, associated constants, payload enum syntax, and
turbofish arguments are rejected inside a declaration. Payload values use only
the free constructors listed below.

`cfg`, `cfg_attr`, Cargo-feature gating, and placement inside a conditionally
compiled parent module are errors. Every linked request must exist in every
target build so the generated catalog is identical for native and WebAssembly
rules engines.

The shared parser and canonicalizer live in an ordinary library consumed by
both the proc macro and CLI. Parser conformance fixtures must prove that every
accepted macro declaration is discovered with identical semantics by the CLI
and that every rejected construct receives the same diagnostic category.

### Normative builder surface

The constructor expression has a request type during macro parsing and becomes
the declared handle type in expanded Rust:

| Constructor | Parsed request type | Declared handle type |
| --- | --- | --- |
| `background(Canvas)` | `BackgroundRequest` | `BackgroundAsset` |
| `nine_slice(Canvas, SliceInsets)` | `NineSliceRequest` | `NineSliceAsset` |
| `text_image(Canvas, TextRun)` | `TextImageRequest` | `TextImageAsset` |

The request types are accepted only as an attributed static initializer. They
are not runtime builders and cannot be returned from functions.

These geometry constructors are shared:

```rust
pub type Number = f64;
pub const fn canvas(width: Length, height: Length) -> Canvas;
pub const fn rect(x: Length, y: Length,
                  width: Length, height: Length) -> SubjectRect;
pub const fn rect_px(x: Number, y: Number,
                     width: Number, height: Number) -> SubjectRect;
pub const fn slices(top: Length, right: Length,
                    bottom: Length, left: Length) -> SliceInsets;
pub const fn px(value: Number) -> Length;
```

`Number` is a public alias for `f64`, but the restricted parser accepts only an
integer or decimal literal where it appears. Geometry constructors require each
`Length` to be a direct `px` value.

Every request type has these const builder methods:

| Method | Value | Default |
| --- | --- | --- |
| `subject` | One `SubjectRect` | Complete canvas |
| `allow_clipping` | Set of `ClipEdge` | Empty set |
| `raster_scale` | Integer `RasterScale` from 1 through 8 | Project default, 2 |
| `filter_mode` | `FilterMode::Bilinear` or `Nearest` | `Bilinear` |
| `wrap_mode` | `WrapMode::Clamp` or `Repeat` | `Clamp` |
| `compression` | Closed `TextureCompression` | `Lossless` |

`ClipEdge` has `Top`, `Right`, `Bottom`, and `Left`. `RasterScale` is created by
`scale` from an integer literal. `FilterMode` has `Bilinear` and `Nearest`;
`WrapMode` has `Clamp` and `Repeat`; and `TextureCompression` has `Lossless`,
`LossyLow`, `LossyNormal`, and `LossyHigh`. The three lossy cases map to Unity's
low-, normal-, and high-quality compressed importer settings.

```rust
pub const fn scale(value: u8) -> RasterScale;
```

The value must be `1..=8`. Clipping edges must be unique and appear in top,
right, bottom, left order.

Every builder method may appear at most once. Ordered CSS values such as
background layers, shadows, filters, transforms, gradient stops, and polygons
are supplied as one inline array in paint order. Duplicate single-valued
methods are errors rather than last-declaration-wins aliases.

`BackgroundRequest` and `NineSliceRequest` accept `background`, `border`,
`border_radius`, `box_shadow`, `clip_path`, `mask`, `opacity`,
`background_blend_mode`, `isolation`, `filter`, `transform`, and
`transform_origin`.

`TextImageRequest` accepts `font`, `font_size`, `font_style`, `font_weight`,
`font_stretch`, `line_height`, `letter_spacing`, `word_spacing`, `text_align`,
`white_space`, `color`, `text_fill`, `text_stroke`, `text_shadow`, `opacity`,
`filter`, `transform`, and `transform_origin`.

`TextRun` is created with `text("...")` and has no spans. `font` takes exactly
one font-valued `LocalFile`; there is no fallback list. `font` and `font_size`
are required. A font file selects one non-variable face. Omitted style, weight,
and stretch
come from that face's metadata. When any is authored explicitly, it must equal
the face metadata. The browser may not synthesize a different face.

The closed value types mirror their CSS grammar:

| Type | Constructors |
| --- | --- |
| `Length` | `px`, `percent`, `em`, `rem`, `vw`, `vh`, `vmin`, `vmax`, typed calculations |
| `Angle` | `deg`, `grad`, `rad`, `turn` |
| `Color` | `hex`, `rgb`, `rgba`, `hsl`, `hsla`, `NamedColor` |
| `Background` | Color plus an ordered array of image layers |
| `BackgroundImage` | Six gradient kinds or one `LocalImage` |
| `Shadow` | Offset, blur, spread, color, and outer/inset kind |
| `ClipPath` | `inset`, `circle`, `ellipse`, `polygon`, typed `path` |
| `Mask` | Ordered local-image or gradient mask layers |
| `Filter` | One supported filter-function array |
| `Transform` | One supported 2D transform-function array |
| `TextFill` | Solid color, gradient, or local image |

Every grammar-specific enum uses the CSS keyword as a Rust case, converted to
UpperCamelCase. Constructors reject values outside the CSS grammar even when
Chrome would recover from an equivalent raw string.

### Exact value constructors

The signatures in this section are the complete initial public value grammar.
`Number` means a finite numeric literal. Every slice argument is an inline
literal array. Functions shown without a body are normative signatures, not an
escape hatch for game-defined implementations.

The three request builders expose these exact methods, with the named return
type matching the receiver:

```rust
// All request types.
pub const fn subject(self, value: SubjectRect) -> Self;
pub const fn allow_clipping(self, edges: &[ClipEdge]) -> Self;
pub const fn raster_scale(self, value: RasterScale) -> Self;
pub const fn filter_mode(self, value: FilterMode) -> Self;
pub const fn wrap_mode(self, value: WrapMode) -> Self;
pub const fn compression(self, value: TextureCompression) -> Self;

// BackgroundRequest and NineSliceRequest.
pub const fn background(self, value: Paint) -> Self;
pub const fn border(self, value: Border) -> Self;
pub const fn border_radius(self, value: BorderRadii) -> Self;
pub const fn box_shadow(self, values: &[BoxShadow]) -> Self;
pub const fn clip_path(self, value: ClipPath) -> Self;
pub const fn mask(self, value: Mask) -> Self;
pub const fn opacity(self, value: Number) -> Self;
pub const fn background_blend_mode(self, values: &[BlendMode]) -> Self;
pub const fn isolation(self, value: Isolation) -> Self;
pub const fn filter(self, values: &[FilterFunction]) -> Self;
pub const fn transform(self, values: &[TransformFunction]) -> Self;
pub const fn transform_origin(self, value: TransformOrigin) -> Self;

// TextImageRequest.
pub const fn font(self, value: LocalFile) -> Self;
pub const fn font_size(self, value: Length) -> Self;
pub const fn font_style(self, value: FontStyle) -> Self;
pub const fn font_weight(self, value: FontWeight) -> Self;
pub const fn font_stretch(self, value: FontStretch) -> Self;
pub const fn line_height(self, value: LineHeight) -> Self;
pub const fn letter_spacing(self, value: Length) -> Self;
pub const fn word_spacing(self, value: Length) -> Self;
pub const fn text_align(self, value: TextAlign) -> Self;
pub const fn white_space(self, value: WhiteSpace) -> Self;
pub const fn color(self, value: Color) -> Self;
pub const fn text_fill(self, value: Paint) -> Self;
pub const fn text_stroke(self, value: TextStroke) -> Self;
pub const fn text_shadow(self, values: &[TextShadow]) -> Self;
pub const fn opacity(self, value: Number) -> Self;
pub const fn filter(self, values: &[FilterFunction]) -> Self;
pub const fn transform(self, values: &[TransformFunction]) -> Self;
pub const fn transform_origin(self, value: TransformOrigin) -> Self;
```

Lengths and angles use:

```rust
pub const fn px(value: Number) -> Length;
pub const fn percent(value: Number) -> Length;
pub const fn em(value: Number) -> Length;
pub const fn rem(value: Number) -> Length;
pub const fn vw(value: Number) -> Length;
pub const fn vh(value: Number) -> Length;
pub const fn vmin(value: Number) -> Length;
pub const fn vmax(value: Number) -> Length;
pub const fn add(left: Length, right: Length) -> Length;
pub const fn subtract(left: Length, right: Length) -> Length;
pub const fn multiply(value: Length, factor: Number) -> Length;
pub const fn divide(value: Length, divisor: Number) -> Length;
pub const fn min(values: &[Length]) -> Length;
pub const fn max(values: &[Length]) -> Length;
pub const fn clamp(min: Length, preferred: Length, max: Length) -> Length;
pub const fn deg(value: Number) -> Angle;
pub const fn grad(value: Number) -> Angle;
pub const fn rad(value: Number) -> Angle;
pub const fn turn(value: Number) -> Angle;
```

`min` and `max` require at least one value. Division by zero and non-finite
results are errors. Mixed length units remain a typed calculation tree and are
resolved by Chrome. Canvas, subject, and slice geometry specifically require a
`px` leaf rather than a calculation or another unit.

Colors use:

```rust
pub const fn hex(value: &str) -> Color;
pub const fn rgb(red: Number, green: Number, blue: Number) -> Color;
pub const fn rgba(red: Number, green: Number,
                  blue: Number, alpha: Number) -> Color;
pub const fn hsl(hue: Angle, saturation: Number,
                 lightness: Number) -> Color;
pub const fn hsla(hue: Angle, saturation: Number,
                  lightness: Number, alpha: Number) -> Color;
pub const fn named_color(value: NamedColor) -> Color;
```

Hex accepts CSS three-, four-, six-, and eight-digit forms with an optional
leading `#`. RGB channels use `0..=255`; saturation, lightness, and alpha use
`0..=1`. `NamedColor` contains the CSS Color named-color table plus
`Transparent` and excludes `CurrentColor`.

File references use:

```rust
pub const fn cargo_file(path: &str) -> LocalFile;
pub const fn unity_file(path: &str) -> LocalFile;
pub const fn local_image(file: LocalFile) -> Paint;
pub const fn solid(color: Color) -> Paint;
```

`TextImageRequest::font` validates its `LocalFile` as a font. A local image
validates its file as PNG.

Gradient paint uses:

```rust
pub const fn stop(position_percent: Number, color: Color) -> GradientItem;
pub const fn color_stop(color: Color, position: Length) -> GradientItem;
pub const fn double_stop(color: Color, first: Length,
                         second: Length) -> GradientItem;
pub const fn transition_hint(position: Length) -> GradientItem;
pub const fn linear_gradient(angle: Angle,
                             items: &[GradientItem]) -> Paint;
pub const fn repeating_linear_gradient(
  angle: Angle, items: &[GradientItem],
) -> Paint;
pub const fn radial_gradient(position: Position,
                             items: &[GradientItem]) -> Paint;
pub const fn radial_gradient_with(
  shape: RadialShape, size: RadialSize,
  position: Position, items: &[GradientItem],
) -> Paint;
pub const fn repeating_radial_gradient(
  shape: RadialShape, size: RadialSize,
  position: Position, items: &[GradientItem],
) -> Paint;
pub const fn conic_gradient(from: Angle, position: Position,
                            items: &[GradientItem]) -> Paint;
pub const fn repeating_conic_gradient(
  from: Angle, position: Position, items: &[GradientItem],
) -> Paint;
```

`RadialShape` has `Circle` and `Ellipse`. Fieldless `RadialSize` values are
`ClosestSide`, `ClosestCorner`, `FarthestSide`, and `FarthestCorner`;
`circle_size` and `ellipse_size` create its explicit forms. An explicit size
must match the selected shape.
`Position` is created from lengths or from explicit horizontal and vertical
anchors. Gradient arrays require at least two color stops and permit transition
hints only between stops.

The exact position and layer controls are:

```rust
pub const fn position(x: Length, y: Length) -> Position;
pub const fn anchored_position(
  x_anchor: HorizontalAnchor, x_offset: Length,
  y_anchor: VerticalAnchor, y_offset: Length,
) -> Position;
pub const fn background_size(
  width: SizeAxis, height: SizeAxis,
) -> BackgroundSize;
pub const fn length_size(value: Length) -> SizeAxis;
pub const fn circle_size(radius: Length) -> RadialSize;
pub const fn ellipse_size(x: Length, y: Length) -> RadialSize;
pub const fn repeat(x: Repeat, y: Repeat) -> Repeat2D;
pub const fn oblique(angle: Angle) -> FontStyle;
```

`HorizontalAnchor` has `Left`, `Center`, and `Right`; `VerticalAnchor` has
`Top`, `Center`, and `Bottom`. The only fieldless `SizeAxis` is `Auto`;
`length_size` creates a concrete axis. Fieldless `BackgroundSize` values are
`Cover` and `Contain`; `background_size` creates an explicit pair.
`circle_size`, `ellipse_size`, and `oblique` create the remaining payload
values. `Repeat` and `PaintBox` have the cases listed below.

One background paint is passed directly to `background`. Multiple layers use:

```rust
pub const fn layer(paint: Paint) -> BackgroundLayer;
pub const fn background_layers(layers: &[BackgroundLayer]) -> Paint;
pub const fn background_layers_with_color(
  color: Color, layers: &[BackgroundLayer],
) -> Paint;
```

`background_layers` requires at least two layers.
`background_layers_with_color` requires at least one layer and a nontransparent
color. These rules prevent alternate spellings of the single transparent-layer
shorthand.

`BackgroundLayer` has `position(Position)`, `size(BackgroundSize)`,
`repeat(Repeat2D)`, `origin(PaintBox)`, and `clip(PaintBox)`. `BackgroundSize`
uses the cases defined above. `Repeat2D` contains independent `NoRepeat`,
`Repeat`, `Round`, or `Space` values. `PaintBox` has `BorderBox`, `PaddingBox`,
and `ContentBox`.

Those layer builders have these signatures:

```rust
pub const fn position(self, value: Position) -> BackgroundLayer;
pub const fn size(self, value: BackgroundSize) -> BackgroundLayer;
pub const fn repeat(self, value: Repeat2D) -> BackgroundLayer;
pub const fn origin(self, value: PaintBox) -> BackgroundLayer;
pub const fn clip(self, value: PaintBox) -> BackgroundLayer;
```

`background(Paint)` is shorthand for one layer over transparent with initial
CSS layer values. The initial layer position is left/top zero, size is auto,
repeat is repeat on both axes, origin is padding-box, and clip is border-box.
An explicit `background_blend_mode` array must contain either one mode or one
mode per layer. `BlendMode` has `Normal`, `Multiply`, `Screen`, `Overlay`,
`Darken`, `Lighten`, `ColorDodge`, `ColorBurn`, `HardLight`, `SoftLight`,
`Difference`, `Exclusion`, `Hue`, `Saturation`, `Color`, and `Luminosity`.

Borders and shadows use:

```rust
pub const fn border_edge(width: Length, style: BorderStyle,
                         color: Color) -> BorderEdge;
pub const fn border(top: BorderEdge, right: BorderEdge,
                    bottom: BorderEdge, left: BorderEdge) -> Border;
pub const fn border_all(width: Length, style: BorderStyle,
                        color: Color) -> Border;
pub const fn radius(horizontal: Length, vertical: Length) -> Radius;
pub const fn radii(top_left: Radius, top_right: Radius,
                   bottom_right: Radius,
                   bottom_left: Radius) -> BorderRadii;
pub const fn outer_shadow(x: Length, y: Length, blur: Length,
                          spread: Length, color: Color) -> BoxShadow;
pub const fn inset_shadow(x: Length, y: Length, blur: Length,
                          spread: Length, color: Color) -> BoxShadow;
```

`BorderStyle` has `None`, `Solid`, `Dashed`, `Dotted`, and `Double`. Blur and
border widths are nonnegative. `border_radius` accepts one `BorderRadii`, while
`box_shadow` accepts an ordered nonempty array.

Clips and paths use:

```rust
pub const fn inset_clip(top: Length, right: Length,
                        bottom: Length, left: Length) -> ClipPath;
pub const fn rounded_inset_clip(
  top: Length, right: Length, bottom: Length, left: Length,
  radii: BorderRadii,
) -> ClipPath;
pub const fn circle_clip(radius: Length,
                         position: Position) -> ClipPath;
pub const fn ellipse_clip(x_radius: Length, y_radius: Length,
                          position: Position) -> ClipPath;
pub const fn polygon(fill: FillRule,
                     points: &[(Length, Length)]) -> ClipPath;
pub const fn polygon_percent(points: &[(Number, Number)]) -> ClipPath;
pub const fn path(fill: FillRule,
                  commands: &[PathCommand]) -> ClipPath;
```

`FillRule` has `NonZero` and `EvenOdd`. `PathCommand` is constructed only by
`move_to`, `line_to`, `horizontal_to`, `vertical_to`, `cubic_to`,
`quadratic_to`, `arc_to`, and `close_path`. Path coordinates are unitless
`Number` values interpreted as logical CSS pixels because CSS `path()` does not
accept length units. Relative path commands are not supported. A path must
start with a move and contain at least one drawing command.

Polygon arrays require at least three points and a nonzero signed area after
unit resolution. Adjacent duplicate points are rejected.

The absolute path constructors are:

```rust
pub const fn move_to(x: Number, y: Number) -> PathCommand;
pub const fn line_to(x: Number, y: Number) -> PathCommand;
pub const fn horizontal_to(x: Number) -> PathCommand;
pub const fn vertical_to(y: Number) -> PathCommand;
pub const fn cubic_to(x1: Number, y1: Number,
                      x2: Number, y2: Number,
                      x: Number, y: Number) -> PathCommand;
pub const fn quadratic_to(control_x: Number, control_y: Number,
                          x: Number, y: Number) -> PathCommand;
pub const fn arc_to(rx: Number, ry: Number, rotation: Angle,
                    large_arc: bool, sweep: bool,
                    x: Number, y: Number) -> PathCommand;
pub const fn close_path() -> PathCommand;
```

Masks use:

```rust
pub const fn mask_layer(source: Paint) -> MaskLayer;
pub const fn mask(layers: &[MaskLayer]) -> Mask;
```

`MaskLayer` has the same `position`, `size`, `repeat`, `origin`, and `clip`
builders as a background layer, plus `mode(MaskMode)` and
`composite(MaskComposite)`. `MaskMode` has `Alpha` and `Luminance`.
`MaskComposite` has `Add`, `Subtract`, `Intersect`, and `Exclude`. Mask arrays
are nonempty and every layer is internal to the request.

Each `MaskLayer` builder takes the corresponding value named above and returns
`MaskLayer`; `mode` takes `MaskMode` and `composite` takes `MaskComposite`.
Its defaults are left/top zero position, auto size, repeat on both axes,
border-box origin and clip, alpha mode, and add composition. A mask layer source
must be a single gradient, local image, or solid paint; layered paint is
rejected.

```rust
pub const fn position(self, value: Position) -> MaskLayer;
pub const fn size(self, value: BackgroundSize) -> MaskLayer;
pub const fn repeat(self, value: Repeat2D) -> MaskLayer;
pub const fn origin(self, value: PaintBox) -> MaskLayer;
pub const fn clip(self, value: PaintBox) -> MaskLayer;
pub const fn mode(self, value: MaskMode) -> MaskLayer;
pub const fn composite(self, value: MaskComposite) -> MaskLayer;
```

Filters use an ordered array of these exact values:

```rust
pub const fn blur(radius: Length) -> FilterFunction;
pub const fn brightness(amount: Number) -> FilterFunction;
pub const fn contrast(amount: Number) -> FilterFunction;
pub const fn drop_shadow(x: Length, y: Length, blur: Length,
                         color: Color) -> FilterFunction;
pub const fn grayscale(amount: Number) -> FilterFunction;
pub const fn hue_rotate(angle: Angle) -> FilterFunction;
pub const fn invert(amount: Number) -> FilterFunction;
pub const fn filter_opacity(amount: Number) -> FilterFunction;
pub const fn saturate(amount: Number) -> FilterFunction;
pub const fn sepia(amount: Number) -> FilterFunction;
```

Blur is nonnegative. Percentage-like amounts are nonnegative except invert,
grayscale, opacity, and sepia, which are restricted to `0..=1`. An empty filter
array is equivalent to omitting the property and is rejected as redundant.

Transforms use an ordered array of:

```rust
pub const fn translate(x: Length, y: Length) -> TransformFunction;
pub const fn rotate(angle: Angle) -> TransformFunction;
pub const fn scale_xy(x: Number, y: Number) -> TransformFunction;
pub const fn skew(x: Angle, y: Angle) -> TransformFunction;
pub const fn skew_x(angle: Angle) -> TransformFunction;
pub const fn skew_y(angle: Angle) -> TransformFunction;
pub const fn matrix(a: Number, b: Number, c: Number,
                    d: Number, tx: Number,
                    ty: Number) -> TransformFunction;
pub const fn transform_origin(position: Position) -> TransformOrigin;
```

An empty transform array is redundant. Matrix translation components are
logical CSS pixels. The request builder's `transform_origin` takes the final
`TransformOrigin`, not the constructor function itself.

Text-only values use:

```rust
pub const fn font_size(value: Length) -> Length;
pub const fn line_height(value: Length) -> LineHeight;
pub const fn normal_line_height() -> LineHeight;
pub const fn font_weight(value: u16) -> FontWeight;
pub const fn font_stretch(percent: Number) -> FontStretch;
pub const fn text_stroke(width: Length, color: Color) -> TextStroke;
pub const fn text_shadow(x: Length, y: Length, blur: Length,
                         color: Color) -> TextShadow;
```

The request methods accept `Length` directly for font size and spacing, so the
font-size wrapper is optional. Line height uses `line_height` or
`normal_line_height`. Font weight is `1..=1000`; stretch is `50..=200` percent.
Fieldless `FontStyle` values are `Normal` and `Italic`; `oblique` creates an
explicit oblique angle. `TextAlign` has
`Start`, `Center`, `End`, `Justify`, `Left`, and `Right`. `WhiteSpace` has
`Normal`, `Pre`, `PreWrap`, and `NoWrap`.

Font size must be positive. Explicit line height must be nonnegative. Request
opacity and filter opacity are restricted to `0..=1`.

`color` defaults to opaque black. `text_fill` defaults to the authored color and
accepts one `Paint`; explicitly setting both is an error. Line height defaults
to CSS `normal`; spacing defaults to `normal`; alignment defaults to `Start`;
white space defaults to `Normal`; stroke, shadows, filters, and transforms
default to absent. `text_shadow` takes a nonempty ordered array of `TextShadow`.
Text fill rejects layered paint; it accepts one solid, gradient, or local image.

Box paint defaults to transparent with no border, radius, shadow, clip, mask,
filter, transform, or blend. Opacity defaults to 1 and isolation defaults to
`Auto`; an authored isolation value can only be `Isolation::Isolate`. Omitted
values do not appear in canonical bytes. Explicit values equal to these defaults
are rejected as redundant so one visual configuration has one canonical form.

A complete background declaration looks like:

```rust
#[asset_generator::generate]
pub static PANEL_GLOW: BackgroundAsset = asset_generator::background(
  asset_generator::canvas(asset_generator::px(640), asset_generator::px(360)),
)
.subject(asset_generator::rect_px(16, 16, 608, 328))
.background(asset_generator::radial_gradient(
  asset_generator::position(
    asset_generator::percent(50.0),
    asset_generator::percent(65.0),
  ),
  &[asset_generator::stop(0.0, asset_generator::hex("153d75")),
    asset_generator::stop(100.0, asset_generator::hex("020611"))],
))
.box_shadow(&[asset_generator::outer_shadow(
  asset_generator::px(0),
  asset_generator::px(0),
  asset_generator::px(16),
  asset_generator::px(0),
  asset_generator::hex("368dff80"),
)])
.allow_clipping(&[
  ClipEdge::Top,
  ClipEdge::Right,
  ClipEdge::Bottom,
  ClipEdge::Left,
]);
```

A complete text declaration looks like:

```rust
#[asset_generator::generate]
pub static PLAY_LABEL: TextImageAsset = asset_generator::text_image(
  asset_generator::canvas(asset_generator::px(480), asset_generator::px(146)),
  asset_generator::text("PLAY"),
)
.subject(asset_generator::rect_px(12, 16, 456, 114))
.font(asset_generator::cargo_file("fonts/barlow-condensed-800-italic.ttf"))
.font_size(asset_generator::px(91))
.text_fill(asset_generator::linear_gradient(
  asset_generator::deg(174.0),
  &[asset_generator::stop(0.0, asset_generator::hex("ffffff")),
    asset_generator::stop(100.0, asset_generator::hex("ff6dda"))],
))
.transform(&[asset_generator::skew_x(asset_generator::deg(-5.0))]);
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

- `cargo_file("...")` resolves relative to the declaring Cargo package.
- `unity_file("...")` resolves relative to the selected Unity project.

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
- `rem`, relative to an explicit request root font size, defaulting to 16 px.
- `vw`, `vh`, `vmin`, and `vmax`, relative to the explicit canvas.
- Unitless numbers where the corresponding CSS grammar permits them.
- `deg`, `grad`, `rad`, and `turn` for angles.
- Typed `calc`, `min`, `max`, and `clamp` expression trees.

Environment-dependent units and values are rejected, including dynamic
viewport units, physical units, container-query units, `env`, `var`, inherited
values, current viewport state, and current system theme.

All color inputs normalize to explicit sRGB RGBA. The accepted spelling
includes typed constructors corresponding to hex, `rgb`/`rgba`, `hsl`/`hsla`,
and the closed CSS named-color table. `currentColor`, system colors, color
profiles, and device-dependent color spaces are rejected.

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
Names use Rust `snake_case`; their documented CSS equivalents remain the
behavioral contract.

### Box paint

| API property | CSS behavior |
| --- | --- |
| `background` | Ordered color and image layers with CSS layer controls |
| `border` | Per-edge width, style, and color |
| `border_radius` | Elliptical per-corner radii |
| `box_shadow` | Ordered outer and inset CSS shadows |
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

| API property | CSS behavior |
| --- | --- |
| `clip_path` | Basic shapes or typed SVG-compatible path data |
| `mask` | Ordered local image or gradient masks and standard mask positioning |
| `background_blend_mode` | Blend background layers within the request |
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
- `drop_shadow`
- `grayscale`
- `hue_rotate`
- `invert`
- `opacity`
- `saturate`
- `sepia`

The `transform` list supports 2D `translate`, `rotate`, `scale`, `skew`,
`skew_x`, `skew_y`, and `matrix`, plus an explicit `transform_origin`.
Perspective and 3D transforms are rejected.

### Text paint

A text request contains exactly one Unicode string and one text run. It may
contain newline characters but cannot mix spans with different styles.

| API property | CSS behavior |
| --- | --- |
| `font` | Explicit local face, size, style, weight, stretch, and line height |
| `letter_spacing` | CSS letter spacing |
| `word_spacing` | CSS word spacing |
| `text_align` | Horizontal alignment within the subject box |
| `white_space` | Closed `normal`, `pre`, `pre_wrap`, or `nowrap` behavior |
| `color` | Solid glyph fill |
| `text_fill` | Solid, gradient, or local-image fill clipped to glyphs |
| `text_stroke` | Width and color corresponding to browser text stroke |
| `text_shadow` | Ordered CSS text shadows |
| `filter` | Filter list applied to the complete text run |
| `transform` | 2D transform applied to the complete text run |

Gradient text uses browser background clipping and transparent glyph fill.
Stroke support uses the system browser's prefixed implementation when the
unprefixed property is unavailable. That browser-dependent raster result is
covered by browser identity in the cache key.

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
Rust dependencies for attributed declarations. Registry dependencies are
scanned only when Cargo resolves them into the selected rules build.

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

- Uses declared property order only where CSS order changes paint.
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

Every `generate` attribute emits an `AssetRegistration` into a
linker-collected Reactant registry. The registration contains only const runtime
metadata:

- Public texture address.
- Logical canvas and subject bounds.
- Optional nine-slice metadata.
- Source symbol for panic diagnostics.

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
work. Compile-time performance has no numeric benchmark requirement.

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

- Compile-pass fixtures cover one declaration of each kind, built-in fieldless
  enum values, duplicate declarations, native and WebAssembly linkage, and
  separate local dependency forms.
- Compile-fail fixtures cover conditional declarations, helper evaluation,
  unsupported properties, invalid units, invalid slices, external blending,
  and native-only requests.
- Parser parity tests feed the same fixture corpus to the macro and CLI parser
  and compare canonical bytes or diagnostic categories.
- Golden canonicalization tests prove equivalent color spellings deduplicate,
  source locations do not change addresses, property ordering is preserved only
  where meaningful, and dependency-byte changes do not change public addresses.
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
