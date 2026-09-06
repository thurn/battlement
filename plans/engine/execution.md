# Run synchronous rules without blocking the display

`Game::execute` runs on a Rust worker with private state and the game's context.
It may publish snapshots or ask for typed choices. The display owns the queue
and animation sequencing; Unity's main thread remains responsive.

Read the [complete API](interfaces.md) and [contract
sketch](../../crates/battlement-reactant/src/proposal.rs) first. Related pages:
[presentation](presentation.md), [Hearts](hearts.md), and
[validation](validation.md).

## Three state values serve different purposes

- The **accepted state** is the initial state or the last action whose final
  presentation has completed. It remains available for explicit saves/recovery.
- The worker mutates a private logical clone while computing the next action.
- The **displayed snapshot** is an immutable logical clone currently read by
  components. It may lag behind the worker.

For example, rules can increase energy while the display animates a card draw.
The current snapshot contains the drawn card and old energy; the next queued
snapshot contains the increased energy. Neither changes when the worker mutates.

`Game::logical_clone` creates both worker copies and display snapshots. These
copies must not share mutable interior data. Immutable sharing is allowed.
`Game::State` is owned and `Send + 'static`; the main thread wraps received
snapshots in `Rc`. Do not add `Sync` unless actual sharing requires it.

Snapshots contain the whole state. Display components are responsible for
concealing hidden information. Only resulting visual declarations reach Unity;
state and game-specific decisions stay in Rust.

## From dispatch to accepted state

`App::start_game(initial_state, make_context)` stops/replaces the old session,
constructs the context using its connection, and attaches the session
internally. It accepts the initial state, queues entry presentation, and returns
a cloneable `GameHandle`. Status is `Busy` until entry presentation completes,
then `Ready`. Starting does not execute an action.

An interactive action follows this sequence:

1. `GameHandle::dispatch(action)` returns `Busy` before validating if entry,
   another action, a prompt, or final presentation is unfinished. It queues no
   extra action. Dispatch to `Failed` or `Stopped` is a programming error.
2. `Game::is_legal_action` checks accepted state. False panics without cloning
   or starting a worker. The UI prevents ordinary illegal inputs from
   dispatching.
3. The engine logically clones accepted state, schedules `Game::execute` with
   the session's context on its Rust worker pool, and returns `Started`.
4. Rules mutate private state and call `present` or `choose` as needed.
5. Interactive `present` reserves queue capacity, logically clones current
   state, invokes its lazy animation builder, and publishes the pair.
6. Interactive `choose` publishes a snapshot and owned prompt in the same queue.
   The context chooses human input or an AI policy. Human input waits on the
   worker; a live AI policy runs there after its prompt is displayed.
7. Normal return reserves capacity and publishes final state with a final
   snapshot and no semantic animation event. It does not accept the result yet.
8. The display finishes required final animation and a subsequent rendered
   frame. Only then does the session install the new accepted state and become
   `Ready`.

The worker pool may reuse threads. The app processes dispatch, host events,
presentation commits, status, and accepted-state updates serially. Context is
available to only one action at a time and returns to the session after normal
completion. Failure/stop discards interrupted context; replacement constructs a
fresh one. The engine does not roll back game-owned external side effects.

`accepted_state()` always returns a clone of the last accepted value. During
steps 3-8, that is still the previous completed state. Saving that previous
state is allowed; saving the new action must wait for acceptance.

## Publish meaningful intermediate states

A **checkpoint** is a queued state snapshot with an optional semantic animation
and optional presented prompt. `present` contributes exactly one animation;
choice and automatic final checkpoints have none. The display can turn one event
into any number of related movements, sounds, and particles.

```rust
draw_card(state);
context.present(state, || StateAnimation::CardDrawn(card));
gain_energy(state);
context.present(state, || StateAnimation::EnergyGained(1));
```

The display controls when it advances; rules may compute ahead. Only one
checkpoint may wait for display, including native preparation. Reserve capacity
before snapshot cloning and animation construction, and release it only when
that checkpoint commits as the displayed checkpoint.

If A is displayed and B is preparing, `present(C)` waits before cloning C. Once
B commits, C may be built and wait while B's required animation finishes. Never
move B into another unbounded queue to release capacity early.

Cloning runs to completion before rules mutate that state again. Retained data
includes accepted and working states, the current snapshot, and one pending
snapshot. Final publication transfers the working state rather than retaining an
unbounded history. Native preparation has its own resource lifetimes.

Assign session/run/checkpoint identities once at publication. The sole semantic
event uses animation index zero where occurrence identity needs an index.
Registration names distinguish its multiple effects. Retries do not mint new
identities. Default layout movement and frame requirements still apply when a
checkpoint has no semantic event.

## Choice waits

`PromptData<T>` owns its choice data and provides a stable iterator, validation,
and conversion to/from the game's shared prompt enum. Simulation and display
read that same enum. Rules get a concrete `ResponseType` back:

```rust
let card: CardId = context.choose(state, PlayCardPrompt { choices });
state.play(card);
```

Interactive choice publication clones current state even without a preceding
`present`. Earlier checkpoints finish first. The prompt becomes active only with
its own snapshot. The worker cannot mutate state during the wait.

Human requests expose `PresentedPrompt { prompt, handle }`. The display matches
the enum and calls `handle.submit(prompt_data, response)`. The concrete prompt
argument determines the Rust response type. The handle supplies request identity;
runtime checks verify the expected concrete prompt and response types.

- Use the actual published prompt as validation authority, never a caller's
  lookalike value. Retain it through request resolution. Do not use prompt
  addresses as identity: zero-sized prompt data is allowed.
- An active invalid response, mismatched prompt/response type, or human response to an
  AI-owned request is a programming error. Panic inside Rust; never unwind
  through the C ABI. An app-callback boundary turns active callback panics into
  session failure; a direct Rust call still exposes the programming-error panic.
- Ended-request replies are ignored before inspecting their payload. This
  includes duplicate replies, replacement, and late callbacks from an old run.
- A valid answer resumes exactly once. Worker wakeup checks the request again
  and checks cancellation before returning the response.
- Local selection, menus, inspection, and settings can change while waiting.
  Illegal UI selections stay local and disabled; they are not submitted as
  intentionally recoverable invalid responses.

A new legal-choice set requires a new request. Run/request identities remain
internal; test-driver observations can expose them for stale-delivery scenarios.

## Live AI and simulation

`HeartsContext` routes using state and prompt. It calls the interactive
connection's `choose` for a human or `choose_with_policy` for an AI. Reactant
owns publication/wait mechanics, not player identification.

The live helper clones the enum for display and keeps the original prompt on the
worker. `Game::Prompt: Clone` permits this without requiring `Sync`. Human
publication moves its prompt; simulation makes no display copy.

The live policy receives `&Game::State` and `&Game::Prompt` on the rules worker
after the prompt's snapshot is displayed. It returns a stable option index. The
helper recovers the concrete prompt, validates the selected answer, and checks
abandonment before returning. Human input cannot win an AI request.

Simulation constructs the same domain context in simulation mode and calls
`Game::execute` directly. It runs synchronously on its caller's thread:

- `present` invokes neither `logical_clone` nor the animation closure.
- `choose` calls the policy inline, maps its index, validates, and returns.
- No primitive needs a display connection, blocking wait, allocation, or vtable.
- Mode branches are allowed. Prompt vectors and policy search may allocate;
  measure those costs separately from primitive overhead.

The game's MCTS code owns hidden-state sampling and rollout heuristics. Reactant
does not construct observations or private controller messages. A bounded live
policy may finish computing after stop, but its result cannot reach a
replacement. Game-owned search can implement its own cancellation; there is no
additional engine cancellation API for arbitrary computation.

## Worker communication is private engine code

Implement the connection with synchronized predicates and bounded payload
storage. A mutex/condition-variable implementation must update predicates before
notification and recheck them in loops so cancellation cannot lose a wakeup.

The connection carries:

- Snapshot and semantic event for `present`.
- Snapshot, prompt, response connection, and human/AI ownership for `choose`.
- Final state and snapshot on normal return.
- Response, presentation readiness, stop, and failure/stopped observations.

The first three publications share one pending slot. Keep failure and cleanup
status separately so a full slot cannot hide shutdown. All output identifies its
session and run. Workers never mutate the tree, call Unity, or reenter the C
ABI.

For live AI, presentation readiness wakes the waiting helper to call its policy
on the rules worker. There is no engine-managed independent AI job or controller
message queue. Simulations may be scheduled elsewhere by game-owned code.

## Worker cancellation

`stop`, replacement, or app teardown immediately invalidates the run, discards
pending/prepared output, and wakes blocked helpers. Public status becomes
`Stopped` immediately. The separate worker-stopped observation occurs only after
worker-owned context/state and destructors have finished.

| Worker position | Cancellation behavior |
| --- | --- |
| Waiting for publication capacity | Wake, release locks, unwind |
| Cloning a snapshot or building an event | Finish, discard the value, unwind |
| Waiting for a human or presentation readiness | Wake, release locks, unwind |
| Entering interactive `present` or `choose` | Unwind before helper work |
| Computing ordinary rules or a policy | Continue until a helper/return boundary |
| Returning an answer or final state | Check and discard if abandoned |

Helpers check internally on entry, after capacity acquisition/construction,
after waits, and before returning or publishing completion. Observed
cancellation wins over a queued answer. A later cancellation is caught at the
next boundary; identity checks already prevent stale output from affecting the
display.

There is no public explicit cancellation check. Detachment keeps Unity from
blocking on a join; it does not forcibly interrupt Rust computation. Builders,
policies, and destructors must terminate. Platform proofs exercise actual helper
waits and controlled ordinary computation, not pretend preemptive cancellation.

## Failure handling

Use a private cancellation payload and `resume_unwind`, which bypasses the panic
hook for expected cancellation. Catch it once at the worker boundary, before
returning through any C ABI frame.

- Normal return publishes final state, subject to abandonment checks.
- Expected cancellation discards interrupted computation silently.
- Other panics discard private state and set the active session to `Failed`.

Release communication locks before unwinding. Inner catch handlers propagate
unknown payloads. Explain each `AssertUnwindSafe` in terms of discarded private
state and valid surviving synchronization. Destructors must not panic again
while unwinding. Process-aborting failures cannot be recovered by this boundary.

Interactive native and threaded WebGL release paths must support Rust unwinding
and demonstrate nested cleanup through Unity. Mobile build/evidence requirements
remain in [validation](validation.md). A late failure from an old session cannot
fail its replacement.

`GameHandle::status` and `use_game_status` expose `Ready`, `Busy`, `Failed`, and
`Stopped`. Detailed errors go to diagnostics. Failed sessions retain accepted
state and offer restart/exit. Required-animation failure follows this path;
unexpected native visible-update failure also stops further work and shows the
failure surface rather than claiming native rollback.

## Accept final state only after it has been presented

The display owns animation sequencing and the advancement gate. Acceptance
requires the final checkpoint to be committed, its required animation/label to
finish, and a subsequent rendering opportunity for that visible generation. Even
a checkpoint without a semantic event needs a rendered frame.

Until then, dispatch returns `Busy` and `accepted_state()` returns the prior
accepted state. When ready, installation and status change occur together.
Explicit save/load is game-owned and uses a returned state copy. There is no v1
autosave, acceptance subscription, or implicit save-on-exit. A write failure
cannot invalidate an already accepted in-memory action. See [Hearts
save/load](hearts.md#explicit-save-and-resume).

## Manual QA

Hold publication capacity, snapshot cloning, human choice, and final animation
with deterministic fixture barriers. Stop/replace at each point, verify
immediate `Stopped`, then release barriers and verify later cleanup without
stale output. Stop during bounded ordinary computation and verify it is not
forcibly stopped. Inject rules, response, and required-animation failures;
verify `Failed`, stable accepted state, and restart. Save the previous accepted
state while busy, then save the new one after final animation and a real
rendered frame.
