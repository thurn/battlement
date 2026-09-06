# 31. Port chess board composition and move presentation in a fixture

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Migration contract](../migration.md)
- [World contract](../world.md)
- [Motion contract](../motion.md)
- [Execution contract](../execution.md)

**Prerequisite:** [Task 30: Migrate tic-tac-toe through the unified rules and
display path](30-tictactoe-migration.md) and all its required follow-ups must be
integrated.

**Source roles:** Chess rules/src/lib.rs, spawn/movement helpers, gameplay
tests; opaque assets; public fixture driver. Resolve these through
source-map.md; its links track the current owner after crate moves. Inspect the
concrete caller and host/fake counterpart before editing.

## Result

A separate deterministic chess fixture renders representative legal moves
through Reactant while the existing playable entrypoint remains intact.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Default-entrypoint cutover, spawn choreography, full AI/audio/save/diagnostics
integration, and removal of the old engine are task 32.

## Manual QA

Open the replacement fixture with normal, capture, castling, en-passant, and
promotion positions and compare to the retained native baseline evidence.
