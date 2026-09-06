# 40. Choose Hearts moves by simulating possible hands

Three AI players choose passes and plays using reproducible bounded rollouts
without observing hidden opponent cards.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Test scenes](../fixtures.md)

**Prerequisite:** [Task 39: Complete Hearts card play, trick collection, and
scoring](39-hearts-play-scoring.md) and all its required follow-ups must be
integrated.

**Starting code:** Hearts public player observations/rules; generic Simulation
policy; worker scheduling.

## Example

Compare candidates on the same possible hidden hands and seeds:

```text
sample 32 possible deals consistent with the actor's knowledge
for each candidate: finish the current hand on each sampled deal
apply normal scoring, including shooting the moon
choose lowest mean additional penalty; use stable tie-breaking
```

## Implementation

1. Sample possible hidden hands consistent with the acting player's known cards,
   public play history, and void-suit information. Ensure sampling cannot
   duplicate/omit cards or violate observed constraints.

2. Use the shared Hearts rules/executor for rollout transitions and legal
   choices. Finish the current hand, apply moon scoring, and minimize mean
   additional penalty for the acting player. Evaluate candidates on the same
   sampled deals/seeds. Passing uses simultaneous sampled opponent passes and
   the same end-of-hand objective. Implement the cheap rollout
   heuristic/shortlist from hearts.md.

3. Apply the default 32 possible deals per decision, one rollout per legal play
   candidate, and top eight passing combinations. Make work counts explicit
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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Advanced difficulty levels and competitive-strength targets are outside scope.
Performance reporting is task 46.

## Manual QA

Play against the three opponents, repeat a saved explicit decision with its
seed, and inspect batch cancellation while opening a menu.
