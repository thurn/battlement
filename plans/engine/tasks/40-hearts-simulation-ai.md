# 40. Add bounded information-respecting Hearts simulations

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [Execution contract](../execution.md)
- [Fixtures contract](../fixtures.md)

**Prerequisite:** [Task 39: Complete Hearts card play, trick collection, and
scoring](39-hearts-play-scoring.md) and all its required follow-ups must be
integrated.

**Source roles:** Hearts public player observations/rules; generic Simulation
policy; worker scheduling. Resolve these through source-map.md; its links track
the current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

Three AI players choose passes and plays using reproducible bounded rollouts
without observing hidden opponent cards.

## Implementation

1. Generate determinizations consistent with the acting player's known cards,
   public play history, and void-suit information. Ensure sampling cannot
   duplicate/omit cards or violate observed constraints.

2. Use the shared Hearts rules/executor for rollout transitions and legal
   choices. Finish the current hand, apply moon scoring, and minimize mean
   additional penalty for the acting player. Evaluate candidates on the same
   sampled deals/seeds. Passing uses simultaneous sampled opponent passes and
   the same end-of-hand objective. Implement the cheap rollout
   heuristic/shortlist from hearts.md.

3. Apply the default 32 determinizations per decision, one rollout per legal
   play candidate, and top eight passing combinations. Make work counts explicit
   fixture/config inputs and stable tie-breaking deterministic.

4. Route presented AI-owned prompts to an application-owned simulation job using
   only that seat's owned observation. Return its answer through the normal
   run/request validation path while the synchronous rules worker waits. Cancel
   bounded batches on abandonment or request replacement; keep simulation
   primitives free of interactive cancellation branches.

5. Record seed/work count and public decisions for reproduction; keep private
   sampled hands out of normal player UI/diagnostics.

## Acceptance

- Identical observations, seed, and work count yield identical choices.

- Two private deals indistinguishable to an AI produce the same decision
  distribution for the same observation/seed.

- Every chosen action is legal; samplers respect cards already played and known
  void suits.

- Abandonment between rollout batches stops further decisions without blocking
  menus, and primitive allocation/codegen guarantees remain intact.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Advanced difficulty levels and competitive-strength targets are outside scope.
Performance reporting is task 46.

## Manual QA

Play against the three opponents, repeat a saved explicit decision with its
seed, and inspect batch cancellation while opening a menu.
