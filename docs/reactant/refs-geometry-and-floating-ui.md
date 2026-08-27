# Reactant Refs and Geometry

This appendix defines stable references to committed Unity elements, queued
host actions, and coherent asynchronous geometry. It is part of the
[Battlement Reactant technical design](reactant-technical-design.md). Reactant
does not expose mutable Unity objects or claim React's synchronous
`useLayoutEffect` timing.

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  host elements, events, transient actions, and ordered command groups.
- [Components and rendering](component-authoring.md) defines the
  `.element_ref` render adapter.
- [Hooks and effects](hooks-and-effects.md) defines positional hook rules and
  explains why `use_layout_effect` is reserved.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines commit ordering and logical event dispatch.
- [Unity `GeometryChangedEvent`][unity-geometry] describes Unity's element-local
  layout notification.
- [Unity `VisualElement.worldBound`][unity-world-bound] describes transformed
  panel-space bounds.

[unity-geometry]: https://docs.unity3d.com/ScriptReference/UIElements.GeometryChangedEvent.html
[unity-world-bound]: https://docs.unity3d.com/ScriptReference/UIElements.VisualElement-worldBound.html

## Element refs

`use_ref` stores arbitrary Rust data. `use_element_ref` instead creates an
**element ref**, a stable handle whose attachment follows one committed host
element.

```rust
pub fn use_element_ref() -> ElementRef;
```

The hook consumes one positional slot and returns a cloneable handle. The
`.element_ref` adapter attaches it to a primitive without introducing another
Unity element.

```rust
let field_ref = use_element_ref();

TextField::new()
    .value(self.name.clone())
    .element_ref(field_ref.clone())
```

A component, fragment, portal, or Suspense boundary cannot receive an element
ref because it has no host object. One ref may attach to at most one committed
host. Attaching clones of the same ref to two hosts panics before commit.

Attachment stores private runtime, document, host, and generation identities.

```rust
ElementAttachment {
    runtime_id,
    document_id,
    object_id,
    generation,
}
```

The generation changes on every attachment. A reused or reordered host remains
attached without changing generation. Replacing the ref, remounting the host,
unmounting it, or beginning a new session detaches the old generation.

`ElementRef::is_attached` reports committed Rust state. It does not
synchronously query Unity.

```rust
impl ElementRef {
    pub fn is_attached(&self) -> bool;
}
```

## Host actions

Element refs expose the one-shot operations already supported by Battlement's
`VisualElementAction` protocol.

```rust
impl ElementRef {
    pub fn focus(&self);
    pub fn blur(&self);
    pub fn capture_pointer(&self, pointer_id: i32);
    pub fn release_pointer(&self, pointer_id: i32);
    pub fn scroll_to(&self, descendant: &ElementRef);
    pub fn select_text(&self, cursor_index: u32, selection_index: u32);
}
```

An action method may run only while Reactant is invoking an event handler or an
effect setup or cleanup. The current callback supplies the command batch and
runtime identity. Calling an action during render, outside a Reactant callback,
or from a callback owned by another runtime panics.

Calling an action on a detached ref is a no-op. `scroll_to` is also a no-op when
either ref is detached. Two attached refs from different runtimes panic;
Battlement validates that an attached scroll target is a descendant of the
scroll view.

Each call snapshots the exact attachment generation of every participating
ref. After reconciliation, Reactant emits the action only if those same
generations are still attached to the same hosts. An unmount, remount, or ref
replacement therefore turns the queued action into a no-op; it never follows a
ref to a new host or targets an object destroyed earlier in the response.

Actions follow all host mutations produced by the same callback batch. This
ordering keeps controlled inputs concise and deterministic.

Action calls retain callback invocation order. Reactant appends one sequential
command group per action after the reconciliation groups, even when two actions
target different elements. Host actions are uncommon, and preserving obvious
call order is more valuable than parallelizing them.

```rust
let field = field_ref.clone();

TextField::new()
    .value(self.name.clone())
    .element_ref(field_ref)
    .on_input_event(move |game: &mut Game, event| {
        let value = normalize_name(&event.payload().value);
        let end = value.encode_utf16().count() as u32;
        game.name = value;
        field.select_text(end, end);
    })
```

The value patch is emitted before `SelectText`, so Unity does not move the caret
after Reactant restores it.

## Geometry values

`use_geometry` observes one element ref and returns its most recent coherent
Unity measurement.

```rust
pub fn use_geometry(element: &ElementRef) -> Option<Geometry>;
```

`Geometry` contains both convenient bounds and the transforms needed for
general coordinate conversion.

```rust
pub struct Geometry {
    pub layout: Rect,
    pub world_bound: Rect,
    pub panel_from_local: Affine2,
    pub panel_from_parent: Affine2,
    pub panel_id: PanelId,
}
```

`Geometry` is `Clone + PartialEq`. `PanelId` and `GeometryObservationId` are
opaque `Copy + Eq + Hash` `u64` newtypes whose numeric values have no
application meaning. The Unity adapter assigns one `PanelId` to each native
panel instance for the session. Documents on the same panel share it.

`layout` is parent-relative. `world_bound` is the transformed axis-aligned bound
in panel coordinates. `panel_from_local` maps the element's local coordinates
to its panel, and `panel_from_parent` maps parent-local coordinates to the same
panel. `Rect` uses finite `f64` `x`, `y`, `width`, and `height`; its origin is
the upper-left, positive x points right, and positive y points down. Unity
`float` values widen directly to `f64`.

`Affine2` stores `m00`, `m01`, `m02`, `m10`, `m11`, and `m12` and maps a point
as `x' = m00*x + m01*y + m02` and
`y' = m10*x + m11*y + m12`. It supports point, vector, inverse, and
rectangle-bound transformations.

The Unity adapter rejects perspective, non-finite, or non-invertible transforms
instead of returning plausible but invalid coordinates. Geometry comparison is
exact over the finite values Unity reported.

## Observation protocol

Every attached element ref with at least one committed geometry hook owns one
opaque `GeometryObservationId` derived from its attachment generation and an
observation epoch. Several `use_geometry` hooks observing clones of the same ref
share that ID and the same cached sample; Reactant reference-counts the hooks
and fans an installed value out to each hook slot. Reactant writes the identity
into the host description when the first hook commits and resets it when the
last hook leaves. A later zero-to-one transition allocates a new epoch and ID,
forcing a new initial native sample. The protocol is separate from authored
`GeometryChanged` event subscriptions.

The common Battlement visual-element properties add one mutable field:

```rust
pub geometry_observation: Prop<GeometryObservationId>;
```

Reactant sets and resets this field from committed geometry hooks. The Unity
adapter uses it only to maintain the observation registry; it does not expose a
normal event callback.

Unity keeps a registry of active observation IDs. After a panel completes
layout, it samples every active target in that panel once. Native geometry
events, scrolling, panel-scale changes, and transform changes request this same
pass; they never submit individual measurements.

One adapter end-of-frame callback runs after UI Toolkit has completed panel
updates for that Unity frame. It samples every observed panel before serializing
anything and assigns one monotonically increasing layout generation to that
whole pass. The wire types belong to `battlement-ui`, allowing the common client
and `ActionBody` protocol to carry them without depending on Reactant.

```rust
pub struct UiGeometryObservationBatch {
    pub layout_generation: u64,
    pub observations: Vec<UiGeometryObservation>,
}

pub struct UiGeometryObservation {
    pub observation_id: GeometryObservationId,
    pub layout: Rect,
    pub world_bound: Rect,
    pub panel_from_local: Affine2,
    pub panel_from_parent: Affine2,
    pub panel_id: PanelId,
}
```

The adapter compares every sample with its previous value. The batch contains
only initial or changed observations, but omission means the other active
observations were sampled and remained equal in that generation. If nothing
changed, Unity creates no batch.

Reactant converts an accepted `UiGeometryObservation` into its public
`Geometry` snapshot. The batch is serialized as
`ActionBody::VisualElementGeometryBatch(UiGeometryObservationBatch)`. It is
never split. If it exceeds Battlement's maximum client-message size, the
adapter reports `ClientMessageTooLarge` and closes that transport session.
Duplicate observation IDs or an invalid transform are adapter invariant
failures and close the session before any part of the batch is submitted. If an
invalid batch nevertheless reaches Rust, `observe_geometry` panics before
changing hook state.

## Frame integration

The runner retains at most one pending geometry batch. If several Unity layouts
finish before the next engine frame, the adapter compares the latest sample
with the last submitted value, overwrites pending entries with their latest
values, and removes entries that returned to the submitted value. The pending
batch therefore describes the net change through its latest layout generation.

On the next ordinary engine frame, the runner submits that batch instead of
calling the transport's empty `poll`. If there is no batch, it polls normally.
Geometry observation therefore does not add a synchronous native or HTTP round
trip. Immediate pointer, keyboard, and other user-event submissions remain
separate calls and are not counted as the scheduled frame exchange.

The engine routes `VisualElementGeometryBatch` to
`Reactant::observe_geometry`, not to logical event propagation.

```rust
let ui = reactant.observe_geometry(&game, batch);
response.append_ui(ui)
```

`observe_geometry` performs the same lifecycle work as `Reactant::poll` after
installing the batch. Resource completions, external-store wakes, and passive
effects therefore do not lose a polling opportunity on a geometry frame.

Reactant validates the complete batch before changing hook state. It ignores
observations for detached or superseded attachment generations, rejects
duplicate live observation IDs, and ignores a batch generation no newer than
the last installed generation. It then installs every accepted changed value
atomically and renders dirty consumers. Separate `use_geometry` calls in that
render cannot observe a partly installed batch.

## Hook lifecycle and freshness

`use_geometry` returns `None` before attachment, while waiting for the first
sample, and after detach or reconnect. A hook newly pointed at an attachment
already observed by another hook immediately reads that attachment's cached
sample. Otherwise, changing the ref argument unsubscribes the old attachment
and returns `None` until the new observation epoch is sampled.

After the first value, the hook retains its last coherent geometry until a
replacement batch arrives. A Reactant commit or Unity layout invalidation does
not clear the value merely because a newer sample may be pending. This avoids a
one-frame disappearance during ordinary resize or movement, but the returned
value may describe the preceding completed Unity layout.

An application that must not display content before its first measurement puts
that content in layout with `visibility: hidden`. It must not use
`display: none`, which prevents useful layout.

```rust
let card_ref = use_element_ref();
let card_geometry = use_geometry(&card_ref);

CardView::new(self.card.clone())
    .style(Style::new().visibility(if card_geometry.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }))
    .element_ref(card_ref)
```

Reactant has no `use_layout_effect`. A synchronous pre-paint Rust transaction
would add at least one blocking transport call and could repeat when the
callback changes layout. The asynchronous hook name makes the one-frame
boundary explicit.

## Coordinate conversion

Coordinates may be compared only within one Unity panel. These helpers panic
when their operands have different `panel_id` values.

```rust
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub struct PanelPosition {
    pub point: Point,
    pub panel_id: PanelId,
}

impl Geometry {
    pub fn assert_same_panel(&self, other: &Geometry);
    pub fn bounds_in(&self, coordinate_space: &Geometry) -> Rect;
    pub fn world_center(&self) -> PanelPosition;
    pub fn panel_point_to_local(&self, point: PanelPosition) -> Point;
    pub fn panel_point_to_parent(&self, point: PanelPosition) -> Point;
    pub fn local_point_to_panel(&self, point: Point) -> PanelPosition;
}
```

`Point` and `PanelPosition` implement `Clone`, `Copy`, and `PartialEq` with
ordinary impl blocks; V1 uses no derive macro.

`bounds_in` transforms the four corners of the element's local layout rectangle
through `panel_from_local` and the inverse transform of `coordinate_space`, then
returns their axis-aligned bound in that element's local coordinates. It does
not transform the already axis-aligned `world_bound`, which would lose rotation
information.

`PanelPosition` carries native panel identity with a point.
`panel_point_to_parent` is the common absolute-positioning conversion. It
checks that identity, then applies the inverse of `panel_from_parent`, producing
the `left` and `top` coordinates expected by a host whose absolute positioning
is relative to that parent.

The helpers deliberately do not convert between panels, cameras, render
textures, or operating-system screen coordinates. Applications needing those
conversions must supply their own camera and panel policy.

## General usage

Several measurements can be read independently because Reactant installs one
whole generation before rendering.

```rust
let source = use_geometry(&self.source_ref);
let destination = use_geometry(&self.destination_ref);

let flight = source.zip(destination).map(|(source, destination)| {
    source.assert_same_panel(&destination);
    RewardFlight::new(
        source.world_center(),
        destination.world_center(),
    )
});
```

Anchored content uses the same primitives without a framework-specific floating
component. A reusable application component can own its floating ref, keep the
host hidden until both measurements exist, and calculate a placement policy
from `world_bound`.

```rust
let panel_point = match (&anchor, &panel, &floating) {
    (Some(anchor), Some(panel), Some(floating)) => {
        Some(place_above_or_below(anchor, floating, panel))
    }
    _ => None,
};

let offsets = panel_point
    .zip(floating.as_ref())
    .map(|(point, floating)| floating.panel_point_to_parent(point));
```

Tutorials, clearances, and battle animations can instead use `bounds_in` or
panel-space centers directly. Reactant supplies observation and coordinate
correctness, while ordinary components own product-specific placement and
collision policy.

## Reconnect behavior

`begin_session` detaches every element ref before serializing fresh documents
for the new Unity session. It clears all geometry values and discards queued
observation batches from the previous session. The fresh host descriptions
attach new generations when `SessionUi` conversion commits the Rust
transaction. This is response handoff, not a synchronous Unity acknowledgement.
The sequential engine rule forbids another Reactant call until Unity applies
that response, so callbacks cannot observe a Rust attachment before its native
host exists.

Logical components, hook state, and ordinary effects remain mounted. Each live
geometry hook returns `None` until Unity reports its new attachment generation.
An observation carrying an old ID is ignored even when its `ObjectId` was
reused.

## Performance and diagnostics

The Unity observation registry is empty when no geometry hook is committed.
Only active targets are sampled, each target is sampled at most once per layout
generation, and unchanged observations produce no payload. Rust dirties only
hooks whose values changed; it does not refresh unrelated roots solely because
a geometry batch arrived.

Unity records `Battlement.Geometry.Sample` around target reads and
`Battlement.Geometry.Batch` around comparison and serialization. Rust records
`Reactant.Geometry.Install` around validation and atomic installation. The slow
frame diagnostic includes the exchange kind, layout generation, active and
changed observer counts, geometry payload bytes, and time in each marker.

The performance fixture observes dozens of stable elements, repeatedly resizes
one container, scrolls an ancestor, and animates one transformed target. It
asserts one scheduled transport exchange per frame, one sample per active
target, no payload for stable generations, and one Reactant render for each
submitted generation. The fixture generates no user input while making the
exchange-count assertion.

## Manual QA

1. Attach one ref, reorder its keyed host, and unmount it. Confirm the
   attachment generation survives the move and clears on unmount.
2. Update a controlled text field and restore its selection from the same event.
   Confirm the value patch precedes `SelectText` in the fake command journal.
3. Observe several elements, then resize, scroll, and transform their ancestors.
   Confirm one atomic batch updates every changed hook and unchanged hooks keep
   their prior values.
4. Observe clones of one element ref from several hooks. Confirm the host has
   one observation ID, one sample fans out to every hook, and observation resets
   only after the last hook unmounts.
5. Change layout and delay the next batch. Confirm existing geometry remains
   available, while a newly attached ref stays `None` and hidden until measured.
6. Convert rotated and scaled element bounds into another element's coordinates.
   Confirm the transformed corners produce the expected local bound and a
   cross-panel conversion panics.
7. Reconnect and deliver an old observation ID. Confirm every hook remains
   `None` until its new attachment generation is reported.
8. Queue an action and unmount or remount either referenced host in the same
   callback. Confirm the action is omitted rather than following the new
   attachment generation.
9. Run the geometry performance fixture over native and HTTP transports.
   Confirm geometry uses only the scheduled frame exchange and the slow-frame
   diagnostics report observer count, payload size, and timing.
