# 31. Port chess board composition and move presentation in a fixture

A separate deterministic chess fixture renders representative legal moves
through Reactant while the existing playable entrypoint remains intact.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [World objects and input](../world.md)
- [Animation](../motion.md)
- [Rules and choices](../execution.md)

**Prerequisite:** [Task 30: Migrate tic-tac-toe through the unified rules and
display path](30-tictactoe-migration.md) and all its required follow-ups must be
integrated.

**Starting code:** Chess rules/src/lib.rs, spawn/movement helpers, gameplay
tests; opaque assets; public fixture driver.

## Example

Run the same fixed position through both displays before changing the default
chess entrypoint:

```text
position: legal kingside castle
action: castle
observe: king and rook follow established paths and reach their squares
compare: same visible result and timing in old and Reactant fixtures
```

## Implementation

1. Create a Reactant chess board component and immutable board-view data using
   the existing opaque piece prefabs, highlights, camera, and table assets.

2. Implement typed move/change descriptions and a worker action adapter around
   the existing chess rules library. Preserve UUIDs for moving pieces; use a new
   incarnation/visual where promotion changes the host contract.

3. Translate normal moves, knight paths, capture retention, castling, en
   passant, and promotion into movement policies/sequences/effects using
   existing behavioral requirements.

4. Expose this replacement through a named review fixture/alternate factory
   without changing the default production factory yet. Reuse the public
   behavior assertions on both implementations for covered cases.

5. Do not create a second permanent chess rules implementation or change legal
   move conventions.

## Acceptance

- Both old and Reactant fixtures produce the same visible board and capture/move
  ordering for explicit test positions.

- Castling moves both identities, knight motion preserves the expected path, and
  captured visuals disappear at the established beat.

- Promotion replaces the visible piece correctly without stale capture/input
  effects.

- The default chess sample and all existing scenarios remain functional
  throughout this task.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Default-entrypoint cutover, spawn choreography, full AI/audio/save/diagnostics
integration, and removal of the old engine are task 32.

## Manual QA

Open the replacement fixture with normal, capture, castling, en-passant, and
promotion positions and compare to the retained native baseline evidence.
