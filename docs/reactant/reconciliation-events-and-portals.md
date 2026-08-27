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
sequence. Keyed siblings still occupy positions when Reactant calculates an
unkeyed sibling's identity.

`node_type_id` distinguishes component types and host element variants. A
`Label` and `Button` with the same key are different nodes. Changing either the
key or type unmounts the old node and mounts the new one.

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
type is unchanged. Inserting an unkeyed sibling before it changes its identity,
matching React behavior.

```rust
Fragment::new((self.notice.clone(), Editor::new()))
```

If `notice` switches from `None` to `Some`, an unkeyed `Editor` after it is at a
new position and remounts. Giving the editor a key preserves it.

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
does not skip keyed children or search later positions for a convenient match.

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

Document root IDs come from the childless `UiDocument` supplied to `mount`.
Reactant never replaces those IDs during ordinary rendering or reconnect.

Host IDs are implementation-owned. Components use `ElementRef`, `ReactantId`,
or domain keys instead of reading an `ObjectId` during render.

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

Ref detachment is committed immediately. Passive effect cleanup runs on the next
poll, child before parent. Host destruction may remove a complete subtree with
one command when Battlement's parent destroy contract guarantees that result.

Changing a component key therefore performs a complete unmount/remount:

```rust
Editor::new(document).key(self.revision)
```

Its hooks reset, old effects clean up, refs detach and attach, `use_id` values
change, and all host descendants receive new `ObjectId` values.

## Property comparison

Host node comparison is field-by-field over the desired primitive value. Equal
fields emit nothing. Changed mutable fields become a sparse
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
    Parent { target: ObjectId, parent: ObjectId },
    Index { target: ObjectId, index: u32 },
    Destroy { target: ObjectId },
}
```

Subscription changes are fields in the sparse `UiElement` patch. Portal output
uses the target container as the physical `parent`. Ref and effect changes stay
in the runtime commit and are not host mutations. The enum is private; tests do
not assert it.

The plan is validated as a whole before committed state changes. Validation
checks object identity uniqueness, valid parents, duplicate keys, handler model
types, ref ownership, portal target ownership, and legal host children.

## Deterministic command lowering

Every mutation receives a stable ordinal from physical-parent depth-first order,
desired child index, and mutation kind. Lowering adds dependencies for these
rules:

- a parent create precedes work targeting that parent;
- reused children reparent before an old ancestor is destroyed;
- removed hosts leave before final sibling indices are assigned;
- index operations for one parent follow the reconciler's right-to-left order;
- properties and subscriptions follow target creation; and
- a removed child is destroyed before a separately destroyed parent.

Each mutation also has a conflict key. Child-list mutations conflict on physical
parent; patches conflict on target. At each barrier, Reactant emits the earliest
ready mutation for every conflict key, sorted by ordinal. It then removes those
mutations from the dependency graph and repeats. Independent parents and targets
therefore share a parallel group, while operations whose order can affect one
Unity child list occupy successive groups.

```rust
while !plan.is_empty() {
    let group = plan.earliest_ready_per_conflict_key();
    commit.push_group(group.by_ordinal());
}
```

One maximal new `UiNode` subtree remains one create mutation, and all sparse
property changes for one target remain one patch. The chosen LIS determines the
only reused hosts eligible for index moves; grouping never adds a move.

An opaque `UiCommit` prevents callers from flattening these barriers by
accident.

```rust
let response = response.append_ui(commit);
```

`into_groups` is available when a game intentionally interleaves other command
families. Each returned group must remain a parallel group in its original
order.

## Native subscriptions

A **physical event island** is one Reactant document or one portaled host range
whose Unity ancestry does not pass through that document. Reactant installs one
coverage subscription per propagating event kind at each island root that needs
that kind anywhere in its logical subtree or logical ancestors.

```rust
document island: document root subscription
portal island:   each top-level portal host subscription
```

The coverage listener uses Unity's earliest supported propagation phase so the
adapter reports the original target once. Reactant's capture and bubble handler
phases are logical and do not create matching native phase subscriptions.

Target-only events such as geometry subscribe directly on each host that uses
the event. They do not enter logical capture or bubble propagation.

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
reactant.dispatch(&mut game, nested_portal_click);
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

Default-phase methods use the snake-case event name directly. Every event kind
has a payload-free `on_<kind>` method and an event-aware `on_<kind>_event`
method. Propagating kinds also have `on_<kind>_capture` and
`on_<kind>_capture_event`.

```rust
.on_click_capture(|game: &mut Game| game.note_capture())
.on_click_capture_event(|game: &mut Game, event| {
    game.note_target(event.target())
})
```

The event-aware callback receives its kind's exact `UiEventBody` payload type.
All four method families own an erased callback with one of these shapes:

```rust
Fn(&mut G) + 'static
Fn(&mut G, ReactantEvent<E>) + 'static
```

These kinds support logical capture and bubble phases:

- `PointerDown`, `PointerMove`, `PointerUp`, and `PointerCancel`;
- `Click`, `PointerOver`, `PointerOut`, and `Wheel`;
- `PointerCapture` and `PointerCaptureOut`;
- `KeyDown`, `KeyUp`, `NavigationMove`, and `NavigationCancel`;
- `FocusIn` and `FocusOut`; and
- `LinkEnter`, `LinkLeave`, `LinkDown`, and `LinkUp`.

All remaining V1 `UiEventKind` values are target-only: `PointerEnter`,
`PointerLeave`, `Focus`, `Blur`, `GeometryChanged`, `AttachToPanel`,
`DetachFromPanel`, all three `Transition` events, `ValueChanging`,
`ValueCommitted`, `Input`, `SelectionChanged`, `ScrollSettled`, `ScrollChanged`,
and the three `Tab*Requested` events. Calling a capture builder for a
target-only kind is impossible because that method is not generated.

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

## Logical propagation

`ReactantEvent<E>` contains:

```rust
pub struct ReactantEvent<E> {
    payload: E,
    target: ElementTarget,
    current_target: ElementTarget,
    phase: EventPhase,
}
```

```rust
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}
```

Each callback receives a cloned event view. The payload and propagation flag
are shared internally, so `E` need not implement `Clone`.

```rust
impl<E> ReactantEvent<E> {
    pub fn payload(&self) -> &E;
    pub fn target(&self) -> ElementTarget;
    pub fn current_target(&self) -> ElementTarget;
    pub fn phase(&self) -> EventPhase;
    pub fn stop_propagation(&self);
}
```

`ElementTarget` is an opaque `Copy + Eq` logical-host handle. `object_id()`
returns its committed host ID and `root()` returns the opaque `Root` containing
it. It offers no direct Unity access and becomes stale after that host unmounts.

It exposes immutable accessors plus `stop_propagation`. The propagation flag is
shared by event views given to successive handlers.

For one Unity event, Reactant:

1. resolves the committed host target;
2. collects its logical ancestors up to the root;
3. invokes capture handlers on strict ancestors from root to target parent;
4. invokes target capture, then target default handlers with `Target` phase; and
5. invokes bubble handlers from target parent to root.

`current_target` changes before each callback. `target` remains the original
logical host. `stop_propagation` prevents later nodes and phases but does not
cancel other handlers already being invoked for the current node.

An event for an unknown, unmounted, or unsubscribed host is ignored and returns
an empty commit. It does not refresh roots.

All callbacks share one state batch and one mutable borrow of `G`. Reactant
refreshes roots once after propagation finishes, even when no handler changed
state, because application methods may have mutated fields Reactant cannot
observe directly.

There is no `prevent_default`. Unity has already performed native default
behavior before Rust receives the event, so such a method would be misleading.

## Portals

A portal renders a logical child into another registered Unity container.

```rust
create_portal(
    ContextMenu::new(self.items.clone()),
    self.overlay.clone(),
)
```

The portal node remains beneath its source component for hooks, context,
Suspense, unmounting, and event propagation. Its top-level host nodes become
physical children of the target container.

A `PortalTarget` is an opaque, cloneable handle owned by one Reactant runtime.
For a Reactant-owned container, application code creates the handle and attaches
it to exactly one host.

```rust
pub fn create_portal_target(&mut self) -> PortalTarget;
```

```rust
VisualElement::new()
    .portal_target(self.overlay.clone())
```

Attaching one target to two committed hosts panics. A work-in-progress tree may
refer to a target before its host is visited, but the complete tree must attach
every referenced target exactly once. Removing a target while a portal still
uses it panics before commit.

For an external Battlement container, the application registers its stable
`ObjectId`.

```rust
pub fn register_external_container(&mut self, id: ObjectId)
    -> PortalTarget;
```

The target must exist in the session snapshot before portal create commands run.
`begin_session` therefore returns external portal commands separately as
the `UiCommit` in `SessionUi::into_parts(snapshot)`.

When reconnect gives the external container a new ID, rebind the existing
handle before `begin_session`.

```rust
pub fn rebind_external_container(
    &mut self, target: &PortalTarget, next_id: ObjectId,
);
```

Reconnect preserves logical portal state because every native host is
recreated by the new session. Rebinding during an active session instead counts
as a target change and remounts the portal subtree on the next refresh.

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

Native Unity propagation may still notify separately registered Battlement
listeners on those physical ancestors. Reactant controls only its own logical
dispatch and does not claim to cancel already-delivered external listeners.

## No-op and abandoned renders

A render that produces host values equal to the committed tree returns an empty
`UiCommit`. Component render calls and dependency checks may still occur, but
Unity receives no mutation.

```rust
let before = world.journal().len();
world.apply(reactant.refresh(&game));
assert_eq!(world.journal().len(), before);
```

Suspension, validation panic, or component panic abandons the work-in-progress
tree. No host ID allocation, callback replacement, ref attachment, hook slot,
or partial mutation becomes committed. Resource cache work is the sole
intentional exception because it must survive a Suspense retry.

## Manual QA

1. Insert, remove, and reorder keyed rows, then rerender unchanged. Confirm
   retained rows keep host IDs and state, the journal contains only the minimum
   create, removal, and moves, and the final rerender adds no commands.
2. Repeat without keys. Confirm position and type determine which instances
   retain state, matching the documented remount behavior.
3. Change one property from set to omitted and replace one callback closure.
   Confirm Unity receives the property reset but no redundant subscription.
4. Dispatch capture, target, and bubble handlers that append visible markers.
   Confirm order, `current_target`, batching, and `stop_propagation` behavior.
5. Portal a menu to an internal and then external target. Confirm physical
   parentage, logical context, logical bubbling, and full remount after changing
   targets.
6. Cause validation to fail after a render proposes several mutations. Confirm
   the fake Unity tree and command journal remain unchanged.
7. Change a stateful child's key. Confirm the old host disappears, the new host
   has a new ID and initial visible state, and the next poll makes its unmount
   cleanup observable through a sibling label.
8. Combine a parent create, child move, and patches on independent targets.
   Confirm dependent commands occupy ordered groups while independent patches
   share a group in the fake command journal.
