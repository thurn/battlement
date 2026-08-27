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
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines Unity element
  attachment, batched measurement, coordinate conversion, and host actions.

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

## Design priorities

V1 provides declarative Rust component trees over every supported
`battlement-ui` primitive, React-compatible identity and named hooks, sparse
Unity mutations, logical events, portals, resources, Suspense, recoverable
render-error boundaries, and asynchronous geometry observation. Normal
authoring uses struct builders and one render expression without a `.build()`
step.

When goals conflict, Reactant chooses brevity and readability of UI code first,
React parity second, type safety third, and performance fourth. Performance
still constrains designs that would add blocking Rust round trips to Unity's
frame loop.

Reactant preserves React behavior whenever it adopts a React API name. When the
host cannot supply React's timing or control contract, V1 reserves the React
name and uses a Reactant-specific API.

## React expectations and deliberate differences

Reactant uses Rust expressions and UI Toolkit hosts, so some familiar React
operations have different syntax or timing. These differences are part of the
public contract rather than incidental implementation details.

- **Components:** component structs render builder expressions. There is no JSX
  or function-component API.
- **Empty output:** `render` returns `impl Render`; `()` is empty and
  `Option<R>` represents a conditional position.
- **State handles:** Rust handles use `.set`, `.update`, and `.send` instead of
  callable setter and dispatch values.
- **Lazy initialization:** `use_state_with` and `use_reducer_with` replace
  React's overloaded initializer arguments.
- **Events:** the brief `on_click` form receives `&mut G`;
  `on_click_event` also receives the typed event.
- **React-named hooks:** callback arguments come first, in React's order.
- **Dependencies:** dependency values are explicit, but Rust cannot lint a
  closure's captures against them. `use_effect_always` represents an omitted
  React dependency array.
- **Equality:** state, reducer, dependency, context, memo, and store comparisons
  use the value type's Rust `PartialEq`, not JavaScript `Object.is`. Domain
  equality, `NaN`, and signed zero can therefore behave differently.
- **External stores:** `use_external_store` subscribes on the next frame call,
  so it can display one stale committed frame during a render-to-subscribe race.
  The React-specific `useSyncExternalStore` name is reserved because V1 does not
  provide its synchronous anti-tearing contract.
- **Rendering:** `dispatch` invokes every root factory once because a handler can
  mutate any part of `G`. An opt-in `memo` component may reuse its committed
  subtree when its props are equal and no local work has dirtied the boundary.
- **Passive timing:** effects run on the next engine frame call after the host
  commit, which is not a universal browser-style post-paint guarantee.
- **Effect backlog:** if several commits precede a frame call, Reactant runs
  each committed setup and cleanup in order, including an obsolete setup that
  had not run yet. React normally flushes earlier passive effects before a
  later commit.
- **Layout:** V1 does not include `useLayoutEffect` because Reactant cannot run
  Rust code synchronously between Unity layout and paint. `use_geometry`
  reports one coherent Unity layout on the next frame exchange instead.
- **Native defaults:** `prevent_default` is absent because Unity has already
  performed native default behavior.
- **Event phases:** Reactant exposes Unity's propagation categories. In
  particular, `Focus`, `Blur`, `PointerEnter`, and `PointerLeave` are
  target-only rather than React synthetic bubbling events.
- **Suspense:** `use_resource` reads a typed runtime cache and follows
  positional hook rules instead of accepting an arbitrary promise.
- **Error boundaries:** `Result<R, E>` is a render value. `Err` propagates as an
  explicit structural render outcome to the nearest `ErrorBoundary`; boundaries
  never catch panics or Reactant invariant failures.
- **Refs:** `ElementRef` exposes attachment, geometry, and a fixed set of queued
  host actions rather than a mutable host object.
- **Roots:** roots are permanently registered before the first session and have
  no per-root unmount operation.
- **Context defaults:** a context stores a pure default factory and evaluates it
  once per runtime, rather than storing React's definition-time default value.
- **Failures:** uncaught render errors and render, hook, and loader developer
  failures reach Battlement's engine panic boundary. A caught render error is
  ordinary declarative fallback state and does not poison the runtime.

The [appendices](#appendix-index) define the exact Rust adaptations and identify
where a React name is intentionally unavailable.

## Battlement host contract

The linked Battlement UI design is a normative dependency, not background
reading. Its exact definitions of `ObjectId`, snapshots, documents, nodes,
elements, sparse properties, command batches, actions, event payloads, legal
children, and property resets are part of this contract. Reactant neither
duplicates nor changes them except for the geometry additions explicitly
specified below. An implementation must compile against those public types
rather than recreate parallel wire models.

Reactant targets four existing `battlement-ui` concepts. A `UiDocument` is a
complete snapshot tree for one Unity `UIDocument`. Each `UiNode` has a stable
`ObjectId` and one concrete `UiElement` variant.

After the snapshot, Reactant changes live hosts through
`VisualElementCreate`, `VisualElementUpdate`, and `VisualElementDestroy` command
bodies. Updates can change sparse properties, parent, or child index. A
`UiEvent` contains the original picked or focused host's `target_id` and a typed
`UiEventBody`. It does not identify the ancestor carrying the subscription;
`battlement-ui` forwards one event and derives subscribed deliveries from the
current host tree.

Battlement responses contain ordered messages. A snapshot precedes any batches
that refer to its objects. Each batch contains ordered parallel command groups;
the next group begins only after the previous group completes. Reactant's
`SessionUi` and `UiCommit` preserve this ordering.

Geometry adds one common visual-element observation field and one batched
client-action body to `battlement-ui`. The
[refs appendix](refs-geometry-and-floating-ui.md#observation-protocol) defines
their wire types and frame scheduling.

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
when the engine thread enters Reactant.

## Public runtime API

The public runtime surface is:

```rust
impl<G: 'static> Reactant<G> {
    pub fn new(spawner: impl Spawner) -> Self;
    pub fn register_root<V, R>(&mut self, document: UiDocument, view: V) -> Root
    where V: Fn(&G) -> R + 'static, R: Render + 'static;
    pub fn begin_session<'a>(&'a mut self, game: &G) -> SessionUi<'a>;
    pub fn dispatch(&mut self, game: &mut G, event: UiEvent) -> UiCommit;
    pub fn observe_geometry(
        &mut self,
        game: &G,
        batch: UiGeometryObservationBatch,
    ) -> UiCommit;
    pub fn refresh(&mut self, game: &G) -> UiCommit;
    pub fn poll(&mut self, game: &G) -> UiCommit;
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

`register_root` accepts a childless `UiDocument` and a `'static` view factory.
The factory receives an immutable model and returns any render value. Reactant
owns the document's children after registration; pre-populated children are a
developer error. Registration immediately rejects a pre-populated document or
a document/root `ObjectId` already owned by another registered root.

Because `Result<R, E>` is a render value, this signature also accepts a
fallible root factory without a runtime API change. Such an error is necessarily
uncaught unless the returned render tree places the fallible work beneath an
`ErrorBoundary`; an error that escapes the root panics before commit. Catching
scope follows the returned tree, so `?` used before constructing the boundary
cannot be caught by that boundary. See
[Error boundaries](component-authoring.md#error-boundaries).

```rust
reactant.register_root(hud_document, |game: &Game| {
    Hud::new()
        .phase(game.phase())
        .players(game.players())
});
```

A **root** is one independently reconciled render tree registered for one UI
document. Roots have separate component identity and context ancestry. They
share the runtime's resource cache, completed-task queue, and ID allocator.

The returned `Root` is an opaque, cloneable identity handle. Dropping it does
not change the UI. Registrations last for the runtime because their documents
correspond to application-owned snapshot objects. A view factory may return an
empty render value to clear its document without unregistering it.

Registration closes permanently when the first `SessionUi::into_response` or
`SessionUi::into_parts` completes successfully. Calling `register_root` after
that point panics. A render or validation panic before successful conversion
does not close registration or change committed runtime state. The fixed root
set makes reconnect snapshots complete without a late-document lifecycle
protocol.

The runtime starts in `Registering` and enters `Active` only at that successful
conversion. In `Registering`, root and portal registration, external-container
rebinding, `preload`, `invalidate`, `clear`, and `begin_session` are legal.
`dispatch`, `refresh`, `poll`, and `observe_geometry` panic because no host has
received Reactant's documents and no live commit can be delivered. Resource
tasks may complete into their queue, but only `begin_session` can freeze them.
A failed or dropped first `SessionUi` leaves the runtime in `Registering`.
After activation, every entry point has its normal behavior and
`begin_session` starts a reconnect transaction.

`begin_session` freezes resource completions, external-store notifications, and
hook updates present at entry. It applies that frozen work to a tentative
transaction, freshly renders every root against the supplied model, and returns
a **session UI** borrowing the runtime. It contains the prospective complete UI
state for a new Unity connection, including Reactant-owned `UiDocument` values
and portal commands targeting containers outside those documents.

```rust
let session = self.reactant.begin_session(&self.game);
let response = session.into_response(snapshot);
```

External portal commands run after the snapshot so their target objects exist.
Effects from the session do not run until the next engine frame call, after
Unity has applied the full session response. That call is `poll`, or
`observe_geometry` when the runner has a pending geometry batch. Resource
completions and store notifications arriving after the entry freeze wait for
the same boundary.

`SessionUi::into_response` validates the whole prospective session against the
snapshot, adds all documents, commits the runtime transaction, and returns a
response whose snapshot precedes the post-snapshot groups. `into_parts`
performs the same validation and runtime commit and returns the augmented
snapshot for a custom command union.

```rust
impl SessionUi<'_> {
    pub fn into_response(self, snapshot: Snapshot) -> Response;
    pub fn into_parts(self, snapshot: Snapshot) -> (Snapshot, UiCommit);
}
```

Both methods call the same private validation routine. It rejects duplicate
document or object IDs across caller and Reactant snapshots and missing
external portal targets before any runtime state changes. Reactant documents
are appended in root-registration order after caller-owned snapshot entries.
There is no API that extracts external portal commands without supplying the
snapshot they target. `SessionUi` is `#[must_use]`; its exclusive runtime borrow
prevents reentry, and dropping it unconsumed panics without committing the
transaction. A panic during conversion likewise leaves the prior runtime tree
and registration state intact. During an existing unwind, dropping `SessionUi`
discards the transaction and poisons the runtime instead of causing a second
panic.

Frozen completions, store wakes, and hook queues are acknowledged only by
successful conversion; abandoning the transaction leaves them pending for a
later `begin_session`. Loader tasks started by resource reads during tentative
rendering remain cached, just as they do for an ordinary suspended or abandoned
render. This resource-start exception does not install a desired tree, close
registration, or consume the entry-frozen work.

`dispatch` routes one Unity event, calls matching handlers, and invokes every
root factory exactly once after propagation finishes. `refresh` performs the
same root render without an event. Reconciliation may skip rendering unchanged
`memo` component subtrees. Both return an empty commit when Unity already
matches.

```rust
game.advance_turn();
let ui = reactant.refresh(&game);
let response = response.append_ui(ui);
```

`poll` performs post-commit work: effect cleanup and setup, ready resource
completion, external-store notifications, and updates queued by those actions.
It returns an empty commit when no Unity response is needed.

`observe_geometry` installs one coherent layout batch and then performs the
same post-commit work as `poll`. The Battlement runner submits a pending
geometry batch instead of making its otherwise empty poll call for that frame.
A frame therefore makes at most one of these transport calls; geometry never
adds a second synchronous Rust round trip.

## Entry-point lifecycle

Every entry point has one fixed responsibility. "Local updates" means queued
state or reducer work and dirty marks created by resource administration.

- `dispatch` handles one event, freezes no cross-thread input, runs no
  lifecycle callbacks, and invokes every root factory.
- `refresh` freezes no cross-thread input, runs no lifecycle callbacks, and
  invokes every root factory.
- `poll` freezes resources and store wakes, runs effects and store lifecycles,
  and renders dirty roots.
- `observe_geometry` additionally freezes geometry, then otherwise behaves like
  `poll`.
- `begin_session` freezes resources and store wakes, runs no lifecycle
  callbacks, and invokes every root factory.

Invoking a root factory does not require Reactant to render every component
beneath it. Each entry point applies the memoized-component bailout defined in
[Reconciliation, events, and portals](reconciliation-events-and-portals.md#memoized-component-bailout).

Every row applies local updates before rendering. Cross-thread arrivals after an
entry freeze remain queued. `begin_session` preserves already queued effects and
subscription operations for the first subsequent frame call; reconnect neither
drops nor reruns a committed effect merely because the native hosts changed.

`poll` and `observe_geometry` use this fixed order. Only the latter supplies a
geometry batch.

1. Freeze geometry, resource completions, and store notifications present at
   entry.
2. Atomically install the frozen geometry generation, apply resource
   completions, and read each notified store once.
3. Run queued store unsubscribe/subscribe operations and their immediate
   race-closing reads in commit order.
4. Run queued effect cleanups and setups in commit order.
5. Render every root dirtied by phases two through four exactly once.
6. Commit that render and return its `UiCommit`.

State setters called by effects and unequal race-closing store reads join the
phase-five render. Geometry, resource completions, and `StoreNotify` calls
arriving after phase one wait for the next frame call. Effects and subscriptions
created by the phase-six commit also wait for the next frame call; neither entry
point recursively flushes work created by its own returned commit.

A current resource-task panic is rethrown in phase two before lifecycle work.
Within phases three and four, an earlier commit's operations complete before a
later commit's operations.

## Components and render values

Components are structs whose fields are props. There are no function components
and no component-definition macro.

```rust
impl Component for Score {
    fn render(&self) -> impl Render {
        Label::new(format!("Score: {}", self.value))
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

Normal component code is one render expression. `()`, `Option`, `Result`,
vectors, fragments, iterator-taking child builders, and conditional combinators
avoid statement-oriented tree assembly.

```rust
Column::new()
    .child(Heading::new(self.title.clone()))
    .child(self.ready.then(|| Details::new(self.item.clone())))
```

Application components remain values in the same builder chain as primitives.

```rust
Column::new()
    .child(Heading::new("Match"))
    .child(Counter::new().initial_count(0))
```

Existing `battlement-ui` primitives are render values directly. Reactant adds
extension traits for keys, callbacks, refs, portal targets, and child values.
There is no parallel wrapper hierarchy and no final `.build()` call. See
[Components and rendering](component-authoring.md).

## Component lifecycle

A component mounts when reconciliation finds no committed sibling with the same
identity and type. Reactant creates its hook slots while rendering, commits its
logical instance with the host plan, and queues passive effect setup for the
next frame call.

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
to Suspense retention rules, and queues passive cleanup child before parent.

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
portals, Suspense boundaries, error boundaries, and Battlement host elements.
Component and fragment nodes have logical identity but create no Unity object.
Host nodes map one-to-one to a `UiDocument` root or `UiNode` with a stable
`ObjectId`.

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
The invariant is that rendering, validation, suspension, or a recoverable
render error cannot partly mutate the committed tree.

## Render, reconcile, and commit

A render-producing entry point has four observable phases after the input work
assigned by the lifecycle table has run.

1. Select roots and apply their queued local hook work.
2. Render components; reducer hooks evaluate queued actions.
3. Compare host output with committed state and form a semantic mutation plan.
4. Commit runtime state and lower the plan to an ordered `UiCommit`.

Cross-thread inputs are never drained implicitly here. The calling entry point
has already frozen and applied the inputs it owns. Each reducer hook evaluates
its actions during phase two with the closure supplied by that render.

Rendering may call components more than once and may abandon a result. Render
methods must therefore be pure: they may read props and hooks, create render
values, and register hook work, but must not perform external effects.

An explicit `Err` returned through a render value abandons the failing primary
subtree and is offered to the nearest `ErrorBoundary`. If that boundary renders
its fallback successfully, the fallback participates in the same reconciliation
and atomic commit as ordinary output. See
[Components and rendering](component-authoring.md#error-boundaries).

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

## UiCommit ordering

`UiCommit` is opaque because callers must not accidentally flatten commands that
have ordering dependencies. It stores ordered groups; commands within one group
may run in parallel, while each group completes before the next begins.

`UiCommit` is `#[must_use]`. A nonempty commit owns a shared delivery receipt
registered with its runtime. Consuming it through `append_ui`, `into_batch`, or
`into_groups` marks the receipt handed off. Dropping it unconsumed or calling
another mutating Reactant method while its receipt is outstanding panics.
During an existing unwind, `Drop` marks the runtime poisoned instead of causing
a second panic; any later runtime call reports the unconsumed commit.

The host must return handed-off groups in their original order before invoking
Reactant again. Battlement's sequential engine contract then guarantees Unity
applies that response before the next `dispatch`, `refresh`, or `poll`. The
same rule applies to `observe_geometry` and `begin_session`. The explicit
`into_groups` escape hatch cannot verify a custom command union, so reordering
or retaining those groups is a documented developer error.

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

One-shot host actions requested through `ElementRef` follow every mutation from
their callback batch. They retain invocation order as one sequential command
group per action.

Component unmount cleanup is a runtime action, not a Unity command. Passive
effect cleanup waits for the next frame call. Ref detachment and logical event
removal become committed immediately so stale Unity events cannot reach an
unmounted component.

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
Button::new(count.to_string()).on_click(move |_game: &mut Game| {
    set_count.update(|old| old + 1);
})
```

The thread-local context is set only while Reactant invokes `render`. Calling a
hook elsewhere panics. Event handlers run in a separate scoped batch context;
setters work there, but calling a hook still panics.

`use_effect` follows React setup, dependency, and cleanup behavior. It runs from
the next `Reactant::poll` or `Reactant::observe_geometry`, after Unity has
synchronously applied the prior response.

```rust
fn poll(&mut self) -> Option<Response> {
    let ui = self.reactant.poll(&self.game);
    ui.into_batch(self.session_id).map(Response::batch)
}
```

This is a passive post-commit effect boundary. It does not promise pre-paint
layout access, so V1 does not include `use_layout_effect`. Geometry hooks
provide measurement asynchronously instead. See
[Hooks and effects](hooks-and-effects.md).

## Resources and Suspense

`Resource<K, T>` owns a typed loader and names a runtime-wide cache. A component
starts or reuses work with `use_resource`, then converts the ready value into a
render value.

```rust
let cards = use_resource(&self.cards, self.player_id);
Suspense::new(Spinner::new()).child(
    cards.then(CardGrid::new),
)
```

A pending read returns a structural pending result to the nearest Suspense
boundary. It does not panic and it does not commit tentative component state.
The resource cache survives the abandoned render so retries reuse the same
future. See [Resources and Suspense](resources-and-suspense.md).

A resource with an expected failure mode may include that failure in `T`, such
as `Result<CardSet, CardLoadError>`. `ResourceRead::then` receives `Arc<T>`, so
turning the failure into a render `Err` requires an owned error obtained by
cloning it or by storing an owned shared error handle in `T`. Reactant does not
implicitly move an error out of the cache. Resource-task panics remain developer
failures and bypass error boundaries.

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

`ElementRef` tracks attachment to a committed host, exposes supported one-shot
host actions, and can be read by `use_geometry`. Unity reports all changed
observations from one completed layout as one atomic generation. Reactant
retains the last coherent value while a newer generation is pending and never
compares coordinates across panels. See
[Refs and geometry](refs-geometry-and-floating-ui.md).

## Reconnect behavior

Calling `begin_session` for a reconnect preserves application-independent
runtime state:

- component identity and hook state;
- pending setter and reducer queues;
- resource cache entries and pending loaders;
- stable `use_id` values; and
- committed desired host properties.

Reactant applies work frozen at session entry, renders every registered root
against the current model, and serializes fresh documents from the prospective
desired tree. Successful `SessionUi` conversion commits that tree. It
invalidates element attachment state and all Unity-derived geometry because the
new session owns new native element instances, even when their `ObjectId`
values are preserved.

Refs attach again at successful response handoff. Geometry remains unavailable
until Unity sends new observations. A reconnect alone does not schedule
ordinary effect cleanup or setup because the logical component tree did not
unmount.

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
- authored native event subscriptions on a Reactant-rendered primitive;
- calling an `ElementRef` host action during render, outside a callback, or from
  another runtime's callback;
- a loader task panic, rethrown on the engine thread.

An `Err` produced by `Result<R, E>` is not a developer error while an
`ErrorBoundary` catches it. It renders the boundary fallback without poisoning
the runtime. An error that reaches a root without a boundary is a developer
error and panics before commit; it uses the same abandoned-render behavior as
other render-time failures. Reactant does not use `catch_unwind` to implement
error boundaries. Panics from components, hooks, fallbacks, loaders, callbacks,
validation, or runtime invariants always retain their documented panic behavior.

Rendering and reconciliation validate before commit. A panic cannot leave the
committed Rust tree half-updated or emit a partial `UiCommit`. Normal Battlement
panic handling determines how the engine reports the failure to Unity.

Callbacks are different because arbitrary Rust side effects cannot be rolled
back. If an event handler, reducer, store operation, effect setup, or cleanup
panics, Reactant emits no partial `UiCommit`, discards updates and host actions
queued by the failing callback, and becomes poisoned. Earlier callbacks in the
same sequence may already have changed external state; later callbacks do not
run. Every later runtime entry panics immediately. Loader-task panics use the
same rule when delivered on the engine thread. Recovery is to discard the
runtime and construct a new one.

## Validation contract

Behavioral crate tests are external integration tests. They use
`battlement-ui-fake::UiWorld` for focused host-state and command-journal checks,
or `battlement-fake::FakeClient` for complete `Engine` interactions. Narrow
rustdoc examples verify required-prop and `ErrorBoundary` typestate, fallible
components using `?`, concrete and wrapped error types, and rejected unsupported
error forms. Tests do not assert virtual nodes, hook slots, caches, or
mutation-plan internals.

When the crate lands, documentation checks must compile every public API and
ordinary usage fence with hidden setup; private algorithm sketches must be
marked non-compiling. Reconciliation also has randomized black-box tests that
compare fake Unity's final physical tree with a simple desired-tree oracle,
while exhaustive property tests cover every `Unset`, `Set`, and `Reset`
transition supported by each primitive.

Each behavioral test must end in a Unity-observable fact. Examples include the
visible label after a setter queue, native child order after a keyed move,
subscriptions after a handler change, fallback visibility while a resource is
pending, and the command journal emitted by a no-op rerender.

```rust
client.click(play_button);
assert_eq!(client.ui().text(status), "Playing");
```

The fake must apply real public snapshots and commands. Tests may arrange an
external store or future completion, but assertions remain on fake Unity state
or its executed-command journal.

## Manual QA

1. Register two documents, with one root returning `()`, and begin a fake
   session. Confirm both documents appear, the empty document stays childless,
   and every allocated document and host ID is stable. Then confirm another
   `register_root` call panics without changing the session.
2. Dispatch one click that mutates `Game` and queues two state updates. Confirm
   one refresh produces the final visible value and no intermediate Unity tree.
3. Reorder a keyed list, change one property, and remove another row. Confirm
   retained rows keep IDs and state while the journal contains only the move,
   property update, and removal required.
4. Suspend a mounted subtree, complete its resource, and poll. Confirm fallback
   visibility, retry, state preservation, and the final committed tree.
5. Return an error below nested error boundaries. Confirm the nearest fallback
   receives the concrete error, the failing subtree unmounts only when that
   fallback commits, and a later successful render mounts a fresh primary
   subtree. Then confirm an error from the fallback reaches the outer boundary.
6. Let an error escape a root after a successful commit. Confirm the call
   panics without changing Unity, then fix the model and confirm a later refresh
   succeeds because the render-time error did not poison the runtime.
7. Reconnect with mounted refs and an external portal. Confirm logical state is
   retained, geometry is cleared, documents are serialized first, and portal
   attachment follows the snapshot.
8. Produce dependent mutations on one physical parent and independent patches
   on two others. Confirm the fake command journal preserves every required
   barrier while placing independent commands in the same parallel group.
9. Submit one geometry batch containing several changed observations instead of
   that frame's poll. Confirm one render sees the complete generation and the
   transport records only one scheduled frame round trip.
