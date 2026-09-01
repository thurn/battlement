# Reactant Reconciliation, Events, and Portals

This appendix defines how a rendered Reactant tree retains identity, becomes
Battlement UI commands, receives Unity events, and renders children outside
their logical parent. It is part of the
[Battlement Reactant technical design](reactant-technical-design.md).

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the host snapshot, mutation commands, subscriptions, and event payloads.
- [React: preserving and resetting
  state](https://react.dev/learn/preserving-and-resetting-state) defines React's
  position, type, and key identity model.
- [React: `createPortal`](https://react.dev/reference/react-dom/createPortal)
  defines physical placement with logical context and event ancestry.
- [Unity event handling][unity-events] defines native target, trickle-down, and
  bubble-up propagation.
- [Components and rendering](component-authoring.md) defines render values and
  keys as application code sees them.
- [Host façades](host-facades.md) defines host-owned keys, refs, portal targets,
  handlers, and private lowering.

[unity-events]: https://docs.unity3d.com/Manual/UIE-Events-Handling.html

## Committed identity

Reconciliation answers whether a newly rendered node is the same logical node
as one in the committed tree. Reused component nodes keep hook state, effects,
refs, IDs, and host descendants. Reused host nodes keep their `ObjectId`.

Identity is local to one sibling sequence. A keyed node uses:

```rust
(key_type_id, key_value, node_type_id)
```

An unkeyed node uses:

```rust
(sibling_position, node_type_id)
```

`sibling_position` is the absolute index in the complete logical sibling
sequence. Keyed siblings and empty render values still occupy positions when
Reactant calculates an unkeyed sibling's identity.

`node_type_id` is the concrete component `TypeId` for a component and the
`UiElement` variant for a host. A `Label` and `Button` with the same key are
different nodes. Changing either the key or type unmounts the old node and
mounts the new one.

`Fragment`, `Suspense`, `ErrorBoundary`, portal, provider, and keyed-range
records each use one fixed semantic marker independent of their generic child
type. `Rc`, `Either`, and `Node` preserve the wrapped value's one position;
`Node` retains the erased child's descriptor. Changing one of these wrapper
types does not itself remount a matching nested component.

Logical sibling positions are assigned recursively from left to right:

- a component, host, fragment, boundary, portal, provider, or keyed range
  consumes one position in its parent's sequence;
- `Option` and `Result` consume one position whether their selected output is
  empty or nonempty;
- `()` consumes one empty position;
- tuples splice their fields recursively into the current sequence;
- arrays, vectors, and iterators splice each entry recursively into the current
  sequence and their collection wrapper consumes no additional position; and
- a fragment's children form a new nested sibling sequence rather than being
  spliced into its parent.

For example, `(Some(A), vec![B, C], (), D)` assigns parent positions `0` through
`4` to `A`, `B`, `C`, the empty value, and `D`. Changing `Some(A)` to `None`
leaves `B` at position `1`. Removing `B` from the vector moves `C` to position
`1`; dynamic entries therefore need keys. A keyed adapter consumes one parent
position and owns its whole nested host range.

```rust
PlayerRow::new(player).key(player.id)
```

A key is an owned `Eq + Hash + Clone + 'static` value. Reactant preserves its
`TypeId` and equality operation after erasure. Hashes select candidates but
never establish equality by themselves.

Duplicate equal keys of the same Rust type in one sibling sequence panic before
commit. Equal values of different key types are not duplicates.

## Component and fragment identity

Components and fragments participate in logical identity even though they
create no Unity elements. Their child host nodes may move or update without
changing the component instance.

```rust
Fragment::new((
    Label::new(self.name.clone()),
    HealthBar::new(self.health),
)).key(self.player_id)
```

Moving this keyed fragment retains state in `HealthBar` and any other nested
components. Its individual top-level hosts are reordered as described below;
Battlement has no range-move command.

An unkeyed component remains at its sibling position only while its component
type is unchanged. Empty render values preserve the structural slot that
created them, matching React's treatment of empty children.

```rust
Fragment::new((self.notice.clone(), Editor::new()))
```

If `notice` switches from `None` to `Some`, the unkeyed `Editor` remains at
position one and keeps its identity. Removing an entry from a `Vec` still moves
every later unkeyed entry to an earlier position; dynamic collections require
keys whenever entries may be inserted, removed, or reordered.

## Memoized component bailout

`memo(component)` creates an opt-in component boundary. Its node identity
contains the memo marker and concrete component type `C`, in addition to the
ordinary sibling position or key. Changing between `C` and `Memo<C>` therefore
remounts the component. Reactant calls `C::render` on mount. On update, it may
reuse the boundary's complete committed subtree when both conditions hold:

1. the new `C` compares equal to the committed `C` through `PartialEq`; and
2. neither the boundary nor any descendant carries dirty work.

The root factory still runs and constructs the new `Memo<C>` before this
comparison. If props differ, Reactant renders the component and reconciles its
new output normally. A keyed memo boundary follows the same key, type, move,
mount, and unmount rules as any keyed component.

Every update source marks its target component and propagates a dirty-descendant
mark through all enclosing memo boundaries. This includes queued state and
reducer work, changed context consumed below the boundary, resource completion
or invalidation, external-store notification, and geometry observation. A dirty
mark defeats the bailout even when the boundary's props compare equal, so memo
cannot hide local work. When a provider value changes during reconciliation,
Reactant marks its mounted consumers before deciding whether an intervening
memo boundary may bail out.

Dirty marks and memo values are transactional. A successful commit acknowledges
the work and stores the component value used by the committed render. A render
that suspends, panics, fails validation, or lets an explicit error escape its
root retains the prior memo value, subtree, and dirty marks. A caught error may
commit its boundary fallback normally; memo values outside the successfully
committed boundary result advance with that commit. Memoization never changes
hook order, lifecycle timing, or the rule that render functions are pure.

## Work-in-progress matching

Each parent reconciliation builds a lookup of committed keyed children and
keeps the committed child array in absolute order. It visits new children from
left to right.

For a keyed child, Reactant:

1. rejects duplicate new keys;
2. finds an equal old key with the same key type;
3. reuses it only when the node type also matches; and
4. otherwise schedules the old node for unmount and mounts a new node.

For an unkeyed child at absolute index `i`, Reactant compares only old child
`i`. It reuses that child only when it is also unkeyed and has the same type. It
does not skip keyed or empty children or search later positions for a convenient
match. Empty positions reconcile as logical records with no host output.

```rust
old: [A, B]
new: [B, A]
```

Without keys, both positions remount because their types changed. With keys,
both instances are retained and reordered.

## Host IDs

Reactant allocates a fresh `ObjectId` when a host node mounts. Reconciliation
stores that ID on the committed node and reuses it for every update and move
until unmount.

```rust
HostNode {
    object_id,
    element,
    children,
}
```

Document root IDs come from the childless `UiDocument` supplied to
`register_root`.
Reactant never replaces those IDs during ordinary rendering or reconnect.

Host IDs are implementation-owned. Components use `ElementRef` or domain keys
instead of reading an `ObjectId` during render.

## Minimal child moves

Reactant flattens each physical parent's logical output to its top-level host
children. Components and fragments contribute zero or more hosts. Portals
contribute no host to the source parent; their hosts appear in the target's
separate physical sequence.

After matching, each reused top-level host has one old native index and one
desired index. Reactant computes a longest increasing subsequence of old
indices. Hosts in that subsequence already have a valid relative order and are
not moved.

```rust
let old_indices = [2, 0, 1, 3];
let retained = [0, 1, 3];
move_host_with_old_index(2);
```

The notation illustrates the calculation; it is not a runtime data format.

Reactant then walks desired order from right to left, issuing one index command
for each reused host outside the subsequence and placing new subtrees at their
required index. A fragment with two moved top-level hosts may therefore require
two commands. Zero-host children require none. Each portal target runs the same
algorithm over its own physical children.

For two keyed fragments with two hosts each, swapping the fragments yields this
physical calculation:

```rust
let old_hosts = [a1, a2, b1, b2];
let new_hosts = [b1, b2, a1, a2];
let old_indices = [2, 3, 0, 1];
```

Either retained subsequence has length two, so the journal contains two index
moves. Reactant does not emit a fictional fragment-range move.

When several longest subsequences exist, Reactant chooses the one with the
lexicographically earliest desired-index sequence. It computes suffix lengths,
then scans desired order left to right and takes the first old index greater
than the last retained index that can still complete the maximum length. This
O(n²) tie-break is deterministic and list sizes do not justify a more obscure
algorithm in V1.

## Mount and unmount

Mount allocates component state and host IDs only in the work-in-progress tree.
A successful commit makes those identities visible.

One `VisualElementCreate` contains the largest wholly new `UiNode` subtree whose
physical parent already exists. Its initial properties and subscriptions travel
inside that subtree. A nested create is separate only when its parent is reused
or another ordering boundary prevents combining them.

Unmount marks the logical instance unavailable immediately at commit. Stale
events targeting its old `ObjectId` are ignored.

Ref detachment is committed immediately. Passive effect cleanup runs on the
next non-session active entry: `dispatch`, `refresh`, `poll`, or
`observe_geometry`, child before parent. Host destruction may remove a complete
subtree with one command when Battlement's parent destroy contract guarantees
that result.

Changing a component key therefore performs a complete unmount/remount:

```rust
Editor::new(document).key(self.revision)
```

Its hooks reset, old effects clean up, refs detach and attach, and all host
descendants receive new `ObjectId` values.

## Property comparison

Host node comparison is field-by-field over the desired lowered `UiElement`
value. Equal fields emit nothing. Changed mutable fields become a sparse
`VisualElementUpdate` or element-specific update.

```rust
old.text = Prop::Set("Ready")
new.text = Prop::Set("Playing")
```

This produces one text update. An old `Set` followed by new `Unset` produces an
explicit `Reset`, because omission in a Battlement update means unchanged.

```rust
old.source = Prop::Set(texture)
new.source = Prop::Unset
```

The generated `ObjectId`, concrete `UiElement` enum variant, document ID, and
document root ID are create-only. Changing a primitive's enum variant remounts
that host. Document identities are fixed by root registration. Every other
public primitive builder property must support `Prop` update and reset; a field
without that host support is not exposed as mutable Reactant API.

Style comparison is structural. Reactant emits changed style properties and
explicit resets for removed properties. A changed class list is replaced in one
sparse `VisualElementUpdate::Properties`; its declared order is preserved.

## Semantic mutation plan

Reconciliation first produces host-independent operations:

```rust
enum Mutation {
    Create { parent: ObjectId, index: u32, node: UiNode },
    Properties { target: ObjectId, element: Box<UiElement> },
    Move { target: ObjectId, parent: ObjectId, index: u32 },
    Destroy { target: ObjectId },
}
```

Subscription changes are fields in the sparse `UiElement` patch. Portal output
uses the target container as the physical `parent`. Ref and effect changes stay
in the runtime commit and are not host mutations. An authored nonempty `events`
value cannot enter through a Reactant host façade because Reactant exclusively
derives that field. Geometry observation uses the separate registry command and
is never a native element field. The enum is private; tests do not assert it.

The plan is validated as a whole before committed state changes. Validation
checks object identity uniqueness, valid parents, duplicate keys, handler model
types, Reactant-owned host fields, ref ownership, portal target ownership,
legal host children, and property-specific Motion capabilities.

## Deterministic command lowering

Every desired host receives a preorder serial from the complete physical tree.
Removed hosts retain their committed preorder serial. A mutation's total
ordinal is `(preorder, mutation_kind, target_object_id)`; mutation kind has the
fixed order create, move, properties, destroy. Lowering adds dependencies for
these rules:

- a parent create precedes work targeting that parent;
- reused children reparent before an old ancestor is destroyed;
- removed hosts leave before final sibling indices are assigned;
- index operations for one parent follow the reconciler's right-to-left order;
- properties and subscriptions follow target creation; and
- a removed child is destroyed before a separately destroyed parent.

Each mutation has a conflict set rather than one key. Create conflicts on its
new parent and every object ID in its maximal subtree; move conflicts on the
target, old parent, and new parent; destroy conflicts on the target and old
parent; and a patch conflicts on its target. At each barrier, Reactant scans
ready mutations by total ordinal and takes each mutation whose set is disjoint
from those already chosen. It removes that group from the dependency graph and
repeats. Independent parents and targets therefore share a parallel group,
while operations whose order can affect one Unity child list occupy successive
groups. Parent and index changes are always one `Move`; no intermediate parent
or index state is serialized.

```rust
while !plan.is_empty() {
    let group = plan.earliest_ready_with_disjoint_conflicts();
    commit.push_group(group.by_ordinal());
}
```

One maximal new `UiNode` subtree remains one create mutation, and all sparse
property changes for one target remain one patch. The chosen LIS determines the
only reused hosts eligible for index moves; grouping never adds a move.

An opaque `ReactantCommit` prevents callers from flattening these barriers by
accident.

```rust
let response = response.append_reactant(commit);
```

`into_groups` is available when a game intentionally interleaves other command
families. Each returned group must remain a parallel group in its original
order.

## Native subscriptions

A **physical event island** is a Reactant document root or an outermost external
portal host whose Unity ancestry contains no Reactant document root. For every
propagating kind needed in an island, Reactant installs a coverage pair at its
root: `Target` observes an event originating on the root and `Trickle` observes
an event originating below it. The dormant phase contributes no duplicate.

```rust
document island: document root Target + Trickle coverage
portal island:   each top-level portal host Target + Trickle coverage
```

The coverage listener uses Unity's earliest supported propagation phase so the
adapter reports the original target once. Reactant's capture and bubble handler
phases are logical and do not create matching native phase subscriptions.
`UiEvent::target_id` is always that original target; it is never the island root
that caused forwarding. The common Battlement adapter deduplicates native
coverage callbacks and forwards at most one `UiEvent` for a native event.

Target-only application events subscribe directly on each host that uses the
event. They do not enter logical capture or bubble propagation. Geometry hooks
use the separate batched observation protocol and do not install an application
`GeometryChanged` handler.

Changing only a Rust callback closure does not require a Unity command. Unity
still reports the same event kind to the same host; Reactant replaces the
logical callback in committed state.

```rust
Button::new("Save")
    .on_click(|game: &mut Game| game.save())
```

Adding or removing handlers recomputes the affected island's event-kind set.
Changing only a callback closure leaves that set unchanged and emits no Unity
subscription update. A portal island includes kinds required only by source
ancestors, ensuring a portal click reaches a handler outside its physical Unity
ancestry.

```rust
source.on_click(handler);
portal_host.subscribe(EventKind::Click);
reactant.dispatch(&mut game, nested_portal_click)?;
```

## Event handlers

Payload-free convenience methods give the callback only the mutable application
model.

```rust
.on_click(|game: &mut Game| game.end_turn())
```

Payload-aware methods also supply `ReactantEvent<E>`.

```rust
.on_pointer_down_event(|game: &mut Game, event| {
    game.select_at(event.payload().position)
})
```

Default-phase methods use the snake-case event name directly. Every application
event kind has a payload-free `on_<kind>` method and an event-aware
`on_<kind>_event` method. Propagating kinds also have `on_<kind>_capture` and
`on_<kind>_capture_event`.

```rust
.on_click_capture_event(|game: &mut Game, event| {
    game.note_capture();
    game.note_target(event.target())
})
```

The event-aware callback receives its kind's exact `UiEventBody` payload type.
All four method families own an erased callback with one of these shapes:

```rust
Fn(&mut G) + 'static
Fn(&mut G, ReactantEvent<E>) + 'static
```

For one event kind and phase, payload-free and event-aware methods write one
logical handler slot. Calling either form replaces the callback previously
written through either form; the last call wins. Capture and default phases
remain separate slots. This matches ordinary Rust builders and JSX's single
event prop rather than registering two callbacks for one phase. Replacing a
rendered host façade replaces its complete handler set.

Familiar names use React semantics rather than exposing Unity terminology:

| Reactant builder | Logical behavior | Battlement source |
|---|---|---|
| `on_focus`, `on_blur` | bubbles through the logical tree | `FocusIn`, `FocusOut` |
| `on_pointer_enter`, `on_pointer_leave` | crossing path; no capture builder | `PointerOver`, `PointerOut` plus related target |
| `on_change` | each user value proposal | `Input` or `ValueChanging`, selected by control |
| `on_input` | each text input proposal | `Input` |
| `on_click` | logical capture and bubble | `Click` |

The exact host event name remains available when its meaning is useful, such as
`on_value_committed`, `on_selection_changed`, `on_scroll_settled`, and
`on_tab_close_requested`. Payload-aware methods append `_event`; logical
capture methods append `_capture` or `_capture_event`. The façade's control
type selects the exact payload type, so `Slider::on_change_event` receives a
numeric proposed value while `TextField::on_change_event` receives text.

Reactant does not expose separate raw builders for Battlement's target-only
`PointerEnter`, `PointerLeave`, `Focus`, or `Blur` cases. The familiar builders
are synthesized from `PointerOver`/`PointerOut` and `FocusIn`/`FocusOut`, which
provide the logical ancestry React semantics require.

`on_change` is available only on controlled input façades and maps exactly:

| Primitive | Source | Event payload |
|---|---|---|
| `TextField` | `Input` | current `String` proposal |
| `Scroller`, `Slider`, `SliderInt`, `MinMaxSlider` | `ValueChanging` | the control's typed live proposal |
| `Toggle`, `RadioButton`, `RadioButtonGroup`, `ToggleButtonGroup`, `DropdownField` | `ValueCommitted` | the control's typed completed proposal |
| `TabView` | `TabSelectionRequested` | proposed tab index |

`on_value_committed` remains available on every continuous or completed-value
control and on `TextField`; TabView proposals use
`on_tab_selection_requested`. `on_input` remains available only on `TextField`.
`on_value_changing` remains available only on the four continuous controls.
Output-only controls do not have `on_change`.

These kinds support logical capture and bubble phases:

- `PointerDown`, `PointerMove`, `PointerUp`, and `PointerCancel`;
- `Click`, `PointerOver`, `PointerOut`, and `Wheel`;
- `PointerCapture` and `PointerCaptureOut`;
- `KeyDown`, `KeyUp`, `NavigationMove`, and `NavigationCancel`;
- `FocusIn` and `FocusOut`; and
- `LinkEnter`, `LinkLeave`, `LinkDown`, and `LinkUp`.

All remaining application event kinds are target-only: `AttachToPanel`,
`DetachFromPanel`, all three `Transition` events,
`ValueChanging`,
`ValueCommitted`, `Input`, `SelectionChanged`, `ScrollSettled`, `ScrollChanged`,
and the three `Tab*Requested` events. Calling a capture builder for a
target-only kind is impossible because that method is not provided.

The model annotation is required because `Component` and `Render` are
application-independent. The adapter stores the model `TypeId` and an erased
callback. Before commit, Reactant confirms every handler in every root expects
the runtime's `G`.

Invocation downcasts the one borrowed `&mut G` only after validation. The
callback cannot retain that reference because its lifetime is limited to the
dispatch call.

Handlers return `()`. Application failures panic. A handler that needs to emit
non-UI commands mutates an application outbox or other state in `G`; Reactant
continues to own only the UI commit.

An event handler may capture an `ElementRef` and invoke one of its host-action
methods. Reactant queues those one-shot actions in the event batch. It first
reconciles model and hook changes, then emits the resulting host mutations, and
finally emits the actions. A controlled text-field value therefore updates
before `select_text` restores its cursor and selection. The
[refs appendix](refs-geometry-and-floating-ui.md#host-actions) defines the
complete method surface and detached-ref behavior.

## Logical propagation

`ReactantEvent<E>` is a cheap view over shared event data:

```rust
pub struct ReactantEvent<E> {
    inner: Rc<EventInner<E>>,
    current_target: ElementTarget,
    phase: EventPhase,
}

struct EventInner<E> {
    payload: E,
    target: ElementTarget,
    propagation_stopped: Cell<bool>,
}
```

```rust
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}
```

Each callback receives a cloned event view. Cloning clones only the `Rc`; the
payload and propagation flag are shared, so `E` need not implement `Clone`.

```rust
impl<E> ReactantEvent<E> {
    pub fn payload(&self) -> &E;
    pub fn target(&self) -> ElementTarget;
    pub fn current_target(&self) -> ElementTarget;
    pub fn phase(&self) -> EventPhase;
    pub fn cancelable(&self) -> bool;
    pub fn default_prevented(&self) -> bool;
    pub fn prevent_default(&self);
    pub fn stop_propagation(&self);
}
```

`ElementTarget` is an opaque `Copy + Eq` logical-host handle. `object_id()` and
`root()` return the event-time host ID and logical source root even after the
host later unmounts. A stale target has no direct Unity access and never
resolves as the target of a later event.

It exposes immutable accessors plus `prevent_default` and
`stop_propagation`. The prevention and propagation flags are independent and
shared by event views given to successive handlers.

For one Unity event, Reactant:

1. resolves the committed host target;
2. collects its logical ancestors up to the root;
3. invokes capture handlers on strict ancestors from root to target parent;
4. invokes target capture, then target default handlers with `Target` phase; and
5. invokes bubble handlers from target parent to root.

`current_target` changes before each callback. `target` remains the original
logical host. `stop_propagation` prevents later nodes and phases. Each node has
at most one handler in each capture or default builder slot for the active
event kind. On the target itself, the capture slot runs first and the default
slot runs second; both event views report `EventPhase::Target`.

An event for an unknown, unmounted, or unsubscribed host invokes no event
callback. The active entry still flushes earlier passive work and renders any
already-dirty roots; it does not perform the unconditional refresh associated
with a recognized event.

All callbacks share one state batch and one mutable borrow of `G`. Reactant
refreshes roots once after propagation finishes, even when no handler changed
state, because application methods may have mutated fields Reactant cannot
observe directly.

`ReactantEvent` exposes `cancelable`, `default_prevented`, and
`prevent_default`. Unity submits the event synchronously while its callback is
active, consumes only the fixed default-action disposition immediately, and
defers the ordinary Reactant response. The complete timing, failure, and native
ownership contract is defined by
[Events and default actions](events-and-default-actions.md).

### Synthetic pointer crossing

For one pointer crossing, Reactant compares the committed logical ancestor paths
of the old and new Rust-owned targets. It finds their lowest common ancestor,
invokes `on_pointer_leave` from the old target upward excluding that ancestor,
then invokes `on_pointer_enter` from the ancestor's entering child downward to
the new target. Each callback has `EventPhase::Target`; these methods have no
capture variants. A missing old or new target uses an empty path. Portal paths
are logical, so crossing into a portal child can enter source ancestors without
entering unrelated physical ancestors.

The host includes `related_target_id` on `PointerOver` and `PointerOut`.
Reactant treats reversed `(target, related_target)` pairs for the same pointer
as one crossing. Before dispatching the first raw over/out event, it runs the
synthetic leave/enter traversal. The complementary raw event still performs its
ordinary capture/target/bubble dispatch, but does not synthesize the crossing a
second time. If only one side is Rust-owned, that event is sufficient.
`stop_propagation` in the synthetic traversal stops its remaining leave or enter
callbacks but does not suppress either raw event or cancel Unity's crossing.

Deduplication applies only when the immediately following Reactant event is the
complementary over/out kind for the same pointer with reversed target and
related-target IDs. Any intervening event, different pointer, or different pair
clears the candidate. Battlement UI derives `related_target_id` from the
previous and next picked paths even though Unity's public over/out event object
does not expose it directly.

## Portals

A portal renders a logical child into another registered Unity container.

```rust
pub fn create_portal<R: Render>(
    child: R,
    target: PortalTarget,
) -> Portal<R>;

impl ElementTarget {
    pub fn object_id(self) -> ObjectId;
    pub fn root(self) -> Root;
}
```

```rust
create_portal(
    ContextMenu::new(self.items.clone()),
    self.overlay.clone(),
)
```

The portal node remains beneath its source component for hooks, context,
Suspense, unmounting, and event propagation. Its top-level host nodes become
physical children of the target container.

One target may receive any number of portal nodes, including portals from
different registered roots in the same runtime. A portal occurrence owns one
contiguous host range. The target's ordinary children come first, followed by
portal ranges ordered by source-root registration order and then source-tree
depth-first order. Reconciliation computes this one physical child sequence
from changed and unchanged roots together, so cross-root updates cannot assign
conflicting indices. `ElementTarget::root()` for portaled content always reports
the logical source root.

A `PortalTarget` is an opaque, cloneable handle owned by one Reactant runtime.
For a Reactant-owned container, application code creates the handle and attaches
it to exactly one host.

```rust
pub fn create_portal_target(&mut self) -> PortalTarget;
```

```rust
View::new()
    .portal_target(self.overlay.clone())
```

Attaching one target to two committed hosts panics. A work-in-progress tree may
refer to a target before its host is visited, but the complete tree must attach
every referenced target exactly once. Removing a target while a portal still
uses it panics before commit.

The uniqueness rule applies to containers, not portal occurrences. Any number
of `create_portal` calls may clone and use the same attached handle. Two
different `PortalTarget` handles may not resolve to the same internal host or
external `ObjectId` in one runtime; registration or reconnect rebinding that
creates such an alias panics.

For an external Battlement container, the application registers its stable
`ObjectId`.

```rust
pub fn register_external_container(&mut self, id: ObjectId)
    -> PortalTarget;
```

External target registration is configuration and is allowed only before the
first session becomes active. The set of registered handles is fixed after
activation.

The supplied session snapshot establishes the external container's existing
direct children as an immutable caller-owned prefix. Reactant appends and owns
only its portal ranges after that prefix. While the target is registered for an
active session, caller-owned commands must not add, remove, reparent, or reorder
the container's direct children; doing so violates the response-handoff
contract because Reactant cannot observe the new indices. Descendant mutations
beneath a caller-owned prefix child remain legal. Rebinding for a reconnect
uses the next snapshot to establish a new prefix.

The target must exist in the session snapshot before portal create commands
run. `begin_session` therefore returns external portal commands separately as
the `ReactantCommit` in `SessionUi::into_parts(snapshot)`.

When reconnect gives the external container a new ID, stage the new ID before
`begin_session`. Staging may occur while the current session is active, but it
must name an existing registered handle and has no effect on that session.

```rust
pub fn stage_external_container_rebind(
    &mut self, target: &PortalTarget, next_id: ObjectId,
);
```

`begin_session` validates all staged IDs against the new snapshot and applies
the rebinds atomically with the new session. Reconnect preserves logical portal
state because every native host is recreated. Registering a new external target
after activation panics; staging never rebinds the current session in place.

Changing a portal target is equivalent to changing its key. The portal subtree
unmounts from the old target and mounts with new host IDs under the new target.
Moving it without remount would incorrectly preserve native state across panels
and host ownership boundaries.

## Portal events and context

Portal context follows the logical parent:

```rust
THEME.provider(dark_theme).child(
    create_portal(Menu::new(), overlay),
)
```

`Menu` reads `dark_theme` even when `overlay` is physically outside the provider
host subtree.

Portal events also bubble logically. A click in `Menu` reaches capture and
bubble handlers on its source ancestors, not unrelated Reactant components that
happen to be physical ancestors of the external container.

An internal portal remains in the event island of the Reactant document root
in its physical ancestry, even when its source is another root. Reactant adds
the event kinds required by incoming portals to that document's coverage set.
For an external container, only top-level portal hosts whose ancestry has no
Reactant document root carry coverage subscriptions. A nested portal uses the
outermost applicable coverage host, so one native event still enters logical
dispatch once.

Native Unity propagation may still notify separately registered Battlement
listeners on those physical ancestors. Reactant controls only its own logical
dispatch and does not claim to cancel already-delivered external listeners.

## No-op and abandoned renders

A render that produces host values equal to the committed tree returns an empty
`ReactantCommit`. Component render calls and dependency checks may still occur,
while
an eligible memoized boundary skips its component render entirely. Unity
receives no mutation in either case.

```rust
let before = world.journal().len();
world.apply(reactant.refresh(&mut game)?);
assert_eq!(world.journal().len(), before);
```

Suspension or an explicit render error abandons the work-in-progress tree. A
caught explicit error abandons only its boundary's tentative primary and
substitutes the fallback before reconciliation continues; an error reaching a
root is returned to the caller. No host ID allocation, callback replacement,
ref attachment, hook slot, or partial mutation from abandoned work becomes
committed. Resource cache work is the sole intentional exception because it
must survive a Suspense or error-boundary retry.

An actual validation or component panic also leaves the committed tree and
Unity unchanged, but it poisons the runtime and every later entry panics.

## Manual QA

1. Insert, remove, and reorder keyed rows, then rerender unchanged. Confirm
   retained rows keep host IDs and state, the journal contains only the minimum
   create, removal, and moves, and the final rerender adds no commands.
2. Repeat without keys. Confirm position and type determine which instances
   retain state, matching the documented remount behavior.
   Also toggle an optional child before an editor and confirm the editor keeps
   its state and host ID.
3. Change one property from set to omitted and replace one callback closure.
   Confirm Unity receives the property reset but no redundant subscription.
4. Dispatch capture, target, and bubble handlers that append visible markers.
   Confirm order, `current_target`, batching, and `stop_propagation` behavior.
5. Portal a menu to an internal and then external target. Confirm physical
   parentage, logical context, logical bubbling, and full remount after changing
   targets.
6. Mix ordinary children with same-target portals from two roots. Confirm
   ordinary children precede portal ranges, source ordering is stable, and one
   native event reaches only its original logical source path.
7. Cause validation to fail after a render proposes several mutations. Confirm
   the fake Unity tree and command journal remain unchanged.
8. Change a stateful child's key. Confirm the old host disappears, the new host
   has a new ID and initial visible state, and the next poll makes its unmount
   cleanup observable through a sibling label.
9. Memoize a component, rerender it with equal props, and use a test-only render
   probe to confirm its render call is skipped. Then independently change its
   props, local state, consumed context, resource, store snapshot, and observed
   geometry; confirm each affected value reaches Unity through the memo
   boundary.
10. Abandon a dirty memoized render through suspension and retry it. Confirm the
   committed subtree remains visible during the abandoned render and the dirty
   update is applied on the successful retry.
11. Combine a parent create, child move, and patches on independent targets.
   Confirm dependent commands occupy ordered groups while independent patches
   share a group in the fake command journal.
12. Update a controlled text field and call `select_text` from the same event.
   Confirm the value mutation precedes the selection action in the journal.
13. Move a pointer between siblings, ancestor and descendant, outside the panel,
    and a portaled child. Confirm one synthetic crossing, exact leave/enter path
    order, related targets, and no physical-ancestor callbacks.
14. Focus and blur a nested field. Confirm familiar handlers bubble logically.
    Exercise `on_change` on text, slider, toggle, dropdown, and TabView controls
    and confirm the source kind and typed proposal match the mapping table.
15. Set one event-and-phase slot first through its payload-free builder and then
    through its event-aware builder, and repeat in reverse. Confirm only the
    last callback runs and callback-only replacement emits no host command. On
    a target with capture and default slots, confirm capture runs first and
    both callbacks observe `EventPhase::Target`.
16. Dispatch unknown, unmounted, and unsubscribed targets while another root is
    dirty. Confirm no event callback or unconditional refresh runs, while the
    already-dirty work still commits.
