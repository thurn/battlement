# 32. Complete chess application integration and remove the old engine

The default playable chess sample runs entirely through Reactant without losing
its current game flow or effects.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Rules and choices](../execution.md)
- [Animation](../motion.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 31: Port chess board composition and move presentation
in a fixture](31-chess-reactant-fixture.md) and all its required follow-ups must
be integrated.

**Starting code:** Task 31 chess fixture; existing chess AI, persistence,
diagnostics, audio, spawn and input code.

## Example

The completed sample must retain more than legal board moves:

```text
start -> opening piece animation and music
move -> AI response, movement sounds, optional check sound
save and reopen -> same accepted board and settings
reset during AI -> new game ignores the old result
```

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

No new chess rules, AI-strength redesign, or completion of unrelated chess-ui
pages.

## Manual QA

Run the full chess native scenario selection, then manually start, move,
capture, resume a saved game, and reset while AI is active.
