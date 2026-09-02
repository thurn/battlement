# Reactant Resources and Suspense

This appendix defines asynchronous resource loading and fallback rendering. It
is part of the
[Battlement Reactant technical design](reactant-technical-design.md). Reactant
copies React Suspense behavior while using explicit Rust values instead of
throwing promises or panicking for control flow.

## Loading in an application

`App` services a wake-driven cooperative executor automatically. Futures may
remain pending and wake later; each servicing pass has a bounded polling budget.
Use asynchronous I/O or externally completed futures, without blocking the engine
thread. `.spawner(custom_spawner)` selects a specialized executor when needed.

A component can invalidate a resource through its owning runtime:

```rust
let control = use_resource_control(&self.cards);
let player_id = self.player_id;
Button::new("Refetch").on_click(move || control.invalidate(player_id))
```

Invalidation cancels the pending value and schedules a fresh read through the
application's work queue. Reconnect discards obsolete resource work; application
destruction cancels owned tasks and runs component effect cleanup. A resource's
loader supplies application data, so a deterministic demonstration can await a
channel completed by its Resolve button.

## Related information

- [Battlement UI technical design](../battlement-ui-technical-design.md) defines
  the snapshots and commands used to show fallbacks and ready content.
- [React: Suspense](https://react.dev/reference/react/Suspense) defines
  fallback, retry, nested-boundary, and state-preservation behavior.
- [React: `use`](https://react.dev/reference/react/use) explains cached
  asynchronous reads and why uncached work created during render is unstable.
- [Hooks and effects](hooks-and-effects.md) defines positional hooks and the
  engine-thread frame call where completions become renders.
- [Reconciliation](reconciliation-events-and-portals.md) defines committed and
  work-in-progress trees that make abandoned renders safe.

## Resource purpose

A **resource** is a typed loader and cache namespace. It gives multiple
components one identity for asynchronous work with the same key.

```rust
let cards = Resource::new(|player_id| async move {
    load_cards(player_id).await
});
```

`Resource<K, T, E = Infallible>` does not store entries itself. Each `Reactant`
runtime stores a separate cache indexed by the resource's identity and `K`.
Cloning a `Resource` preserves identity and therefore shares entries inside a
runtime.

```rust
let cards_for_deck = cards.clone();
let cards_for_search = cards.clone();
```

Two independently constructed resources using identical loader code do not
share entries.

`Resource::new` allocates a nonzero process-unique `ResourceId`; clones retain
it. Each runtime cache maps that ID to one erased bucket recording the exact
`TypeId` values for `K`, `T`, and `E`. Private downcasts check all three IDs and
panic on an internal mismatch. The bucket owns typed keys and uses their
ordinary `Hash` and `Eq` implementations only on the engine thread.

## Public resource API

The public constructor is:

```rust
pub struct Resource<K, T, E = Infallible> { /* private */ }

impl<K, T> Resource<K, T, Infallible> {
    pub fn new<L, F>(loader: L) -> Self
    where L: Fn(K) -> F + Send + Sync + 'static,
          F: Future<Output = T> + Send + 'static;
}

impl<K, T, E> Resource<K, T, E> {
    pub fn try_new<L, F>(loader: L) -> Self
    where L: Fn(K) -> F + Send + Sync + 'static,
          F: Future<Output = Result<T, E>> + Send + 'static;
}
```

Keys identify cache entries and cross the executor boundary.

```rust
K: Eq + Hash + Clone + Send + 'static
T: Send + Sync + 'static
E: Error + Send + Sync + 'static
```

Completed values are stored as `Arc<T>`, so `T` need not implement `Clone`.
Render callbacks receive that shared value.

```rust
let cards = use_resource(&self.cards, self.player_id);
cards.then(|cards: Arc<CardSet>| CardGrid::new(cards))
```

Cache administration belongs to the runtime whose cache is affected.

```rust
reactant.preload(&cards, player_id);
reactant.invalidate(&cards, &player_id);
reactant.clear(&cards);
```

Their exact signatures repeat the resource bounds:

```rust
impl<G: 'static> Reactant<G> {
    pub fn preload<K, T, E>(&mut self, resource: &Resource<K, T, E>, key: K)
    where K: Eq + Hash + Clone + Send + 'static,
          T: Send + Sync + 'static,
          E: Error + Send + Sync + 'static;
    pub fn invalidate<K, T, E>(
        &mut self, resource: &Resource<K, T, E>, key: &K,
    )
    where K: Eq + Hash + Clone + Send + 'static,
          T: Send + Sync + 'static,
          E: Error + Send + Sync + 'static;
}
```

```rust
impl<G: 'static> Reactant<G> {
    pub fn clear<K, T, E>(&mut self, resource: &Resource<K, T, E>)
    where K: Eq + Hash + Clone + Send + 'static,
          T: Send + Sync + 'static,
          E: Error + Send + Sync + 'static;
}
```

These methods repeat the resource bounds. `preload` clones the key for the
loader. `invalidate` borrows one key. `clear` selects entries by resource
identity.

`preload` starts missing work without mounting a consumer. It is idempotent for
a pending, ready, or failed entry. `invalidate` removes one terminal value or
replaces one pending generation. `clear` invalidates every entry belonging to
that resource in the runtime.

`Resource::new` is the concise infallible form. `Resource::try_new` caches an
`Err(E)` as a failed entry. Rendering that read through `.then` automatically
offers the error to the nearest `ErrorBoundary`; application code does not
unwrap or clone the error. Resource-task panics remain developer failures and
bypass error boundaries.

## Injected executor

Reactant depends on a small executor interface rather than Tokio or another
runtime package. `Reactant::new` takes ownership of one spawner and erases it
behind a private trait object, so `Reactant<G>` needs no executor type
parameter.

```rust
pub type BoxFuture<'a, T> =
    Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Spawner: 'static {
    fn spawn(&self, task: BoxFuture<'static, ()>) -> SpawnedTask;
}
```

`SpawnedTask` owns one best-effort cancellation request.

```rust
impl SpawnedTask {
    pub fn new(cancel: impl FnOnce() + Send + 'static) -> Self;
    pub fn detached() -> Self;
    pub fn cancel(self);
    pub fn disarm(self);
}
```

`cancel` invokes the request at most once. Dropping a live handle has the same
effect and never blocks for task termination. `detached` supplies a no-op handle
for executors that cannot cancel accepted work. Panicking or refusing to accept
a task is a developer or runtime configuration failure; `spawn` does not return
`Result`. `disarm` consumes the handle without requesting cancellation;
Reactant uses it after accepting a normal completion.

A cancellation closure is required not to panic. Reactant nevertheless invokes
it through `catch_unwind`. Invalidation and clear first remove the pending task
generation and mark consumers dirty, then request cancellation. A panic
therefore cannot restore or half-invalidate the slot: Reactant poisons the
runtime and resumes that panic on the engine thread. Runtime destruction
attempts every remaining cancellation, retains the first panic, and resumes it
after all handles are consumed. During an existing unwind, cancellation panics
are caught and suppressed to avoid a double panic; the runtime is already being
discarded.

The runtime wraps every loader future. Completion sends the resource identity,
key, generation, and either `Arc<T>` or `Arc<E>` to a synchronized queue. The
task never renders, touches hooks, or mutates the cache directly.

```rust
Completion {
    resource_id,
    key,
    generation,
    outcome,
}
```

Reactant drains that queue only on the engine thread. `poll`,
`observe_geometry`, and `begin_session` may freeze completions at entry. A
current completion changes the cache to ready and schedules every committed
Suspense boundary waiting for that generation.

Reactant invokes the loader and obtains its future before publishing a pending
slot, then stores the returned `SpawnedTask`. A loader-construction or `spawn`
panic poisons the runtime and abandons the render. The same rule applies when
loading starts through `preload`. A future that completes synchronously may
enqueue before `spawn` returns, but its completion cannot be frozen until a
later runtime entry, after the task handle is installed.

## Task panics

The wrapper catches a loader task panic and sends its panic payload through the
same completion queue.

```rust
TaskCompletion::Panicked {
    resource_id,
    key,
    generation,
    payload: Box<dyn Any + Send>,
}
```

A lifecycle entry point that freezes the completion, including `poll`,
`observe_geometry`, or `begin_session`, rethrows a current panic on the engine
thread. Battlement's normal engine panic boundary then reports it consistently.
A panic from a stale generation is ignored because invalidation declared that
work irrelevant. A **current** generation is the exact generation still stored
as pending for that resource identity and key. Delivering its panic poisons the
Reactant runtime under the common callback-panic rule.

Executor failure to accept a task is a developer/runtime configuration error and
panics synchronously while starting the resource.

## Cache entries

Each task attempt receives a nonzero generation from one monotonically
increasing runtime-wide counter. A generation is never reused, even after
`clear`; exhaustion panics rather than wrapping.

```rust
struct CacheSlot<T, E> {
    state: CacheState<T, E>,
    pending_boundaries: WeakSet<SuspenseId>,
    status_consumers: WeakSet<HookId>,
    ready_consumers: WeakSet<HookId>,
}
```

`CacheState` is `Vacant`, `Pending(TaskGeneration, SpawnedTask)`,
`Ready(Arc<T>)`, or `Failed(Arc<E>)`. Weak identities prevent cache retention
from retaining unmounted components or boundaries. A completion matches only
the exact resource identity, owned key, and current task generation.

Entries remain cached until explicit invalidation, resource clearing, or runtime
destruction. V1 has no time-to-live, capacity, eviction, or background refresh
policy.

The cache owns each key clone. Equality and hashing never run on executor
threads after the loader starts; completions carry the owned key back to the
engine thread.

## Invalidating work

Invalidation requests cancellation and makes the slot vacant. A later load gets
a fresh runtime-wide generation.

```rust
let task = slot.state.take_pending();
slot.state = CacheState::Vacant;
mark_consumers_dirty(slot);
task.cancel_if_present();
```

Reactant takes and clears every weak registration set during that operation.
Committed boundaries waiting on the invalidated pending generation, hooks that
committed output from any `ResourceStatus`, and hooks consuming the invalidated
ready value are all marked dirty once.

```rust
invalidate(Generation(4));
render(Generation(5));
ignore_completion(Generation(4));
wake_completion(Generation(5));
```

Cancellation is an optimization. If the future completes anyway, its task
generation does not match and Reactant ignores it. A later read starts a new
generation and cannot be overwritten by the stale result. `clear` cancels and
deletes every slot for the resource; global generation allocation proves that a
late completion cannot collide with a recreated `(resource, key)` entry.

Invalidating a ready or failed entry schedules mounted consumers for rerender.
The next read starts new work and suspends. Invalidating an entry with no
mounted consumer does not render roots.

`invalidate` and `clear` only queue dirty consumers. Their next `dispatch`,
`refresh`, `poll`, `observe_geometry`, or `begin_session` performs the render
and returns or serializes the resulting UI. They cannot return a commit because
they do not borrow `G`.

Clearing a resource applies the same rule to each entry. Runtime destruction
requests cancellation for every pending task and drops all terminal values.

## Reading a resource

`use_resource` is a Reactant-specific hook, not React's `use` API.

```rust
pub fn use_resource<K, T, E>(
    resource: &Resource<K, T, E>,
    key: K,
) -> ResourceRead<T, E>
where K: Eq + Hash + Clone + Send + 'static,
      T: Send + Sync + 'static,
      E: Error + Send + Sync + 'static;
```

It consumes one positional hook slot. Calls must follow normal hook-order rules
and therefore cannot be conditional. The hook records the current resource and
key. Consumer registration depends on how the read contributes to a successful
commit: `status` registers the hook for any observed state, while ready `.then`
content registers it for the ready value.

```rust
let avatar = use_resource(&self.avatars, self.player_id);
avatar.then(Avatar::new)
```

On a vacant slot, Reactant invokes the loader once, stores `Pending`, and
returns a pending read. On an existing pending slot, it returns another pending
read. A failed slot returns a failed read without restarting the loader.
If a pending read suspends through `.then`, the Suspense boundary becomes the
durable waiter when its fallback commits. If a component instead commits output
after inspecting `status`, its hook becomes a status consumer. The same
registration is replaced on every successful commit whether the observed state
is pending, ready, or failed. On a ready slot, a hook whose `.then` output
commits also becomes a ready consumer.

Changing the key releases the old waiter before observing the new entry. It does
not invalidate the old cached value.

## ResourceRead

`ResourceRead<T, E>` is an opaque render-aware snapshot of one cache
observation. Application code normally uses `.then`.

```rust
impl<T, E> ResourceRead<T, E>
where T: Send + Sync + 'static,
      E: Error + Send + Sync + 'static,
{
    pub fn then<R>(self, render: impl FnOnce(Arc<T>) -> R + 'static)
        -> impl Render
    where R: Render + 'static;
}
```

```rust
read.then(|profile| ProfilePanel::new(profile))
```

When ready, `.then` invokes its owned `'static` closure with `Arc<T>` and
renders the result. When pending, the closure is not invoked and the value
reports its pending token to the nearest Suspense boundary. When failed, it
reports an owned shared error to the nearest `ErrorBoundary`. An uncaught
failure reaches the root as the same `Err(RenderError)` runtime result as an
explicit `Err` render value.

The shared cache ownership is private. `RenderError::downcast_ref::<E>()`
returns the resource's concrete domain error even though the cache retains that
error in an `Arc<E>`; callers never downcast to `Arc<E>`.

The `.then` closure runs in a hook-forbidden render context. Stateful ready
content returns a component rather than calling hooks inside the closure.

```rust
struct PendingToken {
    cache_key: ErasedResourceKey,
    generation: TaskGeneration,
}
```

The token identifies one exact cache generation. It owns no component or hook
state and remains meaningful after tentative component state is discarded.

The handle also provides non-suspending inspection for labels or sibling
output:

```rust
match read.status() {
    ResourceStatus::Pending => "Loading",
    ResourceStatus::Ready => "Loaded",
    ResourceStatus::Failed => "Unavailable",
}
```

`status(&self) -> ResourceStatus` exposes neither `T` nor `E`; reading a value
outside `.then` would require an `Option` branch that could accidentally render
no fallback or bypass error propagation. A committed status consumer is
registered in its cache slot for every status. Pending completion and explicit
invalidation of ready or failed state schedule it exactly once, so status UI
cannot become stale. `ResourceRead` does not implement `Deref`.

## Suspense boundaries

`Suspense` renders a fallback whenever any pending read escapes its primary
child subtree.

```rust
pub struct Suspense<F, C = Missing> { /* private */ }

impl<F> Suspense<F, Missing> {
    pub fn new(fallback: F) -> Self;
    pub fn child<R: Render>(self, child: R) -> Suspense<F, R>;
}
```

Only `Suspense<F, Missing>` has `child`, and only a complete specialization
implements `Render`. `Missing` is the required-prop marker from the component
appendix. Supplying a child twice is therefore a compile error rather than a
last-write builder rule.

```rust
Suspense::new(Spinner::new())
    .child(cards.then(CardGrid::new))
```

Pending is a private render outcome, not a panic. Render traversal collects
pending tokens while continuing through the rest of the primary subtree. This
allows independent sibling resources to start in one attempt instead of forming
a request waterfall.

```rust
Suspense::new(Spinner::new()).child((
    profile.then(Profile::new),
    inventory.then(Inventory::new),
))
```

After attempting the complete primary subtree, the nearest boundary abandons
its tentative primary result and renders the fallback. Committing that fallback
registers the boundary's stable `SuspenseId` in every collected token's cache
slot. Completion wakes those boundary IDs.

An inner boundary consumes tokens from its own primary when its fallback
renders; those tokens do not reach an outer boundary. Tokens from a suspending
inner fallback do escape to the next ancestor. A component or fallback panic
always wins over collected pending tokens: Reactant commits no fallback from
that attempt, while resource tasks already started remain cached.

Every fallback commit atomically replaces the boundary's previous token set.
Reactant removes registrations for tokens no longer returned and adds the
current complete set. A primary commit or boundary unmount clears the set, so a
completion from an older key cannot retry an unrelated fallback.

```rust
let tokens = render_primary().pending_tokens();
commit_fallback_and_register(tokens);
complete_token_and_retry_primary();
```

If the fallback also suspends, its pending token escapes to the next ancestor
boundary. A pending token that reaches a root without a Suspense boundary is a
developer invariant violation. It panics before commit and poisons the runtime;
V1 does not install an implicit empty root fallback.

## Initial suspension

When a component suspends before it has ever committed, Reactant discards its
tentative component instance and hook state. The fallback becomes the committed
output of the boundary.

```rust
let value = use_resource(&self.data, self.id);
use_state(0); // Tentative until the primary tree commits.
```

The resource entry is not discarded. When it completes, Reactant retries the
boundary from a fresh primary component instance. This matches React's rule that
state from an initial suspended render is not preserved.

Render-phase state updates belonging only to that tentative instance are also
discarded. No committed setter or reducer queue can target an instance that has
never mounted.

Fallback components have their own ordinary identity and hooks. They remain
mounted while the same boundary stays pending and unmount when primary content
commits.

## Re-suspending committed content

When an already visible boundary suspends again, V1 shows its fallback
immediately because it has no transition API.

Reactant retains the committed primary logical tree and native identities. The
boundary records its primary's top-level host roots separately for every
physical parent, including portal targets. It applies an internal
`Display::None` override to each recorded root.

The fallback occupies the boundary's ordinary physical position and retains
normal component identity across retries while the boundary remains pending.
It may rerender; it is not remounted merely because the primary retries. A
primary consisting only of portals hides hosts at those portal targets while
its fallback appears where the boundary was declared. Components with no hosts
retain logical state and need no hide command.

Within each physical parent, a re-suspended boundary owns one contiguous range:
its retained hidden primary roots first, followed by its current fallback roots.
Unrelated siblings follow that complete range. Portal roots use the same rule
in each portal target's source-ordered range; a fallback portaled elsewhere
occupies its ordinary range in that target. On recovery, removing the fallback
and clearing the primary override exposes the retained roots without an
intermediate visible reorder.

```rust
primary display: internal none
fallback display: desired value
```

The override is not written into desired primitive props. When the resource
becomes ready and the retry succeeds, Reactant removes the override and restores
the exact desired `display` value, including an application's own `Reset` or
`Unset` setting.

Retained primary effects remain active while hidden, matching Suspense's logical
preservation. Refs remain attached, but geometry may change to an unavailable or
zero-layout state and must be treated as a new geometry observation.

Reactant ignores a delayed native event whose committed target is currently
beneath an internally hidden primary range. Ordinary setters may still update
retained components; a retry reconciles those updates transactionally while the
same fallback remains visible if the primary is still pending.

If the retry changes keys or types inside the primary tree, ordinary
reconciliation still unmounts those changed instances.

State and reducer queues on retained primary components are transactional. A
suspended attempt may evaluate them, but it does not acknowledge them. The next
retry applies the same pure state updaters and cloned reducer actions to the
committed hook values again. Only a successful primary commit removes those
queue entries.

```rust
let attempted = apply_queues(&committed, &queue);
suspend(attempted); // Queue remains against committed state.
let retried = apply_queues(&committed, &queue);
commit(retried);    // Queue is now acknowledged.
```

## Retry scheduling

One completion may wake several components and boundaries. Reactant deduplicates
them into one frame-call render.

```rust
complete(resource, key);
wake(boundaries);
render_roots_once();
```

A retry that still observes another pending resource keeps the fallback. Ready
values used successfully remain cached even if another sibling suspends.

Completions arriving during a render remain queued until the current render and
commit finish. Reactant never changes cache readiness beneath one render
attempt.

Ready primary content commits on the first successful retry entry. Reactant
does not reproduce React's timed reveal batching and has no transition or
deferred-value API that delays a fallback or reveal.

## Preloading

Preloading starts work before a component needs it.

```rust
reactant.preload(&card_details, hovered_card);
```

It uses the same cache entry and loader as `use_resource`, so a later read is
ready or joins the existing task. Preloading does not create a hook waiter,
schedule a root render on completion, or require a Suspense boundary.

If the key is already ready or failed, preload is a no-op. If it is pending,
preload does not start a second future. A failed preload does not schedule a
render unless a committed consumer later observes that entry.

## Why this is not React use

React's `use(promise)` may be conditional and depends on a promise cached by a
framework. Rust does not have a native promise value with React's
throw-and-catch render protocol, and Reactant does not use unwinding as a
pending signal.

Naming this API `use` would imply conditional-call and arbitrary-promise
semantics that Reactant cannot provide. `use_resource` states that it is a
positional Reactant hook tied to a `Resource` cache.

The cache and structural pending result preserve the behavior React users need:
deduplicated work, fallback boundaries, abandoned initial state, retries, and
retained committed content.

## Reconnect behavior

A reconnect keeps pending and ready resource entries. Pending tasks continue
and complete into the same runtime queue. Ready values remain available to the
new session render.

```rust
let session = reactant.begin_session(&mut game)?;
// Existing resource cache is reused.
```

`begin_session` freezes completions present at entry, installs them in its
tentative transaction, and freshly renders every registered root before
serializing the session documents. A ready completion in that frozen set can
therefore replace a committed fallback in the new snapshot. Successful
`SessionUi` conversion commits the cache and tree together. A completion
arriving after the freeze waits for the next active entry. This boundary
prevents one session snapshot from mixing cache generations.

## Manual QA

1. Mount two components that read the same resource key. Have each loader task
   resolve to its invocation number. Confirm one fallback appears and both
   consumers display invocation `1` after one completion.
2. Start two sibling resources under one boundary. Confirm both tasks start in
   the first attempt and the fallback remains until both are ready.
3. Suspend before initial commit, queue tentative hook state, then complete the
   resource. Confirm the primary component starts with fresh state.
4. Re-suspend committed content. Confirm primary host IDs and hook-driven values
   are retained, its hosts are hidden, and the fallback is visible immediately.
5. Invalidate a pending entry, complete its stale task, then complete the new
   generation. Confirm only the new value reaches fake Unity.
6. Preload a key and reconnect while it is pending. Resolve it before one
   session-entry freeze and another equivalent key after a later freeze.
   Confirm the first new snapshot contains ready content, the second contains
   its fallback until poll, and neither starts a second loader.
7. Invalidate a pending entry and destroy the runtime. Confirm each
   `SpawnedTask` cancellation callback runs at most once and a late completion
   cannot enter a dropped runtime.
8. Clear a pending resource, recreate the same key, and deliver the old and new
   completions out of order. Confirm runtime-wide generations accept only the
   new value.
9. Render only a loading label through `status`. Confirm completion rerenders
   it to the ready label without a Suspense boundary.
10. Complete a fallible resource with an error beneath nested error boundaries.
    Confirm `.then` skips its closure, the nearest boundary latches, and the
    resource does not restart until invalidation and boundary reset.
11. Fail a preload with no consumer. Confirm no root render occurs, then mount a
    consumer and confirm the cached failure reaches its nearest error boundary.
12. Downcast a failed resource through `RenderError::downcast_ref::<E>()` and
    confirm it exposes the concrete domain error rather than `Arc<E>`.
13. Render pending, ready, and failed labels through `status`, including below
    an equal-props memo boundary. Complete and invalidate each entry. Confirm
    every label updates and each dirty consumer crosses the memo boundary.
