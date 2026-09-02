# Reactant Layout and Stacking

Reactant needs a small layout surface for application interfaces that use more
than UI Toolkit's native flex layout. The immediate examples are settings tabs,
responsive form rows, sticky table headers, layered controls, anchored menus,
and modal dialogs. These patterns must remain ordinary Reactant trees: layout
must not replace reconciliation, logical events, focus, refs, animation,
portals, or sparse host updates.

This design adds explicit `Flex`, `Grid`, and `Stack` hosts. Unity owns their
frame-local measurement and placement. Reactant continues to own logical
identity, state, event ancestry, and desired properties. Sticky content and
overlays build on the same boundary instead of introducing a second renderer.

The surface deliberately does not add `Display::Grid`, `Position::Sticky`, or
an unrestricted `Style::z_index`. Those names would imply browser behavior that
UI Toolkit cannot preserve. The Reactant-specific types below define a smaller,
closed contract whose behavior can be tested exactly.

## Related information

- [Battlement Reactant technical design](reactant-technical-design.md) defines
  the runtime, desired tree, sparse updates, and session lifecycle extended
  here.
- The companion
  [implementation plan](layout-and-stacking-implementation-plan.md) divides
  this contract into independently valid delivery tasks.
- [Reactant host facades](host-facades.md) defines the one-facade-to-one-host
  lowering rule used by the new layout containers.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines logical ancestry and physical portal placement preserved by overlays.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines the committed
  element and viewport measurements used by responsive authoring.
- [Focus and navigation](focus-and-navigation.md) owns modal focus,
  containment, Tab order, and restoration.
- [Reactant animations](animations.md) defines decoration layers, Motion
  ownership, layout projection, presence, and scroll-local work composed with
  this system.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the Rust-to-Unity host protocol and validation boundary extended by the new
  element kinds and descriptors.
- The [settings mockup][settings-mockup] at commit
  `2451ea9cc6f76b356b1102ee37b82c478853122a` is reference evidence for grid,
  responsive, sticky, stacking, dropdown, and modal behavior. It is not a
  source of implementation instructions.
[settings-mockup]:
  https://github.com/thurn/mockups/tree/2451ea9cc6f76b356b1102ee37b82c478853122a

## Design requirements

The layout system exists to make common prototype structures direct to port,
not to implement a web browser. The following requirements determine the
public and native contracts.

- A layout container is one stable Reactant host with one Battlement
  `ObjectId`. Changing its tracks, gaps, or alignment does not remount it.
- Direct children retain their existing host IDs, native control state, hooks,
  handlers, refs, focus eligibility, and animation state when layout changes.
- Logical child order controls reconciliation, grid auto-placement, event
  ancestry, focus intent, and every equal-order tie.
- Unity resolves layout and scroll-dependent placement without a synchronous
  Rust round trip or per-frame Reactant rerender.
- Container and item descriptors are typed, finite, structurally comparable,
  serializable, resettable, and validated before commit.
- Layout never writes the child's authored `Style` position, size, margin, or
  transform fields as an implementation technique.
- Portals remain the only public operation by which an author selects a
  physical target different from logical ancestry. Layout may privately
  reparent a host for presentation without changing public or logical ownership.
  Overlay helpers compose portals rather than creating a competing mechanism.
- Existing decorations remain the preferred representation for noninteractive
  paint. A real child is used when a layer needs input, focus, refs, nested
  content, or independent accessibility.
- Invalid authored descriptors are developer failures. Transient native facts,
  such as an anchor waiting to attach, are represented as waiting states rather
  than authoring failures.

## Public authoring model

The public surface consists of three host facades, item descriptors available
on every compatible host facade, sticky placement, and portal-based overlay
helpers. All builders remain order independent and need no final `.build()`.
Different properties therefore have fixed precedence where a shorthand and an
axis-specific builder overlap. Repeating the same builder remains last-call
wins for property setters, like existing Reactant facades. Constraint-adding
builders are the exception: Sticky's `with_*` methods reject any second edge on
the same axis. The same set of calls is invalid regardless of their order.

### Flex

`Flex` provides ordinary one-dimensional UI Toolkit flow plus reliable gaps.
It reuses the existing `FlexDirection`, `FlexWrap`, `Align`, and `Justify`
values so applications do not learn a second alignment vocabulary.

```rust
Flex::new()
    .direction(FlexDirection::Row)
    .wrap(FlexWrap::Wrap)
    .row_gap(18.0)
    .column_gap(28.0)
    .child(CancelButton)
    .child(ConfirmButton)
```

The builders have these semantics:

- `direction` defaults to `FlexDirection::Row`.
- `wrap` defaults to `FlexWrap::NoWrap`.
- `align_items` defaults to `Align::Stretch`.
- `justify_content` defaults to `Justify::FlexStart`.
- `row_gap` and `column_gap` default to zero and accept finite nonnegative
  pixels.
- `gap(value)` supplies both gaps unless that axis has an explicit `row_gap` or
  `column_gap`. An axis-specific value wins regardless of call order. Repeating
  `gap` or one axis-specific builder uses its last value.

Child margins remain part of the child's own layout. A gap is additional space
between adjacent flex items and wrapped lines; it never replaces or rewrites a
margin.

### Grid tracks

`Grid` lays out direct children against explicit and implicit row and column
tracks. `GridTrack` is a closed track-size catalog.

```rust
pub enum GridTrack {
    Px(f32),
    Fraction(f32),
    Auto,
}
```

Convenience constructors provide `GridTrack::px`, `GridTrack::fr`, and
`GridTrack::auto`. Pixel sizes are finite and nonnegative. Fraction weights are
finite and strictly positive. `Auto` uses the preferred outer size of the
items contributing to that track.

`Grid` owns these container properties:

```rust
Grid::new()
    .columns([GridTrack::px(422.0), GridTrack::fr(1.0)])
    .rows([GridTrack::auto()])
    .auto_rows(GridTrack::auto())
    .auto_flow(GridAutoFlow::Row)
    .column_gap(18.0)
```

- `columns` and `rows` replace the complete explicit track lists.
- An explicitly empty list is valid and has the same track result as omission.
  It remains a set value for sparse structural comparison.
- An omitted column or row list starts with no explicit tracks. Auto-placement
  creates implicit tracks as needed.
- `auto_columns` and `auto_rows` default to `GridTrack::Auto` and select the
  size of every implicit track on that axis.
- `auto_flow` is `Row` or `Column` and defaults to `Row`.
- `row_gap`, `column_gap`, and `gap` have the same finite pixel behavior as
  `Flex`.
- `align_items` and `justify_items` set the default item alignment inside each
  grid area and default to `Align::Stretch`. A container value may be
  `FlexStart`, `Center`, `FlexEnd`, or `Stretch`; `Auto` is invalid.

The catalog intentionally has no string grammar. Named lines, dense placement,
subgrid, and arbitrary CSS functions are absent from the API. Their absence
keeps placement deterministic and prevents visual order from silently
diverging from logical and focus order.

### Grid items

Every host facade receives `.grid_item(GridItem)`. The descriptor is valid only
when that host is a direct logical child of a `Grid` or a top-level portal host
physically targeting a `Grid`. Any other use aborts the commit as a developer
error.

```rust
View::new().grid_item(
    GridItem::new()
        .column(2)
        .row(1)
        .span_columns(2)
        .align_self(Align::Center),
)
```

Grid lines are one based. Spans are positive counts. Start line and span are
independent on each axis. An omitted start line requests auto-placement, and an
omitted span means one. The four valid forms are therefore auto with span one,
auto with an authored span, a start line with span one, and a start line with
an authored span. Auto-placement searches for the complete authored span.

An explicit item may extend beyond the explicit template. The grid creates
enough implicit tracks to contain it. General-cursor auto-placement never
backfills a hole before the cursor. An item with an explicit major start scans
only that authored major band, as defined below. Adding a later general-cursor
item cannot move an earlier item. Adding or changing explicit placement may
move auto-placed items because explicit reservations are resolved first.

`align_self` and `justify_self` default to `Align::Auto`, which uses the
corresponding container value. Grid `justify` values resolve on the physical
horizontal x axis, and `align` values resolve on the physical vertical y axis.
Their other valid values are `FlexStart`, `Center`, `FlexEnd`, and `Stretch`.
Margins and authored width or height remain child properties and participate
inside the resolved grid area. Percentage child dimensions resolve against
that area after track sizing; a 100% height label therefore fills its row even
when the container centers other children.

A Grid or Stack **placement child** means either a direct logical child or a
top-level portal attachment in that target adapter's merged logical stream.
Every placement child must retain the default relative position with all four
Style offsets automatic. `Position::Absolute` or an authored top, right,
bottom, or left offset is invalid because the typed item descriptor owns
placement and intrinsic participation. An application needing ordinary
positioned descendants wraps them in the placed child host.

### Responsive track changes

Track lists are ordinary rendered values. Applications branch on state,
context, resource data, or existing geometry measurements and render a new list
through the same `Grid` identity.

```rust
let columns = if self.large_text {
    vec![GridTrack::fr(1.0)]
} else {
    vec![GridTrack::px(422.0), GridTrack::fr(1.0)]
};
Grid::new().columns(columns).children(self.rows())
```

Viewport-responsive components use `use_geometry(ViewportRef)`.
Container-responsive components attach an `ElementRef` and observe it through
the same hook. The existing geometry convergence diagnostic applies when a
component changes its own size in response to that measurement. This design
does not add media queries, breakpoints, or another client-side condition
language.

### Stack

`Stack` overlaps direct children in one isolated stacking context. It is the
interactive counterpart to `before` and `after` decorations.

```rust
Stack::new()
    .align_items(Align::Stretch)
    .justify_items(Align::Center)
    .child(View::new().stack_item(StackItem::new().order(0)))
    .child(Button::new("Play").stack_item(StackItem::new().order(10)))
```

`Stack::align_items` and `Stack::justify_items` default to `Align::Stretch`.
Like Grid container alignment, they reject `Align::Auto`.
Stack uses the same fixed axes as Grid: `justify` is horizontal and `align` is
vertical.

Every child has a `StackItem`, whether explicitly authored or defaulted. Its
properties are:

- `order: i32`, defaulting to zero;
- `align_self` and `justify_self`, defaulting to `Align::Auto` and therefore
  using the corresponding Stack container value;
- optional finite nonnegative pixel insets from the top, right, bottom, and
  left edges; and
- `contributes_to_size: bool`, defaulting to `true`.

Lower order paints and hit-tests before higher order. Equal order uses logical
source order, so a later equal-order child is visually above an earlier child.
A nested stack never escapes the order assigned to its own host in the parent
stack.

Contributing children determine the stack's intrinsic content size by the
largest preferred outer width and height after insets. Noncontributing children
still receive placement but do not enlarge an automatically sized stack. A
stack with no contributing children has zero intrinsic content size unless its
ordinary `Style` supplies dimensions or constraints.

Insets constrain the child's available rectangle before alignment. Opposing
insets with stretch fill the remaining axis. Insets and margins apply from the
leading and trailing edges in that order. If the resulting edges cross, the
trailing edge clamps to the leading edge. The zero-sized slot stays at that
leading-edge origin, and the child may overflow according to its own minimum
size and the stack's ordinary overflow style.

### Sticky placement

`Sticky` is a child descriptor resolved against the nearest physical
`ScrollView` in the same panel. A typical table header uses a top inset and a
stacking order above its rows.

```rust
Grid::new()
    .columns(columns)
    .child(
        Header::new()
            .grid_item(GridItem::new().span_columns(3))
            .sticky(Sticky::top(0.0).order(4)),
    )
```

`Sticky` supports at most one horizontal edge and at most one vertical edge.
The public constructors are `top`, `right`, `bottom`, and `left`. The distinct
builders `with_top`, `with_right`, `with_bottom`, and `with_left` add a second
orthogonal edge. A builder on an already constrained axis is invalid rather
than last-call wins. Insets are finite pixels. Removing the complete Sticky
descriptor resets all edges; individual edges have no reset builder. `order`
defaults to zero and sorts sticky presentation in the same way as a stack item.

The element keeps its normal-flow size and location. While scrolling, Unity
translates its presentation no farther than:

- the selected scroll-viewport inset on the sticky axis; and
- the far edge of its sticky containing block, so it leaves with that block.

The containing block is the content box of the physical public parent adapter
that owns the flow placeholder. Transparent fragments are skipped. For an
ordinary child this is its direct host parent; for a portal it is the physical
portal target. If the placeholder is directly in ScrollView content, the
ScrollView content root is the containing block. This rule does not search for
the nearest Grid, Flex, or Stack by kind.

A sticky grid item is valid because its placeholder remains in the grid area.
Combining `Sticky` with `StackItem` or overlay placement is invalid: stack and
overlay children already have explicit presentation placement.
Combining Sticky with `Position::Absolute` is also invalid because an
out-of-flow child has no normal rectangle for the placeholder. Relative Style
offsets remain part of the measured normal rectangle.

### Overlay host and layers

An **overlay host** is a root-level layer that owns a `PortalTarget` and sits
outside clipped application content. `OverlayHost` is a Reactant helper that
renders a configured inner Stack; it does not add a new protocol element kind.

The target is an ordinary `PortalTarget` created through
`Reactant::create_portal_target()` before root registration. It remains scoped
to that Reactant runtime and may attach to exactly one `OverlayHost`, following
the existing portal target rules.

```rust
let overlays = reactant.create_portal_target();
reactant.register_root(document, App::new(overlays.clone()));
```

```rust
Stack::new()
    .child(AppScreen)
    .child(OverlayHost::new(self.overlay.clone()))
```

The helper supplies its own stretching, noncontributing `StackItem` at the
highest integer order. It must be the sole OverlayHost and final logical child
of the document-root Stack returned by that registered root. These conditions
are desired-tree validation rules, so an application child cannot tie above it.
Each Unity panel may contain exactly one OverlayHost. A runtime with several
registered roots may own several only when those roots render into different
panels. Portal descendants cannot enlarge the root Stack. The host fills its
stack area, uses visible overflow, and ignores picking on itself. Pickable
portal descendants continue to receive input.

Its standard wrapper kinds are `OverlayLayer::Popover` and
`OverlayLayer::Modal`. The target adapter assigns every top-level portal
attachment a source ordinal from
`(root_registration_ordinal, portal_preorder_ordinal)` at commit. Root ordinals
follow zero-based registration order. Portal ordinals are the zero-based order
of top-level portal attachment nodes in a depth-first preorder traversal of
that committed root's logical tree.

Modal items sorted by source ordinal receive ranks one through N. Application
content has rank zero. A Modal presentation key is `(rank, 0, source_ordinal)`.
A Popover inherits the rank of its nearest logical Modal ancestor, or zero when
it has none, and uses `(rank, 1, source_ordinal)`. Sorting by this key places a
root popover below every modal, a modal-local popover above its owning modal,
and both below the next higher modal. A popover cannot escape a higher modal.
The highest-ranked logically mounted Modal wrapper is the active modal consumed
by the focus coordinator. A wrapper retained only for Motion exit is absent
from this order. Conditional insertion or reordering recomputes keys without
changing retained keyed host identity.
`Overlay::popover` and `Overlay::modal` select those layers automatically.
`Overlay::layer(target)` creates an unanchored, host-filling Popover-layer
wrapper for application-defined overlay content. Only `Overlay::modal` creates
a Modal-layer wrapper. An application needing alignment or different ordering
uses an ordinary nested `Stack` inside the wrapper.

`Overlay` owns one ordinary wrapper host around its child. The wrapper provides
a single physical stack item even when the child is a component, fragment, or
conditional render value. It remains in the child's logical portal ancestry.
`Overlay` exposes `.style(Style)` and the ordinary mirrored visual-element
builders; those declarations apply to this wrapper. Styling the child still
styles only the child.

Every Overlay wrapper rejects margin, position, offset, display, and visibility
declarations. Conditional rendering mounts and unmounts overlay visibility.
Popover wrappers accept authored width, height, min/max, padding, border, and
paint declarations because their preferred border box participates in
collision. Unanchored layer and Modal wrappers also reject authored dimensions
and min/max constraints: their private Stack slot always fills the host. Their
authored padding, border, and paint render inside that fixed border box. Bounded
dialog content belongs in an ordinary child or nested Stack.
Every overlay wrapper ignores picking on itself; only authored descendants are
pickable. The focus coordinator makes content outside the active modal
effectively non-pickable. An authored backdrop remains necessary only when the
application wants a visible click target for dismissal. The programmatically
focusable modal wrapper remains a focus target because focusability is
independent from picking mode.

### Anchored popovers

`Overlay::popover` positions its wrapper against an `ElementRef` in the same
panel. Placement uses one `PlacementSide` (`Top`, `Right`, `Bottom`, or `Left`)
and one `PlacementAlign` (`Start`, `Center`, or `End`).

```rust
Overlay::popover(self.overlay.clone(), trigger_ref)
    .placement(PopoverPlacement::bottom_start().offset(6.0))
    .child(Menu::new(self.options.clone()))
```

`PopoverPlacement` also contains:

- finite main-axis and cross-axis offsets, defaulting to zero;
- finite nonnegative collision padding, defaulting to eight pixels;
- `flip`, defaulting to `true`; and
- `shift`, defaulting to `true`.

Omitting `.placement(...)` uses `PopoverPlacement::bottom_start()` with those
defaults.

`offset(value)` is shorthand for `main_offset(value)`. A positive main offset
moves away from the anchor. A positive cross offset moves toward increasing
panel x for Top and Bottom, and increasing panel y for Left and Right. Side and
alignment are physical panel directions and do not reverse with text direction.

Before offsets, Top puts the wrapper's bottom at the anchor's top, Bottom puts
its top at the anchor's bottom, Left puts its right at the anchor's left, and
Right puts its left at the anchor's right. Start aligns left edges for Top and
Bottom and top edges for Left and Right. End aligns the opposite edges. Center
aligns the corresponding centers.

Placement uses the overlay wrapper's preferred border-box size. Wrapper margin
declarations are invalid; main and cross offsets provide external spacing. The
wrapper derives intrinsic content width from the child's preferred outer width,
including child margins. Authored wrapper width and min/max constraints apply
to that content box using existing Style semantics. Wrapper padding and border
widths are then added to produce border-box width. Height resolves at the final
content width in the same order: child outer height, authored content-box
constraints, then wrapper padding and borders. Collision never shrinks this
result to the overlay host. The final border box is the rectangle tested for
overflow.

Collision uses the overlay host's content rectangle inset by collision
padding. On each axis, effective padding is the smaller of the authored value
and half the current host dimension. Excessive padding therefore produces a
degenerate padded interval at the host center rather than crossed edges. Unity
computes the requested rectangle, including both offsets, before flip or shift.
Main-axis overflow is the sum of the rectangle's positive penetration beyond
the two padded edges on that axis. When `flip` is enabled, Unity computes the
opposite side with the same alignment and scalar offsets. The main offset is
reprojected away from that candidate side, so its physical direction reverses
when Top flips to Bottom or Left flips to Right. Cross offset keeps its defined
physical direction. Unity selects the opposite side only when its main-axis
overflow is strictly smaller. A tie keeps the requested side.

When `shift` is enabled, Unity then translates the selected rectangle only on
its cross axis by the smallest distance that fits within the padded edges.
Oversize popovers align their leading cross edge to the leading collision edge
and retain finite trailing overflow rather than receiving a negative size.

An anchor that has not attached, is hidden, or is temporarily unavailable puts
the popover wrapper in a waiting state: it is hidden and excluded from picking
until both anchor and popover geometry are current. Anchors and overlay hosts in
different panels are invalid and emit a diagnostic naming both object IDs.
The anchor may not be the popover wrapper or any logical descendant of that
popover. That cyclic relationship is a developer error rejected during Unity
preflight before it can create geometry feedback.

Waiting uses a private presentation-visibility layer equivalent to Visibility
Hidden, not `Display::None`. The wrapper and child remain in layout and can be
measured without painting or accepting input. First mount therefore obtains
popover geometry while waiting and cannot deadlock on its own hidden state.

When a visible popover enters waiting while focus is in one of its logical
descendants, Unity restores focus to an eligible anchor or clears panel focus
when the anchor is ineligible. Returning from waiting never moves focus into
the popover automatically.

Popover mounting does not move focus automatically. If focus is inside the
popover when it unmounts, Unity restores focus only when the anchor is attached,
displayed, visible, enabled, focusable, and in the same panel. Otherwise the
panel's ordinary focus controller receives the removal. Selection, arrow-key
behavior, outside click handling, and open state remain ordinary component
logic and Reactant events.

### Modal overlays

`Overlay::modal` fills the overlay host and defaults to the `Modal` layer. The
application renders its backdrop and dialog as ordinary children, so pointer
dismissal and animation remain explicit.

```rust
Overlay::modal(self.overlay.clone())
    .initial_focus(confirm_ref)
    .restore_focus(trigger_ref)
    .child(ArcadeModal::new())
```

Modal focus behavior is owned exclusively by
[Focus and navigation](focus-and-navigation.md#modal-coordination). This layout
design supplies only the wrapper, resolved overlay order, logical membership,
and final physical placement consumed by that coordinator.

Logical ancestry determines whether a host belongs to a modal, including a
same-panel portaled descendant. Physical Unity traversal determines sequential
Tab order within that membership. A logical descendant presented in another
panel is ineligible because no single `FocusController` can contain it.

An initial-focus ref must be a logical descendant of the modal wrapper. Private
layout nodes are never eligible. An attached initial ref outside that modal, or
any attached initial or restore ref in another panel, rejects the complete
mutation group during Unity preflight. An unavailable, hidden, disabled,
detached, or otherwise ineligible ref uses the focus design's fallback. A
restore ref may be anywhere in the same panel.

The modal wrapper is a public host with reserved effective focus values defined
by the focus design. Layout authoring cannot make an active wrapper ineligible.

Escape never closes a modal implicitly. The modal or a logical ancestor handles
the existing keyboard event and updates application state. A full-size picking
backdrop prevents pointer interaction with lower layers; the layout system does
not synthesize a backdrop.

## Host protocol

The protocol adds concrete `UiFlex`, `UiGrid`, and `UiStack` element variants.
Each contains the shared `UiVisualElement` properties plus its layout-specific
properties. This preserves the existing one-facade-to-one-element-kind rule.

Shared visual properties gain sparse optional descriptors for child placement:

```rust
pub struct UiVisualElement {
    pub grid_item: Prop<GridItem>,
    pub stack_item: Prop<StackItem>,
    pub sticky: Prop<Sticky>,
    pub overlay_placement: Prop<OverlayPlacement>,
    // Existing properties remain unchanged.
}
```

The exact wire names follow the existing Rust and C# naming conventions. The
semantic requirements are more important than whether the implementation
stores these fields in one nested descriptor or several sparse fields:

- Omitted fields preserve live state in a sparse update.
- `Prop::Set` replaces the complete descriptor.
- `Prop::Reset` restores the native constructor default.
- Create-only values remain separate from resettable descriptors.
- Rust and C# validation accept and reject the same finite value catalog.

Changing a host from `Flex` to `Grid` or `Stack` changes its element kind and
therefore remounts it, like every other host-kind change. Updating a property on
the same container kind preserves identity.

## Native ownership model

Reactant's tree remains the logical source of truth even though special layout
needs private native nodes. The client maintains a strict boundary between
logical hosts and presentation helpers.

A **layout slot** is a private, unidentified `VisualElement` that represents
one direct logical child in its container's flow. It normally contains the
actual host. While that child is sticky, the same slot remains as an empty flow
placeholder and the actual host is parented through an unidentified
presentation entry on the ScrollView surface. The slot has no Battlement object
ID, picking, focus, event subscriptions, semantics, or public query surface.
Flex, Grid, and Stack own
one stable slot per direct logical child. An ordinary container adapter,
including the ScrollView content root, creates the same kind of slot on demand
when one of its direct children becomes Sticky and removes it after the actual
host returns to ordinary flow. That on-demand slot follows the same identity,
measurement, cleanup, and logical-index rules.

For a top-level portal attachment targeting Grid or Stack, the physical target
adapter creates and owns the stable slot when it attaches the public host. The
portal source retains logical ownership, context, and event ancestry; the target
adapter owns slot measurement, target-relative indexing, presentation order,
and cleanup. Detach removes the target slot before the existing portal lifecycle
destroys or reattaches the public host. Snapshot reconstruction derives these
slots from public portal attachments exactly like direct-child slots.

A **measurement element** is a private in-flow child whose resolved size makes
an automatically sized Grid or Stack contribute to its UI Toolkit flex parent.
It is also unidentified and noninteractive. Grid and Stack layout slots are
positioned over this element without mutating the real child's style.

A **presentation surface** is a private layer used by a ScrollView or overlay
host to display content independently from its flow placeholder. Sticky content
uses a ScrollView presentation surface. The portal contract still determines
which public host owns overlay content.

These private nodes obey the following invariants:

- `TryGet(ObjectId)` and every ref resolve the actual host, never a private
  node.
- Protocol parent and index operations address the logical parent and logical
  sibling index.
- Private nodes never appear in snapshots, commands, events, geometry payloads,
  diagnostics that identify public hosts, or fake-client state.
- Removing or changing a special layout descriptor restores the actual host to
  ordinary native parentage without replacing it.
- Destroying a logical parent destroys all private nodes it owns after its
  public descendants have detached or been destroyed.

### Logical and presentation order

**Logical order** is the direct-child order in the Battlement and Reactant
trees. **Presentation order** is the private native order used solely for
painting and hit testing in Stack and sticky surfaces.

Flex and Grid keep presentation order equal to logical order. A Stack sorts by
`(order, logical_index)`. A ScrollView enumerates sticky placeholders through
container-adapter logical child lists in depth-first pre-order from its content
adapter and assigns each a source ordinal. It does not traverse actual hosts
after presentation reparenting. A portal subtree occupies its target adapter
position in that traversal. Its sticky surface sorts by
`(order, source_ordinal)`.

UI Toolkit paints later siblings above earlier siblings, so these tuples also
determine hit-test precedence. Reordering presentation never changes the
logical index recorded by a container adapter.

For an ordinary Grid or Stack portal target, the adapter's target-relative
logical stream contains ordinary direct children first, followed by top-level
portal attachments sorted by the source ordinal defined for overlays. Each
attachment's position in that merged stream is its logical index for Grid
auto-placement and equal-order Stack ties. Multiple portal sources are
therefore deterministic without changing their logical event ancestry.

The adapter owns a logical child list separate from the native presentation
list. Create, reparent, and index commands update the logical list first, then
reconcile slots and presentation order. This prevents a z-order sort from
changing the meaning of a later `VisualElementUpdate::Index`.

## Layout scheduling

Unity performs layout work after applying a complete Reactant mutation group.
The client marks only affected containers dirty.

A container becomes dirty when:

- its own layout descriptor changes;
- a direct logical child is created, removed, moved, or reindexed;
- a direct child's item descriptor, preferred size, visibility, or relevant
  style constraint changes; or
- the container's resolved content size changes.

The client schedules one layout operation for each dirty container in the
current native layout generation. Ancestors run before descendants so a nested
container receives its resolved grid area or stack rectangle before measuring
its own children. A descendant invalidated by an ancestor may run once more in
the next generation when its available size actually changed.

Stable containers perform no measurement, allocation, or style writes. Sticky
scrolling updates only presentation offsets and does not dirty the underlying
Grid, Stack, or Flex unless a viewport-size change also occurred.

### Intrinsic measurement

The client normally measures the actual child inside its stable flow slot.
Preferred outer size includes the child's resolved size and margins, subject to
its authored minimum and maximum constraints. The slot, not the child, supplies
temporary measurement constraints.

For active Sticky, the ScrollView presentation entry contains the actual host
and supplies the flow slot's normal available-axis constraints. Content, Style,
font, asset, and intrinsic-size changes on that host dirty both its original
parent container and sticky coordinator. The next scheduled layout measures the
actual host through the presentation entry, updates the empty flow slot's
normal-flow size, and then recomputes clamping. It never reparents the host for
measurement, and it uses the same bounded convergence rules as Grid and Stack.

Columns resolve before rows. After column widths are known, each item is given
its grid-area width before the client reads its preferred height. Text therefore
wraps at the final column width rather than contributing an unwrapped row
height.

Grid and Stack update their measurement element only when the computed
intrinsic size changes. The layout operation repeats for at most four
consecutive native generations without a changed descriptor, child, or parent
size. If it still does not settle, the client keeps the last finite result and
emits one diagnostic containing the container ID, pass count, available size,
and changing item IDs. A later real invalidation permits another bounded
attempt.

### Display participation

`Style::display(Display::None)` keeps the child in logical reconciliation order
but removes its slot from Flex flow, Grid occupancy and intrinsic contribution,
and Stack placement and intrinsic contribution. Later Grid children place as if
that item were absent. Returning to `Display::Flex` dirties the owning container
and recomputes placement without replacing the host.

`Style::visibility(Visibility::Hidden)` retains ordinary flow, occupancy,
measurement, and placement while suppressing paint, picking, and focus. The
retained Suspense-hidden state uses the nonparticipating behavior rather than
Visibility Hidden, as specified by its lifecycle contract.

## Grid placement and track sizing

Grid layout separates cell placement from sizing. Placement never depends on a
measured item size, which keeps the occupancy result stable while tracks settle.

### Placement

Placement describes a **major** and **minor** axis. Row flow uses rows as the
major axis and columns as the minor axis; column flow swaps them. The initial
minor extent is at least one track and is otherwise the greatest of the
explicit minor track count, the far edge of any authored minor placement, and
the largest authored minor span.

For one-based start `c` and span `s`, the last occupied minor track is
`c + s - 1`; that track number, not the following line `c + s`, contributes to
the initial extent. With no explicit minor tracks, an item fixed at minor start
four with span two therefore establishes five minor tracks.

The grid creates an occupancy matrix large enough for the explicit template
and applies this algorithm:

1. Reserve every item with both axes explicitly placed, in logical order.
   These reservations may overlap and do not advance an auto-placement cursor.
2. Process items with an explicit major start and automatic minor start in
   logical order. For each item, scan minor starts from line one within its
   explicit major band. Use the first area that does not overlap an occupied
   cell. If none fits, append enough implicit minor tracks and place it after
   the current minor extent.
3. Initialize the general cursor to major line one, minor line one. Process all
   remaining items in logical order.
4. For an item with an explicit minor start, compare it with the cursor's
   previous minor position. If it is earlier, first advance one major track.
   Then set the cursor to the explicit minor start and advance only the major
   coordinate until the complete area is free, growing implicit major tracks
   as needed.
5. For an item automatic on both axes, test the complete span at the cursor.
   Advance the minor coordinate one track until it fits. When the span would
   cross the minor extent, move to the next major track and minor line one.
6. After either general-cursor placement, move the minor coordinate to the
   first line after the placed minor span. If that position is past the minor
   extent, move to the next major track and minor line one.

The algorithm never changes minor extent during general-cursor placement
unless one item's minor span is itself wider than the extent. Fully automatic
items therefore create new major tracks instead of extending one endless row
or column. Apart from the explicit-major phase, the cursor never returns to an
earlier cell, which is the contract's non-dense behavior.

For a concrete Row-flow oracle, start with three explicit columns and no
explicit rows. In logical source order, author:

- A at row one, column two;
- B automatic on both axes with a two-column span;
- C at row one with an automatic column;
- D at column one with an automatic row; and
- E automatic on both axes with span one.

The explicit phases place A at `(1, 2)` and C at `(1, 1)`. The general cursor
places B at row two, columns one through two; D at `(3, 1)`; and E at `(3, 2)`.
Column flow produces the exact transposed coordinates. This sequence is the
golden mixed-placement fixture used by the implementation plan.

Overlapping explicit areas are allowed. Their paint and hit-test order remains
logical source order because Grid does not define a stacking context. An
application that needs controlled overlap wraps that area in a `Stack`.

### Track sizing

Each axis uses the same deterministic sizing procedure. The resolved content
size is **bounded** when UI Toolkit supplied a finite available size and
**intrinsic** when the container is deriving its own preferred size.

1. Assign every pixel track its authored size and every automatic track a base
   of zero. Start one axis-wide fractional unit, `u`, at zero.
2. For each nonspanning item, raise an automatic track's base to the item's
   preferred contribution. For a fractional track with weight `w`, set
   `u = max(u, contribution / w)`. Fixed tracks ignore contributions.
3. Process spanning items in logical order. Its current span size is the pixel
   sizes, automatic bases, `u * w` for fractional tracks, and internal gaps.
   Subtract that value from the item's preferred contribution.
4. If the positive deficit spans automatic tracks, add an equal share of the
   entire deficit to those automatic bases. Otherwise, if it spans fractional
   tracks with total weight `W`, increase `u` by `deficit / W`. A span with
   only fixed tracks retains overflow. Fixed tracks never grow.
5. For intrinsic sizing, each fractional track resolves to `u * w`.
6. For bounded sizing, subtract pixel tracks, automatic bases, and gaps from
   the available size to get `R`. When fractional tracks exist, set
   `u = max(u, max(0, R) / total_fraction_weight)`, then resolve every
   fractional track to `u * w`. When none exist, skip this division. If the
   intrinsic unit consumes more than `R`, the grid overflows by that finite
   difference.
7. When a bounded axis has no fractional tracks, distribute its positive
   remainder equally among automatic tracks. Pixel tracks retain their exact
   size. This lets a minimum-height grid center its children in the full row.

There is one shared fractional unit per axis, so every fractional track always
preserves its authored weight ratio. For example, two `1fr` tracks with a
300-pixel spanning contribution establish `u = 150`. A 200-pixel bounded
container keeps both tracks at 150 pixels and overflows by 100 pixels; it does
not shrink them to 100 pixels.

Automatic tracks receive spanning deficits first. With zero gaps, tracks
`[Auto, 1fr]`, a 100-pixel single-track automatic contribution, a 50-pixel
single-track fractional contribution, and a 300-pixel spanning contribution
resolve intrinsically to 250 and 50 pixels. In 200 available pixels, they keep
those sizes and overflow by 100 pixels.

Automatic tracks never shrink below their resolved preferred contribution.
Fixed tracks never shrink. Fractional tracks never become negative. Content
that still cannot fit retains its finite size and follows the container's
ordinary overflow behavior.

After track sizes are known, the client accumulates track positions with gaps,
forms each item's spanned area, subtracts margins, and applies self-alignment
inside the slot. Stretch affects an auto-sized child on that axis but does not
replace an explicitly authored width or height.

Margin subtraction never creates a negative area. On each axis, the client
moves the leading edge by the leading margin and the trailing edge by the
trailing margin. If they cross, the trailing edge clamps to the leading edge.
The zero-sized slot stays at that leading-edge origin; child minimum size may
then produce ordinary finite overflow.

Grid and Stack read authored margins as placement and intrinsic-size inputs,
then mask the four margin declarations on the actual child through the same
private resolved-style mechanism used by Flex. Their assigned slot rectangle is
the child's border-box rectangle after margin subtraction. The authored Style,
sparse protocol value, and animation layers remain unchanged, and margins are
therefore applied exactly once.

## Flex gap mechanics

Flex delegates line breaking and ordinary flex growth to UI Toolkit. Stable
layout slots isolate gap implementation from child margins.

Flex owns a private in-flow band that fills its content box and receives the
container's direction, wrapping, alignment, and justification. Displayed
relative slots are direct native flex items of that band. Absolute slots are
instead direct positioned children of the outer Flex content box and never
enter the band. Logical child order remains in the adapter rather than either
native list.

For an in-flow child, the adapter projects its flex-facing computed
declarations onto the slot: display, `align_self`, flex basis, grow, and shrink,
aspect ratio, width and height constraints, and margins. For an absolute child,
the outer slot receives its position, offsets, dimensions, and constraints
without gap projection.

The adapter's private resolved-style layer masks the transferred declarations
on the child and stretches the child through the slot content rectangle. This
does not change the authored Style, protocol state, sparse comparison, or
animation layer. Updating one transferred declaration recomputes only that
child's proxy and dirties its Flex container. The declaration therefore affects
line breaking, growth, shrinkage, absolute positioning, and alignment exactly
once, as it would when the actual host were the direct UI Toolkit flex item.

For Row and RowReverse, in-flow band slots receive half the column gap on their
horizontal sides and half the row gap on their vertical sides. Column gap is
therefore between items in a line, and row gap is between wrapped lines. For
Column and ColumnReverse, slots receive half the row gap vertically and half
the column gap horizontally. Row gap is between items in a line, and column gap
is between wrapped lines. The private band compensates with matching negative
outer half-gaps inside the unchanged outer Flex content box. This produces:

- one complete gap between adjacent items;
- one complete axis-appropriate gap between wrapped lines;
- no added space at the container's content edges; and
- no mutation of child margin declarations.

Outer band compensation is present on an axis only while the Flex has at least
one displayed in-flow slot on that axis. An empty band has zero gap
compensation. Because absolute slots resolve against the unchanged outer
content box, a mixed Flex preserves native absolute offsets as well as an
absolute-only Flex.

Direction and wrap reversal remain native UI Toolkit behavior. Logical order
does not change when visual flow reverses.

## Stack sizing and placement

Stack resolves each axis independently. An axis is bounded when UI Toolkit
supplies a finite content extent and intrinsic otherwise. The client resolves
horizontal size first. An intrinsic width is the largest contributing child's
preferred border-box width plus horizontal margins and insets. A bounded width
remains fixed.

The client then measures each child's preferred height at its final available
width, including authored width and minimum and maximum constraints. An
intrinsic height is the largest contributing result plus vertical margins and
insets. A bounded height remains fixed. The measurement element receives only
the intrinsic axes, so fixed width with automatic height wraps content at the
fixed width before choosing height.

For final placement on each axis, the client subtracts insets and margins and
clamps crossed edges as defined above. `FlexStart`, `Center`, and `FlexEnd` use
the child's preferred border-box extent after authored size and min/max
constraints; that extent may overflow the available interval. `Stretch` uses
the interval extent only when the authored child dimension is automatic, then
applies min/max constraints. With an authored dimension, Stretch behaves as
`FlexStart`. The client assigns the two resolved axes to the layout slot and
sorts it by `(order, logical_index)`.

Only slot position and size change. The actual host retains its Style and
Motion presentation.

## Sticky mechanics

Sticky placement uses a flow placeholder and a ScrollView-owned presentation
surface. The actual host remains the identified element; the placeholder is its
stable layout slot. While sticky presentation is active, that slot is empty but
retains the child's measured normal-flow rectangle, and the actual host is the
identified child of the presentation surface. Removing Sticky reparents that
same host into its slot before discarding presentation state.

For one axis, let `[n0, n1]` be the placeholder's current normal rectangle,
`[v0, v1]` the ScrollView viewport, `[c0, c1]` the containing block, `s` the
item size, and `i` the signed inset. Top and Left are leading-edge Sticky:

```text
p0 = min(max(n0, v0 + i), c1 - s)
p1 = p0 + s
```

Bottom and Right are trailing-edge Sticky:

```text
p1 = max(min(n1, v1 - i), c0 + s)
p0 = p1 - s
```

All coordinates are current panel-local coordinates after ordinary scrolling
layout. The containing-block term is applied last and therefore wins when an
oversized item cannot satisfy both limits. The item keeps its finite size and
may cross the opposite viewport inset. With two orthogonal edges, horizontal
and vertical formulas resolve independently.

For a leading example, `n0 = -20`, `s = 40`, `v0 = 0`, `i = 10`, and
`c1 = 200` produce `p0 = 10`; changing `c1` to 30 produces `p0 = -10` because
the containing block wins. An inset of minus five produces `p0 = -5` with the
original block. For a trailing example, `n1 = 220`, `s = 40`, `v1 = 200`,
`i = 10`, and `c0 = 0` produce `p1 = 190`; changing `c0` to 180 produces
`p1 = 220`.

The presentation surface follows the ScrollView viewport rather than its
scrolling content container. It has visible overflow only where the viewport
permits it and is clipped by the ScrollView's native viewport. The surface
paints and hit-tests after ordinary scrolling content, so every presented
sticky host is above non-sticky rows. Sticky hosts sort by
`(order, source_ordinal)` as defined above.

The ScrollView coordinator observes native geometry changes for every active
flow placeholder and its containing-block content box. Sibling reflow,
ancestor layout, insertion, removal, and containing-block resize therefore
invalidate sticky placement even when the sticky host itself is unchanged.
The coordinator coalesces these changes and recomputes after current layout in
the same rendered frame, without a Rust geometry observation or rerender.

When an element stops being sticky, moves to another ScrollView, or is
destroyed, the coordinator removes its presentation entry before restoring or
destroying the actual host. A complete committed move that leaves Sticky
without a ScrollView fails preflight and preserves the prior presentation.

Native teardown and snapshot reconstruction may transiently detach the old
ScrollView before destruction or new attachment is complete. The coordinator
then removes the old presentation entry and keeps the public host in its flow
slot or documented snapshot waiting state. This transient cleanup is not a
valid committed tree. Pointer capture and focus remain on the actual host
throughout an ordinary sticky update.

## Overlay mechanics

Overlay placement runs after the overlay wrapper and its anchor have current
panel geometry. It uses panel-local coordinates and writes only the wrapper's
private stack slot.

Popover placement is invalidated by anchor bounds, overlay preferred size,
overlay-host size, placement options, panel-scale changes, or anchor attachment,
display, and visibility state. Scrolling an anchor recomputes its slot locally
in the same rendered frame. A hidden anchor returning visible retries even when
its bounds are unchanged. Stable overlays do not register a Rust geometry
observation.

At overlay-placement time, Unity evaluates the anchor's current-frame presented
matrix, including layout projection and sampled Motion or CSS-style animation
that will be applied in the later presentation layers. It computes final anchor
bounds from that matrix without changing the layer order. A local animation
tick invalidates the anchored wrapper in the same frame. The popover's own
projection and Motion still apply later on top of its newly resolved slot.

Modal wrappers are ordered by overlay presentation order. The focus coordinator
consumes the highest logically mounted wrapper and owns its panel callbacks.
Private layout nodes never enter the focus ring.

Portal context and events remain logical. A click inside a popover reaches its
source component ancestors, not the OverlayHost's unrelated logical ancestors.
Native event coverage follows the existing physical event-island rules.

## Reconciliation and lifecycle integration

Special layout changes presentation without weakening Reactant's transactional
commit rules.

- Desired-tree validation resolves every item against its intended logical or
  portal target container before producing commands.
- A failed render, suspension, or caught render error creates no native layout
  slots and changes no committed descriptor.
- Mutation barriers preserve parent creation, child movement, property update,
  and destruction ordering. Private adapter work occurs inside the matching
  command and adds no protocol command group.
- A child moved between special containers retains its host ID and native state
  while its old slot is removed and its new slot is created.
- A portal target change retains the existing portal rule: the portal subtree
  remounts with new host IDs. Layout does not weaken that ownership boundary.
- Hidden retained Suspense content keeps its private layout state but does not
  contribute size or receive picking until shown again.
- Presence-retained hosts continue to occupy the committed layout described by
  their presence mode. `PopLayout` uses its existing projection behavior and
  does not contribute to new Grid or Stack intrinsic measurement.

### Focus and picking

Layout slots, measurement elements, and presentation surfaces use
`PickingMode::Ignore` and are not focusable. The actual host remains the target
for pointer, keyboard, focus, and geometry events.

Stack presentation order determines which overlapping actual host UI Toolkit
picks. Reactant dispatch still follows the committed logical path. Moving a
focused host between private slots preserves the same actual host. Focus repair
and modal focus changes follow the focus design rather than this layout design.

### Refs and geometry

`ElementRef` attaches to the actual host. Geometry sampling runs after layout,
sticky, and overlay placement and therefore reports the visible viewport bounds
of that host. It never reports a placeholder or private slot.

Responsive Rust components receive those measurements through the existing
next-frame geometry batch. Grid, Stack, sticky, and popover calculations do not
send their private intermediate measurements to Rust.

### Motion and decorations

Layout placement and Motion use separate native transform layers:

1. UI Toolkit computes ordinary host and intrinsic layout.
2. A layout slot applies Grid, Stack, sticky, or overlay placement.
3. Layout projection applies its inverse geometry transform.
4. Motion, CSS-style animation, gesture, and presence transforms apply to the
   actual host.

An authored Motion translation is therefore relative to the resolved layout
position and cannot unstick an element or overwrite popover collision
placement. A new layout projection samples the currently presented bounds,
including sticky or overlay placement, before constructing its inverse.

`before` and `after` decorations remain within their host and its clip. A
decoration cannot use GridItem, StackItem, Sticky, or Overlay because it has no
logical host or layout participation.

### Session reconstruction

An authoritative snapshot contains every public layout container and item
descriptor but no private native nodes. Unity reconstructs slots, measurement
elements, sticky surfaces, and overlay placement from that snapshot. The focus
coordinator then derives current modal state from the reconstructed wrappers.

Reactant reconnect preserves logical component and hook state under its
existing rules. Old native measurements, recorded focused native instances,
and scheduled layout passes are retired. New overlay anchors wait for the new
hosts to attach. Focus restoration never targets an object from the retired
session.

## Validation and failure behavior

Rust validates the complete desired Reactant tree before commit. Battlement UI
validates snapshots and sparse commands independently, and Unity rejects an
invalid patch without partially applying its layout descriptor.

Panel and attached-ref relationships are known only in Unity. Before applying
any native write, Unity preflights the complete snapshot or mutation group. A
cross-panel or cyclic popover anchor, a second OverlayHost in one panel, or an
attached invalid modal focus ref rejects that complete group as a developer
error, reports the involved public IDs, and leaves the previous native tree and
presentation unchanged. It never commits a hidden new wrapper or preserves a
stale placement from the rejected group.

Validation rejects:

- non-finite or negative pixel tracks and gaps;
- non-finite or nonpositive fraction weights;
- non-finite or negative Stack insets;
- non-finite Sticky insets and popover offsets;
- non-finite or negative popover collision padding;
- `Align::Auto` as a Flex, Grid, or Stack container default;
- line zero, span zero, and line or span arithmetic overflow;
- `GridItem` outside a direct logical Grid child or top-level portal attachment
  whose physical target adapter is a Grid;
- `StackItem` outside a direct Stack placement context;
- absolute position or nonautomatic Style offsets on a direct Grid or Stack
  child;
- `Sticky` without a physical ScrollView ancestor;
- two sticky edges on one axis;
- Sticky combined with Stack or overlay placement;
- Sticky combined with `Position::Absolute`;
- an OverlayHost that is not the sole OverlayHost and final logical child of
  its registered document's root Stack;
- a second OverlayHost attached to the same Unity panel;
- overlay portal content targeting a non-OverlayHost container;
- popover anchors or overlay hosts belonging to different panels; and
- a popover anchored to its own wrapper or logical descendant;
- margin declarations on an Overlay wrapper;
- position, offset, display, or visibility declarations on an Overlay wrapper;
- dimensions or min/max constraints on an unanchored layer or Modal wrapper;
- an attached modal initial ref outside its wrapper ancestry;
- an attached modal initial or restore ref in another panel; and
- reset or update operations that would change a concrete host kind.

These are developer errors and follow existing Reactant panic or Battlement
invalid-property behavior. Diagnostics include the logical host path, object ID
when allocated, descriptor kind, and offending value.

The following runtime states are not authoring failures:

- an overlay anchor or child waiting for its first valid geometry;
- finite Grid or Stack overflow;
- a temporarily hidden anchor;
- a sticky item waiting for ScrollView attachment during snapshot creation; or
- a bounded intrinsic layout that retained its last finite result after a
  convergence diagnostic.

## Sparse updates

Structural descriptor comparison extends the existing desired-property rules.
The smallest owning host receives each update.

| Change | Sparse mutation |
|---|---|
| Grid tracks, flow, gaps, or defaults | Grid properties only |
| Flex direction, wrapping, alignment, or gaps | Flex properties only |
| Stack defaults | Stack properties only |
| Grid or Stack item placement | Child properties only |
| Sticky inset or order | Child properties only |
| Popover placement or modal focus options | Overlay wrapper properties only |
| Unchanged descriptor | none |
| Removed descriptor | reset on the owning host |

A descriptor update may invalidate native layout, but it never causes Reactant
to allocate a replacement ID. Native private nodes may be added, removed, or
reordered as part of applying that one sparse mutation.

## Performance and diagnostics

The steady-state cost is proportional to dirty containers and locally moving
sticky or anchored elements, not to the complete Reactant tree.

- Stable Flex, Grid, and Stack containers perform no per-frame work.
- One native layout generation measures a dirty item at most once per axis.
- Duplicate invalidations coalesce by container and generation.
- Sticky scrolling changes presentation offsets without remeasuring content.
- Popover scrolling reuses current anchor and overlay sizes and recomputes only
  placement.
- Stable slots and occupancy storage are reused after creation. Ordinary layout
  and scrolling allocate no managed objects per frame.
- No local layout operation emits a Rust event, command, or geometry
  observation unless application code independently subscribed to one.

Unity diagnostics record dirty-container count, measured-item count, bounded
pass count, sticky and popover update count, and elapsed layout time. A slow
layout diagnostic includes the public container and item IDs but no private
native node identities.

The performance fixture contains at least 1,000 mixed Grid children, 100 sticky
scroll rows, nested stacks, and ten anchored overlays. It verifies stable-frame
silence, one-pass dirty updates, zero steady managed allocation, and no Rust
round trip while scrolling.

## Automated validation

Validation uses public Rust boundaries, the fake client, and real UI Toolkit
elements. Pure unit tests are reserved for the occupancy and track-sizing
algorithms, whose edge cases are difficult to prove through one visual fixture.

Rust coverage includes:

- public facade construction and order-independent builders;
- serde round trips for every descriptor variant;
- matching Rust and C# validation categories;
- `Prop::Set`, omission, reset, and Reactant structural comparison;
- sparse command counts for container and item changes;
- stable IDs across responsive tracks, stacking order, and sticky changes;
- invalid logical and portal placement contexts; and
- fake-client state after create, update, move, reset, and reconnect.

Unity EditMode coverage uses actual resolved geometry and picking:

- fixed, fractional, automatic, implicit, and spanning Grid tracks;
- wrapped text height after column resolution;
- row and column gaps with margins and overflow;
- source-order and one-axis-explicit auto-placement;
- Stack intrinsic sizing, insets, order ties, nested isolation, and hit target;
- sticky top, bottom, two-axis, containing-block, and detach behavior;
- popover side, alignment, flip, shift, oversize, waiting, and scrolling;
- modal wrapper order, focus-design integration, and destruction;
- layout projection and Motion transforms over every placement kind; and
- snapshot reconstruction without leaked private nodes or handlers.

Reactant black-box composition tests exercise a fixed tab grid, responsive form
rows, a sticky table header, a clipped dropdown portaled to an overlay, layered
interactive controls, and a modal. They assert visible geometry, event trace,
focus target, ref bounds, keyed state, host identity, sparse journal entries,
and complete restoration to the baseline.

## Manual QA

1. Open the Reactant layout sample at its baseline size. Confirm fixed tab
   tracks, automatic labels, fractional controls, row and column gaps, and
   source-order keyboard focus match the documented measurements.
2. Toggle large text and resize the viewport across every responsive specimen.
   Confirm track definitions update without remounting controls, losing focus,
   clearing text drafts, or restarting animations.
3. Add, remove, reorder, explicitly place, and span Grid children. Confirm
   implicit tracks and non-dense placement are stable and finite overflow is
   clipped or visible according to ordinary Style.
4. Resize wrapped labels and controls inside automatic rows. Confirm columns
   settle before row height and no repeated-layout diagnostic appears.
5. Interact with overlapping Stack layers at negative, equal, and positive
   orders. Confirm the visually top eligible host receives input and nested
   stacks cannot escape their parent layer.
6. Compare a noninteractive `after` decoration with an interactive Stack child.
   Confirm only the real child can focus, receive events, expose a ref, and own
   nested content.
7. Scroll the input table by wheel, drag, scrollbar, and touch. Confirm the
   header sticks in the same rendered frame, remains above rows, leaves with
   its containing block, and sends no continuous Rust traffic.
8. Open a dropdown whose logical parent clips overflow. Confirm the menu appears
   in the OverlayHost, follows the trigger while scrolling, flips and shifts at
   viewport edges, and preserves logical capture and bubble handlers.
9. Remove and recreate a popover anchor while the menu is open. Confirm the menu
   waits without accepting input, returns when geometry is valid, and never
   flashes at stale coordinates.
10. Open one modal and then a nested modal. Confirm backdrop input, initial
    focus, Tab and Shift+Tab wrapping, Escape event delivery, nested-modal
    restoration, and final focus restoration from the focus design.
11. Trigger Motion, presence, layout projection, and shared-layout handoffs on
    Flex, Grid, Stack, sticky, and overlay children. Confirm no snap, double
    transform, stretched decoration, lost pointer capture, or animation
    restart.
12. Reconnect with a sticky header, popover, and modal present. Confirm the new
    snapshot reconstructs layout from public descriptors, ignores retired focus
    and geometry state, and reaches the same visible and interactive result.
13. Run the mixed 1,000-item performance fixture. Confirm stable frames are
    silent, dirty work is coalesced, scrolling allocates no managed objects,
    and diagnostics identify only public container and item IDs.

Grid item margins participate once in alignment and sizing. A fixed-size item
keeps its authored border-box size; an item with automatic width fills the
cell width remaining after its margins.
