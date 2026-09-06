# 16. Add stable state selectors and queued display stores

Sparse snapshot/store changes reevaluate only affected subscribers without
tearing one render or breaking moved components.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Identity and state](../identity.md)
- [Rules and choices](../execution.md)
- [Architecture](../architecture.md)

**Prerequisite:** [Task 15: Remove components while keeping unfinished exit
visuals](15-incarnations-and-removal.md) and all its required follow-ups must be
integrated.

**Starting code:** External stores; context/hooks; runtime scheduling; presented
snapshot provider.

## Example

Changing a hand should not reevaluate a component subscribed only to the score:

```rust
let score = use_game_selector::<HeartsGame, _>(|state| state.south_score);
ScoreLabel::new().score(score)
```

## Implementation

1. Expose optional displayed-snapshot selector subscriptions with explicit
   equality. Keep props as a complete equivalent path and avoid introducing a
   game-global mutable model into component handlers.

2. Capture a stable store version for each render/preparation. Queue writes
   during rendering for a subsequent desired revision; do not mutate the current
   prepared update in place.

3. Commit subscription/effect changes only with the corresponding tree
   generation. Cancel obsolete preparation subscriptions and clean up removed
   consumers.

4. Integrate moved-context consumers from task 14 so retained hook state does
   not imply a stale provider or duplicate subscription.

## Acceptance

- Updating an unrelated field leaves the subscribed component's render count
  unchanged. The selector itself may still run to compare its output.

- A store write during rendering appears in a later complete generation, never
  half of the current frame.

- An abandoned render does not publish a subscription, and a moved consumer
  reads the new provider exactly once.

- The same visible fixture works when selectors are replaced with explicit
  props.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Performance captures are task 46. This task establishes correctness and
localized reevaluation, not numerical budgets.

## Manual QA

Update separate score/settings fields while moving a subscribed card between
providers. Inspect stable complete frames and the public evaluation counters.
