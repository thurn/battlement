# Fast chess behavior tests

Replace the chess sample's ordinary tests with deterministic scenarios that
construct a position in memory, operate the visible UI, and inspect the
simulated Unity host. In an optimized build, require p95 below 1 ms for a
complete scenario, including construction, presentation, assertions, and
destruction. Report worst samples separately; this is a performance target, not
a real-time guarantee.

**Reactant** is the Rust component and rules layer that produces Unity commands.
**The simulated host** is Battlement's in-memory execution of those commands,
including object transforms, UI elements, input picking, and animation events.
Tests must exercise both layers. Inspecting the chess rules board alone does not
prove that a piece moved on screen.

Preserve the current synchronous game execution model. Make tests fast by
injecting inline execution and scripted services, while reusing real UI input,
rules, rendering, and simulated command execution. Actual choice-dialog
interaction remains in a separate integration suite.

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

A **scenario** owns one fresh application, its inline rules runner, services,
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
mode, inline test behavior, and synchronous opponent dependencies. Production
startup continues to load saved data. Ordinary scenarios select persistence
disabled: no load hook, save encoding, path construction, or removal callback
runs in that mode.

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
  `move_piece` uses the same gesture but requires an admitted action, returning
  `Completed`; rejection is an immediate helper failure.
- Use visible controls for menu helpers and focused dialog interaction tests.
  Fail immediately when a requested visible control is absent or ambiguous.
  Ordinary promotion helpers inject the answer before the visible board input.
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

## Synchronous execution with injected test behavior

Keep `Game::execute`, `ExecutionMode::present`, and `ExecutionMode::choose`
synchronous with their existing signatures. Games retain their current control
flow, contexts, and choice policies. Production keeps its current worker,
blocking human choices, bounded publication queue, and native delivery behavior.
Do not introduce futures, continuations, coroutines, or a different game API.

The fast harness injects three narrow behaviors at application construction:
inline action execution, immediate scripted human choices, and a publication
queue that collects a complete action without waiting for a consumer. Ordinary
chess code still calls the same rules functions through the same UI callbacks.

```rust
fn execute(context: &mut Self::Context,
           state: &mut Self::State, action: Self::Action);
// Existing game code remains synchronous:
let promotion = context.execution.choose(state, prompt);
let movement = candidates.into_iter()
  .find(|m| m.promotion == Some(promotion)).unwrap();
context.apply_move(state, movement);
```

These are testing dependencies, not branches in individual game actions. Select
execution behavior once through application assembly and pass it to the existing
coordinator and rules context. The normal constructor defaults to the existing
worker behavior; the test constructor explicitly selects inline behavior.

### Inline execution and publication buffering

Extract the common action body from `RulesRun::spawn`: execute against private
state/context, capture each existing presentation checkpoint, and append the
final checkpoint containing `CompletedAction`. The worker and inline paths call
this same body. Neither path installs the final rules state directly.

- Preserve legality validation before cloning, independent snapshots, context
  transfer, publication ordering, and lazy animation construction.
- Add an internal run-backing choice for worker execution or inline completion.
  The inline branch invokes the common body exactly once and returns a run whose
  FIFO is already complete. It never starts, joins, or waits for a worker.
- Use the existing publication storage with an injected capacity behavior:
  production retains its bounded wait; inline tests append to a growable FIFO.
  Keep existing safe ownership and uncontended synchronization initially.
- In inline mode, capacity reservation never waits and publication does not send
  worker notifications. Appending beyond the production queue capacity is valid.
  Test this explicitly; changing the buffer to a larger fixed bound is not a
  fix.
- Drain and release snapshots as they are presented. Peak memory is one action's
  output, so this path is appropriate for finite test actions, not arbitrarily
  long streaming rules work. Backpressure stays covered on the real worker.
- Retain `GameOutput::submitted` and `RulesContext::accept`: accepted state and
  reusable context change only after successful submission of final output.
- Preserve panic/failure handling and cleanup. Inline failure marks the session
  failed and abandons unpublished work; it does not accept partial private
  state. Do not fabricate worker-start or worker-stop events for an inline run.

`GameHandle::dispatch` currently holds a mutable session borrow while starting
work. Inline execution must only write to its private state and publication
storage. It must not recursively render the application, invoke test gestures,
or run a host callback from inside `execute`. Drain publications only after the
input callback has returned and the session borrow has been released.

This avoids duplicating chess rules and does not require the shared runtime to
support suspending an inline game. Direct rules simulation remains unchanged;
its existing suppression of presentation makes it unsuitable for these tests.

### Script choices before executing the action

Install a typed choice resolver on the rules display connection for inline
tests. `ExecutionMode::Interactive` remains in use so `present` still publishes
real animation checkpoints. Override only the human-answer branch; normal
policy-owned choices continue using the game's existing policy.

The resolver receives the prompt through `G::Prompt<'_>` and returns its
selected option index. Convert the index with the existing `select_response`
validation, including `PromptData::is_valid_response`. No test writes a raw
board mutation or bypasses the rules' choice handling.

- Consume answers in order. A chess answer matches prompt kind, source, target,
  and desired piece; derive the index from actual prompt options, not an ordinal
  hardcoded in the test.
- Fail immediately on a missing answer, mismatched prompt, or invalid response.
  Verify that answers attached to a helper were consumed when that helper
  returns.
- In the injected branch, return the answer directly without creating a blocking
  request or publishing an unanswered human prompt. Record a diagnostic choice
  transcript if useful, but never use it as proof of rendered UI.
- Do not change `ChessPolicy` globally to claim human choices are policy-owned.
  The override belongs to test dependencies, and production choice ownership
  remains unchanged.

```rust
let mut game = ChessScenario::at(positions::promotion_capture());
game.move_piece_with_promotion(A7, B8, Knight);
game.expect_piece(B8, White, Knight);
game.expect_empty(A7);
```

The helper supplies a matching answer, then drives the normal visible board
drag. This tests promotion rules and rendered results, not the promotion dialog.
There is no `AwaitingInput` return in the fast runner. Dialog appearance, button
wiring, invalid responses, and restart while awaiting a choice retain focused
real-worker and native tests. Those tests are outside the sub-millisecond suite.

### Consume known output without engine polling

Inline execution removes worker synchronization, but `Display::flush` would
still violate the no-polling requirement. Add a narrow synchronous
output-driving interface to the application/test adapter, reusing existing
rendering and response delivery. Do not build a general scheduler or replace the
native loop.

- After an input callback returns, consume its already-completed run FIFO
  through the existing `GameConsumer` and publication acceptance path. Render
  and submit one known publication at a time; apply it to the fake host in the
  same order.
- Extract submission of an already available publication from the native polling
  entry point. The test path passes concrete work to that shared function; it
  does not call `Engine::poll` or probe the engine until a desired status
  appears.
- Drive a context refresh directly for a known change. Factor current change
  application out of `AppRuntime::poll`; do not call it merely under a new name.
  Only explicit input, consumed publication, due timer, or presentation callback
  can request a refresh on this path.
- Preserve response encoding, validation, decoding, delivery budgets, and fake
  command execution. Apply and release each response before requesting the next;
  retain its buffer until all borrowed command data has been consumed.
- If output admission is blocked, release host-owned responses first. If
  delivery still cannot progress, fail immediately with the retained output
  information. Do not relax the production delivery budget or spin waiting for
  capacity.
- After a presentation callback or due timer, directly process its known output
  and any complete inline action it caused. An undeclared external dependency is
  unsupported in the fast runner and fails immediately.

Draining a finite FIFO of known checkpoints is ordinary work, not engine
polling. A callback may append more concrete work, but an empty worklist is
never retried. Reuse the production render and delivery functions; do not create
a test renderer that predicts commands from the rules board.

## Completion and virtual presentation time

An ordinary helper handles one player input operation, all publications from its
inline action, and their finite presentation events. Automatic opponent dispatch
is held by an injected chess turn policy, described below. No cross-thread
action tracking or reference-counted producer-token system is needed.

- Consume the returned FIFO through its final publication, preserving existing
  host blocking-batch ordering and presentation completion callbacks.
- Advance to actual finite presentation deadlines, not frame-by-frame. Process
  intervening global deadlines chronologically, with stable ordering for ties;
  include newly scheduled events before moving to a later virtual time.
- Presentation callbacks deliver inputs synchronously and their resulting output
  is consumed before advancing again. Finish finite transients and object
  destruction as well as spatial movement.
- Do not settle unrelated future app timers. Apply ambient events crossed while
  advancing presentation, but looping audio and future music transitions cannot
  extend the helper. Timing tests explicitly advance their chosen timers.
- Return `Completed` after final output is accepted and finite presentation
  ends. Return `NoAction` after a rejected input's display-local effects finish.
  Startup and menu helpers similarly consume their concrete output to
  completion.
- Never use the asserted position as the stopping condition. A wrong destination
  still completes normally and fails the subsequent independent assertion.

Reuse the existing fake host's finite-event machinery. Expose the concrete next
event and its callback output where necessary to remove indirect polling; do not
add a new event engine. Restrict the ordinary suite to self-contained finite
presentation, with no unrelated perpetual animation in its fixture.

Use a ceiling of 100,000 concrete work items per helper to diagnose scheduled
cycles, not to retry idle work. A synchronous game action that never returns
cannot be interrupted inline; the outer test process supervisor handles hangs.
The scenario itself has no timeout, busy wait, worker wait, or engine polling.

## Explicit opponent replies with synchronous services

Keep the synchronous `ComputerMove` action. Inject its move-selection
dependency: production calls the existing AI, while tests return a scripted
legal move. Separately inject when the turn coordinator permits automatic
dispatch.

```rust
game.move_piece(E2, E4);
game.expect_piece(E4, White, Pawn);
game.reply_with(E7, E5);
game.expect_piece(E5, Black, Pawn);
```

- Production turn dispatch remains automatic. Ordinary scenarios begin with
  computer turns held, so the player's completed position is observable.
- `reply_with` supplies one move and releases one turn permit through app-owned
  configuration. Refresh the real turn coordinator so it dispatches the normal
  computer action; the helper must not call `GameHandle::dispatch` itself.
- Include the permit's revision in that coordinator's effect dependencies so a
  held turn is reconsidered when the permit changes. Consume the permit once.
- The synchronous selector validates the scripted move against the real board
  and returns it to the unchanged move-application path. There is no pending
  service future or asynchronous reply protocol.
- Fail immediately if the side is wrong, the game is terminal, the reply is
  illegal, or the coordinator did not consume the permit and answer. This makes
  missing automatic-turn wiring observable without polling it.
- Reset clears held answers and permits. Real AI legality, deterministic
  fallback, background execution, and cancellation remain separate integration
  coverage.

These are the only chess-specific execution seams: initial position/persistence,
human answer injection, opponent selection, and opponent dispatch permission.
Game action signatures and the structure of their rules remain synchronous.

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
  scripted promotion results in the fast suite; actual dialog choices,
  cancellation, and rejected stale replies in focused integration tests.
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
  separate real-worker test for in-flight interruption and thread cleanup. The
  inline suite cannot interrupt rules in the middle of a synchronous call.

Delete the public testing-constants module exported by the chess library entry
point. Remove its basename, plural form, and all case variants from filenames,
exports, symbols, tests, comments, and related documentation in the chess sample
and shared testing infrastructure. Assets belong to asset code, save names to
persistence, durations to presentation, and UI identities to their elements.
Remove unused hidden markers and migrate consumers to actual host observations.
Preserve valid accessibility functionality; introduce no replacement umbrella.

Existing games retain their execution APIs and production behavior. Add tests
for the injected runner and the small shared submission extraction; do not
migrate every game to a new abstraction. Replacement is complete when ordinary
chess tests use no worker wait, engine poll, hidden-marker assertion, or
persistence fixture route, and the retained integration suite still covers real
human prompts, worker cleanup, backpressure, and native rendering.

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
misplace a piece, or block it with a modal overlay. Each must fail the relevant
fast scenario without changing expected state. Suppressing the promotion dialog
must fail its separate interaction test; scripted choices cannot detect it.
Changing object IDs or adding a harmless transform wrapper must not break
ordinary gameplay assertions.

Validate the inline backend against the worker using a deterministic fixture
that publishes more than 32 checkpoints, then compare every snapshot and final
accepted state. Also cover missing, mismatched, invalid, and unused scripted
answers; failed output submission; reset with queued presentation; and automatic
opponent dispatch consuming exactly one permit. These are infrastructure checks,
not dependencies of each chess test.

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
