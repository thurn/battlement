# 40a. Build information-safe sampling and shared-rules rollouts

[Task group 40](40-hearts-simulation-ai.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Test scenes](../fixtures.md)

**Prerequisite:** [Task 39: Complete Hearts card play, trick collection, and
scoring](39-hearts-play-scoring.md) is integrated.

**Starting code:** Hearts state, prompt enum, and rules; generic simulation policy;
worker scheduling.

## Implementation

1. Build HeartsPolicy on `ChoicePolicy<HeartsGame>`. It receives &HeartsState and
   &HeartsPrompt<'_>. Sample hidden hands from actor knowledge, public history, and void
   constraints; do not inspect real hidden assignments when sampling or scoring
   heuristic input. Reactant does not sanitize state for the policy.

2. Use Game::execute and a HeartsContext containing a simulation ExecutionMode for
   rollout transitions. Finish the current hand, include moon scoring, and minimize mean
   additional actor penalty. Reuse sampled deals/seeds across candidates. Passing
   commits simultaneous sampled choices and uses the same hand-scoring objective.

3. Implement the cheap legal rollout policy and candidate forcing through the same
   Game::execute path. Keep live Hearts on the deterministic legal policy until 40b.
   Preserve known passed-card information and public void constraints as game-owned
   actor knowledge; never use real unknown assignments.

## Acceptance

- Equivalent actor knowledge and seeds produce the same decision distribution even when
  real hidden assignments differ. Samplers preserve all card and observed void
  constraints.

- For explicit deals, sampled states preserve actor knowledge/card counts/voids, and
  deterministic rollouts reach ordinary moon-aware hand scoring through the public
  simulation API. Compare identical seeds and actor information directly; no statistical
  AI-strength suite.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](40-hearts-simulation-ai.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Change only unknowable opponent cards and verify identical sampled deals and candidate
results for a fixed seed.
