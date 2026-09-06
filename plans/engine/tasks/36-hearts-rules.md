# 36. Implement the fixed Hearts rules through the generic executor

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 35: Prepare Hearts assets and its 3D sample
shell](35-hearts-assets-shell.md) and all its required follow-ups must be
integrated.

**Source roles:** reactant-rules public API; Hearts shell; public display
driver. Resolve these through source-map.md; its links track the current owner
after crate moves. Inspect the concrete caller and host/fake counterpart before
editing.

## Result

One synchronous Hearts rules implementation handles dealing, passing, legal
play, tricks, scoring, and match completion.

## Implementation

1. Define State, public Snapshot, Action, Change, Prompt, Answer, fork_state,
   and saved RNG representation. Use stable rank/suit ordering and a documented
   seeded shuffle.

2. Implement ResolvePassing and PlayTurn with typed choice specs and lazy
   coherent checkpoints. Resolve all four passes simultaneously; include
   trick/hand resolution in the final-card action.

3. Create player observations that expose only their own hand, public
   cards/scores, and inferred void suits. Use separate game-private state for
   simulation/saving.

4. Implement the exact first-trick, broken-heart, queen, moon, 100-point, and
   shared-tie rules from hearts.md without configurable variants.

5. Add a minimal text/public-state projection in the driver so complete scripted
   hands can be validated before visual interactions are connected.

## Acceptance

- Explicit deals cover follow-suit rejection, forced first-trick penalty, queen
  not breaking hearts, only-hearts lead, all pass directions, trick leadership,
  moon scoring, and shared wins.

- The same seeded scripted action sequence produces the same final state in
  interactive and simulation modes.

- Public player snapshots never expose opponent faces or full private hands.

- A complete hand terminates after thirteen tricks and a match ends only after
  hand scoring.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Real card layout and user interactions are tasks 37-39; Monte Carlo opponents
are task 40. Use a deterministic legal policy here.

## Manual QA

Run explicit rare-rule deals through the public projection and inspect prompts,
legal feedback, trick winner, and totals.
