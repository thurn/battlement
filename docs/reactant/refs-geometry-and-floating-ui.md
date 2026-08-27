# Reactant Refs, Geometry, and Floating UI

This appendix defines committed Unity element references, geometry observation,
and a complete two-pass floating-tooltip pattern. It is part of the
[Battlement Reactant technical design](reactant-technical-design.md).

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  host elements, Unity events, and style updates.
- [Unity `GeometryChangedEvent`][unity-geometry] is emitted after an element's
  layout position or dimensions change.
- [Unity `VisualElement.worldBound`][unity-world-bound] is the transformed
  axis-aligned bound in panel coordinates.
- [Unity `VisualElement.worldTransform`][unity-world-transform] maps local
  coordinates into panel coordinates.
- [Unity USS properties][unity-uss] distinguish `visibility: hidden`, which
  retains layout, from `display: none`.
- [React `useLayoutEffect`](https://react.dev/reference/react/useLayoutEffect)
  explains the synchronous pre-paint behavior Reactant deliberately does not
  claim to provide.

[unity-geometry]: https://docs.unity3d.com/ScriptReference/UIElements.GeometryChangedEvent.html
[unity-world-bound]: https://docs.unity3d.com/ScriptReference/UIElements.VisualElement-worldBound.html
[unity-world-transform]: https://docs.unity3d.com/ScriptReference/UIElements.VisualElement-worldTransform.html
[unity-uss]: https://docs.unity3d.com/Manual/UIE-USS-SupportedProperties.html

## Two kinds of refs

`use_ref` stores arbitrary mutable Rust data and does not rerender when changed.
`use_element_ref` instead creates an **element ref**, a stable handle whose
attachment follows one committed host element.

```rust
let counter = use_ref(0_u32);
let button = use_element_ref();
```

The separate types prevent ordinary mutable data from being mistaken for a
Unity attachment and let Reactant apply commit and reconnect rules to element
refs.

## Creating and attaching an element ref

`use_element_ref` consumes one positional hook slot and returns a cloneable
`ElementRef`.

```rust
let anchor = use_element_ref();
Some(Button::new("Details").element_ref(anchor.clone()))
```

The extension attaches the ref to that host after the host becomes committed.
Component and fragment values cannot receive an element ref because they have no
Unity object.

One ref may attach to at most one committed host. Rendering the same ref on two
hosts panics before commit.

```rust
Fragment::new((
    Button::new("A").element_ref(shared.clone()),
    Button::new("B").element_ref(shared.clone()),
))
```

Cloning the handle for hooks and callbacks is valid; attaching those clones more
than once is not.

## Attachment lifecycle

An element ref has detached and attached states. Attachment stores a private
runtime identity, host `ObjectId`, and owning document.

```rust
ElementAttachment {
    object_id,
    document_id,
    generation,
}
```

`generation` changes on every attach, including attachment to a recreated host
with the same `ObjectId`. Reactant derives a fresh serialized
`GeometryObservationId` from it whenever `use_geometry` observes that
attachment.

The public `ElementRef` does not expose a mutable Unity object or C# reference.
It supports identity comparison, attachment inspection, and hooks that ask
Reactant to observe supported host state.

```rust
if anchor.is_attached() {
    // The committed tree contains its host.
}
```

Attachment changes are committed with the tree:

- a new host attaches after its create operation is committed;
- a reused host leaves the ref attached;
- moving a host does not detach it;
- changing the ref detaches the old and attaches the new at one commit; and
- unmount detaches immediately before Unity can route another logical event.

`use_geometry` tracks the attachment generation, not only the `ElementRef`
handle. A detach or a different generation clears the hook's value. Its native
subscription carries the derived observation ID, so a queued event from an
older attachment is distinguishable and ignored.

A suspended initial render never attaches tentative refs. Re-suspended committed
content keeps refs attached while its hosts are internally hidden.

## Geometry protocol

The existing Battlement geometry payload must include both layout and
panel-space data.

```rust
pub struct GeometryChanged {
    pub observation_id: GeometryObservationId,
    pub old_layout: Rect,
    pub new_layout: Rect,
    pub world_bound: Rect,
    pub panel_from_parent: Affine2,
    pub document_id: ObjectId,
}
```

`GeometryObservationId` is an opaque runtime-unique `u64` newtype. Installing a
geometry subscription sends it to Unity; every scheduled or native geometry
report echoes it. Reactant accepts a report only when it equals the current
hook subscription ID.

`new_layout` is Unity's `VisualElement.layout`, whose position is relative to
the parent layout coordinate system. `world_bound` is the transformed
axis-aligned bound in the element's panel coordinate system. `document_id`
identifies that panel because each Battlement `UiDocument` owns its private
runtime panel.

`panel_from_parent` is the invertible two-dimensional affine part of the
element parent's current `worldTransform`. It maps a parent-local position or
vector into panel coordinates. `Affine2` stores six finite `f64` coefficients;
vectors ignore its translation term. The Unity adapter panics when the relevant
transform is perspective, non-invertible, or otherwise cannot be represented by
that contract. Core floating placement supports affine 2D UI panels only.

The old panel-space bound is intentionally absent. Unity's event provides old
and new layout rectangles, but not a trustworthy old `worldBound` after ancestor
transforms. Reactant placement uses the current bound.

The Unity adapter reads `target.layout`, `target.worldBound`, and the parent's
`worldTransform` in one observation pass. The previous sampled layout becomes
`old_layout`; the current value becomes `new_layout`.

## Initial observation

Geometry observation must produce an initial value and notice panel-space
changes caused by ancestors, scrolling, or panel scaling. The Unity adapter
keeps a registry of active observation IDs and samples each target once after
every completed panel layout.

```rust
subscribe(element, observation_id);
schedule_after_layout(observation_id);
```

The target's native `GeometryChangedEvent`, panel scale changes, and scroll
changes request the same scheduled pass early; they do not serialize a second
payload directly. The adapter compares the complete sampled payload and emits
nothing when it is equal. Consumers therefore see one initial value and then
only changed layout, `worldBound`, parent transform, or document values.

Sampling active observers once per frame is deliberate. A target's own layout
event does not fire when only an ancestor transform changes its `worldBound`.
The registry is empty when Reactant has no geometry hooks, so ordinary UI pays
no per-frame geometry cost.

Geometry events target only the observed element. Reactant does not perform
logical capture or bubble propagation for them; it updates the subscribed hook
slot directly and schedules a normal root refresh.

## use_geometry

`use_geometry` consumes one hook slot and observes one `ElementRef`.

```rust
pub fn use_element_ref() -> ElementRef;
pub fn use_geometry(element: &ElementRef) -> Option<Geometry>;
```

It returns `Option<Geometry>`. The value is `None` before committed attachment,
while waiting for the initial Unity observation, and after detach or reconnect.

```rust
pub struct Geometry {
    pub layout: Rect,
    pub world_bound: Rect,
    pub panel_from_parent: Affine2,
    pub document_id: ObjectId,
}
```

`Geometry` is `Clone + PartialEq`. Equal events do not schedule a rerender. A
changed layout, transformed bound, or document does.

Changing the ref argument unsubscribes from the old host and clears the previous
geometry before observing the new host. Unmount removes the subscription.

```rust
let geometry = use_geometry(&self.target_ref);
Some(geometry.map(GeometryLabel::new))
```

Reactant synthesizes the underlying Battlement event subscription. Application
code does not also install an `on_geometry_changed` handler for the hook.

## Coordinate restrictions

`world_bound` is panel-relative, not an operating-system screen rectangle. It is
valid for comparing elements only when their `document_id` values match.

```rust
anchor.assert_same_panel(&tooltip);
let dx = tooltip.world_bound.x - anchor.world_bound.x;
let dy = tooltip.world_bound.y - anchor.world_bound.y;
```

Helpers that compare or convert two `Geometry` values panic when their documents
differ. Silently combining coordinates from overlay, camera, render-texture, or
world-space panels would produce plausible but incorrect placement.

```rust
impl Geometry {
    pub fn assert_same_panel(&self, other: &Geometry);
}
```

Portals may cross panels for ordinary rendering and events. Core floating
placement does not. An application that needs physical-screen conversion must
provide its own camera and panel transform policy outside Reactant.

## Reconnect behavior

Reconnect creates new Unity `VisualElement` instances even when Reactant
preserves host `ObjectId` values. `begin_session` detaches every element ref and
clears all geometry.

```rust
let text = use_geometry(&self.anchor)
    .map_or("Measuring", |_| "Measured");
Some(Label::new(text).name("geometry-status"))
```

A reconnect test observes that state through fake Unity.

```rust
let response = reactant.begin_session(&game).into_response(snapshot);
world.apply(response);
assert_eq!(world.text("geometry-status"), "Measuring");
```

Refs attach to the new committed session tree after snapshot creation. Native
subscriptions produce fresh initial observations. Geometry received while the
ref is detached or carrying an older observation ID is ignored.

Reconnect does not remount logical components, reset their hooks, or rerun
ordinary effects solely because attachment changed.

## Floating UI

**Floating UI** is content positioned from the measured bounds of an anchor and
the floating element itself. Reactant supplies refs, geometry, portals, and
styles; it does not impose one universal collision or placement policy.

The common tooltip uses three measured elements:

- the anchor button;
- the overlay container defining the available panel rectangle; and
- the tooltip, whose size depends on its content.

The caller owns the anchor ref and same-panel overlay target. These consecutive
statements show the complete non-interactive open and close behavior.

```rust
let (open, set_open) = use_state(false);
let close = set_open.clone();
let anchor = use_element_ref();
let button = Button::new("Keyword")
    .element_ref(anchor.clone())
    .on_pointer_enter(move |_game: &mut Game| set_open.set(true))
    .on_pointer_leave(move |_game: &mut Game| close.set(false));
```

```rust
let key = TooltipKey { anchor: self.card_id, text: self.text.clone() };
let tooltip = open.then(|| Tooltip::new(
    anchor, self.overlay_ref.clone(), self.overlay.clone(), self.text.clone(),
).key(key));
Some(Fragment::new((button, tooltip)))
```

The overlay host attaches both a portal target and a ref.

```rust
VisualElement::new()
    .portal_target(overlay.clone())
    .element_ref(overlay_ref.clone())
```

## First measurement pass

On first render, the tooltip's own geometry is unknown. It must enter Unity's
layout tree without becoming visible.

```rust
let tooltip_ref = use_element_ref();
let tooltip_geometry = use_geometry(&tooltip_ref);
```

The first pass uses absolute positioning and `Visibility::Hidden`.

```rust
Label::new(self.text.clone())
    .style(Style::new()
        .position(Position::Absolute)
        .visibility(Visibility::Hidden))
    .element_ref(tooltip_ref.clone())
```

Unity `visibility: hidden` suppresses drawing while retaining layout. Using
`Display::None` would prevent layout and leave the tooltip without a useful
size. Opacity zero is also wrong because it may remain pickable or participate
in rendering work.

The hidden tooltip is portaled into the overlay so its measured size uses the
same panel, scale, styles, and available layout as its final visible form.

```rust
create_portal(hidden_tooltip, self.overlay.clone())
```

## Placement calculation

After Unity reports all three geometries, the application chooses a position.
The following helpers are complete application code, not Reactant API. The
panel point retains its document identity for later conversion.

```rust
#[derive(Clone, Copy, PartialEq)]
struct Point { x: f64, y: f64 }

impl Point {
    const fn new(x: f64, y: f64) -> Self { Self { x, y } }
}
```

```rust
#[derive(Clone, Copy, PartialEq)]
struct PanelPoint {
    value: Point,
    document_id: ObjectId,
}
```

Horizontal placement centers and clamps. Oversized content starts at the panel
origin because `max_left` cannot be smaller than `min_x`.

```rust
fn tooltip_left(anchor: &Rect, tooltip: &Rect, panel: &Rect) -> f64 {
    let centered = anchor.x + anchor.width / 2.0 - tooltip.width / 2.0;
    let max_left = (panel.x + panel.width - tooltip.width).max(panel.x);
    centered.clamp(panel.x, max_left)
}
```

Vertical placement prefers above, falls below when necessary, and then clamps.

```rust
fn tooltip_top(anchor: &Rect, tooltip: &Rect, panel: &Rect) -> f64 {
    let above = anchor.y - 8.0 - tooltip.height;
    let below = anchor.y + anchor.height + 8.0;
    let preferred = (above >= panel.y).then_some(above).unwrap_or(below);
    let max_top = (panel.y + panel.height - tooltip.height).max(panel.y);
    preferred.clamp(panel.y, max_top)
}
```

`place_tooltip` returns `None` until all three observations exist. It validates
the panel before comparing rectangles.

```rust
type GeometryDeps = (Option<Geometry>, Option<Geometry>, Option<Geometry>);
```

```rust
fn place_tooltip((anchor, tip, panel): GeometryDeps) -> Option<PanelPoint> {
    let (anchor, tip, panel) = (anchor?, tip?, panel?);
    anchor.assert_same_panel(&tip);
    anchor.assert_same_panel(&panel);
    let bounds = (&anchor.world_bound, &tip.world_bound, &panel.world_bound);
    let x = tooltip_left(bounds.0, bounds.1, bounds.2);
    let y = tooltip_top(bounds.0, bounds.1, bounds.2);
    Some(PanelPoint {
        value: Point::new(x, y), document_id: anchor.document_id,
    })
}
```

## Visible pass

The desired point is panel-space, while Unity's absolute `left` and `top` are
parent-local. Subtracting the overlay's `world_bound` origin is incorrect when
the parent is scaled or rotated. Application helper `to_parent_offsets`
converts the desired movement through the inverse affine transform.

```rust
fn to_parent_offsets(point: &PanelPoint, tip: &Geometry) -> Point {
    assert_eq!(point.document_id, tip.document_id);
    let origin = Point::new(tip.world_bound.x, tip.world_bound.y);
    let panel_delta = Point::new(
        point.value.x - origin.x, point.value.y - origin.y);
    let local_delta = tip.panel_from_parent.inverse_vector(panel_delta);
    Point::new(tip.layout.x + local_delta.x, tip.layout.y + local_delta.y)
}
```

The helper verifies matching `document_id` values and panics if inversion is not
possible. It returns the offsets applied by the visible pass.

`tooltip_style` combines conversion with the hidden initial state.

```rust
struct TooltipStyle {
    visibility: Visibility,
    left: f64,
    top: f64,
}
```

```rust
fn tooltip_style(point: Option<&PanelPoint>, tip: Option<&Geometry>)
    -> TooltipStyle
{
    point.zip(tip)
        .map(|(point, tip)| TooltipStyle::visible(
            to_parent_offsets(point, tip)))
        .unwrap_or_else(TooltipStyle::hidden)
}
```

The application constructors are ordinary value conversions.

```rust
impl TooltipStyle {
    fn hidden() -> Self {
        Self { visibility: Visibility::Hidden, left: 0.0, top: 0.0 }
    }
    fn visible(point: Point) -> Self {
        Self { visibility: Visibility::Visible, left: point.x, top: point.y }
    }
}
```

The final primitive style uses Unity `f32` lengths. Geometry originated as
finite Unity floats, but the conversion still validates range explicitly.

```rust
fn ui_px(value: f64) -> f32 {
    assert!(value.is_finite());
    assert!(value >= f32::MIN as f64);
    assert!(value <= f32::MAX as f64);
    value as f32
}
```

The hidden and visible passes use the same host beneath the keyed `Tooltip`
component, so Reactant emits property changes rather than destroying and
recreating it.

## Complete tooltip render expression

The complete component uses ordinary Reactant APIs plus the application helpers
`place_tooltip` and `tooltip_style`. The application keys the component with
the full anchor identity and text, rather than a hash that could collide.

```rust
#[derive(Clone, Eq, Hash, PartialEq)]
struct TooltipKey {
    anchor: CardId,
    text: String,
}
```

Changing either field remounts the component, creates a fresh element-ref hook,
and prevents the next render from seeing geometry for the old content.

```rust
let key = TooltipKey { anchor: self.card_id, text: text.clone() };
Tooltip::new(anchor, overlay_ref, overlay, text).key(key)
```

The tooltip itself needs only refs, the portal target, and visible content.

```rust
pub struct Tooltip {
    anchor_ref: ElementRef,
    overlay_ref: ElementRef,
    overlay: PortalTarget,
    text: String,
}
```

```rust
impl Tooltip {
    pub fn new(anchor_ref: ElementRef, overlay_ref: ElementRef,
        overlay: PortalTarget, text: String) -> Self
    {
        Self { anchor_ref, overlay_ref, overlay, text }
    }
}
```

The render method creates its element ref and observes it. Component remounting
creates a new ref identity, so the first value is necessarily `None`.

```rust
let tooltip_ref = use_element_ref();
let tooltip = use_geometry(&tooltip_ref);
```

It observes the other two refs and memoizes an optional panel-space target. The
closure form is required because `use_memo` invokes a zero-argument function.

```rust
let anchor = use_geometry(&self.anchor_ref);
let panel = use_geometry(&self.overlay_ref);
let deps = (anchor.clone(), tooltip.clone(), panel.clone());
let point = use_memo(deps.clone(), move || place_tooltip(deps));
```

`tooltip_style` is application code. It returns hidden zero offsets unless the
current tooltip geometry and point are both present; otherwise it calls
`to_parent_offsets` and returns visible offsets.

```rust
let placement = tooltip_style(point.as_ref(), tooltip.as_ref());
let host_style = Style::new()
    .position(Position::Absolute)
    .visibility(placement.visibility)
    .left(ui_px(placement.left).px())
    .top(ui_px(placement.top).px());
let box_view = Label::new(self.text.clone()).style(host_style)
    .picking_mode(PickingMode::Ignore)
    .element_ref(tooltip_ref);
```

The final render value portals that retained host into the selected overlay.

```rust
Some(create_portal(box_view, self.overlay.clone()))
```

These snippets are consecutive statements from `Tooltip::render`; the last
expression is its return value. The caller's component key and the fresh
attachment ensure a placement from previous content is never shown for new
content.

## Responding to later layout changes

Anchor movement, tooltip content changes, overlay resizing, panel scaling, and
ancestor transforms produce new geometry. The same placement calculation runs
again and emits only changed offsets.

```rust
let deps = (anchor.clone(), tooltip.clone(), panel.clone());
let point = use_memo(deps.clone(), move || place_tooltip(deps));
```

Moving the visible tooltip can itself cause a geometry event. Equal geometry is
ignored. A placement update that changes only position may produce one final
event; its recalculated offsets are equal, so the loop stops.

If content changes the tooltip's size, the full `TooltipKey` changes. Reactant
remounts the component and its host, so the new host remains hidden until its
own measurement arrives.

## Pointer behavior

A non-interactive tooltip uses `PickingMode::Ignore` so it cannot steal hover
from its anchor.

```rust
Label::new(self.text.clone())
    .picking_mode(PickingMode::Ignore)
```

Interactive popovers are different. Their open state should close from logical
outside-click or focus handling rather than relying on anchor hover alone.
Portaled events still bubble through the popover's logical component ancestry.

## Manual QA

1. Attach an element ref, move its keyed host, and then unmount it. Confirm the
   ref remains attached across the move and clears on unmount.
2. Observe a host whose layout is already stable. Confirm Unity sends one
   initial geometry containing parent-relative layout and panel-space
   `worldBound` and parent-to-panel affine transform.
3. Reconnect, deliver an old observation ID, and confirm geometry stays empty
   until the new session's initial observation arrives.
4. Open a tooltip near the panel center. Confirm the first host is hidden but
   measured, then becomes visible above the anchor without changing host ID.
5. Open the tooltip near every panel edge and with oversized content. Then move
   only an ancestor transform and scroll the overlay. Confirm two-axis clamping,
   transformed-parent conversion, resampling, and visible repositioning.
6. Portal an anchor and tooltip into different documents. Confirm the comparison
   is rejected instead of producing a cross-panel position.
