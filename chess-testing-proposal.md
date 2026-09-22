# Fast chess behavior tests

Replace the chess sample's ordinary tests with deterministic scenarios that
construct a position in memory, operate the visible UI, and inspect the
simulated
Unity host. In an optimized build, require p95 below 1 ms for a complete
scenario,
including construction, presentation, assertions, and destruction. Report worst
samples separately; this is a performance target, not a real-time guarantee.

**Reactant** is the Rust component and rules layer that produces Unity commands.
**The simulated host** is Battlement's in-memory execution of those commands,
including object transforms, UI elements, input picking, and animation events.
Tests must exercise both layers. Inspecting the chess rules board alone does not
prove that a piece moved on screen.

This document proposes changes; it does not report an implemented harness or a
measured performance improvement. Its audience is the engineer changing chess
and the shared Reactant testing infrastructure.

## Related implementation

These sources establish the current behavior and the boundaries to change:

- [Chess test support][support]: fixture bytes, host setup, move helpers, and
  simulated-object lookup.
- [Input tests][input], [outcome tests][outcomes], and [service
  tests][services]: the existing thirteen scenarios and their assertions.
- [Application assembly][assembly] and [view composition][view]: initial state,
  persistence, hidden controls, status labels, and automatic opponent turns.
- [Chess rules][chess] and [board rendering][board]: move execution, promotion,
  input callbacks, and piece presentation.
- [Display harness][display] and [fake client][fake]: worker synchronization,
  engine polling, response decoding, and virtual presentation time.
- [Rules execution][game], [publication queue][publication], and [prompt
  responses][response]: the synchronous execution API and blocking waits.
- [Application coordinator][coordinator] and [engine adapter][adapter]: work
  discovery, rendering, and response delivery.
- [Native scenarios][native]: the existing real Unity coverage.

## Findings against the requested behavior

The current suite has a valuable foundation: inputs pass through application
entry points and many assertions inspect objects after the fake host executes
real responses. The problems are in setup, synchronization, observations, and
what some helpers actually click.

### Construct a position directly

The fixtures use `include_bytes!`, so reading fixture files is already absent
from the test's runtime path. However, setup copies JSON into a map, invokes the
persistence hook, deserializes JSON, and parses the saved FEN position.

- The public engine constructor starts with the title screen and persistence.
- Internal assembly already accepts a board and an initial rules state.
- Native review fixtures use that internal route, but still install the
  persistence hook. Ignoring its result does not bypass loading or
  serialization.
- Ordinary behavior tests should use a typed position constructor and exclude
  persistence entirely. Persistence tests should deliberately retain the codec.

### Perform real UI operations

The existing `play_move` helper finds an invisible button named for a legal move
and activates it. This reaches a real callback, but bypasses board picking,
selection, drag behavior, and overlay interception.

Keep high-level test language, but translate it to visible board interaction.
Dedicated keyboard, controller, accessibility, and gesture tests should retain
explicit input sequences because those sequences are their subject.

### Assert simulated presentation independently

Several current assertions are useful: world positions, capture removal, sounds,
and promotion prefabs come from the simulated host. Others give weaker evidence:

- `assert_state` reads text from a label styled with `display: none`. It proves
  that a marker was published, not that the user can see the claimed result.
- `piece_at` finds a `BoxHitRegion` at an exact point. A hit target without its
  rendered piece can satisfy it.
- Promotion assertions require a particular parent/child relationship and
  preservation of an object ID that ordinary gameplay does not require.
- Some capture assertions only establish that a destination is occupied, without
  proving the mover's color/type or the victim's disappearance.
- The seeded AI test compares occupied squares, so it can miss piece-type
  errors.

The replacement should inspect rendered piece prefabs, their accumulated
transforms, and actual visible UI. Expected chess outcomes remain in each test.

### Finish without waiting or polling

`Display::drive_game` waits for worker notifications against an `Instant`
deadline, then calls the engine's polling entry point. `flush` also repeatedly
calls that entry point. These are disallowed in the fast suite even though the
worker synchronization is notification-based rather than a sleep loop.

Presentation is already virtual: `until_presented` advances scheduled events,
not real frames. That useful mechanism still has two problems: its stopping
condition is the test's expected result, and its implementation can reach other
polling machinery. Completing a known action must not depend on whether its
assertion has become true.

The AI has its own real-time search budget. A zero budget avoids search work but
does not remove the rules worker or provide explicit control over replies.

### Keep ordinary tests independent of UI mechanics

The support module centralizes some details, but tests still name the rules game
type, synchronization result, timing budget, hidden markers, asset constants,
object kinds, and tree relationships. The large public testing-constants module
mixes production settings, assets, automation identities, and expected outcomes.

Replace this interface with domain actions and host observations. Keep technical
assertions in focused adapter tests where they represent intentional behavior.

## Scenario interface and fixtures

A **scenario** owns one fresh application, its deterministic executor, services,
and simulated host. Its public interface speaks in chess positions and user
intent. These examples specify the intended API shape, not existing APIs.

```rust
let mut game = ChessScenario::at(positions::capture());
game.move_piece(D4, E5);
game.expect_piece(E5, White, Bishop);
game.expect_empty(D4);
game.expect_piece_count(Black, 1);
```

The fixture above must contain a white bishop on d4, a capturable black piece on
e5, and one remaining black king. Explicit expected outcomes are authored with
the fixture; the assertion helper must not run the move rules to discover them.

- A position fixture contains pieces, side to move, castling rights, en-passant
  state, and move counters. Validate it using the chess library's board builder.
- Use direct typed construction; do not parse JSON, FEN, or another text format
  in ordinary setup. Invalid fixture data is a developer error and must panic.
- Construct fresh mutable state for every scenario. Share only immutable asset
  descriptions; initialize shared data explicitly before benchmark sampling.
- Configure startup as title or playable position. Construct UI state from that
  choice through production assembly; never inject selection, object IDs,
  animation handles, or an expected outcome marker.
- An already playable position has no opening reveal. A title-start test runs
  the real opening presentation to its completion boundary.
- Do not promise save semantics beyond the actual saved data. Chess currently
  persists a FEN position, not interaction state or a complete repetition log.

Application dependencies must accept an explicit initial position, persistence
mode, execution driver, and opponent service. Production startup continues to
load saved data. Ordinary scenarios select persistence disabled: no load hook,
save encoding, path construction, or removal callback runs in that mode.

A persistence test uses the same application with an in-memory byte store and
persistence enabled. This preserves real save/reload coverage without making
serialization part of every move test.

## UI actions and observations

The chess helper owns gesture details. The shared host owns input routing and
physical presentation state. Neither may bypass the other by calling a game
callback or editing the rules board.

### Actions use the visible interaction path

`move_piece(from, to)` uses a pointer press, sufficient motion to begin a drag,
and release on the destination square. Input positions come from the simulated
camera and board geometry. The host performs normal UI/world picking, capture,
drag handling, and callback dispatch.

- Resolve squares through one chess board-coordinate adapter. Keep coordinate
  conversion out of ordinary tests; do not use the production move function to
  obtain either target positions or expected results.
- Use the presented camera transform and projection, including viewport size. Do
  not assume a fixed screen coordinate or directly supply a target object ID.
- Let overlays and disabled input block gestures normally. `try_move` returns
  `NoAction` for a rejected move after display-local effects finish.
  `move_piece`
  uses the same gesture but requires an admitted action, returning `Completed`
  or `AwaitingInput`; rejection is an immediate helper failure.
- Use visible dialog buttons for promotion and menu helpers. Fail immediately
  when a requested visible control is absent or ambiguous.
- Keep separate input-method tests for clicks, keyboard, controller, off-board
  drops, drag cancellation, accessibility, and modal interception.

Removing invisible move controls must not remove genuine accessibility support.
Attach semantic actions to visible board/piece elements and route them through
the same input policy as pointer and keyboard interactions. Accessibility tests
exercise that user-facing surface separately from the default pointer helper.

### Observations come from rendered objects

`expect_piece(square, color, kind)` finds rendered piece prefabs in the
simulated host and evaluates their accumulated world transforms. The helper maps
prefab addresses to chess color and type, and maps board squares to expected
centers.

- Traverse transforms without requiring a particular wrapper type, direct
  parent, or stable object ID. Ignore hit regions as evidence of a visible
  piece.
- Require an active, nonzero-scale rendered piece and exactly one matching piece
  at the square. Extra or duplicate pieces must fail the observation.
- Use a position tolerance of `1e-4` board-square widths after presentation
  ends. Timing-specific tests may instead inspect exact intermediate transforms.
- Use piece multisets and counts when asserting captures or complete positions.
  Exclude effects using asset kind, not arbitrary hierarchy depth.
- Keep the asset-to-piece mapping independent of production piece selection.
  Reusing the production selector would conceal a wrong-prefab bug.
- Query actual visible UI for title, pause, and promotion. A hidden label cannot
  satisfy an assertion about a visible message or control.

If check or game outcome has no visible text today, assert its actual visible or
host-executed feedback, such as the relevant sound, together with the displayed
position. Do not add a hidden marker or invent a new product UI solely to make
an assertion convenient. Native checks establish that the selected feedback is
perceivable in Unity.

## Shared execution without engine polling

The recommended shared change is resumable rules execution using Rust futures.
An action must suspend without blocking an OS thread when it needs a human
answer or publication capacity. Production and tests execute the same future.

This is necessary because the existing synchronous `choose` waits inside the
rules call stack, while publication capacity can also block. Merely replacing
thread creation with an inline call would deadlock promotion or a full queue.

The important API change is to make execution and its suspension points
awaitable:

```rust
fn execute<'a>(context: &'a mut Self::Context,
               state: &'a mut Self::State,
               action: Self::Action)
  -> impl Future<Output = ()> + Send + 'a;
// Within the same chess action used in production and tests:
let promotion = context.execution.choose(state, prompt).await;
let movement = candidates.into_iter()
  .find(|m| m.promotion == Some(promotion)).unwrap();
context.apply_move(state, movement).await;
```

The action task owns private state and context; its outer future borrows them
while executing and returns them in the final publication. No task borrows the
scenario, renderer, or a stack frame that can end before execution finishes.

- Make interactive `choose` a one-shot response future. Register its wakeup when
  returning pending; accepting a valid reply wakes it exactly once.
- Make publication reservation awaitable. Freeing capacity wakes the producer;
  state cloning and animation construction occur only after reservation.
- Preserve publication ordering and independent logical snapshots. Final-state
  acceptance means that the application installs the final private state and
  context only after its matching output is successfully submitted, exactly
  once.
  Preserve response validation and cancellation semantics.
- Keep direct rules simulation available with policy answers and skipped
  presentation; it is not the execution mode of these UI behavior tests.
- Production drives tasks on the rules worker and may park that worker while no
  task is ready. Real AI computation remains off the presentation thread.
- Fast scenarios drive ready tasks on their current thread and never create,
  park, join, or await an OS worker. Runtime-owned await points register task
  dependencies as prompt, service, or publication capacity before suspending;
  wakeup or cancellation clears them. Empty ready work with a pending task and
  no registered dependency fails immediately. Capacity waits require a
  registered
  consumer; declared prompts and manual services may return control to the test.

A wake-driven executor calls `Future::poll` only for a newly runnable task. This
is Rust's continuation mechanism; it must never call `Engine::poll`, repeatedly
probe readiness, or retry a pending future without a wakeup. The existing
cooperative executor is a useful implementation reference, not a sufficient
replacement for the blocking rules pipeline.

### Deliver concrete work instead of discovering it repeatedly

The in-process application driver needs explicit notifications for input,
publication availability, render invalidation, response delivery, timer expiry,
and presentation completion. Each notification schedules a concrete work item.

Extract ready-work processing from the existing engine adapter. The production
polling adapter may call that shared processing function. The fast driver
invokes it only for queued work and receives responses directly; it cannot wrap
the existing polling method under a different name.

- Maintain a FIFO ready queue with deduplicated task wakeups and stable enqueue
  order. Do not repeatedly scan all sessions or timers for changes.
- Route response delivery through the existing response encoding, validation,
  decoding, and fake command execution. Performance does not justify replacing
  the host with a mirror of rules state or unverified predicted commands.
- Retain each response buffer until decoding and host consumption finish; no
  borrowed command data may outlive its response lease or escape into later
  work.
- Reuse input decoding, callback dispatch, reconciliation, and host command
  execution. Keep transport-adapter behavior covered separately where the
  in-process driver no longer exercises native polling.
- On cancellation, invalidate the exact session's tasks, publications, prompts,
  and scheduled presentation work. Drop suspended futures and their private
  state; stale UI responses cannot resume a replacement session.

This driver is a shared application capability, not a chess-only copy of the
engine. No changes to gameplay assertions should be required when the executor
or wire transport is refactored.

## Completion and virtual presentation time

An **action boundary** is completion of one submitted user intent and its finite
presentation work, or presentation of a prompt that needs another user action.
It is separate from the whole application's becoming idle: looping music and
future opponent turns may remain active.

Each input owns a completion group. Producer tokens account for rules work,
publications, render batches, and finite presentation callbacks. Acquire a child
token before enqueueing work; release it on completion or cancellation. An
active
producer retains a token while it can create more work, including suspension.

- A render batch coalescing work from multiple groups retains one child token
  from each contributor until its finite presentation finishes.
- A prompt response joins its original action group. Restart creates a new
  group,
  cancels the old group and its tokens, and invalidates old prompt handles.
- Runtime tokens propagate internally; tests never use their IDs. Groups end as
  completed or cancelled. Cancellation cannot appear as successful completion.
- A looping sound's start may belong to a group; its infinite lifetime cannot.
  Finite capture effects, movement completion, and removal callbacks do belong.

The driver consumes concrete ready work, then advances virtual time only when
finite presentation work remains for the requested group:

- Process the earliest global scheduled deadline up to the group's next
  deadline,
  with stable enqueue order for ties. Run callbacks at their actual virtual due
  times and include newly scheduled intervening events before advancing again.
- Stop advancing after the requested boundary is reached. Future ambient timers
  and infinite audio cannot extend it. Never test an assertion predicate to
  decide whether more work should run.
- Return `Completed` after final-state acceptance and release of all group
  tokens.
  Return `NoAction` when the input admitted no rules action and its effects
  finish.
- Return `AwaitingInput` after the prompt publication is accepted, its visible
  UI
  is applied, and all preceding finite presentation finishes. Only the suspended
  action's continuation token remains; it resumes on the next valid response.
- Assertions are valid after all three return values. A wrong destination still
  completes; the subsequent assertion fails. An unfinished group with neither
  ready work, scheduled events, nor a registered dependency fails immediately.

An opponent request is detached at registration into a service-owned dependency,
after the production turn coordinator issues it. Registration finishes before
the
player's helper returns. `reply_with` creates a new group for the service
result,
computer action, and presentation. An unanswered request owns no player-group
token; cancellation still follows the originating session.

Use a ceiling of 100,000 concrete work items per helper call to diagnose cycles,
not to retry idle work. A task that never yields cannot be preempted by this
driver; the outer process supervisor handles hangs outside the scenario API.
Timing tests can advance a specified virtual duration. Ordinary tests need no
animation durations, timeouts, or predicate-based settling helpers.

## Promotion and opponent services

Promotion must remain interactive, including cancellation and stale responses.
The scenario returns control to the test with a visible dialog; it does not
preselect an answer through a rules policy.

```rust
let mut game = ChessScenario::at(positions::promotion_capture());
game.move_piece(A7, B8).expect_awaiting_input();
game.expect_promotion_choices();
game.choose_promotion(Knight);
game.expect_piece(B8, White, Knight);
game.expect_empty(A7);
```

The opponent is a separate service dependency. Production automatically requests
a real AI move. Fast scenarios use a manually completed deterministic service.
The same turn coordinator must issue the request in both configurations.

```rust
game.move_piece(E2, E4);
game.expect_piece(E4, White, Pawn);
game.reply_with(E7, E5);
game.expect_piece(E5, Black, Pawn);
```

`reply_with` completes an already pending opponent request. It fails immediately
if there is none, the side to move is wrong, or the supplied move is illegal.
The selected move follows the normal rules publication and animation path. It
cannot mutate the board directly or manufacture the missing service request.

An unanswered opponent request does not extend the player's action boundary.
Replacing the game invalidates it. Ordinary setup on Black's turn returns with a
pending request; scenarios explicitly supply the reply they want to inspect.
Keep real AI legality, deterministic fallback, and background execution checks
separate from the sub-millisecond behavior suite.

## Coverage replacement and naming cleanup

Retain each meaningful behavior while removing setup and assertions that depend
on incidental implementation. Splitting combined tests gives failures one clear
cause and makes each benchmark correspond to a small scenario.

- **Title and board startup:** visible Play, activation, complete typed board,
  opening completion; place exact asset counts and input configuration in
  focused presentation or input tests.
- **Keyboard and controller:** explicit input sequences and resulting pieces.
- **Legal, illegal, and outside-board drops:** pointer-driven behavior,
  unchanged board on rejection, and intended feedback in the dedicated gesture
  tests.
- **Both capture styles and castling:** mover identity by color/type, victim
  removal, rook and king destinations; check exact effects separately.
- **En passant and promotion:** piece type/color/count, both source removals,
  dialog choice, cancellation, and rejected stale replies.
- **Check, draw, and either winner:** expected displayed positions plus actual
  feedback; remove hidden-marker assertions.
- **AI determinism:** real AI test with complete typed board observations.
- **Save/reload and storage failures:** persistence-enabled service tests using
  the real codec and visible application behavior.
- **Diagnostics:** focused optional-module tests; diagnostics never serve as the
  ordinary gameplay oracle.
- **Music, reset, and opening effects:** virtual-time presentation tests, plus
  behavior tests for reset and title-to-board interaction.
- **Restart during pending work:** deterministic cancellation checks; retain a
  separate real-worker test for thread cleanup and interruption.

Delete the public testing-constants module exported by the chess library entry
point. Remove its basename, plural form, and all case variants from filenames,
exports, symbols, tests, comments, and related documentation in the chess sample
and shared testing infrastructure. Assets belong to asset code, save names to
persistence, durations to presentation, and UI identities to their elements.
Remove unused hidden markers and migrate consumers to actual host observations.
Preserve valid accessibility functionality; introduce no replacement umbrella.

The shared execution API change requires updating its callers and fixtures,
including worker tests. Preserve their behavioral coverage while replacing the
blocking suspension API. Do not keep a second chess rules implementation for the
fast suite. Replacement is complete only when ordinary chess tests use no worker
wait, engine poll, hidden-marker assertion, or persistence fixture route.

## Performance and correctness acceptance

The audit establishes structural problems, not measured timings. No baseline
benchmark was run for this document. Serialization and scheduling costs must be
measured independently; no evidence currently identifies one dominant cost.

Benchmark optimized builds on a documented developer machine, with one scenario
at a time and compilation excluded. Include fresh setup, every input, finite
presentation, all assertions, and teardown within the timed region.

- Cover default-board construction, a normal move, capture, castling, en
  passant, promotion, rejected move, title startup, reset, and a scripted
  opponent reply.
- Warm immutable shared asset data explicitly; report first-use initialization
  separately so its cost is not hidden in an unmeasured global initializer.
- Run 1,000 complete samples per scenario after warmup. Report median, p95, and
  maximum, together with machine, toolchain, build options, and revision.
- Require p95 below 1 ms for every listed ordinary scenario on that machine.
  Report maxima; an uncontrolled OS scheduling interruption is not a hard
  real-time guarantee. Use the same machine/profile for before/after
  comparisons.
- Report setup, input/rules, reconciliation/transport, presentation,
  observation, and teardown costs separately as diagnostic measurements. Do not
  subtract them from whole-scenario acceptance. Report nanoseconds as
  measurement units, not a promise that a complete UI scenario takes a handful
  of nanoseconds.
- Run behavioral assertions on every benchmark iteration. Exclude real AI
  search, actual Unity, and deliberate worker-thread checks from this
  performance target.

Add instrumented integration checks that fail if an ordinary scenario uses an
engine polling entry point, blocking worker operation, persistence codec, or
real-time timer dependency. The fast driver must reject an undeclared external
wait immediately rather than silently switching to the slow harness.

Test the test interface with representative deliberate defects: disconnect a
board input callback, leave a captured prefab alive, render a wrong-color piece,
misplace a piece, block it with a modal overlay, and suppress a prompt. Each
must fail the relevant scenario without modifying its expected state. Check that
changing object IDs or adding a harmless transform wrapper does not break it.

Compare the in-process and production adapters on the same small deterministic
interaction: drag a pawn two squares and inspect the resulting host board.
Compare normalized pieces, UI, and relevant effects, excluding incidental IDs.
Keep native Unity evidence for geometry, assets, picking, and actual rendering
that an in-memory model cannot independently establish.

## Manual QA

Validate one assembled native chess interaction early: start a real game, drag
e2 to e4, and confirm that the pawn arrives, the old square is empty, and input
is correctly blocked while required presentation work is active. Retain a native
Ditto capture; a component fixture does not establish that this path works.

Also exercise a promotion capture in the assembled player: observe the dialog,
choose a knight, and confirm the displayed piece and victim removal. Repeat with
a restart while the dialog is open and verify that an obsolete answer cannot
change the replacement game. Check the corresponding scenario outputs against
what is visibly present in Unity.

Run the ordinary scenario benchmarks separately and inspect the complete timing
report. Native capture durations do not count toward the sub-millisecond target.

[support]: samples/chess/rules/tests/support/mod.rs
[input]: samples/chess/rules/tests/input_tests.rs
[outcomes]: samples/chess/rules/tests/outcomes_tests.rs
[services]: samples/chess/rules/tests/services_tests.rs
[assembly]: samples/chess/rules/src/app.rs
[view]: samples/chess/rules/src/reactant_view.rs
[chess]: samples/chess/rules/src/reactant_game.rs
[board]: samples/chess/rules/src/chess_board.rs
[display]: crates/reactant-testing/src/display.rs
[fake]: crates/battlement-fake/src/client.rs
[game]: crates/reactant-rules/src/game.rs
[publication]: crates/reactant-rules/src/publication.rs
[response]: crates/reactant-rules/src/response.rs
[coordinator]: crates/reactant/src/game_app.rs
[adapter]: crates/reactant-core/src/app_engine.rs
[native]: samples/chess/ditto.toml
