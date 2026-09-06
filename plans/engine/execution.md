# Run synchronous rules without blocking the display

A game action runs through `Game::apply_action` on a Rust worker that owns a
private copy of game state. The method can publish intermediate views or wait
for a player choice. Unity's main thread presents those views and keeps menus
and animation responsive. Independent AI simulations may run on additional
workers.

Read this when implementing execution, choices, cancellation, or saving. Related
pages: [API examples](interfaces.md), [presentation](presentation.md), [Hearts
rules and AI](hearts.md), and [validation](validation.md).

## Three state values serve different purposes

The engine must distinguish computation from what the player has seen and what
can safely be saved.

- The **accepted state** is the last completed action whose final presentation
  has finished. The application retains it for saving and recovery.
- The worker mutates its own copy while computing the next action.
- The **presented view** is the immutable `Game::View` currently exposed to
  components and gameplay input. It can lag behind the worker.

For example, the worker may already have increased energy while the display is
still animating the preceding card draw:

```text
accepted state: before the action
worker state:  card drawn, energy increased
visible state: card drawn, energy unchanged
next view:     card drawn, energy increased
```

State and views must not share mutable interior data. `Game::logical_clone` and
`Game::view` must enforce that requirement; the engine cannot deep-copy an
arbitrary Rust type. Shared immutable data can use `Arc`. Published values must
be owned and `Send + 'static`; require `Sync` only when their actual sharing
needs it. The main thread can wrap received views in `Rc` for components.

A worker owns ordinary Rust memory and immutable game data. Persistence,
networking, native handles, and display services belong to the application.
Rules receive publication, choice, and cancellation operations, not Unity APIs.

## From dispatch to accepted state

Normal interactive action execution follows one defined sequence. Registering a
game does not call `apply_action` and does not start a worker by itself.

1. A display component dispatches a `Game::Action`.
2. The main thread returns `Busy` if another action, prompt, or required final
   presentation is unfinished.
3. The main thread calls `Game::validate_action` against the accepted state.
   Rejection returns `Invalid(reason)` without starting a worker.
4. The engine calls `Game::logical_clone` and schedules `Game::apply_action`
   with that private state on its Rust worker pool. Dispatch returns
   `Started(run_id)`.
5. `apply_action` runs synchronously on that worker. It mutates only the private
   state and may call `present`, `choose`, or `check_cancelled`.
6. Each `present` asks `Game::view` for an immutable view and publishes it with
   ordered `Game::StateAnimation` values. Unity prepares and commits that view
   on its main thread.
7. A `choose` call publishes a view and prompt, then blocks the worker until a
   valid answer or cancellation arrives. Unity's main thread remains responsive.
8. Returning from `apply_action` publishes the final state, final view, and
   `Game::final_state_animations`. A return does not immediately modify the
   accepted state.
9. After the final presentation gate completes and Unity reports a subsequent
   rendered frame, the engine installs the worker state as accepted. Saving and
   the next action may now proceed.

The engine chooses the interactive worker automatically. It may reuse a pool
thread rather than create one OS thread per action. A direct simulation call to
`Game::apply_action` runs on the caller's thread; the method itself does not
spawn anything.

Invalid and stale prompt answers follow [choice handling](#choice-waits).
Cancellation follows [worker cancellation](#worker-cancellation). Rules panics
and presentation failures follow [failure handling](#failure-handling). None of
those branches installs the worker's private state as accepted.

## Publish meaningful intermediate states

A **checkpoint** contains an immutable `Game::View` and an ordered collection of
game-defined `Game::StateAnimation` values. Assign its run-local checkpoint ID
and animation indices once, when publishing it. Retrying display preparation
does not change them.

`Game::apply_action` publishes after each coherent operation:

```rust
draw_card(state);
cx.present(state, || StateAnimation::CardDrawn(card));
gain_energy(state);
cx.present(state, || StateAnimation::EnergyGained(1));
```

The worker may compute ahead, but only one checkpoint may wait for display. This
limit prevents a fast worker from retaining a long history of views. It also
prevents building views that the display cannot yet accept.

For example:

```text
A is visible; B is being prepared for display.
The worker reaches present(C) and waits before calling C's builders.
B replaces A on screen; the worker may now build C.
C waits while B's required animation finishes.
```

Keep B counted as the one pending checkpoint while its assets and objects are
being prepared. Do not move B into another unbounded queue and free capacity
early. Release capacity when B commits as the displayed checkpoint.

`Game::view` runs to completion on the worker before rules mutate that state
again. The retained values are the current view, one pending view, accepted
state, and worker state. Inactive native resources being prepared have a
separate lifetime described in [presentation](presentation.md).

## Choice waits

`choose` returns a typed answer directly to `Game::apply_action`. Interactive
execution first creates an owned prompt and view, presents them
in checkpoint order, and waits. Simulation instead calls its policy inline. See
[choice APIs](interfaces.md#choices-remain-typed).

```rust
let card = cx.choose(state, PlayCard::new().observation(state.observe(player)));
state.play(card);
cx.present(state, || StateAnimation::CardPlayed(card));
```

During an interactive wait:

- Earlier checkpoints finish their required presentation first.
- The prompt becomes actionable only when its own view is presented.
- The worker's decision and legal-answer set remain unchanged.
- One prompt is outstanding within that action.
- Menus, settings, inspection, and local selection may continue changing.

Each request gets an ID within its action run. A new legal-answer set requires a
new request. Invalid answers produce public feedback and leave the same request
unanswered. A valid answer resumes the function once. The main thread rejects
stale IDs before delivery, and the worker checks them again after it wakes.
Type-erased transport must also validate the answer's Rust type.

For example, a menu can be opened while selecting three cards to pass. Toggling
selection is display state. Pressing Pass submits one typed answer; selecting
two cards is invalid and does not restart the action. Rules-defined cycles of
selection and deselection may issue successive requests and remain cancellable.

## Worker communication is private engine code

Game authors call `present` and `choose`; they do not assemble channel messages.
Implementors need a small synchronized connection between the worker and main
thread. Use a mutex and condition variable with predicates checked in a loop.
Update a predicate before notifying so cancellation cannot lose a wakeup.

The connection carries these values. The names illustrate internal records, not
an additional public API:

```text
worker to display:
  view + state animations
  view + public prompt + typed answer handle
  final state + final view + final state animations
  execution failed / worker stopped

worker to application controller:
  owned private controller prompt + the same typed answer handle

display to worker:
  answer to the current request
  cancellation requested
```

The first three records share the one-pending-checkpoint limit. Failure and
stopped status are stored separately so a full checkpoint queue cannot hide
shutdown. Every output identifies its run. The main thread handles host events,
action dispatch, presentation commits, and accepted-state updates serially in
normal Unity engine callbacks. Workers touch only their state and synchronized
connection; they never reenter the C ABI or mutate the component tree.

The public prompt and private controller prompt belong to one request. The main
thread exposes only the public value to components and routes the private value
only to the application controller. It starts an AI job only after the public
view is presented. Both human and AI answers return through the same run and
request validation path. The private controller value is part of the prompt
publication; it does not create another checkpoint or unbounded queue.

On normal return, reserve checkpoint capacity before building the final view and
state animations. Publish them together with final state. Normal completion
must not bypass lazy construction or create an extra view queue.

## Worker cancellation

Exit, restart, or replacement immediately makes a run inactive. The main thread
discards its queued output, cancels its prepared display work, and wakes its
worker. Old views, answers, animation callbacks, and final results cannot
change the replacement game, even if the old worker has not stopped yet.

The worker stops cooperatively at defined checks. Closing its communication
connection also requests cancellation.

| Worker position | What cancellation does |
| --- | --- |
| Waiting to publish | Wake, release locks, and unwind |
| Building a view or prompt | Finish the builder, discard its result, and unwind |
| Waiting for an answer | Wake, release locks, and unwind |
| Entering `present` or `choose` | Unwind before calling any builder or policy |
| Computing ordinary rules | Stop at the next explicit or built-in cancellation check |
| Returning final state | Check again and discard the result if cancelled |

Check on entry to each primitive, after acquiring capacity, after constructing
data, after every wait, before returning a valid answer, and before publishing
normal completion. If cancellation has already been observed before returning an
answer, cancellation wins over that queued answer. A cancellation arriving just
after a check is handled at the next one; run validation still rejects abandoned
output in the meantime.

Long computations add explicit checks:

```rust
for candidate in candidates {
    cx.check_cancelled();
    evaluate_candidate(state, candidate);
}
```

Builders, external waits, and destructors must terminate. Detaching a worker
keeps Unity from blocking on a join; it does not forcibly stop arbitrary code.
Publish worker-stopped only after worker-owned state and destructors finish.
Process shutdown may proceed while detached workers are still stopping.

## Failure handling

Use a private cancellation payload with `resume_unwind`. It bypasses the panic
hook, so expected cancellation produces no panic report. Catch it once around
the complete worker action, before returning through any C ABI boundary.

```rust
struct Cancelled;
fn unwind_cancelled() -> ! {
    std::panic::resume_unwind(Box::new(Cancelled))
}
```

The boundary distinguishes three outcomes:

- Normal return: publish final state, subject to cancellation and run checks.
- Private `Cancelled` payload: discard the interrupted computation silently.
- Any other panic: discard the private state and report execution failure.

Release communication locks before unwinding. Inner `catch_unwind` handlers must
propagate payloads they do not recognize. Every `AssertUnwindSafe` use needs a
local ownership explanation: interrupted state is discarded, and surviving
shared synchronization remains valid. Destructors must not let a second panic
escape while unwinding.

All interactive release builds require compatible `panic = "unwind"` and
linker/runtime settings. Cancellation must remain inside Rust frames. Validate
nested calls and destructor cleanup through the real Unity integration on native
and threaded WebGL paths; flags alone do not prove support. Mobile build and
physical-device requirements are in [validation](validation.md).

An active execution failure shows exit/restart controls and retains accepted
state. A late failure from an abandoned run is ignored. A required animation
failure follows the same recovery behavior. Unexpected failure during a native
visible update stops the session, as described in
[presentation](presentation.md). Process-aborting failures cannot be caught by
this unwind boundary.

## Simulate with the same rules

Simulation calls the same `Game::apply_action` method on independent mutable
state, using a concrete policy for each choice. It runs in the caller's thread
and may share immutable data with other simulations.

The executor specialization must make these operations cheap:

```text
present(state, animation_builder): call neither view nor animation builder
choose(specification):             call the policy inline
check_cancelled():                 inline no-op
```

No primitive requires heap allocation, a vtable, blocking, interactive
cancellation checks, or an owned UI prompt. Candidate construction and policy
search have their own costs; measure those separately. Verify the primitives
with public-entry benchmarks, allocation traces, and optimized-code inspection.

The application controls scheduling and cancellation between bounded simulation
batches. Display previews run the same rules with a suitable policy and render
the result through the normal display API. Do not implement a second rules
engine for previews or AI.

## Accept final state only after it has been presented

Computing an action is not the same as completing it for the player. Retain the
worker's final state privately until all of these are true:

1. Its final checkpoint is displayed.
2. Required animation completion or the chosen sequence label has occurred.
3. Unity reports a rendering opportunity at or after that event for the current
   checkpoint and visible update.

Only then replace accepted state and allow the next action or save. Even a
checkpoint with no animation needs a rendered frame.

Action dispatch first returns `Busy` if an action or required final presentation
is unfinished. Otherwise it runs the game's bounded, pure validator against
accepted state. Invalid input returns `Invalid(reason)` without cloning state or
starting a worker. A legal action returns `Started(run_id)`; this means work
began, not that its final state is accepted.

Persistence is game-owned and runs outside rules. Save an immutable copy of
accepted state. A write failure leaves the in-memory action valid and reports
nonfatal feedback. [Hearts](hearts.md#autosave-and-resume) specifies ordered
writes, durable browser storage, and restart behavior for that sample.

## Manual QA

Pause a worker while waiting to publish, inside a controlled builder, and at a
prompt. Replace the game each time and verify responsive menus, rejected old
answers, silent cancellation, and stopped status after cleanup. Separately
trigger a genuine panic. Hold final animation completion and confirm that the
next action and saving wait for both completion and a subsequent rendered frame.
