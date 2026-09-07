# 09. Publish immutable checkpoints through a 32-slot queue

A worker can publish 32 pending immutable state snapshots before waiting for
display capacity. Lazy builders run after reservation.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and session API](../interfaces.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 08: Add public display driving and deterministic virtual
time](08-public-display-driver.md) and all its required follow-ups must be
integrated.

**Starting code:** reactant-rules contexts/worker connection from tasks 02-03;
public display driver.

## Example

The pending checkpoint limit must stop construction, not just queue insertion:

```text
checkpoint A visible; hold display advancement
present(B1) through present(B32): enqueue and return
present(B33): wait before cloning or animation construction
B1 commits: one slot opens and B33 may be built
worker changes private state: published snapshots remain immutable
```

## Implementation

1. Add checkpoint IDs, optional index-zero semantic events, and owned payload
   records to the worker connection. Use one FIFO with exactly 32 pending slots.
   Reserve before building snapshot/state-animation data. Reserved builders and
   native preparation count; the displayed checkpoint does not. Release only on
   display commit. Return after enqueueing; wait only when capacity is full.

2. Implement cancellation checks on entry, after capacity acquisition, after
   payload construction, and after waits. Wake capacity waiters on
   abandonment/closure using the shared predicate protocol.

3. Preserve independent accepted, worker-private, and displayed-snapshot state.
   Add a deliberately shared-mutable fixture to document invalid logical-clone
   ownership without designing a runtime deep-copy system.

4. Represent completion as a final-state/final-checkpoint publication using the
   same 32-pending-checkpoint limit. Never drop/coalesce entries. Expose only
   public checkpoint/lifecycle observations to scenarios.

## Acceptance

- With A displayed and held, publication B1-B32 completes. B33's snapshot and
  animation builders have not run. Moving B1 into preparation does not release
  capacity; committing B1 permits exactly one additional construction.

- Mix present, prompt, and final entries in the same bound. Deliver all entries
  in FIFO order without dropping or coalescing. Report peak retained bytes for
  representative states; the count bound does not promise a fixed byte budget.

- Mutation of worker state after publication does not change the displayed
  snapshot in a correct logical-clone fixture.

- Cancellation while blocked or building discards late output, unwinds after
  builder completion, and reports stopped after cleanup.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Prompt publication is task 10, application acceptance task 11, host-acknowledged
commit task 12. Use a public display consumer fixture, not a private channel
assertion.

## Manual QA

Use a controlled `Game::logical_clone` implementation to hold publication,
abandon the run, release the builder, and verify the replacement display never
sees the stale checkpoint.
