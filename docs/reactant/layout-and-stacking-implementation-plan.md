# Reactant Layout and Stacking Implementation Plan

This plan delivers the system specified by
[Reactant Layout and Stacking](layout-and-stacking.md). It is organized as ten
independently valid vertical tasks. Each completed task leaves the repository
passing its focused checks and the repository-wide CI entry point.

The plan assumes that the asset generator and animation designs have already
landed. It does not treat the settings mockup as an architecture. The mockup is
evidence for the porting experience that the finished sample and manual checks
must cover.

## Related information

- [Reactant Layout and Stacking](layout-and-stacking.md) is the normative
  behavior and ownership contract implemented by this plan.
- [Battlement Reactant technical design](reactant-technical-design.md) defines
  the desired tree, runtime, commit, and session boundaries.
- [Reactant host facades](host-facades.md) defines facade lowering and builder
  conventions.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines logical ancestry and physical portal placement.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines ref attachment
  and next-frame geometry observations.
- [Reactant animations](animations.md) defines Motion, layout projection,
  presence, and native transform ownership.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  protocol validation and the Unity host model.
- [Asset generator](asset-generator.md) and
  [animations](animations.md) are completed prerequisites.

## Delivery rules

Every task follows the repository's normal review and validation workflow. A
task is complete only when its public behavior and retained evidence have
landed together.

- Prefer a black-box assertion at the highest available public boundary.
- Use narrow algorithm tests only for track sizing, occupancy, placement, and
  other behavior that is difficult to diagnose through a complete fixture.
- Keep production files near 500 lines and below 1,000 lines. Split
  responsibilities into named components instead of adding more code to the
  existing oversized Reactant host catalog.
- Extract shared facade and descriptor support from
  `crates/battlement-reactant/src/host.rs` before adding the three facades.
- Keep Unity container adaptation, measurement, sticky coordination, overlay
  placement, and focus scope in separate focused types.
- Do not record task history, transitional architecture, or plan language in
  source comments or public documentation.
- Stage every intended change before running `./scripts/ci.py`.
- Retain focused test output, repository-wide CI output, and any required
  performance evidence with the task review.
- Mark a task `[DONE]` only after all of its acceptance statements pass.
- When Task 1 has landed a protocol value whose owning native task has not yet
  landed, Unity preflight rejects that host or nondefault descriptor with a
  deterministic unsupported-layout developer error before applying any native
  write. Each owning task removes only its rejection after full behavior and
  acceptance coverage exist. No protocol state ever has undefined behavior at
  an independently valid task boundary.

Target sizes below count non-test production changes. They are planning
budgets, not permission to create a large multipurpose file. A task that grows
past its budget must be split internally while preserving one independently
valid review boundary.

## Dependency order

| Task | Requires | Unlocks |
|---|---|---|
| 1. Protocol model | completed prerequisites | 2 |
| 2. Layout adapter | 1 | 3, 4, 6 |
| 3. Flex | 1, 2 | 7 |
| 4. Grid sizing | 1, 2 | 5 |
| 5. Grid placement | 4 | 7 |
| 6. Stack | 1, 2 | 7, 8 |
| 7. Sticky | 2, 3, 5, 6 | 9 |
| 8. Overlays | 2, 6 | 9 |
| 9. Integration | 3, 5, 6, 7, 8 | 10 |
| 10. Gallery and release QA | 9 | release |

Tasks 3, 4, and 6 may be developed concurrently after Task 2. Tasks 7 and 8
may be developed concurrently after Stack establishes the shared presentation
ordering contract. Task 9 is intentionally a convergence task and begins only
after all public primitives work independently.

## Shared acceptance vocabulary

The task sections use these terms consistently:

- **Focused checks** are the narrow Rust or Unity suites named by the task.
- **Repository check** is `./scripts/ci.py`, run after all intended changes are
  staged.
- **Black-box acceptance** observes public Reactant output, fake Unity state,
  real UI Toolkit geometry, event delivery, focus, or the protocol journal. It
  does not inspect private slots or renderer caches.
- **Retained evidence** is reviewable test output, a fixture, a sample state,
  or a performance report that remains useful after implementation.
- **Stable identity** means the same public `ObjectId`, native control state,
  ref attachment, and keyed component state survive a sparse layout update.

## Task 1 [DONE]: Closed protocol data model

**Prerequisites:** completed asset generator and animation work.

**Target size:** 350-500 non-test lines, divided among protocol element types,
validation, and fake-client application. Generated serialization output does
not count toward the target but must be reviewed.

Add the complete serializable vocabulary before any layout algorithm depends
on it:

- concrete `UiFlex`, `UiGrid`, and `UiStack` element variants;
- `GridTrack`, `GridAutoFlow`, `GridItem`, `StackItem`, and `Sticky`;
- `OverlayLayer`, overlay placement, and modal focus descriptors needed by
  later tasks;
- independent row and column gaps and container alignment fields;
- sparse optional item descriptors on the common visual element contract; and
- matching Rust, JSON, C#, and fake-client representations.

Validation must enforce finite values everywhere, nonnegative pixel tracks and
gaps, nonnegative collision padding, positive fraction weights and spans,
one-based lines, compatible sticky edges, and all closed enum catalogs. Stack
insets are nonnegative; Sticky insets and popover offsets remain signed.
Container alignment rejects `Align::Auto`; item alignment accepts it as
inheritance. `Prop::Reset` restores constructor defaults without replacing a
host. Protocol validation does not yet require the native layout algorithms to
exist.

**Black-box acceptance**

- Every descriptor variant round-trips through the real JSON boundary.
- Rust and C# fixtures accept and reject the same authored values.
- Unity preflight rejects every not-yet-enabled new host or descriptor without
  changing the prior native tree.
- A snapshot containing each new host kind reconstructs in the fake client.
- Set, omitted, and reset transitions produce the documented fake state.
- Structural equality suppresses commands for unchanged vectors and
  descriptors.
- A changed container descriptor emits one container patch; a changed item
  descriptor emits one child patch.

**Retained evidence**

- Protocol JSON fixtures covering every variant and invalid category.
- Rust serialization and sparse-reset tests.
- Fake-client command-journal tests proving stable IDs and native control state
  across track, stacking, and sticky metadata updates.

**Verification**

- Run the focused `battlement-ui`, `battlement-ui-fake`, protocol interop, and
  C# JSON fixture suites.
- Regenerate checked-in protocol artifacts through the repository generator.
- Stage the task and run `./scripts/ci.py`.

## Task 2 [DONE]: Native layout slots and logical-child adapter

**Prerequisites:** Task 1 `[DONE]`.

**Target size:** 400-500 non-test lines. Use separate types for logical child
bookkeeping, layout-slot ownership, and container dispatch. Do not grow the
existing host catalog beyond its repository limit.

Before the three facade tasks begin, extract shared facade and item-descriptor
plumbing from `crates/battlement-reactant/src/host.rs` into focused modules.
This extraction adds no public facade and preserves all existing host behavior.

Introduce the private native boundary shared by all special layout:

- one stable unidentified slot per direct logical child;
- a logical child list independent from native presentation order;
- one shared portal source ordinal based on zero-based root registration and
  depth-first portal preorder, exposed through every target adapter;
- target-owned stable slots for top-level Grid and Stack portal attachments,
  including measurement, indexing, detach, cleanup, and reconstruction;
- an adapter for create, move, index, destroy, portal attachment, and session
  reconstruction;
- cleanup that restores or destroys the actual public host in the correct
  order; and
- dirty-container notification without implementing a sizing algorithm.

The adapter must expose ordinary logical parent and index semantics to the
protocol. `TryGet(ObjectId)`, event targets, refs, picking, focus, and geometry
continue to resolve the actual host. Private nodes must never appear in the
protocol or fake-client model.

**Black-box acceptance**

- Create, reindex, move, reparent, destroy, and reconnect produce the same
  public hierarchy as ordinary containers.
- Moving a child between adapted containers preserves its public ID and native
  control state.
- Sorting a presentation list does not change the meaning of a later logical
  index command.
- Portal attachments from separate logical branches receive deterministic
  target-relative indices before Grid, Stack, or overlay consumers exist.
- Portal sources retain logical ancestry while targets own private slots without
  leaking another identity.
- Existing Reactant host facade examples and sparse updates remain unchanged
  after the shared extraction.
- Portal attachment uses the target adapter without adding private identities
  to snapshots or events.
- Destroying a container leaves no attached slot, subscription, or scheduled
  callback.

**Retained evidence**

- Unity EditMode fixtures for every hierarchy operation.
- An adapter fixture that presents children in a different order, then applies
  logical index changes and proves the resulting public order.
- Reconnect fixtures comparing the public tree before and after reconstruction.

**Verification**

- Run Unity hierarchy, lifecycle, portal, and reconnect EditMode suites.
- Run Rust fake-client hierarchy and Reactant portal tests.
- Stage the task and run `./scripts/ci.py`.

## Task 3: Flex facade and independent gaps `[DONE]`

**Prerequisites:** Tasks 1 and 2 `[DONE]`.

**Target size:** 250-400 non-test lines, including a focused Reactant facade
module and native gap coordinator.

Deliver `Flex` as a public Reactant host with direction, wrapping, alignment,
justification, `row_gap`, `column_gap`, and `gap`. Lower the facade to one
`UiFlex` host. Implement gaps through stable slots while leaving UI Toolkit in
charge of line breaking and flex growth.

The implementation must preserve authored child margins and remove the outer
half-gap from the container edges. Reversed directions and wrapping affect
presentation only; logical source order remains unchanged.

**Black-box acceptance**

- Public examples compile with builders in different call orders. In
  particular, `.row_gap(1.0).gap(2.0)` and
  `.gap(2.0).row_gap(1.0)` both resolve to row gap one and column gap two.
- Adjacent items and wrapped lines have exact independent gap measurements.
- Child margins add to gaps and are not rewritten.
- Direction, wrapping, item alignment, and justification match native Flex for
  fixed, growing, shrinking, constrained, and absolutely positioned children.
- Row flow uses column gap between items and row gap between lines; Column flow
  swaps those roles. Absolute children receive no gap offset.
- Mixed in-flow and absolute children resolve absolute offsets against the
  unchanged outer Flex content box while gaps stay inside the private flow band.
- Empty, all-`Display::None`, and absolute-only Flex containers apply no outer
  gap compensation. Visibility Hidden children remain in flow.
- Reversed flow preserves event, focus, and logical child order.
- Changing either gap patches only `UiFlex`, preserves every child ID, and
  performs no work after the layout settles.
- Resetting gaps returns to zero without remounting.
- Resetting direction, wrapping, alignment, justification, and gaps restores
  every Flex constructor default in native state without remounting.

**Retained evidence**

- Rust facade compilation and sparse-command tests.
- Unity geometry fixtures for row, column, wrap, reverse, margin, and reset
  cases.
- A stable-frame counter proving no ongoing gap work.

**Verification**

- Run focused Reactant facade, fake-client layout, and Unity Flex fixtures.
- Run rustdoc tests for the public Flex examples.
- Stage the task and run `./scripts/ci.py`.

## Task 4 [DONE]: Grid sizing foundation

**Prerequisites:** Tasks 1 and 2 `[DONE]`.

**Target size:** 400-500 non-test lines. Keep pure track resolution separate
from UI Toolkit measurement and scheduled layout coordination.

Deliver the first useful Grid slice:

- public `Grid` facade with explicit rows, columns, automatic track defaults,
  gaps, and default item alignment;
- stable grid slots and the private in-flow measurement element;
- fixed pixel, automatic, and positive fractional track sizing;
- default row-flow placement for children with no `GridItem`, filling the
  explicit columns in source order with a span of one and creating automatic
  implicit rows as required;
- single-track automatic and fractional contributions;
- columns resolved before wrapped row measurement; and
- bounded native scheduling with last-finite-layout retention.

This is an independently useful public slice for ordinary equal-cell lists and
settings rows. Task 4 does not add `.grid_item` or `.auto_flow` to Reactant host
facades. Those builders arrive with the complete placement contract in Task 5,
so no public builder has temporary behavior. Raw protocol values for a
nondefault `GridItem` or Column flow continue to receive Task 1's deterministic
preflight rejection until Task 5 removes that gate.

**Black-box acceptance**

- Fixed tracks retain exact size under shortage.
- Automatic tracks use preferred outer size and never shrink below it.
- Fractions divide positive remainder by weight and never become negative.
- Fractional intrinsic bases are a floor under shortage. Two `1fr` tracks with
  separate 150-pixel single-track contributions remain 150 pixels each in 200
  available pixels and report finite overflow.
- Source-order children fill one-cell areas across rows and create implicit
  automatic rows when explicit rows run out.
- Omitted and explicitly empty columns both create one implicit automatic
  column before adding implicit rows.
- Authored `auto_rows` and `auto_columns` size every implicit span-one track in
  this default Row-flow slice.
- Container `align_items` and `justify_items` place auto-sized children on the
  documented physical axes.
- Gaps are included once between tracks and excluded at container edges.
- Wrapped content is measured against final column width.
- An auto-sized Grid contributes its intrinsic size to a flex parent.
- Overflow remains finite, and a nonconverging fixture retains its last finite
  layout with one diagnostic.
- Runtime changes to tracks, gaps, implicit defaults, and container alignment
  patch only Grid and preserve container and child IDs and control state.
- Resetting those Task 4 properties restores native constructor defaults
  without remounting.

**Retained evidence**

- Pure sizing vectors for bounded, intrinsic, single-track shortage, and
  overflow cases.
- Unity EditMode geometry fixtures using real text wrapping and flex parents.
- A convergence fixture with pass counts and diagnostic payload assertions.

**Verification**

- Run Rust track-sizing tests and Unity Grid sizing fixtures.
- Run serialization and fake-client sparse-update tests from Task 1.
- Stage the task and run `./scripts/ci.py`.

## Task 5: Complete Grid placement and runtime updates

**Prerequisites:** Task 4 `[DONE]`.

**Target size:** 400-500 non-test lines. Keep occupancy and source-order scan
logic separate from measurement and native slot placement.

Complete the closed Grid contract:

- `.grid_item` and `.auto_flow` Reactant builders;
- `GridAutoFlow::Row` and `GridAutoFlow::Column`;
- non-dense source-order auto-placement;
- one-axis and two-axis explicit placement;
- positive row and column spans;
- advanced placement growth using Task 4's authored implicit-track defaults;
- per-item alignment overrides;
- explicit overlap with source-order painting;
- finite overflow; and
- ordinary rerender updates to tracks, flow, gaps, and item placement.

Desired-tree validation must reject item descriptors outside a Grid and invalid
line or span arithmetic before commit. Updating tracks or placement must reuse
the existing container and child hosts.

A direct logical child and a top-level portal attachment whose physical target
adapter is a Grid are both valid Grid placement contexts.

**Black-box acceptance**

- A mixed explicit and automatic fixture matches the normative occupancy
  sequence for both flow directions.
- The A-through-E golden example produces its exact documented coordinates and
  their transpose under Column flow.
- With no explicit minor tracks, start four and span two establishes five
  tracks rather than six.
- General-cursor items never backfill earlier holes; explicit-major items scan
  only their authored major band.
- Explicit placement grows implicit tracks and supports intentional overlap.
- Overlapping explicit areas paint and hit-test in logical source order after
  keyed reordering.
- Spans cross gaps exactly once and align children inside the complete area.
- Spanning deficits grow automatic tracks before the axis-wide fractional unit
  and match the normative finite-shortage example.
- Margins and explicit child dimensions interact with stretch as documented.
- Absolute position and nonautomatic Style offsets on a direct Grid child fail
  validation without a partial commit.
- Responsive track changes preserve typed input drafts, focus, refs, keyed
  state, and host IDs.
- A portaled top-level Grid item uses the target adapter's placement while its
  event path remains in the logical portal ancestry.
- Multiple Grid portal attachments follow ordinary children in source-ordinal
  order and receive deterministic auto-placement indices.
- Portaled Grid placement children reject absolute position and nonautomatic
  Style offsets under the same rule as logical children.
- Removing `GridItem` restores default auto-placement with one child reset.
- Resetting auto flow restores Row without remounting.
- `Display::None` removes a child from occupancy and contribution, while
  Visibility Hidden retains both without painting or picking.
- Invalid placement aborts the Reactant commit without changing Unity.

**Retained evidence**

- Occupancy property tests, compact golden placement tables, and spanning
  sizing vectors including the normative finite-shortage example.
- Unity geometry fixtures for implicit tracks, spans, overlap, alignment, and
  overflow.
- Reactant fake-client tests for responsive sparse updates and transactional
  invalidation.

**Verification**

- Run Grid occupancy, Reactant reconciliation, fake-client, and Unity Grid
  suites.
- Compile the normative Grid examples as rustdoc or integration examples.
- Stage the task and run `./scripts/ci.py`.

## Task 6: Stack sizing, placement, and isolation

**Prerequisites:** Tasks 1 and 2 `[DONE]`.

**Target size:** 350-500 non-test lines, split between facade, intrinsic sizing,
placement, and presentation ordering.

Deliver `Stack` and `StackItem` with integer order, horizontal and vertical
alignment, four optional insets, and intrinsic-size contribution. Every Stack
is an isolated stacking context. Sort presentation by `(order, logical_index)`
without changing logical hierarchy.

Desired-tree validation rejects `StackItem` outside a direct logical Stack
child or top-level portal attachment whose physical target adapter is a Stack.
It also rejects the documented Sticky and overlay combinations before commit.

Stack placement must use slots rather than authored child transforms or
positions. A private measurement element reports the maximum preferred outer
size of contributing layers to a flex parent.

**Black-box acceptance**

- Lower-order layers paint and hit-test below higher-order layers.
- Equal-order ties follow source order after keyed reordering.
- A nested child cannot paint or receive hits outside its parent's Stack layer.
- Insets and alignment produce exact child rectangles on both axes.
- Fixed width with automatic height measures wrapped preferred height at the
  final width; automatic width with fixed height resolves independently.
- Negative or non-finite Stack insets fail validation without a partial commit.
- Absolute position and nonautomatic Style offsets on a direct Stack child fail
  validation without a partial commit.
- Noncontributing layers do not enlarge an automatic Stack.
- An all-noncontributing Stack has zero intrinsic content size unless ordinary
  Style constraints size it.
- Updating order or contribution patches only the child and preserves identity,
  focus, refs, and control state.
- An invalid Stack placement aborts the commit and leaves Unity unchanged.
- `Display::None` removes Stack placement and contribution, while Visibility
  Hidden retains both without painting or picking.
- A top-level portal attachment targeting a Stack receives the target's
  placement and ordering while identity and logical event ancestry remain at
  the portal source.
- Multiple Stack portal attachments use source-ordinal indices for equal-order
  paint and hit-test ties.
- Portaled Stack placement children reject absolute position and nonautomatic
  Style offsets under the same rule as logical children.
- Removing `StackItem` resets order, alignment inheritance, insets, and
  intrinsic contribution without replacing the host.
- Resetting Stack container alignment restores constructor defaults without
  remounting.

**Retained evidence**

- Unity picking and geometry fixtures for negative, equal, and positive order.
- Nested isolation and keyed-reorder fixtures.
- Reactant sparse-command and stable-identity tests.

**Verification**

- Run focused Stack facade, fake-client, Unity layout, and pointer suites.
- Run existing event-routing and focus suites to detect logical-order changes.
- Stage the task and run `./scripts/ci.py`.

## Task 7: Same-frame sticky positioning

**Prerequisites:** Tasks 2, 3, 5, and 6 `[DONE]`.

**Target size:** 350-500 non-test lines. Keep ScrollView coordination,
containing-block calculations, and presentation sorting separate.

Deliver `Sticky` against the nearest physical ScrollView:

- one chosen inset per axis;
- a normal-flow placeholder preserving any physical parent contribution;
- on-demand placeholder slots for ordinary parents and direct ScrollView
  content, using the same container adapter rules as special layout slots;
- a ScrollView-owned presentation surface clipped to its viewport;
- containing-block clamping;
- ScrollView-wide `(order, source_ordinal)` presentation sorting; and
- local updates for wheel, drag, scrollbar, touch, and programmatic scrolling.

The public host remains the actual event, focus, ref, and geometry target.
Missing scroll ancestry, contradictory edges, invalid combinations, and
non-finite values are rejected before commit except for the documented
snapshot-attachment waiting state.

**Black-box acceptance**

- Top, bottom, left, right, and two-axis sticky fixtures clamp correctly.
- Leading and trailing formula vectors cover positive and negative insets,
  ordinary sticking, oversized items, and containing-block precedence.
- Every `with_*` builder accepts only the orthogonal axis, and removing Sticky
  resets the complete descriptor.
- Sticky combined with absolute positioning fails preflight and preserves the
  prior native tree. Relative offsets participate in the normal rectangle.
- A sticky header remains in normal-flow sizing and leaves with its containing
  block.
- Nested and portaled fixtures prove that the flow placeholder's physical
  public parent, rather than a container kind search, defines that block.
- Direct ScrollView content and children of an ordinary parent keep their exact
  normal-flow contribution while the actual host is presented as sticky.
- Overlapping sticky items use integer order and source-order ties.
- Sticky items from different physical parents use the ScrollView-wide source
  ordinal, paint above ordinary scrolling rows, and produce a total order.
- An oversized sticky item honors the containing-block edge before the viewport
  inset and retains finite overflow.
- Scrolling updates visible bounds in the same rendered frame without a Rust
  event, geometry observation, command, rerender, or managed allocation.
- Removing Sticky, moving between ScrollViews, and destruction cleanly restore
  or remove native presentation.
- Pointer capture, focus, logical events, refs, and host identity survive
  sticky updates.
- Content, font, Style, and intrinsic-size changes while sticky remeasure the
  presentation entry, update the empty flow slot, and recompute clamping.
- Sibling insertion, removal, and resize plus containing-block reflow update
  sticky placement after native layout in the same rendered frame.

**Retained evidence**

- Unity ScrollView fixtures for every edge, nested containing blocks, ordering,
  reparenting, and destruction.
- A command and geometry journal proving scroll-local silence.
- Allocation and update-count evidence for a 100-row sticky fixture.

**Verification**

- Run Unity ScrollView, geometry, pointer, and sticky suites.
- Run Reactant event, ref, reconciliation, and fake-client tests.
- Stage the task and run `./scripts/ci.py`.

## Task 8: OverlayHost, popovers, and modals

**Prerequisites:** Tasks 2 and 6 `[DONE]`.

**Target size:** 500-800 non-test lines across focused files. Keep host,
popover, and modal-focus files near 500 lines and below 1,000. The task lands
atomically; all Overlay descriptors retain the Task 1 preflight gate until the
complete Task 8 acceptance set passes.

Build overlays on the existing portal contract:

- one ordinary runtime-scoped `PortalTarget`, created before root registration
  and attached to exactly one `OverlayHost`;
- `OverlayHost` as a root-level layer with a `PortalTarget` and inner Stack;
- standard Popover and Modal layers plus Popover-tier `Overlay::layer` for
  unanchored host-filling content;
- `Overlay` wrappers for component, fragment, and conditional children;
- anchored popover side, alignment, offsets, padding, flip, and shift;
- local placement invalidation for layout, scrolling, and panel scale;
- waiting behavior for unavailable anchors;
- modal viewport filling, native focus containment, nested scopes, and focus
  restoration; and
- picking behavior in which an empty overlay host is transparent while a modal
  backdrop blocks lower content.

Open, close, Escape, outside click, selection, and application state remain
ordinary Reactant events. The layout system performs no implicit dismissal.

**Black-box acceptance**

- Portaled content escapes a clipped application subtree without changing its
  logical event path or context.
- A target attaches to exactly one OverlayHost, and its stretching,
  noncontributing root layer cannot enlarge the application Stack.
- The document's sole OverlayHost is the final child of its root Stack and
  remains above an application sibling authored at the highest integer order.
- Separate registered document roots may each own one independent target and
  OverlayHost only when they render into different panels. A second host in one
  panel rejects its complete mutation group without changing native state.
- Popovers match every side and alignment, flip to the better side, shift
  within padding, and retain finite overflow when oversized.
- Main and cross offsets follow the documented physical signs; equal flip
  overflow keeps the requested side.
- Flipping reprojects main offset away from the chosen side while preserving
  the physical cross-offset direction.
- Omitted placement resolves to `bottom_start` with documented defaults.
- Disabling flip retains the requested side, and disabling shift retains the
  unshifted cross-axis position even when either overflows.
- Collision padding larger than half a host axis collapses that padded interval
  to its center and still produces finite placement.
- Popover preferred border-box size includes child outer size, wrapper authored
  content-box constraints, padding, borders, and width-dependent height exactly
  as documented; collision does not shrink it implicitly.
- `.style(Style)` and mirrored visual builders constrain the overlay wrapper
  independently from child Style.
- Every wrapper rejects margin, position, offset, display, and visibility
  declarations. Unanchored and Modal wrappers also reject authored dimensions
  because their private slots fill the host.
- Scrolling an anchor updates placement in the same rendered frame with no Rust
  round trip.
- Missing and hidden anchors make the wrapper hidden and unpickable until both
  geometries are current.
- A hidden anchor returning visible retries placement even when its bounds did
  not change.
- A focused popover entering waiting restores eligible anchor focus or clears
  panel focus, and returning to visible does not autofocus it.
- Popover mount never moves focus. Focused popover unmount restores an eligible
  anchor and otherwise follows ordinary panel focus behavior.
- Empty overlay space passes input through; interactive descendants receive it;
  a full modal backdrop blocks lower layers.
- Host-filling overlay wrappers ignore picking. A modal without an authored
  backdrop passes empty-space pointer input to lower layers.
- Initial modal focus, Tab wrapping, nested containment, and restoration follow
  the normative eligibility and fallback order.
- A modal with no eligible descendants focuses its negative-tab-index wrapper
  and retains focus there during Tab traversal.
- A negative-tab-index descendant may be an explicit or last-focused target but
  is skipped by fallback search and Tab traversal.
- `Overlay::layer` fills the Popover layer without anchoring and accepts an
  ordinary nested Stack for application-defined alignment and ordering.
- Anchor resize, popover-content resize, overlay-host resize, and panel-scale
  changes recompute placement after current native layout.
- A cross-panel anchor rejects the complete mutation group, emits the
  documented IDs, and preserves the previous native tree without stale new
  content.
- Simultaneous overlays order by modal rank, wrapper kind, and source ordinal.
  A modal-local popover paints above its owner and below every higher modal.
  Only Modal items participate in active-modal order.
- Attaching an inactive lower modal does not move focus. Keyed source-order
  changes suspend and activate existing scopes using the documented transfer
  sequence.
- Removing an inactive modal performs no focus operation while a higher modal
  remains active.
- The panel-level application return target survives insertion of a lower
  inactive modal and removal of the higher modal, then restores when the final
  modal closes.
- Same-panel portaled logical descendants remain inside the modal focus scope
  and use logical preorder for Tab ties; cross-panel descendants are outside it.
- An attached out-of-scope initial ref, cross-panel focus ref, or cyclic
  popover anchor rejects the complete mutation group and preserves native state.
- Resetting popover placement restores `bottom_start` defaults, and removing
  modal focus refs restores fallback focus behavior without remounting.
- First-mount waiting remains layout-participating, obtains current popover
  geometry, and becomes visible without an external geometry change.
- Escape reaches logical handlers and never closes a modal by itself.

**Retained evidence**

- Unity placement matrices for side, alignment, flip, shift, oversize, and
  waiting states.
- Portal event traces and picking fixtures.
- Modal keyboard, nesting, destruction, and focus-restoration fixtures.
- A no-round-trip scroll journal for anchored content.

**Verification**

- Run Unity overlay, pointer, keyboard, focus, portal, and geometry suites.
- Run Reactant portal, event, context, ref, and state-preservation tests.
- Stage each independently valid slice and run `./scripts/ci.py` before marking
  the complete task `[DONE]`.

## Task 9: Cross-system integration and reconstruction

**Prerequisites:** Tasks 3, 5, 6, 7, and 8 `[DONE]`.

**Target size:** 300-500 non-test lines. Integration work belongs in focused
adapters owned by the affected systems, not in one central conditional catalog.

Prove and complete composition across the complete runtime:

- portal attachment and external portal targets;
- ref attachment and final visible geometry;
- Motion and CSS-style animation transform layers;
- layout projection and shared-layout handoffs;
- presence retention and `PopLayout`;
- logical event and focus ancestry;
- keyed reconciliation moves and equal-order ties;
- Suspense hidden-tree retention;
- failed render and transaction rollback; and
- authoritative snapshot reconstruction and reconnect retirement.

Apply layout, projection, and animation in the normative order. No integration
may write authored child Style fields or serialize private native nodes.

**Black-box acceptance**

- A portaled, animated, focusable child reports final visible ref bounds while
  events follow committed logical ancestry.
- Flex, Grid, Stack, sticky, and overlay moves preserve layout projection and
  authored Motion without double transforms or animation restarts.
- An anchored popover follows a layout-projected or Motion-animated anchor in
  the same frame while retaining its own projection and animation.
- Presence modes retain or remove layout contribution exactly as specified.
- Keyed children preserve component and native state through responsive tracks,
  stack reordering, sticky movement, and ordinary same-target portals.
- A failed render leaves the committed native layout unchanged.
- Reconnect rebuilds all private presentation from public descriptors, clears
  retired geometry and focus records, and leaks no handlers or nodes.

**Retained evidence**

- Reactant black-box composition tests spanning events, refs, focus, keys,
  portals, presence, and animations.
- Unity transform-order fixtures for every placement type.
- Before-and-after reconnect journals and native leak assertions.

**Verification**

- Run Reactant motion, portal, ref, event, Suspense, reconciliation, and session
  suites.
- Run Unity layout projection, physical motion, geometry, lifecycle, and portal
  suites.
- Stage the task and run `./scripts/ci.py`.

## Task 10: Sample gallery, performance, and release evidence

**Prerequisites:** Task 9 `[DONE]`.

**Target size:** 300-500 non-test lines for reusable sample components. Split
sample screens and fixture support so no source file approaches the repository
limit.

Add a Reactant layout gallery that demonstrates one coherent application flow:

- a fixed tab grid;
- responsive settings rows that change track lists at runtime;
- a sticky table header inside a ScrollView;
- a dropdown that escapes clipping through an OverlayHost;
- layered decorative and interactive Stack content; and
- a viewport-filling modal with focus containment and restoration.

The sample must use the public API exactly as application authors will use it.
It may not call native layout helpers, inspect private slots, or rely on test-
only descriptors.

Complete the automated matrix from the design, add Unity validation fixtures,
and record performance evidence. The performance scenario contains at least
1,000 mixed Grid children, 100 sticky rows, nested stacks, and ten anchored
overlays.

**Black-box acceptance**

- The complete sample flow preserves visible geometry, source-order focus,
  logical event traces, refs, keyed state, presence, and animations.
- Stable frames perform no layout work.
- Each dirty item is measured at most once per axis per native layout
  generation.
- Scrolling sticky and anchored content causes no Rust round trip.
- Ordinary steady layout and scrolling allocate no managed objects.
- Diagnostics identify public container and item IDs, never private nodes.
- Every public example in both design documents matches the shipped facade.

**Retained evidence**

- The checked-in Reactant layout gallery and its interaction script.
- Rust black-box composition coverage and Unity validation fixtures.
- A performance report with frame counts, dirty-container counts, measured
  items, allocations, and Rust traffic.
- Completed release and Manual QA checklists with platform and Unity version.

**Verification**

- Run the complete focused Rust and Unity layout matrices.
- Run the sample interaction flow from a clean session and after reconnect.
- Run the performance fixture in the repository's standard measurement mode.
- Check all local documentation links and compile public Rust examples.
- Stage all intended changes and run `./scripts/ci.py`.

## Manual QA

1. Start the Reactant layout gallery at its baseline viewport. Verify the fixed,
   automatic, and fractional tracks, independent gaps, source-order focus, and
   ordinary control behavior.
2. Resize the viewport and toggle large text. Verify responsive track changes
   preserve focus, drafts, refs, keyed state, host IDs, and active animation.
3. Exercise Grid auto-placement, explicit lines, spans, implicit tracks,
   alignment, overlap, and overflow in both flow directions.
4. Compare interactive Stack layers at negative, equal, and positive orders.
   Verify hit targets, nested isolation, insets, alignment, and intrinsic-size
   contribution.
5. Scroll the sticky specimen by wheel, drag, scrollbar, touch, and programmatic
   update. Verify same-frame placement, containing-block clamping, ordering,
   final ref bounds, and the absence of continuous Rust traffic.
6. Open the clipped dropdown at every viewport edge. Verify portal escape,
   requested placement, flip, shift, waiting behavior, logical events, and focus
   restoration.
7. Open nested modals. Verify the backdrop blocks lower input, initial focus and
   Tab wrapping stay in the active scope, Escape reaches the application, and
   closing restores the correct prior focus.
8. Trigger Motion, layout projection, presence, and shared-layout handoffs on
   every layout primitive. Verify placement and authored animation compose
   without snaps, double transforms, or restarts.
9. Reorder keyed children and remove layout descriptors. Verify public identity
   and native control state remain stable while presentation resets correctly.
10. Reconnect while sticky content, a popover, and nested modal scopes are
    present. Verify reconstruction reaches the same visible and interactive
    result without retired focus, geometry, handlers, or private nodes.
11. Run the mixed performance scenario. Verify stable-frame silence, bounded
    passes, one measurement per dirty item per axis per native generation, zero
    steady managed allocation, and no scrolling round trip.
12. Review diagnostics from invalid descriptors, a nonconverging intrinsic
    fixture, and a cross-panel anchor. Verify each message identifies public
    hosts and the actionable offending value without exposing private nodes.
