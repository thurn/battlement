# Run synchronous rules without blocking the display

`Game::execute` runs on a Rust worker with private state and the game's context.
It may publish snapshots or ask for typed choices. Reactant renders queued
snapshots
into Reactant trees and ordinary Battlement batches. The existing command queue
sequences blocking animation; Unity's main thread remains responsive.

Read the [complete API](interfaces.md) and [contract
sketch](interfaces.md#complete-contract-sketch) first. Related pages:
[presentation](presentation.md), [Hearts](hearts.md), and
[validation](validation.md).

## Rules state and rendered state can be ahead of Unity

- The **accepted state** is the initial state or the last normally completed
  action whose final publication has been consumed in Rust. It is available for
  explicit saves/recovery, even while Unity still animates earlier commands.
- The worker mutates a private logical clone while computing an action.
- The **rendered snapshot** is the immutable clone most recently consumed by
  Reactant. It supplies the next tree diff; it need not match Unity's current pose.

For example, Reactant can render the drawn card and increased energy while Unity
is still animating the played card. Their commands wait in order on Unity.
There is no separate accepted-display state or host acknowledgement handshake.

`Game::logical_clone` creates worker copies and snapshots. These copies must not
share mutable interior data. Immutable sharing is allowed. State is owned and
`Send` and the app may wrap received snapshots in `Rc`; do not add `Sync` unless
actual sharing requires it. Only generated visual commands reach Unity.
Components must conceal hidden information in player output.

## From dispatch to accepted state

`App::start_game(initial_state, make_context)` replaces the old session,
accepts initial state, and queues its initial render. It constructs the context
and returns a handle. `Busy` lasts until that entry is consumed in Rust, not
until Unity finishes entry animation. Starting executes no rules action.

An interactive action follows this sequence:

1. `dispatch` returns `Busy` if initial publication or another rules action is
   pending, including a human choice. It queues no extra action in that case.
2. `is_legal_action` checks accepted state. False panics before cloning or
   starting a worker; normal UI offers legal actions.
3. Clone accepted state, move context to the worker, and return `Started`.
4. Rules mutate private state and call `present` or `choose` as needed.
5. `present` reserves publication capacity, clones state, builds the animation
   event, and publishes it. The Rust consumer renders and submits commands in
   order, without waiting for Unity.
6. `choose` publishes a snapshot and owned prompt. A human response resumes the
   worker. A live AI policy runs after publication without waiting for display.
7. Normal return publishes the final state with no semantic animation event.
8. After the final entry is consumed and its commands submitted, install accepted
   state, return context to the session, and set `Ready`. Empty output needs no
   host command. Unity may still be playing any earlier submitted animation.

AI policies receive state and a prompt, never a `GameHandle`. A UI-dispatched
action can perform a complete AI turn with successive choices inside `execute`
without returning to UI dispatch between cards. Live policies run immediately
after enqueueing; only a full Rust publication queue delays that enqueue.

The app processes publications, responses, stop/failure, and acceptance serially.
`Ready` means rules readiness, not visual completion. The application may compute
and append another action while animation runs. Native gameplay controls are
queued at the intended decision point; menus remain independently usable.
Interrupted context is discarded on failure/stop and replaced on restart.
The engine does not roll back game-owned external side effects.

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

The Rust consumer renders snapshots in order, submits their commands, and
continues immediately. One FIFO has exactly 32 pending slots shared by present,
human/AI prompts, and final publication. Reserve before cloning/building;
release a slot when the consumer takes its entry. The snapshot being rendered
is outside the pending queue. Never drop or coalesce entries.

Hold the Rust consumer on A: B1-B32 can enqueue, then B33 waits before its
builders run. Taking B1 opens one slot. Pausing Unity alone does not hold slots:
generated batches can accumulate under existing transport/queue limits. This
bounds snapshot count, not bytes or downstream commands; measure peak queue bytes.

Keep accepted state, working state, the snapshot being rendered, and up to 32
waiting snapshots as needed. Final publication transfers working state instead of keeping
an unbounded snapshot history. Internal session/run identity rejects abandoned
worker output; existing batch/command IDs govern native delivery. No host
checkpoint ID or animation index is needed. Consume each semantic event once;
ordinary rerenders do not emit it again.

## Choice waits

`PromptData<G>` owns its choice data and provides a stable iterator, validation,
and infallible wrapping into the game's lifetime-parameterized prompt enum.
`as_prompt` borrows the typed data; `into_prompt` owns it. There is no
conversion back out of the enum. Simulation and display inspect the same enum
definition. Rules get a concrete `ResponseType` back:

```rust
let card: CardId = context.choose(state, PlayCardPrompt { choices });
state.play(card);
```

Interactive choice publication clones current state even without a preceding
`present`. Its render queues prompt controls after preceding gameplay commands.
The request exists in Rust immediately; native controls become usable when Unity
reaches them. No visibility acknowledgement is needed. The worker cannot mutate
state while waiting for a human response.

Human requests expose `PresentedPrompt { prompt, handle }`. The display matches
the enum and calls `handle.submit(prompt_data, response)`. The concrete prompt
argument determines the Rust response type. The handle supplies request
identity; runtime checks verify the expected concrete prompt and response types.

- Retain the original typed prompt in the internal request as validation
  authority; publish its owned clone in the enum. Never validate against a
  caller's lookalike value. Keep both alive through request resolution. Do not
  use prompt addresses as identity: zero-sized prompt data is allowed.
- An active invalid response, mismatched prompt/response type, or human response
  to an AI-owned request is a programming error. Panic inside Rust; never unwind
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

Both live helpers retain the concrete prompt in an internal `Arc<P>` request and
publish `prompt.clone().into_prompt()` after reserving capacity. The display
owns the enum copy and validates replies through the original typed request;
there is no enum extraction. Live AI also borrows that original for its policy.
Concrete prompt data requires `Clone + Send + Sync` and remains immutable for
the request lifetime. Human requests make the same one display copy. Simulation
keeps `P` locally, allocates no request, and makes no clone.

The live policy receives `&Game::State` and a borrowed `Game::Prompt<'_>`
wrapper on the rules worker after publishing the snapshot, without waiting for
Unity. It returns a stable option index. The helper selects directly from
retained `P`, validates, and checks abandonment before returning. Human input
cannot win an AI request.

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
- Response, stop, and failure/stopped observations.

The first three publications share the 32-slot pending FIFO. Keep failure and cleanup
status separately so a full slot cannot hide shutdown. All output identifies its
session and run. Workers never mutate the tree, call Unity, or reenter the C
ABI.

Live AI calls its policy after publishing the snapshot. It needs no display
readiness signal, independent engine AI job, or controller-message queue.
Simulations may be scheduled elsewhere by game-owned code.

## Worker cancellation

`stop`, replacement, or app teardown immediately invalidates the run, discards
pending output, cancels outstanding host work through the existing queue, and
wakes blocked helpers. Public status becomes
`Stopped` immediately. The separate worker-stopped observation occurs only after
worker-owned context/state and destructors have finished.

| Worker position | Cancellation behavior |
| --- | --- |
| Waiting for publication capacity | Wake, release locks, unwind |
| Cloning a snapshot or building an event | Finish, discard the value, unwind |
| Waiting for a human response | Wake, release locks, unwind |
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
state and offer restart/exit. A host gameplay failure stops presentation and
reports recovery, but it cannot revert rules actions already accepted. Rebuild
from accepted state on restart. Late nonblocking cosmetic failures are diagnostic
and cannot fail a replacement session.

## Accept completed rules without waiting for animation

When normal return's final publication is consumed in Rust, install the final
state and set `Ready`. Submission to the existing command queue is sufficient;
Unity need not report completion. A worker failure before that point retains
the previous accepted state. Host failures never roll accepted state backward.

`accepted_state()` copies the most recently completed rules action. During rules
execution it returns the previous action; during playback it may return a state
Unity has not yet reached visually. Explicit saves capture that logical state.
There is no v1 autosave or implicit save-on-exit, and write failures do not undo
accepted gameplay. See [Hearts save/load](hearts.md#explicit-save-and-resume).

## Manual QA

Hold the Rust consumer on A, publish B1-B32, and verify B33 waits before building.
Taking B1 opens one slot. Run five consecutive live AI choices while Unity is
paused; simulation still runs without a queue even when the live FIFO is full.
Hold snapshot cloning and human choice with fixture barriers.
Stop/replace at each point and verify immediate `Stopped`, then cleanup without
stale output. Separately pause Unity while Rust consumes multiple snapshots and
accepts a completed action. Save that accepted state, resume playback, and verify
command order. Inject worker failure before acceptance and host failure after
acceptance; recovery retains the correct logical state in both cases.
