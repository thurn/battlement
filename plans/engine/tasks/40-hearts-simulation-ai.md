# 40. Choose Hearts moves by simulating possible hands

Three live AI players use game-owned sampling and bounded rollouts through the
same rules and shared prompt enum, returning stable legal-option indices.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Test scenes](../fixtures.md)

**Prerequisite:** [Task 39: Complete Hearts card play, trick collection, and
scoring](39-hearts-play-scoring.md) and all its required follow-ups must be
integrated.

**Starting code:** Hearts state, prompt enum, and rules; generic simulation
policy; worker scheduling.

## Example

Compare candidates on the same sampled deals:

```text
sample 32 hidden deals from the actor's known/public information
for each candidate: finish the current hand on each sampled deal
apply normal hand scoring, including shooting the moon
return the candidate's original legal-option index
```

## Implementation

1. Build HeartsPolicy on `ChoicePolicy<HeartsGame>`. It receives &HeartsState
   and &HeartsPrompt<'_>. Sample hidden hands from actor knowledge, public
   history, and void constraints; do not inspect real hidden assignments when
   sampling or scoring heuristic input. Reactant does not sanitize state for the
   policy.

2. Use Game::execute and a simulation HeartsContext for rollout transitions.
   Finish the current hand, include moon scoring, and minimize mean additional
   actor penalty. Reuse sampled deals/seeds across candidates. Passing commits
   simultaneous sampled choices and uses the same hand-scoring objective.

3. Apply the 32-deal default, one rollout per legal play candidate, and eight
   heuristic-shortlisted passing combinations from hearts.md. Preserve a mapping
   from any shortlist back to the original prompt order. Stable tie-breaking and
   seeded rollout heuristics remain game-owned.

4. Route live AI through DisplayConnection::choose_with_policy immediately after
   its snapshot/prompt is queued. Do not wait for visibility or animation. The
   shared queue applies backpressure only at 32 pending entries. Run bounded
   computation on the rules worker, never Unity's thread. Stop invalidates
   output immediately; the policy may finish computation before the helper
   observes cancellation. No independent controller-message job or explicit
   engine cancellation primitive is required.

5. Measure owned prompt construction, policy work, and primitive overhead
   separately. Record reproducible seed/work count and public choices; do not
   reveal sampled or real hidden hands through player UI/diagnostics.

## Acceptance

- Equivalent actor knowledge and seeds produce the same decision distribution
  even when real hidden assignments differ. Samplers preserve all card and
  observed void constraints.

- Every returned index selects a legal option from the original prompt order;
  rollout and full search may use different heuristics without extra enums.

- Replacement stays responsive during bounded AI work. Old results are discarded
  at the choice boundary and cannot answer a replacement request.

- Hold display while running five consecutive AI choices in one execution with
  fewer than 32 pending entries. Each policy runs without a display-ready
  signal; snapshots remain ordered. A subsequent human choice still waits for
  visibility.

- Simulation skips snapshot/event builders and display waits, even when the live
  queue is full. Report its construction/search allocations instead of claiming
  all prompts are free.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Competitive-strength targets and extra difficulty levels remain outside this
task. Fixed workload performance reporting is task 46.

## Manual QA

Repeat an explicit decision with fixed seeds, alter only unknowable real cards,
and compare decisions. Restart during a rollout while using menus; verify no old
result appears in the replacement game.
