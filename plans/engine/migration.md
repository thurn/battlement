# Migration and behavioral test preservation

Read this before changing an existing sample, removing an old API, or adapting a
test. See [validation](validation.md), [architecture](architecture.md), and the
selected task.

## Sample destinations

The migration preserves existing player-visible behavior and licensed assets.
Only implemented chess-ui pages are migrated; unfinished gallery pages are
separate work.

| Sample | Destination |
| --- | --- |
| basic | Remains a direct Battlement example; regression coverage stays active |
| ui | Remains a direct Battlement UI protocol example |
| tictactoe | Reactant components, worker actions, rendered snapshots |
| chess | Reactant world composition/actions/Motion; opaque piece prefabs permitted |
| reactant | Unified component APIs plus neutral engine laboratory |
| chess-ui | Migrated currently implemented UI/gallery with behavior preserved |
| hearts | New playable reference sample |

Mechanical crate/API edits needed to keep all callers compiling belong to the
task that changes the API. The named later sample tasks change runtime behavior
and ownership. Do not leave a sample broken until its migration number arrives.

## Establish behavior before changing it

Task 01 records the existing scenario inventory and classifies assertions:
observable behavior, host/protocol contract, or implementation coupling. Do not
simply label every current test black-box because it calls FakeClient.

Examples of coupling already present include exact TransformTween command
variants, TimeWait entries, command order/counts, prefab host-kind checks, exact
internal object counts, and sleep/poll loops tied to engine scheduling.

For each coupled assertion:
1. Identify the behavioral guarantee, if any.
2. Rework it to observe displayed state, timed motion, sound/effect occurrence,
   input eligibility, or public lifecycle state.
3. Prove the replacement against the old implementation before that sample's
   behavioral migration, using native Ditto when the old fake cannot model it.
4. Delete an assertion with no meaningful external guarantee and record why.
5. Retain an equivalent native or public-driver scenario for meaningful
   coverage.

A protocol command test can remain in Battlement when it tests that protocol
capability directly; it is not game-level evidence of behavior preservation. No
compatibility shim should emit obsolete command patterns solely for tests.

For example, a capture test should observe the captured piece through time:

```rust
display.dispatch(capture_move);
display.advance_time(before_capture);
display.object(captured_piece).assert_visible();
display.advance_time(remaining_capture_time);
display.object(captured_piece).assert_absent();
```

Keep the established timing and visible outcome. Whether the engine emitted a
particular tween command is a lower-level protocol concern.

## Timing and state snapshots

The legacy world fake jumps to tween endpoints and ignores waits. Extend
virtual-time behavior and explicit settle helpers before relying on intermediate
motion assertions. Do not globally reinterpret an old synchronous click helper
as a hidden wall-clock sleep.

For tic-tac-toe, preserve the visible immediate human move and 100 ms AI delay.
The public driver may synchronize until the human checkpoint is presented at
unchanged virtual time; that does not require `Game::execute` to run inside the
click callback. At 99 ms the AI mark is absent; at 100 ms it can become visible
after deterministic worker/frame synchronization.

Preserve chess move paths, capture timing, castling, promotion, spawn beats,
audio, input/focus, saved-game behavior, diagnostics, and reset flows. Observe
their visible outcomes rather than retaining obsolete batch representation.
Existing chess save triggers remain game-owned and may use status-driven app
logic plus accepted_state; do not add an engine acceptance callback or impose
Hearts' explicit-save UI on chess.

## No permanent duplicate engine

Use existing implementations where they fit, but eliminate old game-specific
command construction after each sample's complete migration. Any temporary
adapter introduced by a task must have an explicit removal owner in a later task
and cannot control the same property as new Motion simultaneously.

The old low-level APIs used by basic/ui can remain as generic Battlement
capabilities. They must not force a second Reactant component or motion runtime.
Where low-level tween commands remain, route execution through shared drivers
when their semantics match, keeping protocol behavior tests.

## Visual evidence

Retain current native baselines and semantic fixtures as the starting contract.
Do not refresh all baselines after changing host composition. Compare affected
initial, changed, and reset states and inspect the actual images/geometry.

A host implementation change is not permission to redesign a sample. If a
rendering difference is unavoidable, identify the visible difference and seek a
contract decision rather than silently accepting a replacement baseline.

## Manual QA

For each migrated sample, use its existing native scenario selection and replay
the principal user flow. Check initial/changed/reset states, persistence where
present, input modes, effects and timing. Verify basic/ui still operate through
their direct Battlement entrypoints.
