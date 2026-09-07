# 30. Migrate tic-tac-toe through the unified rules and display path

Tic-tac-toe preserves its visible behavior using Rust components, worker-owned
rules, and checkpoint-paced presentation.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [World objects and input](../world.md)

**Prerequisite:** [Task 29: Build the reusable presentation
inspector](29-presentation-inspector.md) is integrated.

**Starting code:** Tic-tac-toe rules/src and gameplay tests; new App/driver;
existing assets/native scenarios.

## Example

Keep the existing observable AI delay using virtual presentation time:

```rust
display.click(empty_cell);
display.wait_for_render_submission(); // synchronize without advancing time
display.advance_time(Duration::from_millis(99));
display.assert_ai_mark_absent();
display.advance_time(Duration::from_millis(1));
display.advance_frame();
```

## Implementation

1. Keep board/outcome/RNG in clonable rules state for display snapshots.
   Implement game actions with lazy human-move and AI-response checkpoints and
   stable mark identities.

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

No new game features or UI redesign. The sample should now be a complete simple
reference for later implementors.

## Manual QA

Play/reset several rounds, including clicks during thinking and outside the
board; compare existing native selections and verify the public test suite.
