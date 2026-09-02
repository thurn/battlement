# Reactant Refs and Geometry

This appendix defines stable references to committed Unity elements, queued
host actions, and coherent asynchronous measurement across UI panels, displays,
cameras, and world objects. It is part of the
[Battlement Reactant technical design](reactant-technical-design.md). Reactant
does not expose mutable Unity objects or claim React's synchronous
`useLayoutEffect` timing.

## Related information

- [Battlement technical design](../technical-design.md) defines world objects,
  cameras, displays, actions, and the core observation protocol.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  UI hosts, panel modes, events, transient actions, and viewport mapping.
- [Host façades](host-facades.md) defines the order-independent `.element_ref`
  host builder.
- [Hooks and effects](hooks-and-effects.md) defines positional hook rules,
  geometry effects, and why `use_layout_effect` is reserved.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines commit ordering and logical event dispatch.
- [Unity `GeometryChangedEvent`][unity-geometry] describes UI Toolkit layout
  notification.

[unity-geometry]: https://docs.unity3d.com/ScriptReference/UIElements.GeometryChangedEvent.html

## Reference kinds

`use_ref` stores arbitrary Rust data. Reactant supplies three native-target
handles for geometry and imperative operations.

- An **element ref** identifies one committed Reactant UI host.
- A **world ref** identifies one explicit projection of a Battlement world
  object through a selected camera.
- A **viewport ref** identifies one display viewport and its safe area.

`ElementRef` is a cloneable, single-threaded identity owned by one Reactant
runtime. `WorldRef` and `ViewportRef` are immutable `Clone + Eq + Hash` target
values and may be reconstructed or shared. Equal values in consecutive renders
retain one observation; equal values used by two runtimes are independent.

```rust
pub fn use_element_ref() -> ElementRef;
```

The `.element_ref` façade method attaches an element ref to one host without
introducing another Unity element. It may appear before or after any other
valid host method.

```rust
let field_ref = use_element_ref();

TextField::new()
    .value(self.name.clone())
    .element_ref(field_ref.clone())
```

A component, fragment, portal, `Suspense`, or `ErrorBoundary` cannot receive an
element ref because it has no host object. Attaching clones of one element ref
to two hosts panics before commit.

World refs make the requested native meaning explicit. Reactant never guesses
whether an object should be represented by its root, an authored anchor, or its
visible renderers.

```rust
impl WorldRef {
    pub fn origin(object_id: ObjectId, camera: CameraTarget) -> Self;
    pub fn named_anchor(
        object_id: ObjectId,
        anchor: impl Into<AnchorName>,
        camera: CameraTarget,
    ) -> Self;
    pub fn rendered_bounds(
        object_id: ObjectId,
        camera: CameraTarget,
    ) -> Self;
}
```

World-ref identity is the complete constructor value: mode, object ID, authored
anchor name when present, and camera target. There is no rebind method. Changing
any field constructs a different value and waits for a sample of that target;
reconstructing an equal value does not churn the native registry.

`CameraTarget::Input` uses Battlement's selected input camera.
`CameraTarget::Object(id)` uses the enabled root camera component on that
object. A camera target is part of world-ref identity; changing it uses another
observation.

`WorldRef::origin` projects the root transform position.
`WorldRef::named_anchor` projects a child carrying one matching authored
`BattlementGeometryAnchor`. Prepared-asset validation rejects duplicate anchor
names. A missing anchor on an otherwise live object is a host contract failure,
not an empty measurement. `WorldRef::rendered_bounds` projects the bounds of
enabled renderers beneath the object. Choosing that mode explicitly accepts
that renderer visibility and effects may change the result.

```rust
impl ViewportRef {
    pub fn display(display: DisplayId) -> Self;
}
```

Viewport-ref identity is its `DisplayId`. Reconstructing
`ViewportRef::display(0)` on every render retains the same observation.

The runner reports display `0` even on a single-display build. A display that
becomes unavailable produces an unavailable observation while preserving its
last sample.

## Element attachment

An attached element ref stores private runtime, document, host, and generation
identities.

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
unmounting it, or successfully converting a new session detaches the old
generation.

`ElementRef::is_attached` reports committed Rust state without querying Unity.
It is intended for callbacks and diagnostics, not as render input. Calling it
during rendering panics. Attachment does not itself schedule another render;
use `use_geometry` when rendered output depends on native availability.

```rust
impl ElementRef {
    pub fn is_attached(&self) -> bool;
}
```

## Host actions

Element refs expose the one-shot operations supported by Battlement's
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

An action may be requested from an event handler, passive or geometry effect,
or ordinary engine-thread application code. Calling one during render or from
another runtime panics. A request made outside a Reactant callback waits for the
next active runtime entry.

Calling an action on a detached ref is a no-op. `scroll_to` is also a no-op when
either ref is detached. Two attached refs from different runtimes panic;
Battlement validates that an attached scroll target is a descendant of the
scroll view.

Each call snapshots the exact attachment generation of every participating
ref. Reactant emits the action only if those generations still identify the
same hosts after reconciliation. An unmount, remount, or ref replacement turns
the queued action into a no-op; it never follows a ref to another host.

`begin_session` freezes queued host actions without acknowledging them. An
explicit render error retains them against the still-active attachment. A
successful reconnect detaches that generation, so conversion acknowledges the
frozen actions as stale no-ops; they never run on the recreated native host.

Actions follow all host mutations produced by the same entry. Calls retain
invocation order as one sequential command group per action.

```rust
let field = field_ref.clone();

TextField::new()
    .value(self.name.clone())
    .on_input_event_with_model(move |game: &mut Game, event| {
        let value = normalize_name(&event.payload().value);
        let end = value.encode_utf16().count() as u32;
        game.name = value;
        field.select_text(end, end);
    })
    .element_ref(field_ref)
```

The controlled value patch precedes `SelectText`, so Unity cannot move the
caret after Reactant restores it.

## Coordinate spaces and values

All public projected geometry uses a **viewport coordinate space** whose origin
is the upper-left of one physical display, with positive x to the right and
positive y downward. UI panels and cameras on the same display therefore share
one comparison space even when their panel-local coordinates differ.

```rust
pub struct ViewportPoint {
    pub x: f64,
    pub y: f64,
    pub display_id: DisplayId,
}

pub struct ViewportRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub display_id: DisplayId,
}
```

Values are finite `f64`. Cross-display conversion returns `None`; it never
silently combines unrelated coordinates.

UI geometry retains the local transforms needed for positioning descendants.

```rust
pub struct ElementGeometry {
    pub layout: Rect,
    pub viewport_bound: ViewportRect,
    pub viewport_from_local: Projective2,
    pub viewport_from_parent: Projective2,
    pub panel_id: PanelId,
}
```

`layout` is parent-relative. The element-local rectangle is
`Rect::new(0, 0, layout.width, layout.height)`. Transforming `layout.x` and
`layout.y` through `viewport_from_local` would apply the parent translation
twice and is invalid.

`Projective2` maps local or parent-local points into viewport coordinates. It is
a finite invertible homography, so it represents affine screen panels and
perspective world-space panels. Mapping divides by the homogeneous coordinate;
a point at zero divisor returns `None`.

Viewport geometry includes the complete display and current safe area.

```rust
pub struct ViewportGeometry {
    pub viewport: ViewportRect,
    pub safe_area: ViewportRect,
    pub scale: f64,
    pub dpi: Option<f64>,
    pub orientation: DisplayOrientation,
}
```

World measurements preserve projection facts needed to reject unusable
animation endpoints.

```rust
pub struct WorldPointGeometry {
    pub point: ViewportPoint,
    pub depth: f64,
    pub is_inside_viewport: bool,
}

pub struct WorldBoundsGeometry {
    pub bound: ViewportRect,
    pub nearest_depth: f64,
    pub farthest_depth: f64,
    pub is_inside_viewport: bool,
}

pub enum WorldGeometry {
    Point(WorldPointGeometry),
    Bounds(WorldBoundsGeometry),
}
```

World points behind the camera are unavailable. Rendered bounds intersecting
the near plane are clipped before projection. Bounds are not clamped to the
camera viewport, allowing offscreen placement policies to select a direction.

## Measurement snapshots

A **measurement** separates the latest usable value from the host's current
knowledge about that target.

```rust
pub struct Measurement<T> {
    pub latest: Option<T>,
    pub status: MeasurementStatus,
}

pub enum MeasurementStatus {
    Waiting,
    Current,
    Unavailable(GeometryUnavailable),
}

pub enum GeometryUnavailable {
    Detached,
    Hidden,
    ObjectMissing,
    CameraDisabled,
    DisplayUnavailable,
    NoRenderers,
    BehindCamera,
    NoViewportMapping,
    ProjectionUnavailable,
}
```

`Waiting` means Reactant has no sample for the current observation epoch or
knows a newer native sample is required. `latest` may still contain the last
completed value. `Unavailable` uses this exact precedence:

| Target | Ordered outcome checks |
|---|---|
| UI element | detached host → `Detached`; hidden from layout → `Hidden`; missing display → `DisplayUnavailable`; target-texture panel → `NoViewportMapping`; missing or disabled world-panel camera → `CameraDisabled`; all required corners behind the near plane → `BehindCamera`; finite singular or horizon-crossing projection → `ProjectionUnavailable` |
| Viewport | missing display → `DisplayUnavailable` |
| World origin or anchor | missing measured object → `ObjectMissing`; missing or disabled camera → `CameraDisabled`; point behind the near plane → `BehindCamera`; finite projection degeneracy → `ProjectionUnavailable` |
| World rendered bounds | missing measured object → `ObjectMissing`; missing or disabled camera → `CameraDisabled`; no qualifying renderers → `NoRenderers`; bounds entirely behind the near plane → `BehindCamera`; finite projection degeneracy → `ProjectionUnavailable` |

The first matching check wins and an unavailable result may retain a last
value. Nonfinite transform data, duplicate observation identity, a missing
authored anchor on a live object, or the wrong object kind is a host contract
failure instead. A finite singular homography or zero homogeneous divisor is an
ordinary `ProjectionUnavailable`, not a malformed-transform failure.

```rust
pub struct GeometrySnapshot<T> {
    pub generation: Option<GeometryGeneration>,
    pub measurements: T,
}
```

All measurements in one `GeometrySnapshot` describe one completed native
sampling pass. `generation` is absent only before any pass has covered the
complete committed target set. It is opaque, monotonically increasing within a
session, and has no application meaning beyond ordering and coherence.

The latest value is preserved while a newer sample is waiting or a target is
temporarily unavailable. This avoids flashes during resize and animation.
Content that must never show an initial or known-stale placement checks
`MeasurementStatus::Current` and uses `visibility: hidden`; `display: none`
would prevent useful UI layout.

## Reactive target sets

Geometry consumers retain their runtime context when native motion callbacks
trigger a render. They read the latest coherent snapshot, just as they do after
UI events or application-state refreshes.

`use_geometry` consumes one positional hook slot regardless of target count.

```rust
pub fn use_geometry<T>(targets: T) -> GeometrySnapshot<T::Measurements>
where T: GeometryTargets;

pub trait GeometryTargets: private::Sealed {
    type Measurements;
}
```

`GeometryTargets` is sealed. Reactant implements it for one native ref,
heterogeneous tuples containing one through twelve target sets, arrays, and
`Vec<T>`. Tuple output preserves tuple shape; vector output preserves input
order. A vector may grow, shrink, or reorder without changing hook count.
Duplicate refs share one native observation and fan its sample into every
output position.

```rust
let geometry = use_geometry((
    self.source.clone(),
    self.destination.clone(),
    self.viewport.clone(),
));
```

Changing the target set commits one registry diff. Removed targets stop
observation after the same commit; added targets report `Waiting` until the
next native generation covers the complete set.

`ElementRef` also exposes a nonreactive read of its last individual
measurement. It does not install native observation and returns `Waiting`
until a committed geometry hook or geometry effect has observed that ref.

```rust
impl ElementRef {
    pub fn geometry(&self) -> Measurement<ElementGeometry>;
}
```

The cache belongs to the attached host identity, which makes this convenience
safe for an element ref. `WorldRef` and `ViewportRef` are freely reconstructible
target values and deliberately have no `geometry` method: application code
captures a `GeometrySnapshot` when a callback needs their last coherent values.
Capturing the snapshot is also preferred whenever several endpoints must remain
from the same generation.

`ElementRef::geometry` is a callback and diagnostic convenience, not reactive
render input. Calling it during rendering panics. Components whose output
depends on measurement call `use_geometry` and render from that hook's coherent
snapshot.

## Observation protocol

All types in this section are the canonical `battlement` protocol types defined
by the core technical design. Reactant imports them; the repeated signatures are
usage documentation, not parallel wire declarations.

Reactant allocates one opaque `GeometryObservationId` for each distinct target
in a committed target set. The ID contains a runtime-unique observation epoch;
it is never derived only from `ObjectId`. Reattaching an element, changing a
world-ref value, or reconnecting allocates another epoch.

Reactant updates the core host registry with one command.

```rust
pub struct GeometryObservationUpdate {
    pub added: Vec<GeometryObservation>,
    pub removed: Vec<GeometryObservationId>,
}

pub struct GeometryObservation {
    pub observation_id: GeometryObservationId,
    pub target: GeometryObservationTarget,
}

pub enum GeometryObservationTarget {
    UiElement { object_id: ObjectId },
    Viewport { display_id: DisplayId },
    WorldOrigin { object_id: ObjectId, camera: CameraTarget },
    WorldAnchor {
        object_id: ObjectId,
        anchor: AnchorName,
        camera: CameraTarget,
    },
    WorldRenderedBounds { object_id: ObjectId, camera: CameraTarget },
}
```

`GeometryObservationTarget` has exact variants for an element, viewport, world
origin, named anchor, and rendered bounds. World variants include their camera
target. Observation registration for a newly created element follows its host
create command. Removal precedes destruction when both occur in one commit.
After a reconnect snapshot, Reactant re-adds the complete live registry.

Unity samples every active target once at the end of a frame after UI Toolkit
layout, camera updates, transforms, and animations have settled for that frame.
Native geometry events, scrolling, display changes, safe-area changes, panel
scale changes, camera changes, and world transforms all request the same pass.

```rust
pub struct GeometryObservationBatch {
    pub generation: GeometryGeneration,
    pub changed: Vec<GeometryObservationValue>,
}

pub struct GeometryObservationValue {
    pub observation_id: GeometryObservationId,
    pub result: GeometryObservationResult,
}

pub enum GeometryObservationResult {
    Current(GeometryValue),
    Unavailable(GeometryUnavailable),
}

pub enum GeometryValue {
    Element(ElementGeometry),
    Viewport(ViewportGeometry),
    WorldPoint(WorldPointGeometry),
    WorldBounds(WorldBoundsGeometry),
}
```

The batch contains initial, changed, and newly unavailable values. Omission
means that the active observation was sampled in this generation and remained
equal. Reactant therefore advances unchanged cached observations to the same
generation before rendering. If every active value is unchanged, Unity sends
no batch because the preceding snapshot remains coherent.

The batch is one core `ActionBody::GeometryObservations` value and is never
split. Duplicate live IDs, an unknown value kind, or invalid numeric data close
the transport session before any partial batch is submitted. Observations for
retired epochs are stale input and are ignored by Rust.

## Frame integration

The runner retains at most one pending geometry batch. If several native frames
finish before the next engine exchange, it keeps the latest value for each
observation and the newest generation. Values that return to the last submitted
state are removed from the pending change set.

On the next ordinary engine frame, the runner submits that batch instead of an
empty poll. Immediate pointer, keyboard, and other user events remain separate
calls. Geometry does not add a second synchronous Rust round trip.

The engine routes `ActionBody::GeometryObservations` to
`Reactant::observe_geometry` rather than logical event propagation.

```rust
let commit = reactant.observe_geometry(&mut game, batch)?;
response.append_reactant(commit)
```

Reactant validates the complete batch, installs all accepted values atomically,
runs committed geometry effects, and renders dirty consumers once. A render
cannot observe a partly installed generation.

## Geometry effects

`use_geometry_effect` is the asynchronous replacement for measurement-driven
imperative layout and world choreography.

```rust
pub fn use_geometry_effect<G, T, D, S, C>(
    setup: S,
    targets: T,
    dependencies: D,
)
where
    G: 'static,
    T: GeometryTargets,
    D: Dependencies,
    S: FnOnce(&mut G, GeometrySnapshot<T::Measurements>) -> C + 'static,
    C: IntoGeometryEffectCleanup<G>;
```

The hook consumes one slot. Its committed setup runs when the complete target
set first has a generation, whenever its measurement values or statuses change,
or when dependencies change. If dependencies change while the cached snapshot
is coherent, setup runs at the next non-session active Reactant entry without
waiting for an unrelated native change.

Setup may return `()` or one cleanup closure accepting `&mut G`. Cleanup runs
before replacement and on unmount. Geometry effects use child-before-parent
ordering and the same panic-poisoning rule as passive effects. State setters
join the entry's single refresh. Domain commands queued in `G` remain
application-owned and require the explicit batch composition below.

```rust
pub trait IntoGeometryEffectCleanup<G>: private::Sealed {
    fn into_cleanup(self)
        -> Option<Box<dyn FnOnce(&mut G) + 'static>>;
}
```

The crate implements the sealed trait for `()` and every
`FnOnce(&mut G) + 'static`.

```rust
use_geometry_effect(
    move |game: &mut Game, geometry| {
        if let Some((source, destination)) = reward_endpoints(geometry) {
            game.queue_reward_flight(reward, source, destination);
        }
    },
    (self.source.clone(), self.destination.clone()),
    self.sequence_id,
);
```

The callback is never invoked during render or before its targets have one
coherent generation. Product-specific placement, collision, sequencing, and
animation policy remain application code.

When a geometry effect both changes Reactant output and queues world commands,
the application consumes `ReactantCommit::into_groups` and creates one
Battlement batch with the required order. For a reward flight that must begin
after its source hides, append the Reactant groups first and the world-animation
group last. If the world command must capture the pre-mutation presentation,
place its group first. `append_reactant` intentionally cannot guess this policy.
The application must not emit the two families as unrelated `Now` batches when
their order is visible.

## Coordinate conversion

These helpers operate only when their inputs share a display.

```rust
impl ElementGeometry {
    pub fn bounds_in(&self, coordinate_space: &ElementGeometry)
        -> Option<Rect>;
    pub fn viewport_point_to_local(
        &self, point: ViewportPoint,
    ) -> Option<Point>;
    pub fn viewport_point_to_parent(
        &self, point: ViewportPoint,
    ) -> Option<Point>;
    pub fn local_point_to_viewport(
        &self, point: Point,
    ) -> Option<ViewportPoint>;
}
```

`bounds_in` transforms the four corners of
`Rect::new(0, 0, layout.width, layout.height)` through
`viewport_from_local` and the inverse of the destination transform. It does not
transform an already axis-aligned `viewport_bound`, which would lose rotation
information.

`viewport_point_to_parent` is the common absolute-positioning conversion. It
applies the inverse of `viewport_from_parent`, producing the `left` and `top`
coordinates expected by an absolutely positioned child. Conversion also
returns `None` when a projective mapping reaches its horizon.

## Application patterns

Cumulus overlays observe the floating panel, anchor, and viewport in one target
set. They may retain their last placement while resizing and hide only before
their first current sample.

Tutorial, reward, and battle sequences observe vectors of card, slot, portrait,
lane, and destination refs. One geometry effect receives coherent endpoints and
queues the resulting world commands in `G`.

Journey HUD clearances observe the deck, Dreamsign strip, battle board, and
viewport. The component publishes the latest current bounds through normal
rendered state. Guide dialogue uses an explicit UI child ref or named world
anchor representing the visible character; Reactant does not scan texture alpha
or infer meaningful artwork bounds.

Loading callouts observe illustrated features and annotation containers.
Cumulus controls use measured available dimensions for text fitting and panel
scaling. Such feedback calculations must converge; Reactant records consecutive
geometry-driven commits and emits a diagnostic after 120 generations without a
user event, resource completion, store change, or explicit refresh. It does not
panic because continuous world animation may legitimately update every frame.

Editors observe field anchors for tooltips and use `select_text` to restore a
known UTF-16 caret and selection after a controlled value patch. Geometry does
not expose glyph or caret rectangles.

## Reconnect behavior

`begin_session` prospectively detaches element refs, retires every observation
epoch, and marks native-derived values `Waiting` while retaining their last
samples. These changes become committed only when the `SessionUi` converts
successfully. An explicit render error leaves attachments, epochs, values, and
the old session unchanged. The new session snapshot recreates native hosts,
then its post-snapshot commit adds the complete observation registry.

A successful reconnect queues cleanup for every geometry-effect setup tied to
the retired coherent snapshot. The next non-session active entry runs that
cleanup even when no new geometry is available. A later complete generation
runs a fresh setup; when that generation arrives on the same entry, cleanup
immediately precedes replacement setup. An abandoned reconnect queues neither.

Logical components, hook state, world refs, viewport refs, and ordinary effects
remain mounted. Old observation IDs are ignored. Geometry becomes current only
after the new session reports a complete generation.

## Performance and diagnostics

The native registry is empty when no geometry hook or effect is committed. Each
active target is sampled at most once per generation, duplicate refs share one
sample, and unchanged values produce no payload. Rust dirties only consumers of
changed values or statuses.

Unity records sampling and batch serialization time by target kind. Rust records
batch validation, installation, geometry effects, and resulting render time.
Slow-frame diagnostics include generation, active and changed counts, payload
bytes, display and camera identities, and consecutive geometry-driven commit
count.

## Manual QA

1. Observe one UI element, a changing vector of world cards, and a viewport in
   one hook. Add, remove, and reorder cards and confirm one hook slot and one
   coherent generation.
2. Resize the display, change its safe area, scroll and transform UI ancestors,
   and animate a world card. Confirm changed values arrive atomically while
   unchanged targets advance to the same generation without duplicated native
   samples.
3. Compare UI bounds in two panels on one display and project a world anchor
   through the selected camera. Confirm all values share viewport coordinates
   and cross-display conversion returns `None`.
4. Hide an observed Suspense primary, destroy a world object, disable its
   camera, and restore each target. Confirm status changes preserve the last
   sample and never submit invalid transforms.
5. Queue a reward flight from a geometry effect. Confirm the callback receives
   `&mut G`, its state and domain commands join one response, cleanup precedes a
   replacement, and unmount cleanup runs child before parent.
6. Update a controlled text field and request `select_text` from an event and
   from ordinary engine code. Confirm the value patch precedes the action and a
   remounted ref does not receive an old queued action.
7. Reconnect and deliver observations from the retired epoch. Confirm they are
   ignored, last samples remain waiting, and only the new complete generation
   becomes current.
8. Run the geometry performance fixture with duplicate refs, stable elements,
   continuous world animation, and changing safe areas. Confirm one scheduled
   exchange per frame, one sample per distinct target, and complete diagnostics.
9. Call `ElementRef::is_attached` and `ElementRef::geometry` during render and
   confirm both panic. Call them from an event callback and confirm committed
   attachment and geometry are available there.
10. Queue a host action and keep one geometry-effect setup active, then fail a
    reconnect render. Confirm the old attachment, action, epoch, and effect stay
    intact. Convert a retry successfully; confirm the old action becomes a
    no-op, cleanup runs on the next non-session entry, and fresh geometry runs a
    replacement setup.
