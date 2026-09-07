# 09. Publish immutable checkpoints through a 32-slot queue

A worker publishes immutable snapshots through a 32-slot FIFO to the Rust
consumer. Lazy builders run only after reserving capacity.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and session API](../interfaces.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 08: Add public display driving and deterministic virtual
time](08-public-display-driver.md) is integrated.

**Starting code:** reactant-rules contexts/worker connection from tasks 02-03;
public display driver.

## Example

The pending checkpoint limit must stop construction, not just queue insertion:

```text
Rust consumer held on A
present(B1) through present(B32): enqueue and return
present(B33): wait before snapshot/animation builders
consumer takes B1: B33 may be built, even while Unity is paused
worker changes private state: A and B remain immutable
```

## Implementation

1. Add owned snapshot/optional-event/optional-prompt records to the worker
   connection. Preserve publication order and internal cancellation identity.
   Use exactly 32 pending slots shared by all publication kinds. Reserved
   builders count; taking an entry for rendering releases its slot. Reserve
   before building and wait only if full. No host checkpoint ID is needed.

2. Implement cancellation checks on entry, after capacity acquisition, after
   payload construction, and after waits. Wake capacity waiters on
   abandonment/closure using the shared predicate protocol.

3. Preserve independent accepted, worker-private, and rendered-snapshot state.
   Document logical_clone ownership at its API; do not build a runtime
   deep-copy validator or a fixture that deliberately violates its contract.

4. Represent completion as a final-state/final-checkpoint publication using the
   same 32-slot limit. Never drop/coalesce entries. Expose public publication
   observations. Reuse this fixture in task 10 rather than duplicating the queue
   matrix. Memory profiling belongs to task group 46.

## Acceptance

- With the Rust consumer held on A, B1-B32 enqueue and B33 waits before its
  builders. Taking B1 permits one more construction. Unity pause alone does not
  prevent publication or consumption. Mix present/final entries and prove
  FIFO order without dropping/coalescing. Task 10 extends this same scenario
  with actual prompts. Record snapshot counts; byte profiling belongs to task 46.

- Mutation of worker state after publication does not change the published
  snapshot in a correct logical-clone fixture.

- Cancellation while blocked or building discards late output, unwinds after
  builder completion, and reports stopped after cleanup.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Prompt publication is task 10, application acceptance task 11, and existing
command-queue integration task 12. Use a public display consumer fixture, not a
private channel assertion.

## Manual QA

Use a controlled `Game::logical_clone` implementation to hold publication,
abandon the run, release the builder, and verify the replacement display never
sees the stale checkpoint.
