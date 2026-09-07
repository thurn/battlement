# Preserve object state when its position changes

A card moving from a hand to the table should keep its component state, refs,
and compatible native objects. Reactant uses a stable UUID to recognize that it
is the same presentation even when its parent changes. Removing the card is
different: it ends that component's lifetime, even if an exit animation remains
visible for a while.

Read this for matching, portals, context, subscriptions, refs, and removal.
Related pages: [component architecture](architecture.md), [visible
updates](presentation.md), [animation](motion.md), and [world
objects](world.md).

## Put the identity on the component whose state should move

Any component, UI element, or world object may declare `.id(Uuid)`. This UUID
identifies one live presentation across all active application roots. Ordinary
`.key()` remains scoped to siblings under one parent.

The same card declaration can move between layouts without remounting:

```rust
Hand::new().child(Card::new().card(card).id(card.presentation_id))
// A later render puts that same card on the table:
Table::new().child(Card::new().card(card).id(card.presentation_id))
```

Place `.id()` on `Card` if its hooks and refs should move with the visual.
Putting it only on an inner sprite does not preserve its parent's hook state.
Allocate or derive the UUID once and retain it in game or display data; a new
UUID on every render describes a new object every time.

A card and its simultaneous inspection copy need separate presentation UUIDs,
even though they show the same rules object. The same rule applies to a UI
thumbnail and a second enlarged view. A rules ID may double as a presentation
UUID only when exactly one live presentation uses it.

## Match before removing the former parent

Build an application-wide index of proposed UUIDs before changing native
objects. Reject duplicate live declarations across UI, world, portals, and
active roots before committing anything.

Match the identified component before reconciling its descendants. This allows
Reactant to preserve its existing hooks instead of evaluating a new component
and trying to copy state afterward.

For example, removing a hand must not destroy the card that moves out of it:

```text
before: Hand contains Card A, whose local counter is 3
after:  Hand is absent; Table contains Card A
result: Card A still has counter 3 and the same compatible refs
```

Extract surviving moved objects before destroying unmatched ancestors. Preserve
compatible descendants and native handles. Changing a component's type creates
fresh hook storage; do not interpret one type's hook slots as another's.

After a move, resolve context and capture/bubble events from the new logical
ancestry. Reevaluate context consumers and clean up effects whose dependencies
changed before installing their replacements. A retained subscription must not
continue reading the old provider or be installed twice.

## Reuse only compatible native objects

A Unity sprite cannot become a UI Toolkit label merely by keeping its ID. Reuse
a native host only when its kind and property contract are compatible. A stable
world group may retain its transform while replacing its face sprites or text.
Prepare incompatible replacements before making them visible.

Moving a UI element to a world representation requires an explicit mapping
between its screen rectangle and a world plane/camera. Preserve logical state
while connecting the old and new visuals through that mapping. See [UI/world
projection](world.md#move-between-ui-and-world-space). Missing required
projection data is a render validation error; the engine must not guess pixel scale.

## Removal ends logical state immediately

Absence from a committed tree unmounts the component. An abandoned proposed
render does not. Unmount detaches handlers and subscriptions and discards hooks.

With rendering ahead, logical unmount can precede native removal. Queue native
removal after earlier commands that use the object; retain their data and native
resources until execution and any retained effects finish. Dropping hooks must
not cancel earlier queued movement or destroy its targets immediately.

An **incarnation** is one continuous mounted lifetime of a UUID. Native refs and
callbacks carry this lifetime ID as well as the UUID and host kind. This lets an
old exit animation coexist safely with a newly mounted object of the same UUID:

```text
Card A, incarnation 1: removed; dissolving; no input or hooks
Card A, incarnation 2: newly mounted; fresh hooks; accepts current input
old dissolve completes: release incarnation 1 only
```

**Exit visuals** are the native objects and resources retained to finish removal
animation. Their component no longer renders or receives input. Retained
animation may change their visual properties, but it cannot access a live
component closure to keep old hooks running.

Uniqueness applies to live declarations, so the old retained visual does not
make incarnation 2 a duplicate. Events from incarnation 1 cannot update or
destroy incarnation 2. Release each native object, material instance, anchor,
and asset after its last retained animation or effect use ends. A projectile may
retain an old attachment point without retaining the logical card.

## Read only the state a component needs

Components may use props or optional selectors to read the rendered snapshot. A
selector compares its output and suppresses subscription-triggered component
evaluation when equal. The selector may still run to compute that comparison.
This helps sparse updates without changing visible behavior.

For example, a score label need not rerender when only a hand changes:

```rust
let score = use_game_selector::<HeartsGame, _>(|state| state.south_score);
ScoreLabel::new().score(score)
```

External stores hold display state such as selection and settings. Their writes
queue notifications. Each render captures a stable version; a write during that
render appears in a later complete update, never halfway through the current
one. Abandoned rendering must not install new subscriptions or clean up those
belonging to the current tree. Use Reactant's existing render/commit lifecycle;
these are local tree changes, not a host acknowledgement protocol.

Test the same scene using explicit props as well as selectors. A moved consumer
keeps its hook identity but reads from its new provider.

## Manual QA

Move a card with a visible local counter between layouts, roots, and a UI
portal. Change the provider at its new parent and verify the counter survives
while context and event propagation change. Remove and recreate its UUID during
a dissolve; confirm fresh state, isolated input, and correct cleanup of both
incarnations. Attempt a duplicate live UUID and verify the old display remains.
