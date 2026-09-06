# 09. Publish immutable checkpoints with at most one waiting

A worker publishes ordered immutable snapshots with at most one pending
checkpoint, using lazy builders after reservation.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [API examples and defaults](../interfaces.md)

- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 08: Add public display driving and deterministic virtual
time](08-public-display-driver.md) and all its required follow-ups must be
integrated.

**Starting code:** reactant-rules modes/worker connection from tasks 02-03;
public display driver.

## Example

The pending checkpoint limit must stop construction, not just queue insertion:

```text
checkpoint A visible; B pending
worker reaches present(C): C builders have not run
B commits: C may be built
worker changes private state: A and B remain immutable
```

## Implementation

1. Add checkpoint ID/change indices and owned payload records to the worker
   connection. Reserve the single pending-checkpoint capacity before building
   snapshot/change data; keep it reserved while the main thread prepares that
   checkpoint.

2. Implement cancellation checks on entry, after capacity acquisition, after
   payload construction, and after waits. Wake capacity waiters on
   abandonment/closure using the shared predicate protocol.

3. Preserve independent accepted, worker-private, and snapshot state. Add a
   deliberately shared-mutable fixture to document invalid game snapshot
   ownership without designing a runtime deep-copy system.

4. Represent completion as a final-state/final-checkpoint publication using the
   same one-pending-checkpoint limit. Expose only public checkpoint/lifecycle
   observations to scenarios.

## Acceptance

- With checkpoint A presented and B pending, a worker attempting C has not
  invoked C's builders. Releasing the capacity held by B permits exactly one
  construction.

- Mutation of worker state after publication does not change the displayed
  snapshot in a correct game-owned snapshot fixture.

- Cancellation while blocked or building discards late output, unwinds after
  builder completion, and reports stopped after cleanup.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Prompt publication is task 10, application acceptance task 11, host-acknowledged
commit task 12. Use a public display consumer fixture, not a private channel
assertion.

## Manual QA

Use a controlled snapshot builder to hold publication, abandon the run, release
the builder, and verify the replacement display never sees the stale checkpoint.
