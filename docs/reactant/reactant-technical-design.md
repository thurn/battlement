# Battlement Reactant Technical Design

`battlement-reactant` is a React-inspired component and reconciliation layer for
[`battlement-ui`](../battlement-ui-technical-design.md). It keeps the desired UI
as an in-memory tree, renders that tree from Rust component structs, compares it
with the last committed tree, and emits only the Battlement commands needed to
make Unity match. The design follows React where the host protocol can preserve
React's behavior. It uses explicit Reactant APIs where Unity or Rust makes that
behavior impossible.

## Appendix index

- [Components and rendering](component-authoring.md) defines component structs,
  builders, render values, props, children, and required-property typestate.
- [Hooks and effects](hooks-and-effects.md) defines every V1 hook and its
  scheduling and cleanup behavior.
- [Reconciliation, events, and portals](reconciliation-events-and-portals.md)
  defines identity, tree comparison, command generation, event routing, and
  physical placement outside logical parents.
- [Resources and Suspense](resources-and-suspense.md) defines asynchronous work,
  caching, fallback rendering, and retries.
- [Refs, geometry, and floating UI](refs-geometry-and-floating-ui.md) defines
  Unity element attachment, measurement, and the two-pass tooltip pattern.

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the snapshots, commands, events, and Unity UI Toolkit objects Reactant drives.
- [React documentation](https://react.dev/reference/react) is authoritative for
  the React behavior named by Reactant APIs.
- [Unity UI Toolkit event handling][unity-events] defines the native
  propagation layer beneath Reactant's logical events.
- [Unity `GeometryChangedEvent`][unity-geometry] defines the layout notification
  extended by Reactant geometry observation.

[unity-events]: https://docs.unity3d.com/Manual/UIE-Events-Handling.html
[unity-geometry]: https://docs.unity3d.com/ScriptReference/UIElements.GeometryChangedEvent.html

## Goals and V1 scope

V1 provides declarative Rust component trees over every supported
`battlement-ui` primitive, React-compatible identity and named hooks, sparse
Unity mutations, logical events, portals, resources, Suspense, and asynchronous
geometry observation. Normal authoring uses struct builders and one render
expression without a `.build()` step.

Reactant preserves React behavior whenever it adopts a React API name. When the
host cannot supply React's timing or control contract, V1 reserves the name and
uses a domain-specific API. `use_layout_effect` and `prevent_default` are the
two concrete cases.

V1 does not include `memo`, `PureComponent`, `cloneElement`, `useTransition`,
`useImperativeHandle`, `useInsertionEffect`, `useOptimistic`, React's `use`,
form actions, `flushSync`, or animation. The semantic mutation plan and stable
host-presence identity are retained so V2 can offer Framer Motion-style motion
variants over ordinary primitives, delay destruction, and lower animated
transitions.

## Battlement host contract

Reactant targets four existing `battlement-ui` concepts. A `UiDocument` is a
complete snapshot tree for one Unity `UIDocument`. Each `UiNode` has a stable
`ObjectId` and one concrete `UiElement` variant.

After the snapshot, Reactant changes live hosts through
`VisualElementCreate`, `VisualElementUpdate`, and `VisualElementDestroy` command
bodies. Updates can change sparse properties, parent, or child index. A
`UiEvent` returns one subscribed target `ObjectId` and a typed `UiEventBody`.

Battlement responses contain ordered messages. A snapshot precedes any batches
that refer to its objects. Each batch contains ordered parallel command groups;
the next group begins only after the previous group completes. Reactant's
`SessionUi` and `UiCommit` preserve this ordering.

## Runtime boundary

`Reactant<G>` owns UI runtime state. `G` is the application's mutable model. The
game owns both as sibling fields so Reactant can borrow the model while it is
dispatching an event.

```rust
struct GameEngine {
    game: Game,
    reactant: Reactant<Game>,
}
```

Reactant does not implement or wrap `Engine`. The game remains responsible for
connections, responses, action attribution, snapshots, non-UI commands, and
error policy. Reactant supplies documents and grouped UI commands for the game
to place in those responses.

This ownership also avoids an invalid self-borrow. `G` must not contain the
`Reactant<G>` currently lending it to a handler.

```rust
let (reactant, game) = (&mut self.reactant, &mut self.game);
let ui = reactant.dispatch(game, event);
```

Rendering and event dispatch are synchronous and confined to the engine thread.
Future loaders may work on other threads, but their completion is observed only
when the engine thread polls Reactant.

## Public runtime API

The public runtime surface is:

```rust
impl<G: 'static> Reactant<G> {
    pub fn new(spawner: impl Spawner) -> Self;
    pub fn mount<V, R>(&mut self, document: UiDocument, view: V) -> Root
    where V: Fn(&G) -> R + 'static, R: Render + 'static;
    pub fn begin_session(&mut self, game: &G) -> SessionUi;
    pub fn dispatch(&mut self, game: &mut G, event: UiEvent) -> UiCommit;
    pub fn refresh(&mut self, game: &G) -> UiCommit;
    pub fn poll(&mut self, game: &mut G) -> UiCommit;
}
```

Portal and resource administration complete the runtime surface.

```rust
impl<G: 'static> Reactant<G> {
    pub fn create_portal_target(&mut self) -> PortalTarget;
    pub fn register_external_container(&mut self, id: ObjectId)
        -> PortalTarget;
    pub fn rebind_external_container(
        &mut self, target: &PortalTarget, id: ObjectId,
    );
}
```

```rust
impl<G: 'static> Reactant<G> {
    pub fn preload<K, T>(&mut self, resource: &Resource<K, T>, key: K);
    pub fn invalidate<K, T>(&mut self, resource: &Resource<K, T>, key: &K);
    pub fn clear<K, T>(&mut self, resource: &Resource<K, T>);
}
```

The three resource methods carry the `K` and `T` bounds shown in
[Resources and Suspense](resources-and-suspense.md#public-resource-api).

`Root` is `Copy + Clone + Eq + Hash` with private fields. It identifies a root
inside the runtime that created it and has no Unity serialization.

`mount` accepts a childless `UiDocument` and a `'static` view factory. The
factory receives an immutable model and returns a component. Reactant owns the
document's children after mounting; pre-populated children are a developer
error.

```rust
reactant.mount(hud_document, |game: &Game| {
    Hud::new()
        .phase(game.phase())
        .players(game.players())
});
```

A **root** is one independently reconciled component tree mounted into one UI
document. Roots have separate component identity and context ancestry. They
share the runtime's resource cache, completed-task queue, and ID allocator.

The returned `Root` is an opaque, cloneable identity handle. Dropping it does
not change the UI. Root registrations last for the runtime because their
documents correspond to application-owned snapshot objects. A view factory can
return `None` to make its document childless without unregistering it.

A root mounted after a session begins remains inactive until the next
`begin_session`; `refresh` does not try to create its application-owned document
GameObject. This permits reconnect-time root registration without inventing a
Reactant document-lifecycle command.

`begin_session` renders every root and returns a **session UI**, the complete UI
state for a new Unity connection. It contains Reactant-owned `UiDocument` values
and an optional command commit for portals targeting containers outside those
documents.

```rust
let session = self.reactant.begin_session(&self.game);
let response = session.into_response(snapshot);
```

External portal commands run after the snapshot so their target objects exist.
Effects from the session do not run until the next engine poll, after Unity has
applied the full session response.

`SessionUi::into_response` validates external portal targets against the
snapshot, adds all documents, and returns a response whose snapshot precedes
the post-snapshot groups. `into_parts` performs the same validation and returns
the augmented snapshot for a custom command union.

```rust
impl SessionUi {
    pub fn into_response(self, snapshot: Snapshot) -> Response;
    pub fn into_parts(self, snapshot: Snapshot) -> (Snapshot, UiCommit);
}
```

Both methods call the same private validation routine. There is no API that
extracts external portal commands without supplying the snapshot they target.
`SessionUi` is `#[must_use]` and participates in the same outstanding-delivery
check as `UiCommit`; dropping it or reentering Reactant before consuming it
panics.

`dispatch` routes one Unity event, calls matching handlers, and refreshes every
root exactly once after propagation finishes. `refresh` performs the same root
render without an event. Both return an empty commit when Unity already matches.

```rust
game.advance_turn();
let ui = reactant.refresh(&game);
let response = response.append_ui(ui);
```

`poll` performs post-commit work: effect cleanup and setup, ready resource
completion, external-store notifications, and updates queued by those actions.
It returns an empty commit when no Unity response is needed.

## Poll algorithm

One `poll` uses this fixed order:

1. Freeze resource completions and store notifications present at entry.
2. Apply frozen resource completions and read each notified store once.
3. Run queued store unsubscribe/subscribe operations and their immediate
   race-closing reads in commit order.
4. Run queued effect cleanups and setups in commit order.
5. Render every root dirtied by phases two through four exactly once.
6. Commit that render and return its `UiCommit`.

State setters called by effects and unequal race-closing store reads join the
phase-five render. Resource completions and `StoreNotify` calls arriving after
phase one wait for the next poll. Effects and subscriptions created by the
phase-six commit also wait for the next poll; `poll` never recursively flushes
work created by its own returned commit.

A current resource-task panic is rethrown in phase two before lifecycle work.
Within phases three and four, an earlier commit's operations complete before a
later commit's operations.

## Components and render values

Components are structs whose fields are props. There are no function components
and no component-definition macro.

```rust
impl Component for Score {
    fn render(&self) -> Option<impl Render> {
        Some(Label::new(format!("Score: {}", self.value)))
    }
}
```

Prop reuse uses Rust's struct update operator. Reactant supplies no separate
spread syntax or spread macro.

```rust
PlayerRow { selected: true, ..defaults }
```

`Component` and `Render` are intentionally not generic over `G`. This keeps
reusable components independent from an application's model type. Event handler
model types are recorded with `TypeId`; a render containing a handler for the
wrong model panics before any command is committed.

Normal component code is one render expression. `Option`, vectors, fragments,
iterator-taking child builders, and conditional combinators avoid
statement-oriented tree assembly.

```rust
Some(Column::new().children((
    Heading::new(&self.title),
    self.ready.then(|| Details::new(&self.item)),
)))
```

Existing `battlement-ui` primitives are render values directly. Reactant adds
extension traits for keys, callbacks, refs, portal targets, and child values.
There is no parallel wrapper hierarchy and no final `.build()` call. See
[Components and rendering](component-authoring.md).

## Component lifecycle

A component mounts when reconciliation finds no committed sibling with the same
identity and type. Reactant creates its hook slots while rendering, commits its
logical instance with the host plan, and queues passive effect setup for the
next poll.

```rust
render_mount();
commit_hosts();
poll_effect_setup();
```

An update reuses the component instance. It applies pending hook queues, renders
from current props and state snapshots, and reconciles the new output. An equal
host result emits no Unity mutation even though the component rendered.

A component unmounts when its key, type, or parent identity disappears. The
commit makes handlers and refs unreachable, removes or retains hosts according
to Suspense and presence rules, and queues passive cleanup child before parent.

```rust
commit_unmount();
detach_refs();
poll_effect_cleanup();
```

Suspended initial work never mounts. Re-suspended committed work remains mounted
but hidden behind its boundary fallback. Reconnect recreates native hosts
without logically mounting or unmounting their components.

## Virtual tree

The **virtual tree** is Reactant's owned description of components, fragments,
portals, Suspense boundaries, and Battlement host elements. Component and
fragment nodes have logical identity but create no Unity object. Host nodes map
one-to-one to a `UiDocument` root or `UiNode` with a stable `ObjectId`.

Each mounted component instance stores:

- its component type and sibling identity;
- the committed props and rendered children;
- its hook slots and pending hook updates;
- registered effects and cleanup functions;
- logical parentage for context and events; and
- host descendants used during command generation.

Reactant has separate committed and work-in-progress trees. Rendering only
changes the work-in-progress tree. Reconciliation validates the complete result
before replacing committed state or exposing commands.

```rust
let work = runtime.render_roots(game);
let plan = runtime.reconcile(&committed, &work);
runtime.commit(work, plan)
```

The sample describes phase boundaries, not a required internal call layout.
The invariant is that rendering, validation, or suspension cannot partly mutate
the committed tree.

## Render, reconcile, and commit

A Reactant update has four observable phases.

1. Drain resource completions and store wakes, then mark dirty roots.
2. Render components; state and reducer hooks apply their queued work.
3. Compare host output with committed state and form a semantic mutation plan.
4. Commit runtime state and lower the plan to an ordered `UiCommit`.

Phase one drains cross-thread inputs and marks affected roots. It does not
evaluate reducer queues. Each reducer hook evaluates its actions during phase
two with the closure supplied by that render.

Rendering may call components more than once and may abandon a result. Render
methods must therefore be pure: they may read props and hooks, create render
values, and register hook work, but must not perform external effects.

The semantic plan names operations such as create, remove, reparent, reorder,
update properties, and change native subscriptions. It remains independent from
Battlement command serialization until the complete plan is valid.

```rust
Mutation::Move {
    node: row_id,
    parent: list_id,
    index: 2,
}
```

That separation is also the V2 animation seam. A future presence layer may
retain a logically removed node, animate it, and release its final destroy
operation without changing component identity or the tree comparison rules.

## UiCommit ordering

`UiCommit` is opaque because callers must not accidentally flatten commands that
have ordering dependencies. It stores ordered groups; commands within one group
may run in parallel, while each group completes before the next begins.

`UiCommit` is `#[must_use]`. A nonempty commit owns a shared delivery receipt
registered with its runtime. Consuming it through `append_ui`, `into_batch`, or
`into_groups` marks the receipt handed off. Dropping it unconsumed or calling
another mutating Reactant method while its receipt is outstanding panics.

The host must return handed-off groups in their original order before invoking
Reactant again. Battlement's sequential engine contract then guarantees Unity
applies that response before the next `dispatch`, `refresh`, or `poll`. The
explicit `into_groups` escape hatch cannot verify a custom command union, so
reordering or retaining those groups is a documented developer error.

```rust
impl UiCommit {
    pub fn is_empty(&self) -> bool;
    pub fn into_groups(self) -> Vec<Vec<CommandBody>>;
    pub fn into_batch(self, session: SessionId) -> Option<Batch>;
}
```

`ResponseUiExt` is implemented for `Response<C>` when `C: From<Command>`. It
offers `append_ui(commit)` and `append_ui_for_action(action_id, commit)`. Empty
commits add no batch. Each nonempty call creates one batch and preserves every
group returned by `into_groups`.

```rust
pub trait ResponseUiExt: Sized {
    fn append_ui(self, commit: UiCommit) -> Self;
    fn append_ui_for_action(
        self, action: ActionId, commit: UiCommit,
    ) -> Self;
}
```

```rust
let response = response.append_ui(commit);
```

Games that must interleave domain commands can consume the explicit groups.

```rust
let groups = commit.into_groups().into_iter()
    .map(ParallelCommandGroup::from_bodies)
    .collect();
let batch = Batch::new(BatchId::new_v4(), session_id, groups);
response.messages.push(ResponseMessage::Batch(batch));
```

A custom command union instead wraps every body with `Command::new_v4`, maps
it through `C::from`, and constructs `ParallelCommandGroup<C>` in the same
group order.

Creates are ordered parent before child. Reparents and index changes happen only
after required parents exist. Property and subscription changes run after the
target exists. Destruction is ordered child before parent unless a single parent
destroy already removes the whole native subtree.

Component unmount cleanup is a runtime action, not a Unity command. Passive
effect cleanup waits for the next poll. Ref detachment and logical event removal
become committed immediately so stale Unity events cannot reach an unmounted
component.

## Desired properties and resets

Reactant needs a declarative property state, while current `battlement-ui`
updates treat omission as "leave unchanged." Mutable host properties therefore
use `Prop<T>`.

```rust
pub enum Prop<T> {
    Unset,
    Set(T),
    Reset,
}
```

On create, `Unset` omits the field, `Set` serializes a value, and `Reset`
serializes `null`. On update, they mean no requested value, assign this value,
and remove authored state respectively. Style reset restores USS or Unity's
initial style. Other mutable fields restore the value recorded immediately
after their Unity control constructor runs.

Builders continue to accept ordinary values.

```rust
Label::new("Ready")
    .name("status")
    .color(Color::WHITE)
```

`From<T>` maps a value to `Set`, while `From<Option<T>>` maps `Some` to `Set`
and `None` to `Unset`. Primitive setters accept `impl Into<Prop<T>>`.

When an old rendered property is `Set` and the new render leaves it `Unset`,
Reactant emits `Reset`. This lets conditional props remove old native state.

```rust
Image::new()
    .source(self.selected.then_some(self.preview.clone()))
```

The comparison is exact:

| Committed | Desired | Mutation |
|---|---|---|
| `Unset` | `Unset` | none |
| `Unset` | `Set(value)` | set value |
| `Unset` | `Reset` | reset |
| `Set(old)` | equal `Set(old)` | none |
| `Set(old)` | different `Set(new)` | set new value |
| `Set(_)` | `Unset` or `Reset` | reset |
| `Reset` | `Unset` or `Reset` | none |
| `Reset` | `Set(value)` | set value |

The `battlement-ui` migration applies `Prop<T>` uniformly to mutable ordinary
properties. Create-only identity and host configuration fields use separate
types instead of pretending they can be reset.

## Events and application state

Unity events contain a target `ObjectId` and a typed payload. Reactant maps that
host target to the committed logical tree, then performs capture and bubble
propagation through components and portals.

```rust
Button::new("Play")
    .on_click(|game: &mut Game| game.start_game())
```

Rust cannot infer `Game` from the enclosing `Reactant<Game>` at this closure
site, so the parameter annotation is the idiomatic form. Reactant validates the
recorded type before commit and downcasts the borrowed model only after that
validation.

Handlers may queue hooks and mutate `G`. All handler changes are visible to the
single root refresh that follows propagation. A game method may also append
non-UI commands to an application-owned outbox; Reactant does not interpret or
order that outbox.

```rust
let ui = reactant.dispatch(&mut game, event);
let commands = game.drain_commands();
response.extend(commands).append_ui(ui);
```

`ReactantEvent<E>` supplies target, current target, phase, typed payload, and
`stop_propagation`. Reactant does not expose `prevent_default`: Rust receives
the event only after Unity has performed any native default action. See
[Reconciliation, events, and portals](reconciliation-events-and-portals.md).

## Hooks and post-commit effects

Free-function hooks use a thread-local render context to locate the current
component and positional hook slot.

```rust
let (count, set_count) = use_state(0);
Some(Button::new(count.to_string()).on_click(move |_game: &mut Game| {
    set_count.update(|old| old + 1);
}))
```

The thread-local context is set only while Reactant invokes `render`. Calling a
hook elsewhere panics. Event handlers run in a separate scoped batch context;
setters work there, but calling a hook still panics.

`use_effect` follows React dependency, setup, and cleanup behavior. It runs from
the next `Reactant::poll`, which the game calls from its next `Engine::poll`.
Unity has synchronously applied the prior response by then.

```rust
fn poll(&mut self) -> Option<Response> {
    let ui = self.reactant.poll(&mut self.game);
    ui.into_batch(self.session_id).map(Response::batch)
}
```

This is a passive post-commit effect boundary. It does not promise pre-paint
layout access, so `use_layout_effect` is reserved and absent. Geometry hooks
provide measurement asynchronously instead. See
[Hooks and effects](hooks-and-effects.md).

## Resources and Suspense

`Resource<K, T>` owns a typed loader and names a runtime-wide cache. A component
starts or reuses work with `use_resource`, then converts the ready value into a
render value.

```rust
let cards = use_resource(&self.cards, self.player_id);
Some(Suspense::new(Spinner::new()).child(
    cards.then(CardGrid::new),
))
```

A pending read returns a structural pending result to the nearest Suspense
boundary. It does not panic and it does not commit tentative component state.
The resource cache survives the abandoned render so retries reuse the same
future. See [Resources and Suspense](resources-and-suspense.md).

## Portals, refs, and geometry

A portal changes physical Unity parentage without changing component ancestry,
context, or event propagation.

```rust
create_portal(
    Menu::new(self.items.clone()),
    self.overlay_target.clone(),
)
```

`PortalTarget` may identify a Reactant host carrying the corresponding target or
an external Battlement container registered with the runtime. Changing a
portal's target or key unmounts and remounts its subtree.

`ElementRef` tracks attachment to a committed host. `use_geometry` observes
Unity geometry events and reports local layout and panel-space `worldBound`. It
does not compare coordinates across panels. See
[Refs, geometry, and floating UI](refs-geometry-and-floating-ui.md).

## Reconnect behavior

Calling `begin_session` for a reconnect preserves application-independent
runtime state:

- component identity and hook state;
- pending setter and reducer queues;
- resource cache entries and pending loaders;
- stable `use_id` values; and
- committed desired host properties.

Reactant serializes fresh documents from the current committed desired tree.
It invalidates element attachment state and all Unity-derived geometry because
the new session owns new native element instances, even when their `ObjectId`
values are preserved.

Refs attach again after the session response. Geometry remains unavailable until
Unity sends new events. A reconnect alone does not schedule ordinary effect
cleanup or setup because the logical component tree did not unmount.

External portal targets must be registered again if the new session changes
their object identities. A referenced target missing from the new snapshot is a
developer error; `SessionUi::into_response` panics before returning a response.

## Failure behavior

Reactant panics for developer errors, including:

- a hook outside rendering or inconsistent hook order;
- duplicate same-typed keys among siblings;
- a handler whose model type differs from `G`;
- a portal target owned by another runtime;
- two mounted hosts claiming one exclusive `ElementRef`;
- a loader task panic, rethrown on the engine thread.

Rendering and reconciliation validate before commit. A panic cannot leave the
committed Rust tree half-updated or emit a partial `UiCommit`. Normal Battlement
panic handling determines how the engine reports the failure to Unity.

## Validation contract

All crate tests are external integration tests. They use
`battlement-ui-fake::UiWorld` for focused host-state and command-journal checks,
or `battlement-fake::FakeClient` for complete `Engine` interactions. There are
no inline unit tests, compile-fail tests, or assertions against virtual nodes,
hook slots, caches, or mutation-plan internals.

Each test must end in a Unity-observable fact. Examples include the visible
label after a setter queue, native child order after a keyed move, subscriptions
after a handler change, fallback visibility while a resource is pending, and
the command journal emitted by a no-op rerender.

```rust
client.click(play_button);
assert_eq!(client.ui().text(status), "Playing");
```

The fake must apply real public snapshots and commands. Tests may arrange an
external store or future completion, but assertions remain on fake Unity state
or its executed-command journal.

## Manual QA

1. Mount two documents, with one root returning `None`, and begin a fake
   session. Confirm both documents appear, the empty document stays childless,
   and every allocated document and host ID is stable.
2. Dispatch one click that mutates `Game` and queues two state updates. Confirm
   one refresh produces the final visible value and no intermediate Unity tree.
3. Reorder a keyed list, change one property, and remove another row. Confirm
   retained rows keep IDs and state while the journal contains only the move,
   property update, and removal required.
4. Suspend a mounted subtree, complete its resource, and poll. Confirm fallback
   visibility, retry, state preservation, and the final committed tree.
5. Reconnect with mounted refs and an external portal. Confirm logical state is
   retained, geometry is cleared, documents are serialized first, and portal
   attachment follows the snapshot.
6. Produce dependent mutations on one physical parent and independent patches
   on two others. Confirm the fake command journal preserves every required
   barrier while placing independent commands in the same parallel group.
