# 30. Migrate tic-tac-toe through the unified rules and display path

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Migration contract](../migration.md)
- [Execution contract](../execution.md)
- [Presentation contract](../presentation.md)
- [World contract](../world.md)

**Prerequisite:** [Task 29: Build the reusable presentation
inspector](29-presentation-inspector.md) and all its required follow-ups must be
integrated.

**Source roles:** Tic-tac-toe rules/src and gameplay tests; new App/driver;
existing assets/native scenarios. Resolve these through source-map.md; its links
track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

Tic-tac-toe preserves its visible behavior using Rust components, worker-owned
rules, and checkpoint-paced presentation.

## Implementation

1. Separate board/outcome/RNG state from view construction. Implement game
   actions with lazy human-move and AI-response checkpoints and stable mark
   identities.

2. Build board, title, status, and marks as world components using existing
   artwork. Route the board's committed hit through Reactant events and dispatch
   typed actions.

3. Preserve the 100 ms AI presentation delay as a required finite presentation
   wait after the human-move checkpoint, not a sleep or a poll-time mutation in
   the rules worker.

4. Keep round reset, seed behavior, messages, hit geometry, input eligibility,
   and existing native visual-state fixtures. Replace old command assembly once
   all behavior uses the new path.

5. Adapt test setup only where the public driver now synchronizes worker/frame
   progress; retain the established observable assertions.

## Acceptance

- The human mark and thinking text present at unchanged virtual time after a
  valid click; occupied/outside clicks change nothing.

- AI mark is absent at 99 ms and visible at 100 ms after explicit worker/frame
  synchronization.

- Wins/draws, next-round reset, seeded moves, and initial/changed/reset native
  captures preserve behavior.

- Rules state is private to the worker; no sample event handler writes it
  directly.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

No new game features or UI redesign. The sample should now be a complete simple
reference for later implementors.

## Manual QA

Play/reset several rounds, including clicks during thinking and outside the
board; compare existing native selections and verify the public test suite.
