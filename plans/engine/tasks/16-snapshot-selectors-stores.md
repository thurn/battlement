# 16. Add stable snapshot selectors and queued display stores

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Identity contract](../identity.md)
- [Execution contract](../execution.md)
- [Architecture contract](../architecture.md)

**Prerequisite:** [Task 15: Separate logical unmount from retained visual
lifetime](15-incarnations-and-removal.md) and all its required follow-ups must
be integrated.

**Source roles:** External stores; context/hooks; runtime scheduling; presented
snapshot provider. Resolve these through source-map.md; its links track the
current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

Sparse snapshot/store changes reevaluate only affected subscribers without
tearing one render or breaking moved components.

## Implementation

1. Expose optional presented-snapshot selector subscriptions with explicit
   equality. Keep props as a complete equivalent path and avoid introducing a
   game-global mutable model into component handlers.

2. Capture a stable store version for each render/preparation. Queue writes
   during rendering for a subsequent desired revision; do not mutate the current
   proposal in place.

3. Commit subscription/effect changes only with the corresponding tree
   generation. Cancel obsolete preparation subscriptions and clean up removed
   consumers.

4. Integrate moved-context consumers from task 14 so retained hook state does
   not imply a stale provider or duplicate subscription.

## Acceptance

- Updating an unrelated field leaves an equal selector's evaluation count
  unchanged through a public fixture counter.

- A store write during rendering appears in a later complete generation, never
  half of the current frame.

- An aborted proposal does not publish a subscription, and a moved consumer
  reads the new provider exactly once.

- The same visible fixture works when selectors are replaced with explicit
  props.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Performance captures are task 46. This task establishes correctness and
localized reevaluation, not numerical budgets.

## Manual QA

Update separate score/settings fields while moving a subscribed card between
providers. Inspect stable complete frames and the public evaluation counters.
