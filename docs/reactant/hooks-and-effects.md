# Reactant Hooks and Effects

This appendix defines Reactant's free-function hooks. It is part of the
[Battlement Reactant technical design](reactant-technical-design.md). The APIs
copy React behavior where the Rust and Unity execution model can provide the
same contract.

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the committed host mutations and Unity event boundary used by hooks.
- [React: built-in hooks](https://react.dev/reference/react/hooks) is the
  behavioral reference for APIs that keep a React name.
- [React: rules of hooks](https://react.dev/reference/rules/rules-of-hooks)
  explains positional hook ordering.
- [React: state as a snapshot](https://react.dev/learn/state-as-a-snapshot)
  explains why a setter does not change the value captured by the current
  render.
- [React: `useEffect`](https://react.dev/reference/react/useEffect) defines the
  post-commit synchronization contract Reactant preserves.
- [React: `useLayoutEffect`](https://react.dev/reference/react/useLayoutEffect)
  defines the pre-paint layout contract Reactant cannot preserve.
- [React: queueing state
  updates](https://react.dev/learn/queueing-a-series-of-state-updates) explains
  batching and updater order.
- [Refs and geometry](refs-geometry-and-floating-ui.md) defines refs attached to
  Unity elements and the measurement replacement for `use_layout_effect`.

## Hook context

Whenever Reactant invokes `Component::render`, it installs a per-thread
**render context** containing the current runtime, component instance, hook
cursor, and work-in-progress hook slots. A free-function hook reads that context
and consumes the next positional slot.

```rust
let (open, set_open) = use_state(false);
let menu_id = use_id();
```

Each committed slot records its hook kind and Rust `TypeId`. On a later render,
the hook at that position must have the same kind and stored types.

```rust
if self.enabled {
    let _ = use_state(0); // Invalid: conditional hook.
}
```

Reactant also checks that the completed render used exactly the committed number
of hooks. A changed count, kind, or type panics before commit.

Calling a hook without a render context panics. Event handlers use a separate
batch context, so setters work there but hooks still do not.

```rust
.on_click(move |_game: &mut Game| {
    set_open.set(true); // Valid.
    use_id();           // Panics.
})
```

The context is always restored with a scope guard, including when rendering
panics. Nested Reactant runtimes on the same thread therefore cannot leak their
current component into one another.

The implementation uses no thread-local macro. One
`OnceLock<Mutex<HashMap<ThreadId, Vec<usize>>>>` stores a stack of scoped
context addresses for each rendering thread. A render pushes the address of its
stack-pinned context and a guard pops it before that context can move or drop.
A hook looks up only `ThreadId::current`, releases the mutex, and dereferences
the top address in the crate's one audited unsafe block. `Reactant: !Send`
prevents another thread from owning that context, and the guard makes an
address unreachable before its stack storage expires. An empty stack is the
hook-outside-render panic. This ordinary static registry preserves nested
rendering without `thread_local!` or another macro.

`Reactant`, `StateSetter`, `ReducerDispatch`, `Ref`, `ElementRef`, and
`Callback` are deliberately `!Send + !Sync`; private `Rc` ownership enforces
engine-thread use. `Context<T>` is only a `Copy + Send + Sync` static identity;
its values remain in the engine-thread tree. Cross-thread delivery is limited
to the explicitly thread-safe `StoreNotify`, resource loader, and completion
interfaces.

## State snapshots

`use_state` stores component-local state and returns an immutable clone plus a
stable setter.

```rust
pub fn use_state<T>(initial: T) -> (T, StateSetter<T>)
where
    T: Clone + PartialEq + 'static;
```

`initial` is used only when the component instance mounts. For expensive or
stateful initialization, `use_state_with` calls its closure once.

```rust
pub fn use_state_with<T>(initial: impl FnOnce() -> T)
    -> (T, StateSetter<T>)
where T: Clone + PartialEq + 'static;
```

```rust
let (deck, set_deck) = use_state_with(|| Deck::starting());
```

The returned value is the snapshot for this render. Calling its setter does not
modify that clone or the tree currently being calculated.

```rust
let (count, set_count) = use_state(0);
let click_count = count;
let click = move || set_count.set(click_count + 1);
```

Invoking `click` later queues `1`; the captured `count` remains `0`.

Use `Rc`, `Arc`, or another cheap shared value when cloning a large state object
would be wasteful.

## State setters and batching

`StateSetter<T>` has stable identity for the lifetime of its component and hook
slot. It is cloneable and may outlive one rendered callback.

```rust
impl<T: Clone + 'static> StateSetter<T> {
    pub fn set(&self, value: T);
    pub fn update(&self, update: impl Fn(T) -> T + 'static);
}
```

```rust
set_count.set(4);
set_count.update(|old| old + 1);
```

`.set(value)` queues a replacement. `.update(function)` queues a calculation
against the state produced by earlier entries in the same queue. Entries are
applied in call order before the next render.

Updater functions are pure and reusable. Reactant may invoke one again when a
render suspends or is otherwise abandoned; the queue is acknowledged only by a
successful commit. Replacement values are cloned for the same reason.

```rust
set_count.update(|n| n + 1);
set_count.update(|n| n + 1);
set_count.set(10); // Final state is 10.
```

All setters called during one Unity event, including capture and bubble
handlers, form one batch. Reactant refreshes roots after propagation, not after
each setter.

Setters called outside a Reactant event on the engine thread enqueue work for
the next `dispatch`, `refresh`, `poll`, `observe_geometry`, or `begin_session`.
V1 setters are not cross-thread handles. External threads notify Reactant
through resources or `ExternalStore`.

After applying a queue, Reactant compares final and committed state. Equal state
does not by itself rerender the component. A parent refresh may still render it
with new props.

Calling a setter after its component unmounts is a no-op. The setter retains no
component state and cannot resurrect the instance.

Reactant copies React's render-phase update rule. A setter belonging to the
component currently rendering may queue an update. Reactant discards that
component output, applies the queue, and renders it again before reconciling.

```rust
if previous != self.value {
    set_previous.set(self.value);
}
```

A setter for another component during render panics. More than 25 consecutive
render-phase retries also panics, preventing an infinite loop. Render-phase
updates remain an escape hatch; derived values should normally be calculated
directly.

## Reducers

`use_reducer` centralizes related state transitions. A reducer is pure and
returns the next state from the previous state and one action.

```rust
let (state, dispatch) = use_reducer(|state, action| {
    reduce_game_ui(state, action)
}, initial);
```

The public API is:

```rust
pub fn use_reducer<S, A, F>(reducer: F, initial: S)
    -> (S, ReducerDispatch<A>)
where
    S: Clone + PartialEq + 'static,
    A: Clone + 'static,
    F: Fn(&S, A) -> S + 'static;
```

`use_reducer_with` lazily constructs the initial state. When the hook is
consumed during render, its current reducer closure processes every queued
action in order. A later closure at the same source location has the same
concrete type and handles actions queued before that render as well as later
actions.

```rust
let step_size = self.step_size;
let (state, dispatch) = use_reducer(move |state, action| {
    reduce_game_ui(state, action, step_size)
}, initial);
```

The action stores no reducer snapshot. If `step_size` changes before a queued
action is rendered, that render's newly supplied closure processes the action.
This matches React: dispatch identifies the hook, while the render processing
the queue supplies the reducer.

Actions queue and batch like state updates.

```rust
dispatch.send(Action::Select(card_id));
dispatch.send(Action::Confirm);
```

`ReducerDispatch<A>` has stable identity and becomes a no-op after unmount.

```rust
impl<A: Clone + 'static> ReducerDispatch<A> {
    pub fn send(&self, action: A);
}
```

The lazy initializer variant changes only construction of the mount value.

```rust
pub fn use_reducer_with<S, A, F>(reducer: F, initial: impl FnOnce() -> S)
    -> (S, ReducerDispatch<A>)
where S: Clone + PartialEq + 'static,
      A: Clone + 'static,
      F: Fn(&S, A) -> S + 'static;
```

## Dependency lists

Memoization and effects compare explicit **dependency lists**, values whose
fields collectively decide whether captured calculations are stale.

```rust
let filtered = use_memo(|| {
    filter_cards(&self.cards, &self.query)
}, (self.cards.clone(), self.query.clone()));
```

`Dependencies` is a public marker with one blanket implementation.

```rust
pub trait Dependencies: Clone + PartialEq + 'static {}

impl<T> Dependencies for T
where T: Clone + PartialEq + 'static {}
```

Therefore `()` is an empty list, one ordinary value is a one-item dependency,
tuples are heterogeneous lists, `Vec<T>` is a dynamic homogeneous list, and a
named struct is a readable long list.

```rust
let label = use_memo(|| format_score(self.score), self.score);
```

Rust cannot lint closure captures against declared dependencies. The caller
must include every changing prop, state snapshot, context value, and local value
read by the calculation. Stable setters, dispatchers, refs, and IDs may be
omitted.

## Memoized values

`use_memo` calculates a value on mount and whenever its dependencies differ.

```rust
pub fn use_memo<D, T>(calculate: impl FnOnce() -> T, deps: D) -> T
where
    D: Dependencies,
    T: Clone + 'static;
```

The committed value is cloned into the current render. A suspended or abandoned
render does not replace the committed memo. Reactant may discard memoized values
when their component unmounts; memoization is a performance tool, not storage.

```rust
let sorted = use_memo(|| {
    sorted_cards(&self.cards)
}, self.cards.clone());
```

`calculate` runs during rendering and must be pure. It must not call hooks.

## Memoized callbacks

`use_callback` returns a cloneable callback with stable equality while its
dependencies remain equal.

```rust
pub fn use_callback<D, F>(callback: F, deps: D) -> Callback<F>
where D: Dependencies, F: 'static;
```

```rust
let dependency = self.card_id.clone();
let captured = dependency.clone();
let inspect = use_callback(move |game: &mut Game| {
    game.inspect(captured);
}, dependency);
```

`Callback<F>` contains an `Rc<F>`, implements `Deref<Target = F>`, and compares
by `Rc` identity. It therefore supports normal closure call syntax. Reactant
returns a clone of the committed callback while dependencies compare equal and
constructs a new callback when they differ. Old clones keep their old captures.

The callback owns its captures and is `'static`. Its identity is useful when a
callback is itself a dependency or a prop stored in `Node`. Native subscriptions
depend on event kind and physical event island, not callback identity.

`use_callback` is equivalent to memoizing the callback value. It does not call
the closure and does not make captured values reactive automatically.

## Mutable refs

`use_ref` returns one stable, cloneable `Ref<T>` for the mounted hook slot.

```rust
pub fn use_ref<T: 'static>(initial: T) -> Ref<T>;
pub fn use_ref_with<T: 'static>(initial: impl FnOnce() -> T) -> Ref<T>;
```

```rust
let attempts = use_ref(0_u32);
attempts.with_mut(|value| *value += 1);
```

Changing a ref does not schedule a render. The safe public operations are
`get`, `replace`, `with`, and `with_mut`; no borrow guard may escape the method
call. `get` requires `T: Clone`, while the closure operations do not.

```rust
let current = attempts.get();
attempts.replace(0);
```

`Ref<T>` is single-threaded in V1. Render code must not mutate a ref whose value
affects the same render; that hides state from reconciliation.

Element attachment uses `use_element_ref`, not `use_ref`. The specialized ref
has commit and reconnect semantics described in
[Refs and geometry](refs-geometry-and-floating-ui.md).

## Context

`Context<T>` passes an owned value through logical component ancestry without
threading it through every intermediate prop.

```rust
static THEME: Context<Theme> = Context::new(Theme::default);
```

A context has a `'static` identity and a `fn() -> T` default factory.
`Context::new` and `RequiredContext::new` are `const fn`, so the shown static
declarations compile. The factory must be pure and deterministic. Reactant
evaluates it on the first provider-free read in one runtime, stores that value
for the runtime's lifetime, and clones it for every later root and reconnect.
`use_context` returns the nearest provider value or a clone of that stored
default.

```rust
pub fn use_context<T>(context: &'static Context<T>) -> T
where T: Clone + PartialEq + 'static;
```

```rust
let theme = use_context(&THEME);
Panel::new().class(theme.panel_class())
```

Providers are transparent render nodes.

```rust
THEME.provider(self.theme.clone())
    .child(GameScreen::new())
```

`T` must be `Clone + PartialEq + 'static`. When a provider value changes,
Reactant schedules consumers beneath that provider even if their ordinary props
are unchanged.

Context follows the logical tree through portals. It does not cross independent
roots, including two roots mounted into the same Unity panel.

For values that must not have a default, `RequiredContext<T>` makes a missing
provider a render-time panic.

```rust
static SESSION: RequiredContext<Session> = RequiredContext::new();
```

## Stable IDs

`use_id` returns an opaque `ReactantId` stable for one mounted component slot.

```rust
pub fn use_id() -> ReactantId;
```

```rust
let label_id = use_id();
TextField::new().labelled_by(label_id)
```

IDs are unique within one `Reactant` runtime, survive rerenders and reconnects,
and change after unmount/remount. They serialize to stable strings only through
their `Display` implementation.

`ReactantId` is for host relationships, accessibility, and application lookup.
It is not an `ObjectId` and must not be used as a component key.

## External stores

`use_external_store` reads state owned outside Reactant and closes a
render-to-subscribe race on the next frame call.

```rust
pub fn use_external_store<S>(store: S) -> S::Snapshot
where S: ExternalStore;
```

```rust
let settings = use_external_store(self.settings.clone());
Label::new(settings.language_name())
```

One comparable store object supplies both operations.

```rust
pub trait ExternalStore: Clone + PartialEq + Send + Sync + 'static {
    type Snapshot: Clone + PartialEq + Send + Sync + 'static;
    fn snapshot(&self) -> Self::Snapshot;
    fn subscribe(&self, notify: StoreNotify) -> Subscription;
}
```

`StoreNotify` is a cloneable `Send + Sync` callback with one public `notify`
method. `Subscription::new(cleanup)` accepts one
`FnOnce() + Send + 'static`; dropping the subscription invokes that cleanup at
most once.

```rust
impl StoreNotify { pub fn notify(&self); }
impl Subscription {
    pub fn new(cleanup: impl FnOnce() + Send + 'static) -> Self;
}
```

```rust
let subscription = Subscription::new(move || source.unsubscribe(id));
notify.notify();
```

During render, Reactant calls `snapshot`. After commit, it subscribes on the
next frame call and immediately reads another snapshot. If that value differs,
it queues a new render, closing the render-to-subscribe race.

`StoreNotify` is cloneable and thread-safe. Calling it queues a wake without
reading the store on the notifying thread. The engine thread reads and compares
the snapshot during `poll` or `observe_geometry`.

When the comparable store object changes, Reactant unsubscribes from the old
object before subscribing to the new one. Dropping `Subscription` performs the
unsubscribe. Unmount drops the active subscription on the next frame call.

Notifications coalesce to one pending wake per hook generation until the
engine thread reads the snapshot. A notification during old-store unsubscribe
is stale and ignored. A notification during `subscribe` may queue a wake, and
the mandatory immediate snapshot read closes the race even if the store does
not call `notify`. A changed immediate snapshot joins the current frame render.
Snapshot, subscribe, or unsubscribe panics use the runtime poisoning rule.

There is no `getServerSnapshot` argument because Reactant has no server-rendered
or hydration tree. `begin_session` reads the normal snapshot.

The hook deliberately does not use React's `useSyncExternalStore` name.
Subscription and the race-closing read happen on the next frame call, so one
already committed response may contain the earlier snapshot. Applications that
require the model and UI to change in the same response keep that state in `G`
or explicitly update it before `refresh`.

## Passive effects

`use_effect` synchronizes a committed component with an external system. Like
React, it accepts setup first and dependencies second.

```rust
pub fn use_effect<D, S, C>(setup: S, deps: D)
where D: Dependencies,
      S: FnOnce() -> C + 'static,
      C: IntoEffectCleanup;
```

```rust
let room_id = self.room_id;
use_effect(move || {
    let connection = chat.connect(room_id);
    move || connection.disconnect()
}, room_id);
```

Setup may return `()` or one `'static` cleanup closure through a sealed
`IntoEffectCleanup` conversion. Dependencies are cloned into committed effect
state.

```rust
pub trait IntoEffectCleanup: private::Sealed {
    fn into_cleanup(self) -> Option<Box<dyn FnOnce()>>;
}
```

The crate implements it for `()` and every `FnOnce() + 'static`. Effect setup,
cleanup, subscription cleanup, and their panics all run on the engine thread.

Rust requires one static setup return type. Conditional cleanup returns one
closure that owns an `Option` instead of conditionally returning `()` or a
closure.

```rust
use_effect(move || {
    let connection = enabled.then(|| chat.connect(room_id));
    move || drop(connection)
}, (enabled, room_id));
```

```rust
use_effect(move || analytics.screen("inventory"), ());
```

`()` means setup after mount and cleanup after unmount. It does not mean every
commit. The explicit every-commit form is:

```rust
use_effect_always(move || analytics.frame());
```

Its setup and cleanup contract is otherwise identical.

```rust
pub fn use_effect_always<S, C>(setup: S)
where S: FnOnce() -> C + 'static,
      C: IntoEffectCleanup;
```

After a commit with changed dependencies, Reactant queues the old cleanup and
new setup as one ordered effect operation. On the next frame call, cleanup runs
first and setup runs second.

```rust
old_cleanup();
let next_cleanup = new_setup().into_cleanup();
```

An effect closure captures the props and state snapshot from the render that
registered it. A later render does not change those captures.

If several Unity commits occur before one poll, Reactant retains their effect
operations in commit order. It does not collapse an unrun setup into a later
version: the older setup, its cleanup, and the newer setup execute in the same
order those committed trees became visible.

## Effect timing

React's passive effects run after a commit and do not provide the pre-paint
guarantee of `useLayoutEffect`. Reactant maps that boundary to the next engine
frame call into Reactant.

```rust
fn poll(&mut self) -> Option<Response> {
    let ui = self.reactant.poll(&self.game);
    ui.into_batch(self.session_id).map(Response::batch)
}
```

Unity applies responses synchronously on its main thread and invokes
`Engine::poll` once per frame. When a geometry batch is pending, the runner
submits it instead, and the engine calls `Reactant::observe_geometry`. Both
Reactant entry points perform the same effect work. An effect registered by one
response therefore runs after Unity applies that response and no later than the
next frame call serviced by the game.

Reactant does not call this a universal post-paint boundary. Unity may or may
not paint between applying the response and servicing the next frame call. The
guarantee is committed host state before setup, matching the portable part of
React's `useEffect` contract.

React does not expose cross-component passive-effect order as a public
contract. Reactant nevertheless needs deterministic engine-thread behavior, so
effects queued by one commit traverse child components before parents. For a
changed effect, its cleanup immediately precedes its replacement setup.
Unmount cleanups use the same child-before-parent traversal.

An effect setter schedules another render after all effect operations from the
current committed batch finish.

```rust
use_effect(move || set_ready.set(true), ());
```

The follow-up `UiCommit` is returned from `poll` or `observe_geometry`. Reactant
does not merge it into the already-applied response that caused the effect.

If the runtime is destroyed, remaining cleanups run synchronously because no
future poll exists. A reconnect does not rerun effects when the logical tree and
dependencies are unchanged.

## Why there is no use_layout_effect

React guarantees that `useLayoutEffect` runs after host mutation and before the
browser repaints, including any state update it schedules. Battlement has no
synchronous Rust callback between Unity layout and paint.

Running on the next frame call is too late:

```rust
render_hidden();
// Unity lays out and paints a frame.
poll_and_measure();
```

Calling an API named `use_layout_effect` with that behavior would violate React
expectations and cause visible placement bugs. V1 reserves the name and instead
offers `use_element_ref` plus `use_geometry`. Content that cannot appear before
its first measurement uses `visibility: hidden`; see
[Refs and geometry](refs-geometry-and-floating-ui.md).

`use_insertion_effect` is also absent because Reactant has no pre-host-mutation
style-insertion phase.

## Manual QA

1. Click a fake button that queues two updater functions and one replacement.
   Confirm Unity shows only the final state and receives one update commit.
2. In one event, queue reducer actions whose order changes the result. Confirm
   the final Unity label reflects call order and the journal contains no
   intermediate render.
3. Change reducer actions and memo dependencies independently. Confirm visible
   output changes only for the affected dependency or final reducer state.
4. Mount an effect that sets state. Confirm its setup does not run in the event
   response, then poll once and observe the follow-up Unity mutation.
5. Change effect dependencies and unmount the component. Drive cleanup effects
   that alter visible test state and confirm cleanup-before-setup and
   child-first unmount order through `UiWorld`.
6. Notify an external store between render and subscription, then from another
   thread. Poll and confirm both changes become visible without a missed value.
7. Reconnect and confirm state, IDs, and unchanged effect subscriptions persist
   while the new Unity document reflects the latest state.
