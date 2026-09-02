# Reactant Focus and Navigation

Reactant needs a declarative input-focus and navigation layer that can
coordinate components, portals, overlays, reconciliation, Motion presence, and
reconnects without replacing Unity UI Toolkit's focus engine.

The design keeps UI Toolkit's `FocusController`, focused element, focus ring,
focus events, and built-in control behavior authoritative. Rust describes
persistent policy before input begins. A synchronous Reactant handler may also
prevent the current native default, while every resulting Unity mutation stays
deferred until event dispatch is safe.

This division is the central invariant:

- Rust decides which elements belong to scopes and generic roving groups.
- Rust declares initial targets, restoration targets, explicit neighbors, and
  navigation policy.
- Unity decides whether a live element can actually receive focus.
- Unity performs focus changes, default actions, and scrolling.
- Rust may synchronously return a default-action disposition without directly
  mutating Unity's focus tree during the callback.

This project is the complete prerequisite for
[Reactant accessibility](accessibility-technical-design.md). Accessibility
later composes these focus APIs and reads the coordinator's settled state. It
does not add another input-focus owner, scope stack, modality tracker,
navigation engine, or focus wire protocol.

## Related Information

- [Battlement Reactant technical design](reactant-technical-design.md) defines
  runtime ownership, sessions, commits, desired trees, and reconnects.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines host identity, logical event routes, and physical portal placement.
- [Hooks and effects](hooks-and-effects.md) defines state batching and the
  post-commit timing available to application code.
- [Events and default actions](events-and-default-actions.md) defines the
  synchronous event disposition and deferred response boundary.
- [Reactant animations](animations.md) defines Motion gestures, presence,
  physical overlays, and reconnect reconstruction.
- [Reactant accessibility](accessibility-technical-design.md) defines the
  successor semantic and assistive-technology layer that consumes this
  completed focus foundation.
- The [focus and navigation implementation
  plan](focus-and-navigation-implementation-plan.md) delivers this design
  before accessibility implementation begins.
- The [accessibility implementation
  plan](accessibility-implementation-plan.md) records that strict dependency
  and does not reopen this project's contracts.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines `ElementRef`,
  queued host actions, geometry observation, and scrolling actions.
- [Host facades](host-facades.md) defines order-independent Reactant host
  authoring and private lowering to Battlement UI values.
- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the snapshot, command, event, and Unity UI Toolkit host contract.
- [Battlement technical design](../technical-design.md) defines transport,
  command batches, input gating, reconnects, and controller input.
- [Ditto technical design](../ditto-technical-design.md) defines released-player
  input injection and observable black-box assertions.
- The [settings mockup][mockup] at commit
  `2451ea9cc6f76b356b1102ee37b82c478853122a` supplies behavioral evidence for
  modal restoration, roving tabs, exiting inert panels, listboxes, and
  keyboard-only focus styling. Its React implementation is not an architecture
  dependency.
- Unity's [focus order][unity-focus-order], [focus events][unity-focus-events],
  [navigation events][unity-navigation-events], and
  [runtime input FAQ][unity-input] define the native behavior preserved by this
  design.

[mockup]: https://github.com/thurn/mockups/tree/2451ea9cc6f76b356b1102ee37b82c478853122a
[unity-focus-order]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-focus-order.html
[unity-focus-events]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Focus-Events.html
[unity-navigation-events]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-Navigation-Events.html
[unity-input]: https://docs.unity3d.com/6000.0/Documentation/Manual/UIE-faq-event-and-input-system.html

## Goals and Constraints

The target behavior is complete focus management for ordinary forms,
controller-driven games, composite widgets, and stacked overlays.

The design must provide:

- focusability and sequential order without a second focus ring;
- initial and programmatic focus;
- stable focus across keyed reconciliation and physical reparenting;
- logical scope membership through same-panel portals;
- modal and non-modal scopes with trapping, looping, exclusion, and
  restoration;
- nested overlay behavior tied to visible stacking;
- deterministic handling of hidden, exiting, removed, and reconnected hosts;
- distinct Tab and directional navigation behavior;
- controller-only operation from initial focus through directional movement,
  submit, cancel, held repeat, and reconnect;
- native automatic neighbors and declarative explicit neighbors;
- roving focus for tabs, menus, radio groups, toolbars, listboxes, and trees;
- keyboard/controller-only focus-visible styling;
- automatic reveal through nested scroll views;
- black-box observability in Rust, Unity, and Ditto.

The following constraints determine the architecture:

- Unity `6000.5.8f1` and Input System `1.20.0` are the host baseline.
- UI Toolkit owns one focus controller per panel.
- Reactant event handlers execute only after Unity has sent an event to Rust.
- Reactant cannot run Rust code between Unity mutation and paint.
- Input is already disabled while Battlement applies a snapshot or reconnect.
- Backward compatibility and protocol version negotiation are not required.
- Cross-panel focus scopes and roving groups are rejected. Reactant does not
  invent a process-wide focus controller above UI Toolkit.
- Controller UI navigation uses one active `Gamepad.current`. Reactant does not
  provide independent focus, selection, or navigation state per controller.

## System Terms

This section defines the repository concepts used by the focus contract. The
linked Reactant appendices remain normative for their complete behavior, but an
implementer does not need to infer these terms.

- A **desired tree** is the logical host tree produced by one Rust render. It
  contains stable host IDs, keys, properties, logical children, portal edges,
  and ref targets before Unity mutations are generated.
- The **committed logical tree** is the last desired tree Unity acknowledged.
  Reactant routes application events through this tree, including across
  portals.
- An **entry** is one Rust runtime turn caused by startup, an event, queued
  work, or reconnect. It may render and emit at most one ordered UI commit.
- A **commit** is one transaction envelope containing ordered command groups.
  A group completes before the next starts. Player input is disabled while a
  snapshot or reconnect commit is being installed.
- An **object ID** is the stable `ObjectId` assigned to a keyed host. Keeping a
  key keeps the ID and native `VisualElement`; replacing the key replaces both.
- A **document** owns one physical UI Toolkit tree. Multiple documents may
  render on one native panel and therefore share one `FocusController`.
- A **panel ID** is the stable `ObjectId` assigned in the snapshot to one
  logical native panel across reconnects. Documents that share a
  `FocusController` carry the same panel ID; documents on different panels may
  not. The Unity host validates that declared sharing matches actual panel
  identity.
- A **portal** keeps a child in its logical parent tree but attaches its host to
  another physical container. An external portal binding names a container
  supplied by a snapshot rather than created by Reactant.
- An **element ref** is a stable Rust handle to desired host identity. Ref
  actions are queued during an entry and execute after that entry's host
  mutations; Rust never receives a mutable `VisualElement`.
- A **session** is one connection generation between Rust and Unity. A complete
  snapshot starts it. Receipts acknowledge command batches and focus report
  sequences. Reconnect creates new native objects in a new session while
  preserving acknowledged logical bookmarks.
- **Suspense-retained hidden content** remains logically mounted while its
  fallback renders, but its physical hosts use hidden display and are inert.
- A **Motion presence exit** begins after logical removal. Hosts may remain
  physically attached for animation, but handlers, focus, picking, and
  navigation participation end at logical removal. Successor subsystems use
  the same logical-removal boundary.
- A **settle boundary** is the end of the Unity update in which a commit or
  input event finishes native default actions, focus changes, reveal scrolling,
  and one coalesced focus report.

The later accessibility subsystem consumes only committed focus state. It may
query effective inertness, the active modal scope, the focused host, modality,
and focus-visible state at this boundary. Those queries are read-only and do
not make accessibility a focus authority.

Focus metadata outside an authored scope belongs to an implicit panel-root
scope. It has no anchor, containment, exclusion, or opener restoration. It
provides one activation domain for root `auto_focus`, fallback, and reconnect
bookmarks without adding a synthetic native focus target.

## Ownership Boundary

Reactant supplies policy that UI Toolkit does not know, while UI Toolkit keeps
all low-level focus state.

The project boundary is exact:

| Area | Focus and navigation | Accessibility successor |
|---|---|---|
| UI Toolkit input focus | Owns focus eligibility, movement, reports, and reconnect | Observes and submits validated requests |
| Tab and directional navigation | Owns native-ring boundaries, neighbors, geometry, and roving position | Selects completed focus declarations in pattern hooks |
| Scopes and overlays | Owns activation, stacking, effective inertness, restoration, and fallback | Maps semantic presentation to the active scope |
| Semantic tree | Exposes no roles, names, states, relationships, or semantic wire fields | Owns canonical semantics and Unity projection |
| Assistive-technology focus and actions | Exposes only the coordinator request surface | Owns accessibility focus, actions, proposals, and notifications |
| Session protocol | Owns `UiFocusSnapshot`, plan updates, reports, and bookmarks | Carries a separate snapshot that depends on one focus generation |

### Unity remains authoritative

Unity owns:

- `FocusController.focusedElement`;
- `VisualElement.focusable`, `tabIndex`, `delegatesFocus`, and
  `canGrabFocus`;
- the panel's native focus-ring ordering;
- `FocusInEvent`, `FocusEvent`, `FocusOutEvent`, and `BlurEvent`;
- keyboard and controller event synthesis;
- built-in control editing, selection, submit, and cancel behavior;
- the ordinary automatic directional-navigation target;
- the final live eligibility check immediately before `Focus()`; and
- physical scrolling through `ScrollView.ScrollTo`.

Reactant never mirrors `focusedElement` as an authority. A cached Rust focus
bookmark is only reconnect input and diagnostics. Unity's next focus report
replaces it.

### Rust owns declarative policy

Rust owns:

- stable scope, group, item, and explicit-neighbor identities;
- logical ancestry and ordered logical group membership;
- which scopes are modal or non-modal;
- trapping, looping, outside exclusion, and restoration policy;
- initial, fallback, and explicit neighbor references;
- the application-selected item that seeds a roving group;
- whether a focused element should be revealed by scrolling; and
- the complete focus plan sent in a reconnect snapshot.

Rust must finish these decisions before returning the command batch whose
result can receive input. Resolution of a predeclared focus move does not depend
on a fresh Rust policy decision. A subscribed application handler may still run
synchronously in Rust and return the current event's default-action disposition.

This is an ownership and event-phase rule, not a claim that calling Rust is too
slow. The live decision occurs after native controls have had first refusal and
depends on the current `FocusController`, native focus-ring or geometry
candidate, effective eligibility, and Unity-owned roving position. Keeping that
state in the panel coordinator avoids a second focus authority and also works
when no application handler is subscribed.

A future application requirement may justify a narrowly typed synchronous
focus proposal alongside the event disposition. Such a proposal would name one
target; it would not carry arbitrary response commands. Unity would retain it
until the root bubble callback, discard it if a native control consumed or
prevented the event, and validate the originating focus, plan generation,
same-panel ownership, active scope, and live target eligibility. Only after the
Rust call returned would the coordinator call `Focus()` and prevent the later
native default. This design adds no such proposal because its current APIs need
only synchronous prevention and predeclared navigation policy.

### Unity owns synchronous coordination

Each panel has one **focus coordinator**, a Unity-side policy interpreter that
filters and augments native focus behavior without replacing the panel's focus
controller.

The coordinator:

- installs the latest generation-checked focus plan;
- indexes object, scope, group, and item records;
- captures native focus and focus-ring order before host mutations;
- preserves or repairs focus after mutations;
- enforces the active modal scope;
- resolves scope-boundary Tab movement and declared directional neighbors;
- updates roving `tabIndex` values synchronously;
- tracks input modality and focus-visible state;
- reveals focused elements in native scroll views; and
- reports the final native state to Rust.

The coordinator also exposes a Unity-internal, read-only integration surface
for successor subsystems. It resolves current focus eligibility, effective
inertness, active modal membership, modality, and settled focused host. A
successor may request focus through the same validated coordinator operation
used by `ElementRef::focus_with`; it may not call `VisualElement.Focus()` or
maintain parallel focus state.

The coordinator calls `VisualElement.Focus()` or `Blur()`. It never assigns a
parallel focused-element field or synthesizes a replacement focus event.

### Unity event integration

The coordinator installs one callback set on each panel root. The ordering is
part of the host contract for Unity `6000.5.8f1`.

1. A trickle-down `PointerDownEvent` callback records pointer modality before
   UI Toolkit performs pointer focus.
2. The existing Input System bridge stamps keyboard or controller source before
   it dispatches `NavigationMoveEvent`, `NavigationSubmitEvent`, or
   `NavigationCancelEvent`.
3. The Reactant coverage callback synchronously submits a subscribed application
   event during root trickle-down and applies its default-action disposition.
4. Native target callbacks and `ExecuteDefaultActionAtTarget` run before root
   bubble callbacks. Built-in controls may stop propagation or prevent the
   default there.
5. A root bubble `NavigationMoveEvent` callback sees only events that reached
   it. It applies a roving, explicit-neighbor, or containment rule only when the
   event is not already prevented.
6. When the coordinator focuses a declared destination, it calls
   `PreventDefault()` before the later UI Toolkit default action. Otherwise it
   does nothing and UI Toolkit completes ordinary navigation.
7. Native focus events complete through UI Toolkit. Reactant observes them and
   reads `FocusController.focusedElement` at the settle boundary.

The concrete interception paths are:

| Input | Interception path | Policy use |
|---|---|---|
| Tab / Shift+Tab | bubble `KeyDownEvent` after target default-at-target | query native Next/Previous; apply only a scope boundary |
| arrows, D-pad, stick | bubble `NavigationMoveEvent` | roving, neighbor, or directional containment |
| Home / End | bubble `KeyDownEvent` after target default-at-target | first/last item in the current roving group |
| controller submit | native `NavigationSubmitEvent` target path | native activation; no generic focus move |
| controller cancel | root Reactant coverage path | logical cancel handling; no implicit scope close |

The callback runs only when the event reached the root and is not already
prevented. Tab outside a scope boundary is untouched. Home or End outside a
roving group is untouched. When a listed policy actually supplies or retains a
destination, the callback calls `PreventDefault()` before UI Toolkit's later
default action. The Unity conformance fixture fails if target controls no
longer receive their default-at-target first or if root prevention can no
longer suppress the later default.

The baseline exposes event propagation publicly but does not expose all native
focus-ring queries publicly. The Unity host therefore contains one
version-pinned `NativeFocusRingAdapter`. It binds once at startup to
`VisualElementFocusRing.GetNextFocusable` and related native traversal in the
installed UI Toolkit assembly. The adapter returns native candidates; it does
not reproduce the ordering algorithm. Cached delegates allocate nothing during
input.

Host startup fails with a named unsupported-Unity diagnostic if that binding or
the event-order conformance fixture fails. Reactant must not silently switch to
a Rust ring, a C# replica ring, reflection on each event, or a guessed
destination. An upgrade may replace the adapter with a public API when Unity
exposes one.

## Focusability and Sequential Order

Reactant exposes UI Toolkit's existing properties unchanged. Native defaults
remain different for different control classes.

- `focusable = true` makes a host eligible for programmatic focus when Unity's
  live `canGrabFocus` check also succeeds.
- `tab_index < 0` removes a host from sequential Tab navigation while leaving
  programmatic focus eligibility to `focusable`.
- `tab_index == 0` participates in the normal native focus-ring order.
- `tab_index > 0` uses UI Toolkit's positive-index ordering.
- `delegates_focus = true` lets UI Toolkit select the delegated descendant.

Reactant recommends `0` for an ordinary tab stop and `-1` for a programmatic or
roving-only target. Positive values remain supported because they are native UI
Toolkit behavior, but Reactant does not build another numeric ordering scheme.

Sequential order follows the physical Unity tree and native focus ring. A
portal can therefore change sequential order without changing component or
event ancestry. This matches React portals: focus follows the physical DOM,
not the React component tree.

An element is **live focus eligible** when all of these conditions hold at the
moment Unity considers it:

- the object exists and is attached to the expected panel;
- UI Toolkit reports `canGrabFocus`;
- no logical ancestor is explicitly inert;
- the host is not in Suspense-retained hidden content;
- the host is not retained solely for a Motion exit; and
- the host is inside the active modal scope, when one exists.

An element is **sequentially eligible** when it is live focus eligible and its
effective native `tabIndex` participates in the ring. An enabled inactive
roving item remains live focus eligible with effective `tabIndex = -1`; it can
receive programmatic, explicit-neighbor, or roving focus without first becoming
an ordinary Tab stop.

`opacity: 0` does not make an element ineligible. `display: none`, hidden
visibility, disabled hierarchy, explicit inertness, and detached panels do.

## Public Rust API

The public API separates reusable focus properties, scope policy, one-shot
focus options, directional neighbors, and roving navigation. The types have
private fields and fluent builders.

### Composable focus properties

`FocusProps` is the single reusable bundle for host-level focus authoring. It
exists in this project so later behavior hooks can return focus configuration
without creating another focus abstraction.

```rust
pub struct FocusProps;

impl FocusProps {
    pub fn new() -> Self;
    pub fn focusable(self, value: bool) -> Self;
    pub fn tab_index(self, value: i32) -> Self;
    pub fn delegates_focus(self, value: bool) -> Self;
    pub fn auto_focus(self, value: bool) -> Self;
    pub fn inert(self, value: bool) -> Self;
    pub fn scroll_on_focus(self, value: bool) -> Self;
    pub fn layout_direction(self, value: LayoutDirection) -> Self;
    pub fn navigation_neighbors(self, value: NavigationNeighbors) -> Self;
    pub fn roving_item(self, value: RovingFocusItem) -> Self;
}
```

Every compatible host facade exposes `.focus_props(FocusProps)`. Existing
single-purpose builders such as `.focusable(true)` and `.inert(true)` are
convenience forms that merge into the same bundle. Two bundles that assign
different values to one exclusive property are a developer error.

`FocusProps` does not contain semantic roles, accessible names, actions, or
assistive-technology focus. The successor accessibility project returns this
same type from pattern hooks and composes it beside its own semantic and
interaction bundles.

`layout_direction` is inherited through the logical focus tree and defaults to
`LeftToRight`. It affects horizontal roving interpretation only; it does not
rewrite physical neighbor references or sequential Tab order.

### Focus scope types

```rust
pub enum FocusScopeMode {
    NonModal,
    Modal,
}

pub enum FocusContainment {
    None,
    Trap,
    Loop,
}

pub enum FocusRestore {
    None,
    Opener,
}
```

`FocusContainment::Trap` prevents focus from leaving but does not wrap at a
boundary. `Loop` traps and wraps the first and last sequential members.

```rust
pub struct FocusScope;

impl FocusScope {
    pub fn non_modal() -> Self;
    pub fn modal() -> Self;
    pub fn sequential(self, value: FocusContainment) -> Self;
    pub fn directional(self, value: FocusContainment) -> Self;
    pub fn initial(self, target: &ElementRef) -> Self;
    pub fn fallback(self, target: &ElementRef) -> Self;
    pub fn restore(self, value: FocusRestore) -> Self;
}
```

`FocusScope::modal()` defaults to sequential `Loop`, directional `Trap`, and
opener restoration. `FocusScope::non_modal()` defaults to no containment and
no restoration. Builders override those defaults explicitly.

A scope is authored on its physical anchor host:

```rust
View::new()
    .element_ref(&dialog)
    .focusable(true)
    .focus_scope(FocusScope::modal().initial(&cancel))
    .child(contents)
```

The anchor participates in the scope and is the final focus fallback when it is
focusable. Every modal anchor must author `focusable(true)`, even when the
desired tree contains eligible descendants. Rust validates that property.
Unity prospectively validates attachment, panel ownership, visibility, enabled
hierarchy, and effective inertness before activation.

Only container facades expose `focus_scope`. Every host facade exposes the
remaining common focus builders.

### Programmatic and initial focus

```rust
pub enum FocusVisibility {
    Auto,
    Visible,
    Hidden,
}

pub enum FocusScroll {
    None,
    Nearest,
}

pub struct FocusOptions;
```

```rust
impl FocusOptions {
    pub fn new() -> Self;
    pub fn visibility(self, value: FocusVisibility) -> Self;
    pub fn scroll(self, value: FocusScroll) -> Self;
}

impl ElementRef {
    pub fn focus_with(&self, options: FocusOptions);
}
```

`FocusOptions::new()` uses `Auto` visibility and `Nearest` scrolling. Existing
`ElementRef::focus()` is equivalent to `focus_with(FocusOptions::new())`.
Existing `blur()` remains unchanged.

The common host builders are:

```rust
pub fn auto_focus(self, value: bool) -> Self;
pub fn inert(self, value: bool) -> Self;
pub fn scroll_on_focus(self, value: bool) -> Self;
```

`auto_focus` is a mount transition, not a persistent demand. It participates
only when the keyed host changes from absent to present or enters a newly active
modal or newly available non-modal scope. Re-rendering the same keyed host with
`true` does not steal focus.

At most one mounted `auto_focus` candidate may exist in one scope. A nested
scope owns its candidate separately. Duplicate candidates panic during desired
tree validation.

`inert` excludes the logical subtree from focus, picking, and Reactant input
subscriptions. The coordinator also exposes effective inertness to the later
accessibility subsystem. Unity stores and restores authored native values
rather than losing them when effective inertness changes.

`scroll_on_focus` defaults to `true` for keyboard, controller, initial,
restored, and fallback focus. Pointer focus never auto-scrolls. A programmatic
request can override the default with `FocusOptions`.

### Directional neighbors

```rust
pub struct NavigationNeighbors;

impl NavigationNeighbors {
    pub fn new() -> Self;
    pub fn left(self, target: &ElementRef) -> Self;
    pub fn right(self, target: &ElementRef) -> Self;
    pub fn up(self, target: &ElementRef) -> Self;
    pub fn down(self, target: &ElementRef) -> Self;
}
```

Every host facade exposes:

```rust
pub fn navigation_neighbors(self, value: NavigationNeighbors) -> Self;
```

References resolve against the same desired Reactant tree. A foreign runtime
is a Rust developer error. A detached target, cross-panel target, or target
outside the active modal scope is ineligible at use. An absent or ineligible
explicit target falls back to UI Toolkit's ordinary automatic directional
behavior.

Explicit neighbors never affect Tab order. They apply only to an unconsumed
directional `NavigationMoveEvent`.

### Roving focus

```rust
pub enum RovingKind {
    Tabs,
    Menu,
    RadioGroup,
    Toolbar,
    Listbox,
    Tree,
}

pub enum Orientation {
    Horizontal,
    Vertical,
}

pub enum LayoutDirection {
    LeftToRight,
    RightToLeft,
}

pub enum RovingActivation {
    Manual,
    Automatic,
}
```

```rust
pub struct RovingFocusGroup;
pub struct RovingFocusItem;
```

```rust
impl RovingFocusGroup {
    pub fn tabs() -> Self;
    pub fn menu() -> Self;
    pub fn radio_group(orientation: Orientation) -> Self;
    pub fn toolbar(orientation: Orientation) -> Self;
    pub fn listbox() -> Self;
    pub fn tree() -> Self;
    pub fn orientation(self, value: Orientation) -> Self;
    pub fn looped(self, value: bool) -> Self;
    pub fn activation(self, value: RovingActivation) -> Self;
}

impl RovingFocusItem {
    pub fn new() -> Self;
    pub fn active(self, value: bool) -> Self;
    pub fn disabled(self, value: bool) -> Self;
}
```

Container and item hosts use these builders:

```rust
pub fn roving_focus_group(self, value: RovingFocusGroup) -> Self;
pub fn roving_focus_item(self, value: RovingFocusItem) -> Self;
```

Items belong to the nearest logical group ancestor, including through a portal
whose target is on the same panel. Item order is logical declaration order.
This order drives arrow, Home, and End movement; physical order still drives
the surrounding Tab sequence.

A nonempty group has exactly one active, enabled item in the desired tree. Rust
panics for zero or multiple active items. An empty group is legal and has no tab
stop. Disabled items remain group members but are skipped.

Unity owns the ephemeral roving position after mount. Rust's active item seeds
it. A directional move immediately shifts effective `tabIndex` from the old
item to the new item, focuses the new item, and reports the result. A later Rust
commit changes the seed only when the application changes the desired active
item from its previous committed desired value.

Reactant stores that previous desired value and increments the wire
`seed_revision` only for such a change. An unrelated render that still declares
the old item does not undo a newer Unity-owned position. To deliberately return
to that unchanged authored item, the application calls `focus_with` on its ref
or remounts the group with a new key.

Automatic activation reports a selection request after focus moves. The focus
move does not wait for Rust. Application state and controlled selection update
in the later response.

When `focus_with`, an explicit neighbor, or a roving command targets another
enabled item, Unity first adopts that item as the ephemeral position, updates
the two effective `tabIndex` values, and then calls `Focus()`. This local move
does not change Rust's `seed_revision`. An automatic group emits a selection
request for user or programmatic moves; a programmatic move uses direction
`None`. Plan seeding, initial focus, fallback, and reconnect do not request
selection because they restore already-declared application state.

Horizontal roving movement follows the nearest inherited `LayoutDirection`.
Right advances in `LeftToRight`; Left advances in `RightToLeft`. Vertical
movement, Home, End, logical item order, and Tab order do not reverse. A tree is
a vertical one-dimensional group; hierarchy, expanded state, and semantics are
declared by the later accessibility project.

A menu group handles only its configured roving axis. Submenu patterns declare
ordinary `NavigationNeighbors` from the trigger to the first submenu item and
from submenu items back to the trigger. The later accessibility hook chooses
physical Left or Right from `LayoutDirection`; the completed coordinator still
owns and executes the move.

The presets are:

- `Tabs`: horizontal, looped, manual activation by default; Left/Right and
  Home/End move focus. Automatic activation is explicit.
- `Menu`: vertical, looped, manual activation; Up/Down and Home/End move focus.
- `RadioGroup`: authored orientation, looped, automatic activation.
- `Toolbar`: authored orientation and non-looped by default.
- `Listbox`: vertical and non-looped by default; automatic activation controls
  the active option. Type-ahead is outside this design.
- `Tree`: vertical and non-looped by default; hierarchy and expansion do not
  change the one-dimensional roving order.

An unsupported axis does not move focus and is not prevented; UI Toolkit or an
ancestor may handle it. Home, End, and a non-looped edge are handled by the
group and prevent the default even when the effective item does not change.

Native `RadioButtonGroup`, `TabView`, and other controls retain their built-in
navigation. The generic roving API is for composed controls. Applying it to a
host whose native control already owns the same navigation is a developer error.

### Focus-visible Motion

Motion adds `while_focus_visible` beside existing `while_focus`:

```rust
Button::new("Play")
    .while_focus(focused_target)
    .while_focus_visible(keyboard_focus_target)
```

`while_focus` means exact native focus from any modality. `while_focus_visible`
means exact native focus while the coordinator's local visibility decision is
true. Unity changes the gesture layer immediately without a Rust render.

Reactant does not expose or promise a USS pseudo-class for this state. Authors
use `while_focus_visible`; Ditto observes the coordinator's reported Boolean.

Rust behavior hooks can subscribe to the same settled signal:

```rust
pub fn use_focus_visible(target: &ElementRef) -> bool;
```

The hook reads acknowledged coordinator state. It does not infer modality or
install another input listener.

## Focus Plan Wire Contract

Focus policy is a first-class Battlement UI snapshot object. It is not encoded
as ad hoc event handlers or duplicated element properties.

### Complete snapshot state

```rust
pub struct UiFocusPlan {
    pub plan_id: ObjectId,
    pub generation: u64,
    pub scopes: Vec<UiFocusScope>,
    pub nodes: Vec<UiFocusNode>,
    pub roving_groups: Vec<UiRovingGroup>,
}
```

One Reactant runtime owns one stable `plan_id`. A snapshot contains the complete
plan after every referenced document and external portal container has been
declared. One plan may contain independent nodes on multiple panels; Unity
partitions it among panel coordinators. Each scope, roving group, explicit
neighbor, and reconnect bookmark must remain inside one panel. A plan with no
focus metadata is omitted.

```rust
pub struct UiFocusNode {
    pub object_id: ObjectId,
    pub logical_parent_id: Option<ObjectId>,
    pub scope_id: Option<ObjectId>,
    pub auto_focus: bool,
    pub inert: bool,
    pub scroll_on_focus: bool,
    pub layout_direction: LayoutDirection,
    pub neighbors: UiNavigationNeighbors,
    pub roving_group_id: Option<ObjectId>,
    pub roving_item: Option<UiRovingItem>,
}
```

Every host logically inside an authored scope appears, including non-focusable
backdrops, layout containers, and portal fragments. Every host affected by
explicit inertness, every roving member, every navigation source, and all
logical paths needed to connect those records also appear. Hosts in the
implicit root with no such policy may be omitted.

This complete authored-scope membership drives physical stacking intervals,
modal picking exclusion, and the effective-inert state later consumed by
accessibility; it is not inferred from focusability. `logical_parent_id` is
independent of physical Unity parentage. Unity validates that every included
object exists before activating the plan.

The supporting node records are complete wire values:

```rust
pub struct UiNavigationNeighbors {
    pub left_id: Option<ObjectId>,
    pub right_id: Option<ObjectId>,
    pub up_id: Option<ObjectId>,
    pub down_id: Option<ObjectId>,
}

pub struct UiRovingItem {
    pub active: bool,
    pub disabled: bool,
}
```

An absent neighbor means native automatic navigation. `disabled` is the
composite policy value in addition to the host's effective native enabled
state.

```rust
pub struct UiFocusScope {
    pub scope_id: ObjectId,
    pub anchor_id: ObjectId,
    pub parent_scope_id: Option<ObjectId>,
    pub mode: FocusScopeMode,
    pub sequential: FocusContainment,
    pub directional: FocusContainment,
    pub initial_id: Option<ObjectId>,
    pub fallback_id: Option<ObjectId>,
    pub restore: FocusRestore,
}
```

Scope identity is the anchor host's stable `ObjectId`. A keyed anchor therefore
keeps one scope identity. Replacing the key creates a new activation.

```rust
pub struct UiRovingGroup {
    pub group_id: ObjectId,
    pub anchor_id: ObjectId,
    pub kind: RovingKind,
    pub orientation: Orientation,
    pub looped: bool,
    pub activation: RovingActivation,
    pub item_ids: Vec<ObjectId>,
    pub active_id: Option<ObjectId>,
    pub seed_revision: u64,
}
```

Group identity is also the keyed anchor host ID. `item_ids` is logical order.
`seed_revision` changes only when Rust changes the desired active item, not on
an unrelated render. Unity ignores a seed revision it has already applied.

### Snapshot and reconnect state

The existing UI snapshot has one optional focus section:

```rust
pub struct UiFocusSnapshot {
    pub plan: UiFocusPlan,
    pub resume: Option<UiFocusResume>,
}

pub struct UiFocusResume {
    pub source_session_id: ObjectId,
    pub report_sequence: u64,
    pub modality: InputModality,
    pub panels: Vec<UiPanelFocusResume>,
}

pub struct UiPanelFocusResume {
    pub panel_id: ObjectId,
    pub focused_id: Option<ObjectId>,
    pub scope_entries: Vec<UiFocusScopeBookmark>,
    pub roving_positions: Vec<UiRovingPosition>,
    pub focus_visible: bool,
}

pub struct UiFocusScopeBookmark {
    pub scope_id: ObjectId,
    pub opener_id: Option<ObjectId>,
    pub activation_serial: u64,
}

pub struct UiRovingPosition {
    pub group_id: ObjectId,
    pub item_id: ObjectId,
    pub seed_revision: u64,
}
```

`panels` is sorted by stable panel ID. Within a panel, `scope_entries` is
ordered from oldest outer activation to newest inner activation and
`roving_positions` is sorted by group ID. Unity treats every resume field as a
candidate and revalidates it against the new complete plan.

The focus section is omitted only when there is no focus metadata and no
resume bookmark. A snapshot with focus metadata but no eligible focusable host
carries an explicit valid plan. An explicit empty plan is invalid because
omission has that meaning.

### Sparse live updates

```rust
pub struct UiFocusPlanUpdate {
    pub plan_id: ObjectId,
    pub generation: u64,
    pub changes: Vec<UiFocusPlanChange>,
}
```

```rust
pub enum UiFocusPlanChange {
    UpsertNode(UiFocusNode),
    RemoveNode { object_id: ObjectId },
    UpsertScope(UiFocusScope),
    RemoveScope { scope_id: ObjectId },
    UpsertRovingGroup(UiRovingGroup),
    RemoveRovingGroup { group_id: ObjectId },
}
```

Each live Reactant commit emits at most one update. Changes are sorted by stable
kind and object identity so serialization is deterministic. Empty updates are
omitted. A generation must equal the installed generation plus one. Duplicate,
stale, skipped, or foreign-plan updates fail the command batch.

The update is staged with visual-element mutations. It is not made active at
the instant its command is encountered.

### Native focus state reports

Unity reports the actual settled result independently of application event
subscriptions:

```rust
pub struct UiFocusState {
    pub plan_id: ObjectId,
    pub generation: u64,
    pub sequence: u64,
    pub panel_id: ObjectId,
    pub focused_id: Option<ObjectId>,
    pub scope_stack: Vec<ObjectId>,
    pub roving_positions: Vec<UiRovingPosition>,
    pub modality: InputModality,
    pub focus_visible: bool,
    pub reason: FocusReason,
    pub request_results: Vec<UiFocusRequestResult>,
}
```

```rust
pub enum InputModality {
    Pointer, Keyboard, Controller, Programmatic, Unknown,
}

pub enum FocusReason {
    Pointer, Sequential, Directional, Programmatic, Initial,
    Restored, Fallback, Reconnect, Cleared,
}
```

Each report describes exactly one panel. `scope_stack` is ordered from that
implicit panel root, when applicable, through
usable authored ancestors to the physically active modal. Occluded sibling
modals are not included. Roving positions are included only when changed since
the last acknowledged report or required for reconnect.

Programmatic ref actions use the same report channel and therefore remain
observable even when focus does not change:

```rust
pub struct UiFocusRequest {
    pub request_id: u64,
    pub target_id: ObjectId,
    pub action: UiFocusAction,
}

pub enum UiFocusAction {
    Focus {
        visibility: FocusVisibility,
        scroll: FocusScroll,
    },
    Blur,
}

pub struct UiFocusRequestResult {
    pub request_id: u64,
    pub outcome: UiFocusRequestOutcome,
    pub focused_id: Option<ObjectId>,
}

pub enum UiFocusRequestOutcome {
    Applied, AlreadyFocused, Ineligible, OutsideActiveModal,
    Detached, NotFocused, NativeRejected, Superseded,
}
```

The engine acknowledges policy reports with the highest accepted sequence in
its ordinary response envelope:

```rust
pub struct UiFocusReportAck {
    pub plan_id: ObjectId,
    pub sequence: u64,
}
```

Reports use a session-local monotonically increasing sequence. Rust ignores an
older sequence or a report for an older plan generation. The latest accepted
report replaces focused ID, scope stack, modality, visibility, and reason in
that panel's reconnect bookmark. Its sparse roving positions merge into that
panel's per-group bookmark map; they do not erase unchanged groups.

The existing transport envelope supplies `session_id` on every report, event,
acknowledgement, and response. Report and application-event sequences start at
`1` in each new session. A complete snapshot may use any nonzero generation
greater than the last Rust generation; the first is `1`. Unity accepts that
complete generation without requiring a predecessor. Live updates require
exactly `installed + 1`. Old-session packets are rejected before comparing
their plan generation or sequence.

Report sequence is session-global, so one acknowledgement covers interleaved
panel reports without collision. Input modality is also session-global because
it identifies the last physical input family. Each panel retains its own
focused ID, focus-visible decision, scope/opener stack, roving positions, and
reason. Programmatic focus updates the copied modality only in the report for
its target panel; later physical input updates the global modality and reports
every focused panel whose focus-visible decision changes.

The coordinator emits a report when the focused ID, governing scope stack,
changed roving position, modality, `focus_visible`, reason, or request result
changes.
Consecutive states coalesce, but request results remain until acknowledged.
There is no per-frame focus message.

Every normal client exchange that can carry UI events may include the latest
unacknowledged report. The engine acknowledges the highest accepted sequence in
its response. Retransmission is idempotent.

Application focus payloads add `modality` and `reason` while preserving native
`related_target_id` and `direction`:

```rust
pub struct FocusEvent {
    pub related_target_id: Option<ObjectId>,
    pub direction: NavigationDirection,
    pub modality: InputModality,
    pub reason: FocusReason,
}
```

The policy report is still required when no application subscribed to focus
events.

Directions use one closed wire enum:

```rust
pub enum NavigationDirection {
    None, Next, Previous, Left, Right, Up, Down, First, Last,
}
```

Focus payloads and selection requests both use `NavigationDirection`; there is
no separate undefined `FocusDirection` wire type.

Automatic roving activation produces this application event:

```rust
pub struct UiRovingSelectionRequested {
    pub plan_id: ObjectId,
    pub generation: u64,
    pub event_sequence: u64,
    pub group_id: ObjectId,
    pub previous_id: ObjectId,
    pub proposed_id: ObjectId,
    pub direction: NavigationDirection,
    pub modality: InputModality,
    pub reason: FocusReason,
}
```

It uses the existing application-event delivery and acknowledgement envelope.
Rust rejects a request whose session, plan ID, generation, or event sequence is
stale. Retransmission is idempotent. The public `RovingSelectionEvent` exposes
the same fields except transport-only plan, generation, and sequence values.

## Scope Activation State

A scope record alone does not imply that it can govern focus. Unity derives one
state for each record after every prospective commit:

- **Dormant:** the anchor is detached, `display: none`, hidden, disabled in its
  hierarchy, explicitly inert, Suspense-hidden, or logically removed for exit.
- **Available:** the anchor and scope are live on one panel. A non-modal scope
  remains available while mounted. A modal is available before stacking is
  considered.
- **Occluded:** a modal is available but another modal ranks above it on the
  same panel.
- **Active:** the modal is the highest-ranked available modal on its panel.

Removing the scope record makes it absent. Dormant scopes do not contain focus,
exclude outside content, receive initial focus, or retain an opener entry.

Each available scope has a total physical stacking key:

1. `UIDocument` sort order;
2. panel visual-tree paint order;
3. the highest painted physical member of the logical scope; and
4. `scope_id` as a deterministic final tie-break.

All physical members of one modal must occupy one contiguous interval in that
order. A same-panel portal may create multiple fragments, but no other modal's
member may fall between the scope's lowest and highest member. An interleaved
or cross-panel modal is a developer error. The highest key selects the active
modal; the backdrop and content are members, so the anchor need not be the
highest descendant.

Unity compares old and new states after applying all changes conceptually, not
in command order. These transitions are normative:

| Old | New | Result |
|---|---|---|
| absent or dormant | available non-modal | one non-modal activation |
| absent or dormant | active modal | capture opener, then initialize |
| active modal | occluded | retain opener entry; do not restore |
| occluded modal | active | reuse its opener entry, then validate retained focus |
| available or active | dormant or absent | close and discard nested entries |
| available | available with new stacking | activate only if modal rank changes |

One commit can change many scopes. Unity first computes the final total order,
closes removed or dormant scopes from inner to outer, updates retained entries,
then activates newly topmost scopes from outer to inner. One
`activation_serial` is assigned per actual activation and makes equal-stack
diagnostics deterministic. Reconnect revalidates old serials but does not
create a second entry for the same surviving scope bookmark.

The **governing scope** for a panel is its active modal, otherwise the nearest
available non-modal containing the relevant target, otherwise the implicit
panel root. `Active` is reserved for the formal modal state. Initial and
fallback algorithms use “governing” when they also apply to non-modal
scopes.

## Commit Lifecycle

Focus policy participates in the existing ordered Reactant commit. It does not
create a new application-visible transaction API.

### Preflight

Before executing the first UI mutation from one Reactant batch, Unity builds a
prospective shadow graph and:

1. validates the complete focus-plan update against the prospective object set;
2. resolves every hard reference, document, portal target, and panel and
   indexes resolvable soft references;
3. rejects cross-panel scopes, groups, and explicit neighbors;
4. captures `focusedElement`, its nearest owned `ObjectId`, and modality;
5. captures each affected panel's native sequential focus-ring order;
6. captures scope states, roving positions, and restoration entries; and
7. records authored focusability, picking, and `tabIndex` values that effective
   policy may temporarily override.

Preflight validates every structural command, legal parent/child relationship,
focus reference, panel relationship, modal anchor, and hard limit that can be
known without mutating UI Toolkit. If it fails, no native mutation or focus
policy change begins. The batch fails through Battlement's ordinary command
failure path.

### Mutation and activation

Unity then applies the existing host create, update, move, and destroy commands.
Focus events generated by UI Toolkit remain native events, but the event
forwarder queues commit-caused focus messages until focus finalization.

At the final command-group barrier Unity activates the staged generation,
computes final scope states, applies effective inertness and roving `tabIndex`,
and chooses one focus cause in this priority order:

1. the last queued programmatic focus or blur request in the commit;
2. initial focus for a newly created or newly revived highest modal, but not an
   occluded modal merely reactivated by an inner close;
3. opener restoration after the highest modal closes;
4. retention of the captured focused object when still eligible;
5. initial focus for a newly available non-modal only when no eligible focus is
   retained anywhere on the panel;
6. deterministic removal or eligibility fallback; and
7. initial focus for a first snapshot or reconnect without a valid bookmark.

An ineligible candidate at one priority continues within that cause's candidate
list. It does not resurrect a lower-priority scope that no longer exists. A
denied explicit request records its result, then continues at step 2 so a new
modal cannot remain uninitialized.

Unity next reveals the final target when required, releases queued native focus
events in native order, emits application events, and emits one settled focus
state report. Only then does the ordinary batch receipt permit input again.

All groups in the Reactant batch finish in one Unity update before later player
input. No partially installed policy can receive input.

Focus integration does not add general native rollback. If an unexpected Unity
exception occurs after preflight, Unity discards the staged plan, leaves input
disabled, invalidates the session, and requests a complete snapshot. It does
not claim that destroyed native objects or old focus were restored. This
fail-closed path emits one fatal batch diagnostic and no application events
from the partial commit.

Commit-caused events targeting a host that was unmounted are not delivered to
application handlers that no longer exist. The settled policy report still
records the loss and fallback. Events between surviving hosts route through the
new committed logical tree.

## Initial and Programmatic Focus

Commit-time initial focus runs only when a modal is newly created or revived, a
non-modal becomes available, a root receives its first plan, or reconnect cannot
restore a bookmark. Reactivating an occluded outer modal uses opener restoration
and fallback, not its initial or `auto_focus` candidates.

The ordered candidate list is:

1. the same retained focused object, when it remains eligible;
2. the scope's explicit `initial_id`;
3. the one newly mounted `auto_focus` descendant;
4. the first eligible native focus-ring member inside the scope;
5. the focusable scope anchor; and
6. no focused Reactant object for an implicit root or non-modal scope only.

Unity calls `canGrabFocus` immediately before focusing each candidate. A stale
or currently ineligible candidate is skipped rather than treated as a batch
failure.

A controller directional action arriving while the panel has no focused
element is the one input-time use of this ladder. The initiating action has
already set `Controller` modality. Unity skips the retained and newly mounted
`auto_focus` steps, then tries the active scope's explicit initial target, first
eligible ring member, and anchor. Success handles the action with reason
`Initial`; failure leaves it unhandled. Submit and cancel do not run this
recovery ladder.

An active modal may not be focusless. Its anchor must be authored
`focusable(true)` and prospectively attached, visible, enabled, and non-inert.
Unity validates that invariant before mutation. If every descendant candidate
fails, it focuses the anchor. An unexpected native failure to focus that
validated anchor takes the fail-closed session path; input is not exposed to a
focusless modal.

Programmatic focus and blur are one-shot `UiFocusRequest` values. Each receives
a monotonically increasing request ID when queued. Requests execute after host
mutations from their entry and before restoration or fallback finalization.

Within one commit, only the last focus-or-blur request per panel can determine
final focus. Earlier requests receive `Superseded` results. Requests in later
commits remain ordered after earlier receipts. A request whose ref is already
known to be foreign panics during Rust lowering. A ref that was valid when
queued but detached before execution receives `Detached`.

An eligible programmatic request outside the active modal scope receives
`OutsideActiveModal` and does not break the trap. A request inside an occluded
modal receives the same result. A request to a disabled, hidden, inert, or
exiting target receives `Ineligible`.

Calling `Focus()` on a host with native `delegatesFocus` lets UI Toolkit choose
the descendant. `UiFocusRequestResult.focused_id` and the settled state contain
the actual resulting owned descendant, not the requested container.

`blur()` affects focus only when its ref contains the actual focused element.
Outside a modal it clears focus. Inside an active modal, clearing would violate
the trap, so Unity immediately applies the modal fallback ladder and reports
that final target. The request result is `Applied`; the settled reason is
`Fallback` rather than `Cleared`.

A blur ref that does not contain actual focus receives `NotFocused` and changes
nothing. A non-mandatory focus target that passes the prospective checks but
does not become UI Toolkit's actual focused element receives `NativeRejected`;
finalization then retains existing eligible focus or continues its normal
cause ladder. Native rejection of a mandatory active-modal anchor takes the
fail-closed session path.

For programmatic focus, request-level `FocusScroll` is authoritative.
`ElementRef::focus()` uses `Nearest`. The host's `scroll_on_focus` controls
keyboard, controller, initial, restored, and fallback focus only.

`FocusVisibility::Auto` follows the current local modality:

- keyboard or controller makes focus visible;
- pointer hides focus-visible styling;
- programmatic focus with `Auto` retains the preceding known decision; and
- unknown reconnect state defaults to hidden unless initial focus was triggered
  by a keyboard or controller action already observed in the new session.

## Stable Focus During Reconciliation

Reactant host identity already follows keyed reconciliation. Focus stability
uses that same identity rather than component position or displayed label.

When a keyed host survives:

- the same `ObjectId` and native `VisualElement` remain authoritative;
- property updates do not refocus it;
- sibling reordering preserves focus;
- same-panel physical reparenting preserves focus when UI Toolkit does; and
- the coordinator restores that exact element after mutation when reparenting
  caused a transient native blur and the host remains eligible.

The repair focus uses reason `Fallback` only when the original object became
ineligible. A pure reparent repair retains the prior reason and focus-visible
decision so styling does not flash.

Changing a host key unmounts the old host and mounts a new one. Equal properties
or text do not make the new host the same focus target.

## Deterministic Fallback

Fallback runs when the focused element is destroyed, hidden, disabled, made
inert, excluded by a new modal, removed from its roving group, or retained only
for Motion exit.

The coordinator uses the pre-mutation native ring captured during preflight.
It evaluates this ordered list:

1. the nearest governing scope's explicit fallback target;
2. the next surviving eligible object after the removed object in the old ring;
3. the previous surviving eligible object before it in the old ring;
4. the governing scope's eligible anchor;
5. the first eligible current ring member inside the governing scope;
6. the same sequence in the nearest governing parent scope; and
7. clear focus.

Candidates outside the active modal scope are skipped. A roving group
contributes only its current active item to sequential fallback unless the
removed object
was inside that group; in that case the next, then previous, enabled group item
is tried before leaving the group.

This algorithm is deterministic for one pre-mutation tree and does not depend
on dictionary iteration or geometry ties.

## Logical and Physical Ancestry

Reactant uses two trees for distinct purposes.

The logical tree controls:

- focus-scope and roving-group membership;
- parent scope and restoration nesting;
- Reactant capture and bubble routes; and
- which application handlers remain mounted.

The physical Unity tree controls:

- panel membership;
- native sequential focus order;
- actual visual stacking and picking;
- native event dispatch before Reactant routing;
- automatic directional geometry; and
- nested scroll-view ancestry.

A portal does not create a new logical focus scope. Its descendants remain in
their logical ancestor scope when the target resolves to the same panel.

The host rejects a portal that causes one scope or roving group to span panels.
An external portal rebind is validated against the replacement snapshot before
the session commits. The error is a developer failure because there is no
single native focus controller that can enforce the declared policy.

## Focus Events Through Reactant

UI Toolkit remains the source of all focus events. Reactant changes their route,
not their occurrence or order.

- `FocusOutEvent` becomes `FocusOut` and logically propagates from root to
  target in capture, then target to root in bubble.
- `FocusInEvent` becomes `FocusIn` with the same logical phases.
- `BlurEvent` becomes target-only `Blur`.
- `FocusEvent` becomes target-only `Focus`.
- `related_target_id` maps to the nearest Reactant-owned host when one exists.
- `direction`, modality, and reason preserve the Unity-side transition data.

For portals, the target remains the physically focused host, but capture and
bubble traverse its Reactant logical ancestors. Physical portal containers do
not receive Reactant handlers unless they are also logical ancestors.

Native Unity listeners registered outside Battlement may still observe physical
propagation. Reactant does not cancel an event already delivered to them.

For one input-caused move, transport order is deterministic:

1. synchronously submit the original pointer or navigation application event;
2. if its disposition is `Continue`, apply a synchronous declared move, if any;
3. synchronously submit `FocusOut`, `FocusIn`, `Blur`, and `Focus` as Unity
   emits them in native before-change then after-change order;
4. submit a roving selection request after the focus events; and
5. attach the settled `UiFocusState` report after all application events.

Commit-caused focus events use the same focus-event order but have no preceding
input event. On receipt, Rust validates and stores the separate policy report
before invoking any application handler, then dispatches application events in
their listed order through the committed logical tree. This keeps reconnect
state even when no handler exists or a handler fails.

Rust application handlers may prevent the remaining default action of the
currently submitted cancelable event. Focus movement, containment, and
restoration still come from the preinstalled focus plan or a later deferred
response; a handler cannot mutate Unity's focus tree during its callback.

## Modal and Non-modal Scopes

Scopes define overlay focus behavior without requiring every overlay to be
modal.

### Modal scopes

The physically topmost active modal on one panel is the **active modal scope**.
Physical stacking is authoritative because it matches what pointer picking and
the player actually display.

The active modal:

- excludes all outside panel content from effective focusability;
- excludes outside content from picking and Reactant input subscriptions;
- loops sequential focus by default;
- traps directional focus by default;
- receives initial focus before input resumes; and
- pushes one opener record for restoration.

The modal backdrop belongs inside the scope when it must receive an outside
click that closes the modal. Content beneath the backdrop is outside and inert.

Effective exclusion records original focusability, `tabIndex`, and picking
values. Closing the modal restores those authored values. A concurrent Rust
property update changes the stored authored value, not the temporary effective
override. Accessibility later reads the same effective-inert decision instead
of implementing another modal stack.

### Non-modal scopes

A non-modal scope groups initial and restoration policy but does not exclude
outside content. It defaults to no sequential or directional containment.

When a non-modal scope closes, opener restoration occurs only if native focus
is still inside that scope. If the user moved focus elsewhere, closing the
overlay does not steal it back.

### Trapping and looping

Tab and Shift+Tab remain native inside a scope. The coordinator intervenes only
at a boundary:

- `None` permits the native destination outside the scope.
- `Trap` keeps focus on the current boundary member.
- `Loop` focuses the first or last eligible member and prevents the native
  boundary action.

Directional containment uses the same values. `Trap` retains the current item
when UI Toolkit's candidate is outside. `Loop` uses the opposite eligible edge
in the movement axis after explicit and native candidates fail.

## Nested Overlays and Restoration

Each panel keeps a LIFO scope activation stack. Entries contain the scope ID,
native opener ID when owned, opener panel, and the latest eligible fallback.

Opening a nested modal:

1. captures the actual native focused element as its opener;
2. makes the new physical topmost modal active;
3. makes every prior modal effectively outside and inert; and
4. resolves the inner initial target.

Closing it:

1. removes the inner scope from active policy;
2. reactivates the physically topmost remaining modal;
3. restores the opener when it still exists and is eligible there; and
4. otherwise runs deterministic fallback in the reactivated scope.

An unrelated sibling modal that becomes physically topmost supersedes the old
one even without logical nesting. Simultaneously visible sibling modals are
therefore deterministic, but authors should use logical nesting when they need
opener restoration to form a meaningful stack.

Closing an outer scope removes every nested restoration entry beneath it.
Completion of a later exit animation cannot replay a discarded restoration.

## Hidden and Exiting Elements

Logical participation ends before physical presence when Suspense or Motion
retains hosts.

Suspense-retained primary content is already rendered with `display: none`.
The focus plan marks it inert as well so no programmatic or navigation target
can reach it while fallback content is active.

When `AnimatePresence` retains a removed child:

- the child keeps its component state, effects, host IDs, and physical hosts;
- the child immediately leaves scopes and roving groups;
- effective picking and focusability are disabled for the retained subtree;
- focused content runs fallback before exit descriptors begin; and
- final physical removal performs no focus restoration.

This is an intentional Reactant difference from Motion's generic browser
behavior. Reactant-managed exiting UI is not interactive after logical removal.
An application that needs an interactive dismissal phase keeps the content in
its logical tree and changes ordinary state before removing it.

Motion's `while_focus` and `while_focus_visible` exit when fallback changes the
native focused element. Exit animation may independently animate the old
focused presentation from its captured target.

## Tab, Keyboard, and Controller Navigation

Reactant consumes logical UI actions after the Input System has interpreted the
physical device. Keyboard supplies sequential Tab actions, directional arrows,
Home, End, submit, and cancel. A controller supplies directional move, submit,
and cancel. Controller input never synthesizes Tab, Shift+Tab, Home, or End.

### Controller input contract

The Battlement Input System bridge owns raw controller processing. It converts
the D-pad and left stick into cardinal `NavigationMoveEvent` values and converts
the configured UI submit and cancel bindings into `NavigationSubmitEvent` and
`NavigationCancelEvent`. The focus coordinator consumes those normalized Unity
events; it does not read raw axes, buttons, controller glyphs, or platform names.

Dead-zone selection, dominant-axis diagonal resolution, D-pad precedence,
initial-tilt emission, held repeat, input gating, and held-state synchronization
use the controller contract in the
[Battlement technical design](../technical-design.md#pointer-keyboard-and-controller-input).
Every repeated move resolves from the then-current focused element and live
focus plan. It does not retain the origin or destination from the first tilt.

Exactly one controller participates in UI navigation: Unity's current gamepad.
When `Gamepad.current` changes, the bridge suppresses already-held buttons and
directions until they return to rest, then the new current gamepad continues the
same panel focus state. Events from two controllers are never merged into
independent cursors or focus positions. Per-controller focus, simultaneous local
multiplayer navigation, controller-to-panel assignment, and controller identity
in focus events or reports are outside this design.

Physical submit and cancel bindings remain host configuration. The focus plan
depends only on their normalized Unity events, so rebinding does not change
Reactant focus semantics. Preventing a UI event does not suppress Battlement's
separate global gameplay controller action; applications needing exclusive
capture use the input-capture contract from the events design.

The first focus plan runs the normal initial-focus ladder before input is
exposed. A controller-first screen therefore starts with its explicit initial
target, `auto_focus` target, or first eligible native ring member. Authors use an
explicit initial target when visual order is not the intended controller entry.
If focus is later cleared and a controller move arrives, the coordinator reruns
that ladder for the active scope with `Controller` modality, handles the move,
and reports reason `Initial`; it does not also apply a directional step from an
undefined origin. If no eligible target exists, the move remains unhandled.

### Tab and Shift+Tab

The panel focus ring owns ordinary next and previous movement. Reactant changes
only effective eligibility, roving tab stops, and scope boundaries.

Tab never consults `NavigationNeighbors`. A positive `tabIndex`, physical portal
placement, and native control delegation retain their UI Toolkit meaning.

### Directional navigation

Arrow keys, D-pad, and left-stick navigation produce `NavigationMoveEvent`.
Unity controls may consume the event for editing or selection before the focus
coordinator handles it.

The resolution order for an unconsumed event is:

1. the current roving group rule;
2. an eligible explicit neighbor;
3. UI Toolkit's native automatic destination, adopting the destination as its
   group's roving position when entering a group;
4. scope directional containment or looping; and
5. no focus change.

The Reactant coverage callback forwards the original navigation event during
root trickle-down. If Rust returns `PreventDefault`, Unity applies that
disposition before native target default actions and the coordinator makes no
generic move. Otherwise, the coordinator observes the event later during bubble
propagation. When it remains unconsumed and a declared destination applies, the
coordinator focuses the native element and calls `PreventDefault` in Unity so
the remaining native default cannot move focus a second time. Resulting focus
events then use the normal Reactant event path.

Keyboard arrows set modality to `Keyboard`. D-pad and stick events set it to
`Controller`. The existing native repeat cadence remains authoritative.

Keyboard arrows, D-pad moves, and stick moves use the same directional policy
after normalization. The event's source affects modality, diagnostics, and test
coverage, but never changes neighbor, containment, geometry, or roving order.

### Controller submit and cancel

Native controls receive controller submit and cancel before generic Reactant
behavior. A native button converts an unprevented submit into the same logical
`Click::NavigationSubmit` used by keyboard submit. Text and composite controls
may consume submit or cancel according to their native contract.

Manual roving focus means that moving focus does not select or activate the new
item. A manual tab, menu item, toolbar item, or listbox item must therefore use
an activatable host, normally a button with a logical click handler. Controller
submit activates that host through its native submit-to-click path. The focus
coordinator does not synthesize a second activation or selection request.
Automatic roving groups continue to request selection when focus moves and do
not need a separate submit to accept that move.

An unconsumed cancel follows the normal Reactant `NavigationCancel` logical
route. A handler may prevent its remaining native default and close an overlay
in the later deferred response. Cancel never closes a focus scope by itself.
Submit or cancel with no eligible focused target does not invent one; a
controller-first surface relies on the initial-focus rule above.

### Automatic spatial candidates

The native ring adapter asks UI Toolkit for its proposed destination. When that
destination is eligible in the active containment boundary, the coordinator
does nothing and UI Toolkit performs the move. A scope-filtered fallback runs
only when the native destination is absent or outside a declared directional
trap or loop.

The fallback uses each candidate's clipped panel-space `worldBound` after
layout, transforms, and scroll offsets. For a direction, discard candidates
whose center is not strictly in the forward half-plane. Score the remainder by
this lexicographic tuple:

1. `0` when its cross-axis interval overlaps the current interval, otherwise
   `1`;
2. forward edge-to-edge primary-axis gap;
3. absolute cross-axis center distance;
4. squared center-to-center distance;
5. physical paint-order index; and
6. `ObjectId`.

The lowest tuple wins. `Trap` retains the current element when no candidate
exists. `Loop` repeats the tuple after replacing the forward-half-plane test
with the opposite extreme edge: lowest left edge for Right, highest right edge
for Left, lowest top edge for Down, and highest bottom edge for Up.

The coordinator caches eligible bounds after layout and rebuilds lazily on the
next directional event. It invalidates the affected panel on
`GeometryChangedEvent`, attach or detach, display/visibility/enabled changes,
scope or group changes, portal moves, scroll-offset changes, and Motion
transform updates. Animation can mark the cache dirty each frame, but no scan
or rebuild occurs until navigation needs it.

For an origin outside a roving group, every enabled member remains an automatic
spatial candidate. When UI Toolkit or the contained fallback chooses one, the
coordinator adopts that destination as the group's roving position before focus
settles and updates the old and new effective `tabIndex` values. Controller
entry therefore follows visible geometry, while keyboard Tab still enters only
through the group's current sequential stop.

A panel may contain at most 16,384 automatic directional candidates. The scan
uses reusable storage. This fallback is not used for ordinary unconstrained
navigation and is not a replacement global navigation engine.

## Roving Composite Behavior

A roving group exposes one sequential tab stop while supporting internal arrow
navigation.

On plan activation Unity:

1. preserves the acknowledged native roving position when it remains enabled
   and no higher Rust seed revision arrived;
2. otherwise applies Rust's active enabled item and records its seed revision;
3. assigns effective `tabIndex = 0` to that item; and
4. assigns effective `tabIndex = -1` to every other item.

An empty-to-nonempty group must arrive with one active enabled item. The empty
group has no position. A stale or equal seed revision cannot move the native
position. A newer revision with an invalid active item fails plan validation.

Leaving a group retains its effective position. Removing or disabling the
positioned item requires the same commit to provide a new active enabled item.
If focus was in the group, Unity focuses that replacement before exposing the
commit; otherwise it only updates the group's one sequential tab stop.

Every position change, including one in an unfocused group, is carried in
`UiFocusState.roving_positions` until acknowledged. Reconnect can therefore
restore all groups without inferring position from the currently focused host.

The authored `tabIndex` values are retained underneath the effective layer and
restored when an item leaves the group.

Whenever an enabled group item receives native focus through pointer input,
programmatic focus, an explicit neighbor, or another native path, Unity adopts
that item as the ephemeral roving position before reporting the settled state.
It updates the previous and new effective `tabIndex` values even when focus did
not arrive through the group's own movement rule. An automatic group then emits
its ordinary selection request; a manual group waits for activation.

Directional movement from keyboard or controller skips disabled items. Home
selects the first enabled item and End selects the last for keyboard input;
controller input has no First or Last action. A non-looped edge keeps focus on
the current item. A looped edge wraps.

For automatic activation Unity emits a `UiRovingSelectionRequested` application
event after focusing the new item:

```rust
pub struct RovingSelectionEvent {
    pub group_id: ObjectId,
    pub previous_id: ObjectId,
    pub proposed_id: ObjectId,
    pub direction: NavigationDirection,
    pub modality: InputModality,
    pub reason: FocusReason,
}
```

Reactant routes the event from the proposed item through logical ancestry. The
application accepts the proposal by rendering matching selection state. If it
renders another active item, that explicit Rust state becomes the new seed and
Unity moves the roving tab stop without replaying the original input event.

Focus and selection are deliberately distinct for manual tabs, menus, toolbars,
and listboxes. Radio groups and automatic tabs request selection on focus move.

## Focus-visible Modality

The focus coordinator tracks the last input family locally.

- A qualifying pointer down sets `Pointer` before pointer focus occurs.
- Tab, Shift+Tab, Home, End, or arrow input sets `Keyboard`.
- D-pad, stick, submit, or cancel input sets `Controller`.
- `focus_with` sets `Programmatic` while applying its explicit visibility
  option.
- Initial, restored, and fallback focus inherit the initiating modality.

The coordinator retains both the reported modality and the preceding user-input
visibility decision. `FocusVisibility::Auto` on a programmatic request uses
that decision; `Visible` and `Hidden` replace it for the resulting focus. The
settled report always carries the resulting `focus_visible` Boolean, so no
consumer must derive it from `Programmatic`.

Pointer movement and hover alone do not change modality. A pointer down on an
already focused element hides focus-visible styling without changing native
focus and emits a coalesced state report.

A focused text field reached by pointer therefore has native focus and editing
behavior without Reactant focus-visible styling. Pressing a navigation key
while it remains focused changes modality and may reveal styling even when the
control consumes the key.

The pre-dispatch Input System stamp makes that modality change before native
control consumption. A modality-only report uses reason `Pointer` for pointer
down, `Sequential` for Tab, and `Directional` for arrows or controller moves,
while keeping the same focused ID.

The heuristic is stable Reactant behavior, not a promise to match every browser
`:focus-visible` exception.

## Scrolling Focus Into View

Focus reveal is a Unity-local post-focus operation.

The reveal decision is:

| Cause | Reveal policy |
|---|---|
| pointer focus | never |
| keyboard/controller sequential or directional focus | target `scroll_on_focus` |
| initial, restored, fallback, or reconnect focus | target `scroll_on_focus` |
| `focus_with` | request `FocusScroll`, regardless of host default |
| plain `focus()` | `Nearest` |

After focus settles, the coordinator walks physical ancestors from the target
toward the panel root. For each `ScrollView`, from innermost to outermost, it
calls native `ScrollTo` when the focused target is not fully visible in that
view's viewport.

The operation runs after layout for a newly created or moved target. If layout
is not current, the coordinator queues one panel-local reveal for the
post-layout callback in the same rendered frame. It does not send geometry to
Rust or wait for `use_geometry`.

The queued reveal stores the focused object ID and focus-state sequence. A
later focus change supersedes it. Detach, ineligibility, session replacement,
or a mismatched sequence cancels it. Several changes before layout coalesce to
the final eligible target.

Multiple focus changes in one input event reveal only the final target. User
scroll inertia is canceled only where UI Toolkit's native `ScrollTo` requires
it. Nested scroll offsets are preserved when the target is already visible.

## Reconnect Behavior

Reconnect destroys native panel objects but preserves logical Reactant state.
Focus reconstructs from the latest accepted report rather than pretending the
old native object survived.

Before `begin_session`, Reactant retains:

- the latest focused `ObjectId` per panel, if any;
- the latest scope stack and acknowledged modal opener IDs per panel;
- the latest acknowledged roving position per group and panel; and
- the session-global modality plus each panel's focus-visible decision.

The replacement `UiFocusSnapshot` contains the complete plan and
`UiFocusResume`. Rust sends only data from the highest acknowledged report in
the named source session. Unity keeps input disabled until documents, external
portal children, focus policy, Motion reconstruction, and any successor
session subsystems are installed.

Unity validates each panel's scope bookmarks from outer to inner against final
availability and physical stacking. It discards a bookmark at the first missing
or reordered scope and every nested entry after it. Opener IDs are revalidated
only when their modal becomes active. Roving bookmarks apply only when group
ID, item ID, panel, and acknowledged seed revision remain valid.

Unity then chooses independently for each panel:

1. the bookmarked focused object when it exists, belongs to the governing
   scope,
   and can grab focus;
2. the bookmarked active roving item when reconnecting inside a group;
3. the governing scope's normal initial-focus sequence; or
4. no focus.

The resulting reason is `Reconnect`. Unity emits normal native focus events for
the new native objects and one settled report per changed panel with all
restored roving positions. It does not replay old-session blur events.

A stale bookmark is an ordinary fallback, not an error. A missing external
portal target, cross-panel scope, foreign object ID, or invalid focus-plan
generation remains a session validation failure.

## Stacking and Picking Integration

Focus policy must agree with the interface users can see and reach.

Physical UI Toolkit stacking selects the active modal. The coordinator uses the
same panel and visual-tree order that pointer picking uses, after accounting for
document sort order. Style opacity does not change stacking eligibility;
display and panel attachment do.

Modal outside exclusion applies these effective layers together:

- focus eligibility and sequential membership;
- pointer picking and Reactant input subscriptions; and
- the effective-inert snapshot exposed at the settle boundary.

Applying only one layer is invalid because pointer and focus input would
disagree. The successor accessibility manager consumes the settled
effective-inert snapshot and never determines modal presentation on its own.

## Input Default Actions

Native control behavior has priority over generic navigation.

The host registers its generic directional callback in bubble propagation. A
text field, slider, list view, radio group, tab view, or other native control
may consume navigation first. Reactant does not move focus when the event was
stopped or its default was prevented.

Keyboard and current-gamepad submit and cancel events retain the same normalized
behavior:

- a native button converts submit into its logical click;
- a text field may consume submit;
- cancel does not automatically close a scope; and
- application state changes caused by submit or cancel arrive in a later Rust
  response.

An overlay handler may prevent the current cancel default and commit closed
application state. Unity removes the overlay only when the deferred response
is applied. Native removal during the callback still requires a predeclared
Unity behavior; Reactant does not infer one from `Escape` or controller cancel.

## Failure Handling and Diagnostics

Focus authoring errors are developer failures. Rust panics before emitting a
commit when the desired tree alone proves an invalid contract.

Rust rejects:

- a foreign-runtime `ElementRef`;
- duplicate scope, group, or item identity;
- multiple or missing active items in a nonempty roving group;
- multiple `auto_focus` candidates in one scope activation;
- incompatible nested roving groups;
- a scope on a leaf host;
- a generic roving policy on a native control owning that policy; and
- a foreign-runtime or structurally invalid focus reference.

Unity rejects:

- missing plan objects after prospective host mutations;
- skipped, stale, or duplicate plan generations;
- cross-panel scopes, groups, or explicit neighbors;
- a plan ID owned by another runtime;
- impossible restoration-stack structure; and
- configured hard-limit violations.

Recommended hard limits are 100,000 focus nodes, 4,096 scopes, 4,096 roving
groups, 16,384 items in one group, 256 nested scopes, and four explicit
neighbors per node. A panel may expose at most 16,384 automatic directional
candidates. Existing response-byte and hierarchy-depth limits still apply.

Reference handling depends on what Rust can know:

Scope anchors, group anchors and items, and logical parents are **hard
references**. Initial, fallback, and neighbor targets are **soft references**
because their host may be conditional or disappear between authoring and use.
A dangling hard reference fails validation. A dangling soft reference remains
encoded and is skipped when evaluated.

| Reference and discovery time | Result |
|---|---|
| foreign runtime, any builder or ref action | Rust developer panic |
| scope initial/fallback target absent or live-ineligible | encode ID, then skip live candidate |
| resolved initial/fallback outside its declared scope or panel | plan validation failure |
| neighbor absent, cross-panel, or live-ineligible | native automatic fallback |
| queued focus target detached after a valid queue | `Detached` request result |
| scope or group resolved cross-panel | plan validation failure |

Rust validates stable identity and desired topology. Unity validates prospective
panel ownership and final live eligibility. A diagnostic names which layer made
the decision.

Developer tracing records:

- plan ID and generation;
- focused object before and after finalization;
- modality, reason, and focus-visible decision;
- D-pad or stick source for controller moves and current-gamepad replacement;
- governing scope stack and physical topmost modal;
- candidate rejection reasons;
- fallback step selected;
- explicit, native, or contained directional resolution;
- scroll views changed during reveal;
- plan bytes, update counts, and validation time; and
- focus-event and report sequences.

Production diagnostics avoid displayed text. Object IDs and enum reasons are
sufficient for correlation.

## Performance Requirements

Input-event handling must not allocate managed memory after a plan is installed.
Object, scope, group, and item lookup uses prepared dictionaries and reusable
scratch buffers.

The performance contract is:

- focus coordination adds no Rust call or transport wait beyond the existing
  synchronous application-event exchange;
- no per-frame focus message or geometry sampling;
- no managed allocation for steady-state Tab, arrow, D-pad, or stick movement;
- O(1) direct policy lookup by object ID;
- O(group size) worst-case roving edge search;
- O(old ring size) fallback only when focused eligibility changes;
- sparse live updates for unchanged focus policy;
- one candidate-cache rebuild per dirty panel after layout; and
- one coalesced focus report for the final state of one input update.

The first requirement preserves panel-local authority and navigation when no
application handler is subscribed. It is not based on an assumption that the
managed-to-Rust call is slow. The existing synchronous exchange is measured by
the complete callback-latency gates in the events and default-actions design;
focus benchmarks separately measure the coordinator work that follows its
disposition.

Rust diffs focus metadata with the desired tree during normal reconciliation.
An unchanged render emits no focus command. A snapshot sends the complete plan
once because reconnect correctness is more important than sparse reconstruction.

CI records plan serialization bytes, Rust diff time, Unity validation and
activation time, candidate-cache rebuild time, navigation latency, focus-report
bytes, and managed allocations.

On the pinned Apple Silicon CI host in a non-development player build, the
release gates are:

- local direct Tab, explicit-neighbor, and roving policy resolution after the
  existing event disposition returns: 99th percentile at or below `0.25 ms`
  over 10,000 warm events;
- a contained automatic scan at the 16,384-candidate limit: 99th percentile at
  or below `4 ms` over 1,000 warm events;
- validation and indexing of a 100,000-node complete plan: at or below `50 ms`;
- focus-only reconnect finalization after host creation and layout: at or below
  `16 ms` for 10,000 focus nodes; and
- encoded focus data: at or below `16 MiB`, with the lower existing response
  limit winning.

Zero managed allocation is measured after 100 warm events. Any CI hardware
change records a new baseline with equivalent fixtures before implementation
results are measured; thresholds are not chosen from the implementation's
observed performance.

## Browser and React Compatibility

Reactant uses familiar focus concepts but does not promise browser timing where
the Rust-to-Unity boundary cannot provide it.

- Portals preserve React-style logical event ancestry.
- Sequential order follows the physical Unity tree as browser order follows
  the physical DOM.
- `auto_focus` is a Reactant mount policy applied at host commit, not a browser
  attribute applied during DOM construction.
- `ElementRef::focus()` is queued for the next eligible Reactant commit and is
  not synchronous from Rust.
- `while_focus_visible` follows the documented Reactant modality heuristic,
  not every browser user-agent exception.
- Reactant callbacks may prevent the current cancelable default. Declarative
  Unity policy remains necessary for coordination that must mutate native focus
  state during input handling.
- `FocusIn` and `FocusOut` use Reactant logical propagation. `Focus` and `Blur`
  remain target-only, and all four retain UI Toolkit timing.
- Browser `inert` inspires Reactant's behavior, but Reactant applies it through
  native focus, picking, and subscription layers. Accessibility later consumes
  the same effective-inert result.
- Positive `tabIndex` retains Unity's order even where a browser implementation
  differs in details.
- Roving focus behavior uses actual focused UI Toolkit elements. A later
  accessibility active-descendant policy may keep input focus on its owner
  without changing this coordinator contract.

## Behavioral Acceptance Scenarios

These scenarios define the client-visible contract independently of internal
implementation.

Unless a step says otherwise, **settled** means one Unity update completed and
the resulting client exchange was acknowledged. Scenarios use these public
oracles:

- Ditto `focused` and `focus-visible` object states;
- the specimen's application values and public logical event journal;
- pointer activation counters and native `ScrollView` offsets;
- Motion presence state plus rendered host visibility; and
- named session or plan validation diagnostics for rejected commits.

Unity EditMode tests own native focus-event order, picking, scroll offsets, and
the no-input-window assertion. Ditto owns production input and visible
focus/application outcomes. Rust owns logical event routes and failure values.
No one oracle is claimed to prove all layers.

### Ordinary form

- Render two text fields, a toggle, a slider, and a submit button.
- Start a fresh session with only a controller connected and observe initial
  focus on the declared first field before any pointer or keyboard input.
- Tab follows the native physical ring.
- Arrow keys edit or adjust the focused native control when it consumes them.
- Shift+Tab reverses native order.
- D-pad and stick movement use directional geometry between controls, while a
  focused slider retains controller moves it consumes for adjustment.
- Keyboard or controller submit on the button produces the same logical click.
- No Reactant policy report changes the native value by itself.

### Open and close a modal

- Pointer-activate an opener behind a modal portal.
- Opening captures the opener and focuses the explicit cancel button.
- Outside content is not focusable or pickable, and the effective-inert
  observation is true.
- A pointer attempt leaves its activation counter unchanged.
- Tab and Shift+Tab loop across the modal controls.
- Controller directional movement remains inside the modal, submit activates
  the focused button, and cancel follows the overlay's logical cancel handler.
- Closing restores the opener when it remains eligible.
- Opening from keyboard or controller gives the initial button focus-visible
  styling; opening from pointer does not.

### Nested overlays

- Open a modal, then a nested modal, then a non-modal picker inside it.
- The inner modal is the active modal and makes the outer modal inert.
- Closing the picker does not steal focus if focus moved elsewhere in the inner
  modal.
- Controller cancel closes only the top overlay whose handler accepts it; it
  does not implicitly close every active scope.
- Remove the inner modal's opener while the inner modal is open.
- Closing the inner modal uses deterministic outer-scope fallback.
- Closing the outer modal clears all nested restoration entries.

### Tablist with roving focus

- Tab enters on the one active tab.
- Controller spatial navigation entering from outside lands on the enabled tab
  selected by visible geometry and adopts it as the new roving position.
- Right and Left move focus and roving `tabIndex`, wrapping at edges.
- Home and End select the first and last enabled tabs.
- A disabled tab is skipped.
- Automatic activation emits a selection request after focus moves.
- In manual activation mode, controller movement changes only focus and
  controller submit activates the focused tab through its logical click.
- The event journal records focus events before the selection request.
- Tab then leaves the group from its current roving item.

### RTL roving, tree, and submenu neighbors

- In a right-to-left horizontal group, Left advances and Right retreats without
  changing logical item order or the surrounding Tab ring.
- A tree uses Up/Down and Home/End in one logical sequence while accessibility
  remains responsible for hierarchy and expanded state.
- A vertical menu leaves its cross axis unconsumed.
- Explicit physical Left/Right neighbors move between a submenu trigger and its
  submenu in the direction chosen from inherited layout direction.

### Controller navigation

- Begin from a fresh controller-only session and verify the initial-focus ladder
  establishes an eligible focused target before input.
- D-pad and stick input independently move through the same native automatic
  geometry and produce `Controller` modality.
- A declared right neighbor overrides the automatic destination.
- A text or range control that consumes the move retains native behavior.
- A missing explicit target falls back to the native destination.
- Holding a direction follows the configured delay and interval; every repeat
  resolves from the latest focused element and plan.
- Submit activates the focused native button exactly once. Cancel reaches the
  nearest subscribed logical overlay handler and does not close an unhandled
  scope.
- Clearing focus and then moving reruns initial focus without also taking a
  directional step.
- Disconnecting or replacing `Gamepad.current` while a direction or button is
  held emits no synthetic transition; after controls return to rest, the current
  gamepad continues the same focus state.
- No controller API, event, report, or test step exposes an independent
  per-controller focus position.
- Controller focus is focus-visible and uses the controller reveal policy.

### Focused-node removal

- Focus the middle item in a keyed sequential list.
- Remove it while preserving the later keyed item.
- Focus moves to the next surviving old-ring member.
- Removing the last item moves focus to the previous member.
- Removing every member focuses the scope anchor or clears focus.

### Exit animation

- Focus a control inside an `AnimatePresence` child.
- Remove the logical child while its physical exit lasts several frames.
- Focus moves before the exit animation begins.
- In the first retained frame, the host is still rendered, Motion reports
  exiting, and `focused` already names the fallback.
- The exiting subtree cannot receive pointer or focus input.
- Physical exit completion causes no later restoration or focus event.

### Portal reconnect

- Focus a child whose logical scope crosses a same-panel portal.
- Reconcile keyed physical moves without changing logical ancestry.
- Rebind the external portal during reconnect.
- Restore the bookmark only after the new target and plan are installed.
- Reject a rebind that moves group membership onto another panel.
- The rejected rebind produces the cross-panel session diagnostic and never
  exposes input against the replacement panel.

### Pointer, keyboard, and controller styling

- Pointer-focus a text field and observe native focus without focus-visible
  styling.
- Press Tab and observe styling on the next focused control.
- Pointer-down the already focused control and observe styling disappear
  without focus changing.
- Controller navigation makes the destination focus-visible.

### Nested scrolling

- Focus an off-screen target inside two nested scroll views through directional
  navigation.
- Reveal it from inner to outer using native scroll offsets.
- Preserve both offsets when focus later moves among already visible items.
- Pointer focus does not cause an unsolicited scroll.

### Reconnect with nested modal and roving state

- Open an outer modal, then an inner same-panel portalled modal.
- Move a tablist's Unity-owned roving position away from Rust's original seed
  and focus that tab.
- Disconnect before any unrelated application render and reconnect from the
  last acknowledged focus report.
- After one settled reconnect, the scope stack contains outer then inner, the
  inner modal is active, and the same tab is focused and focus-visible.
- A Tab or controller move continues from the restored roving position.
- If that tab is absent in the replacement plan, reconnect uses the new valid
  Rust seed, then the scope initial-focus ladder, without replaying an opener.

## Observable Automated Validation

Tests prove behavior through public state and native effects rather than private
implementation snapshots.

### Rust tests

Black-box Rust and fake-host coverage includes:

- façade authoring and exact focus-plan lowering;
- deterministic sparse updates and generation increments;
- duplicate candidate, invalid focus reference, and roving validation panics;
- keyed identity preserving plan IDs and refs;
- logical scope and event routes through same-panel portals;
- focused-node removal producing the documented fallback inputs;
- Motion exit removing focus participation before physical destruction;
- queued `focus_with` ordering after host mutations;
- reconnect bookmarks and stale-report rejection;
- sparse roving-position merge and seed-revision acknowledgement;
- programmatic request supersession and denied-result delivery;
- external portal rebind validation; and
- no-op renders emitting no focus update.

Shared Rust/C# JSON fixtures cover every union case, focus snapshot and resume
field, default omission, unknown field, invalid ID reference, generation
gap, request result, selection request, acknowledgement, and hard limit.

### Unity tests

Public EditMode fixtures inspect actual UI Toolkit state:

- `focusController.focusedElement` remains the source of truth;
- the native ring adapter binds once and fails closed when unavailable;
- target default action, root bubble policy, later default action, and settled
  report retain the required order;
- native focus-event order and related targets are preserved;
- ordinary Tab order matches the native focus ring;
- text, range, list, radio, tab, submit, and cancel defaults remain native;
- a fresh controller-only session establishes initial focus before input;
- D-pad and stick moves produce the same directional result while retaining
  distinct test sources, and held repeats resolve from current focus;
- controller submit produces one native activation, unconsumed cancel follows
  logical propagation, and native controls retain first refusal;
- changing `Gamepad.current` synchronizes held state and preserves the one shared
  focus position without creating per-controller state;
- modal exclusion, trapping, looping, stacking, and restoration work;
- reparent repair does not flash focus-visible styling;
- explicit, native automatic, and contained directional paths are distinct;
- roving `tabIndex` changes occur before the next input event;
- same-panel portals work and cross-panel plans fail before activation;
- nested scroll views reveal only the final target;
- effective inertness and active modal membership are exposed at the settle
  boundary;
- reconnect applies focus after portal creation;
- state reports coalesce and retransmit idempotently; and
- steady-state navigation allocates no managed memory.

### Ditto tests

Ditto adds two observable object states:

```toml
assert = { object = "cancel", state = "focused" }
assert = { object = "cancel", state = "focus-visible" }
```

It also adds controller navigation through the same virtual Input System used
by the released player:

```toml
navigate = { direction = "right", source = "d-pad" }
navigate = { direction = "down", source = "stick" }
controller = { action = "submit" }
controller = { action = "cancel" }
```

The controller step has no controller ID because the product contract admits
only the virtual current gamepad. Ditto fails setup if a scenario attempts to
create simultaneous controller focus or route one controller independently.

`focused` reads the production panel's actual focused element.
`focus-visible` reads the production coordinator's public observable state.
Neither state invokes `Focus()` or a private test hook.

The focus suite also observes activation counters, application event journals,
effective inertness, and scroll offsets through existing public Ditto
object-state adapters. These are observations, not coordinator commands.

Retained scenarios cover controller-only initial focus, ordinary forms, modal
open and close through submit and cancel, nested overlays, manual and automatic
roving tabs, D-pad and stick movement, held repeat, explicit and automatic
neighbors, current-gamepad replacement, node removal, exit presence, portal
reconnect, nested scrolling, and modality styling.

## Operational Safeguards

Unity's public focus-controller surface is intentionally small. The coordinator
uses root callbacks, public event propagation, public focus properties, and
`Focus()`. The sole non-public dependency is the startup-bound native ring
adapter described above. It may use one cached reflection binding in this Unity
baseline; per-event reflection and a private replacement ring are forbidden. A
Unity upgrade must pass adapter binding and native conformance before Reactant
enables focus support.

Transient blur/focus events during physical reparenting can vary by Unity
release. Reactant promises the final focused object and ordered events it
observes; the repair path suppresses only duplicate policy reports, not native
events already dispatched by UI Toolkit.

Reconnect bookmarks can be stale because a session can disappear before its
last report is acknowledged. The deterministic initial and fallback sequence
makes stale state safe and visible in diagnostics.

Large plans can increase snapshot size. Sparse nodes, deterministic updates,
existing response-byte limits, and explicit focus-plan limits bound the cost.

Cross-panel rejection limits some portal compositions. This preserves a clear
native authority boundary and avoids inconsistent focus among independent panel
controllers.

## Rejected Alternatives

The following approaches conflict with the ownership or timing requirements.

- **Centralize focus resolution in Rust.** A synchronous Rust query is fast
  enough to be technically plausible, but it would need Unity to serialize or
  mirror live native eligibility, control consumption, focus-ring and geometry
  candidates, and ephemeral roving state. Unity would still have to validate
  the answer and perform the move, splitting authority without adding a required
  capability. A future narrowly typed focus proposal remains possible under the
  validation and event-phase rules above; arbitrary focus commands do not.
- **Replace UI Toolkit's focus ring.** A second ring would diverge from native
  control defaults, delegation, panel behavior, and future Unity fixes.
- **Apply a complete Rust response during the event.** Tree, focus, and ref
  commands could invalidate UI Toolkit's active propagation path. Only the
  fixed disposition is consumed before the callback returns.
- **Synthesize Reactant focus events.** Synthetic events would lose native
  related targets, direction, timing, and external Unity listener behavior.
- **Use one key handler per component.** Ad hoc handlers cannot coordinate
  nested scopes, overlays, portals, reconnects, or controller navigation.
- **Infer scopes from physical ancestry.** Portals would silently lose logical
  membership and Reactant event ancestry.
- **Order sequential focus by the logical tree.** That would replace the native
  ring and disagree with physical UI and browser portal behavior.
- **Support cross-panel scopes.** Enforcing them requires a focus authority
  above UI Toolkit's panel controllers.
- **Maintain focus per controller.** UI Toolkit exposes one focused element per
  panel, and Reactant does not add virtual controller cursors or per-player
  selection state above it. Exactly one current gamepad drives the shared UI
  focus stream. Independent local-multiplayer navigation requires a separate UI
  and input-routing architecture.
- **Keep exiting UI interactive.** Logical removal would leave ghost controls
  reachable during Motion retention.
- **Let a successor subsystem call `VisualElement.Focus()` directly.** That
  would bypass scope, eligibility, modality, reporting, and reconnect rules.

## Completion Criteria

The focus and navigation implementation is complete only when all of these are
true:

- UI Toolkit remains the sole native focus authority.
- Every persistent focus decision is present in Unity before its input event;
  current APIs limit dynamic Rust participation to the default-action
  disposition, without treating Rust call latency as an architectural barrier.
- Ordinary controls pass native default-action conformance tests.
- Keyed reconciliation preserves focus without a visible style flash.
- Modal, non-modal, nested, portal, presence, and reconnect scenarios pass.
- Tab, directional keyboard, and controller paths follow their distinct rules.
- A controller-only session supports deterministic initial focus, D-pad and
  stick movement, held repeat, native submit, logical cancel, composite entry,
  manual activation, reveal, and reconnect without pointer or keyboard input.
- Controller navigation uses only the current gamepad and creates no
  per-controller focus or selection state.
- Every roving preset passes focus and selection-request scenarios.
- Focus-visible styling changes locally with modality.
- Nested scrolling reveals the final focused target without Rust geometry.
- Rust and C# accept the same complete and sparse wire fixtures.
- Ditto can assert actual focus and focus-visible state in released players.
- Steady-state navigation allocates no managed memory and sends no per-frame
  traffic.
- Diagnostics identify plan generation, focus reason, scope, and fallback.
- The Reactant sample documents every public focus module in the feature ledger.

## Manual QA

Use the Reactant focus specimen in a packaged macOS player and desktop WebGL
build. Start each run from a fresh engine session and use only visible controls,
keyboard input, a controller, and the specimen's reconnect action.

1. Open the ordinary form. Tab forward and backward through every control.
   Edit text, adjust the slider with arrows, toggle the checkbox, and submit.
   Restart without touching pointer or keyboard; use only a controller and
   confirm initial focus, directional movement, slider consumption, submit, and
   focus-visible styling.
2. Open the modal once with pointer, once with keyboard, and once with
   controller submit. Confirm initial focus, focus-visible styling, outside
   exclusion, directional trapping, Tab looping, cancel handling, and
   restoration.
3. Open nested modal and non-modal overlays. Move focus, invalidate an opener,
   and close in reverse order. Confirm the documented fallback and no focus
   theft from the non-modal overlay.
4. Exercise tabs, menu, composed radio group, toolbar, and listbox with keyboard
   and controller. Verify one Tab stop, geometric controller entry that adopts
   the chosen roving item, directional orientation, keyboard Home/End,
   disabled-item skipping, looping, controller submit, and manual versus
   automatic activation.
5. Test D-pad and stick navigation separately, held repeat, explicit neighbors,
   native automatic geometry, and control-consumed moves. While holding a
   direction, disconnect the current gamepad; confirm no synthetic move or
   activation. Connect a replacement, return its controls to rest, and continue
   from the same focus. Confirm no controller ID or second focus state is
   exposed.
6. Focus keyed list items, reorder them, portal them, remove the focused item,
   and remove the complete list. Confirm stable identity and fallback order.
7. Focus content that exits through Motion. Confirm focus moves before the
   animation, the exit remains visible but inert, and completion does not
   restore focus.
8. Focus off-screen content through keyboard and controller navigation. Confirm
   nested scroll views reveal it and pointer focus does not change offsets.
9. Reconnect with a modal and roving group active, including an external portal
   rebind. Without using pointer or keyboard, confirm controller focus restores
   after reconstruction with no input window before policy activation and that
   the next D-pad or stick repeat continues from the restored item.
10. Repeat pointer, keyboard, programmatic, and controller focus transitions.
    Confirm exact focus and focus-visible presentation remain distinguishable.
11. Use UI Toolkit Debugger and structured diagnostics to verify the native
    focused element, effective inertness, governing scope stack, reason,
    modality,
    and plan generation agree with visible behavior.
12. Run the retained Ditto scenarios in controlled mode. Review screenshots,
    focus assertions, event logs, allocation counters, plan bytes, and
    navigation latency before accepting release evidence.
