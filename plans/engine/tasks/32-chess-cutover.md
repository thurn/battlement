# 32. Complete chess application integration and remove the old engine

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Migration contract](../migration.md)
- [Execution contract](../execution.md)
- [Motion contract](../motion.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 31: Port chess board composition and move presentation
in a fixture](31-chess-reactant-fixture.md) and all its required follow-ups must
be integrated.

**Source roles:** Task 31 chess fixture; existing chess AI, persistence,
diagnostics, audio, spawn and input code. Resolve these through source-map.md;
its links track the current owner after crate moves. Inspect the concrete caller
and host/fake counterpart before editing.

## Result

The default playable chess sample runs entirely through Reactant without losing
its current game flow or effects.

## Implementation

1. Connect the existing AI search policy through bounded off-main-thread action
   execution. Preserve difficulty/think-time behavior and cancellation between
   search work units.

2. Port play/start/reset, spawn beats, selection effects, music and
   move/capture/check/castle sounds to checkpoint registrations and shared
   sequences. Reuse existing licensed assets and fallback selection logic in
   Rust.

3. Adapt persistence to accepted-state notifications and preserve
   diagnostics/controller/global-key behavior. Save failure must not invalidate
   an already accepted in-memory action.

4. Switch the default factory and Ditto fixtures to the Reactant implementation
   only after the complete old behavior suite passes on it. Remove the old
   command-building engine and task 31's transition-only factory.

5. Keep opaque piece prefabs, but remove game-specific imperative presentation
   decisions outside Rust components/policies.

## Acceptance

- All established chess gameplay/input/audio/persistence/diagnostic assertions
  pass through the new default entrypoint.

- Opening spawn beats, capture timing, castling sound composition, AI response,
  and reset match native evidence.

- Exiting during AI or required presentation leaves the replacement responsive
  and rejects old results.

- No legacy chess engine or temporary adapter remains necessary for normal
  gameplay.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

No new chess rules, AI-strength redesign, or completion of unrelated chess-ui
pages.

## Manual QA

Run the full chess native scenario selection, then manually start, move,
capture, resume a saved game, and reset while AI is active.
