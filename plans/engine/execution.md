# Rules execution, prompts, and accepted state

Read this for tasks involving rules, simulation, checkpoints, action admission,
saving, cancellation, or run failures. See [architecture](architecture.md),
[presentation](presentation.md), and [validation](validation.md).

## State ownership and public types

A **checkpoint** is an immutable game snapshot plus ordered game-defined
semantic change records. Its run-local checkpoint ID and change indices are
allocated once at publication. A **presented snapshot** is the checkpoint
currently visible to components and gameplay input. An **accepted state** is a
completed action's mutable-state value retained after presentation acceptance.

Use a game trait with associated State, Snapshot, Action, Change, Prompt, and
Answer types. State is Send + 'static. Snapshot is owned, immutable, and Send +
'static; wrap received snapshots in Rc on the display thread. Change and
published prompt data are owned and Send + 'static. Game authors choose
collections and how immutable data is shared.

Require a game-owned fork_state(&State) -> State operation. The application
retains the last accepted state and forks a private state for each worker.
Ordinary Clone is a valid implementation; a persistent data structure is also
valid. Never move the only accepted state into a fallible worker. Snapshot and
fork implementations must not share mutable interior state with accepted or
presented values. A game may use Arc for actually shared immutable data; Sync is
required only where that sharing needs it, not on every snapshot type.

The rules function is ordinary synchronous Rust with an execution mode/policy
parameter. Type aliases may be generic:

~~~rust
type GameExecution<M> = Executor<HeartsGame, M>;

fn play<M: ExecutionMode<HeartsGame>>(
    state: &mut HeartsState, cx: &mut GameExecution<M>,
) {
    let card = cx.choose(state, PlayCard::new().player(state.turn));
    state.play(card);
    cx.present(|| state.snapshot(), || vec![Change::CardPlayed(card)]);
}
~~~

Names may improve during task 02. That task must compile a representative nested
rules function in both modes before other tasks depend on the API.

## Typed choices

A choice specification has a typed Answer, creates the game's owned Prompt
envelope, converts from the game's Answer envelope, and validates against the
unchanged rules state. Define one game enum for prompt variants and another for
answer variants. Wrong variants are invalid answers, not panics.

Interactive choose reserves capacity, builds a snapshot and owned prompt,
publishes it in order, and waits. Publish the prompt's run/request identity
through the public display. Only a presented prompt is actionable.

The worker retains the concrete choice specification and validates submissions
before returning a typed answer. A request can receive several invalid answers;
each produces public feedback while leaving the same request unanswered.
Selecting/deselecting within a prompt is display state. A rules-defined
select/deselect cycle can issue successive requests and must remain cancellable.

Prompt identity and checkpoint identity are separate. A materially changed legal
set gets a new request ID. Reject abandoned runs and stale requests. Menus,
inspection, and settings remain available during all waits.

## Publication and worker lifecycle

Use one mutex-protected endpoint state and condition-variable protocol for
capacity, prompt answers, and cancellation. Wait predicates are checked in a
loop under the mutex. Notify after changing that shared predicate; never rely on
a notification alone. Release communication guards before unwinding.

The publication slot has capacity one, including prompt and final records.
Reserve it before invoking snapshot/change builders. The application may retain
one admitted/preparing checkpoint, but must not drain output into another queue.
Release the slot when that checkpoint commits, not when preparation starts. This
leaves at most the presented snapshot, one pending snapshot, the accepted state,
and worker-private state; inactive host preparation is separately owned.

The worker may compute ahead only until its next publication reservation.
Builders finish before the worker mutates its private state again. Completion
uses the same slot and contains the final State plus a final checkpoint.

Check cancellation:
- On entry to present/choose and explicit check_cancelled().
- After acquiring capacity and after completing payload construction.
- After any blocking wait, before returning a validated choice.
- Before publishing normal completion.

Abandonment immediately invalidates the run, discards queued output and prepared
work, closes the endpoint, and wakes all waits. Closing the endpoint is itself a
cancellation signal. Unity never joins a worker during exit or replacement.

## Unwinding and failure

Use a private cancellation payload with resume_unwind, caught by one boundary
around the complete worker action. This bypasses the panic hook for expected
cancellation. Downcast the payload to distinguish cancellation from a real rules
panic; never classify all panics as cancellation.

The boundary owns the private state. Dispose of it on failure or cancellation.
Publish worker-stopped only after worker-owned state and cleanup probes have
been dropped. Inner catch_unwind handlers must resume unknown payloads. Document
the ownership argument beside any AssertUnwindSafe use.

No unwind may cross a non-unwinding C ABI. Shipping interactive profiles must
support panic=unwind. Destructors must terminate without an escaping second
panic. Builders and long computations must reach cancellation points; detached
workers are not a mechanism for terminating arbitrary blocked foreign code.

An active rules panic or required presentation failure abandons that run and
shows exit/restart controls while retaining the last accepted state. A late
failure from an abandoned run must not replace the new game's display.
Unexpected host commit failure stops the session; see presentation.

## Simulation fast path

Simulation runs inline in the caller on private State using a concrete policy.
There are no mandatory executor heap allocations, vtable calls, blocking waits,
interactive cancellation checks, snapshots, or change construction.

present does not call either lazy builder. choose evaluates the concrete policy
against State/specification without creating the owned UI prompt. Cancellation
is an inline no-op. Random sampling and AI search allocations belong to the game
policy and are measured separately.

Use public-entry benchmarks, an allocation counter scoped to primitives, and
optimized-code inspection. Do not infer zero overhead from source generics. A
caller coordinating simulations owns their scheduling/outer cancellation;
bounded simulation batches check abandonment between batches.

## Acceptance and persistence

A worker's normal return is not acceptance. Install its final State only after
the final checkpoint gate is satisfied and the host acknowledges a rendering
opportunity at or after satisfaction in the active committed generation.

Action admission first checks whether an active action/prompt or required final
presentation makes the application busy. Otherwise run the game's pure, bounded
validate_action against the accepted state. Invalid input returns public
feedback without forking state or starting a worker. A valid action returns
Started(run_id); reserve the word accepted for completed-state acceptance.

Only after final presentation acceptance expose next-action dispatch and an
accepted-state notification. Persistence runs outside rules and may
asynchronously serialize the accepted state. Save failure leaves the in-memory
accepted state valid and produces nonfatal feedback. Queued saves retain
immutable/forked state; never serialize a state concurrently being mutated by a
worker.

## Manual QA

Pause a worker at publication, inside a controlled builder, and at a prompt.
Replace the game each time: the replacement stays interactive, stale answers do
nothing, cancellation is silent, and worker-stopped follows cleanup. Trigger a
genuine panic separately and verify the active run's failure surface.
