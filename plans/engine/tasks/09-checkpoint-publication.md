# 09. Implement immutable checkpoint publication and backpressure

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Minimum interfaces](../interfaces.md)

- [Execution contract](../execution.md)
- [Presentation contract](../presentation.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 08: Add public display driving and deterministic virtual
time](08-public-display-driver.md) and all its required follow-ups must be
integrated.

**Source roles:** reactant-rules modes/worker endpoint from tasks 02-03; public
display driver. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

A worker publishes ordered immutable snapshots with at most one pending
checkpoint, using lazy builders after reservation.

## Implementation

1. Add checkpoint ID/change indices and owned payload records to the endpoint.
   Reserve the single publication slot before building snapshot/change data;
   keep it reserved while the main thread prepares that checkpoint.

2. Implement cancellation checks on entry, after capacity acquisition, after
   payload construction, and after waits. Wake capacity waiters on
   abandonment/closure using the shared predicate protocol.

3. Preserve independent accepted, worker-private, and snapshot state. Add a
   deliberately shared-mutable fixture to document invalid game snapshot
   ownership without designing a runtime deep-copy system.

4. Represent completion as a final-state/final-checkpoint publication using the
   same bounded slot. Expose only public checkpoint/lifecycle observations to
   scenarios.

## Acceptance

- With checkpoint A presented and B pending, a worker attempting C has not
  invoked C's builders. Releasing B's slot permits exactly one construction.

- Mutation of worker state after publication does not change the displayed
  snapshot in a correct game-owned snapshot fixture.

- Cancellation while blocked or building discards late output, unwinds after
  builder completion, and reports stopped after cleanup.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Prompt publication is task 10, application acceptance task 11, host-acknowledged
commit task 12. Use a public endpoint consumer fixture, not a private channel
assertion.

## Manual QA

Use a controlled snapshot builder to hold publication, abandon the run, release
the builder, and verify the replacement display never sees the stale checkpoint.
