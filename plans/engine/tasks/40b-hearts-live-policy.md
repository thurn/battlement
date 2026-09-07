# 40b. Select live Hearts moves with bounded Monte Carlo work

[Task group 40](40-hearts-simulation-ai.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Test scenes](../fixtures.md)

**Prerequisite:** [40a: Build information-safe sampling and shared-rules
rollouts](40a-hearts-rollouts.md) is integrated.

**Starting code:** Hearts state, prompt enum, and rules; generic simulation policy;
worker scheduling.

## Implementation

1. Apply the 32-deal default, one rollout per legal play candidate, and eight
   heuristic-shortlisted passing combinations from hearts.md. Preserve a mapping from
   any shortlist back to the original prompt order. Stable tie-breaking and seeded
   rollout heuristics remain game-owned.

2. Classify each live choice in `HeartsPolicy::owner`; `ExecutionMode` routes AI through
   DisplayConnection::choose_with_policy immediately after its snapshot/prompt is
   queued. Do not wait for visibility or animation. The shared queue applies
   backpressure only at 32 pending entries. Run bounded computation on the rules worker,
   never Unity's thread. Stop invalidates output immediately; the policy may finish
   computation before the helper observes cancellation. No independent
   controller-message job or explicit engine cancellation primitive is required.

3. Measure owned prompt construction, policy work, and primitive overhead separately.
   Record reproducible seed/work count and public choices; do not reveal sampled or real
   hidden hands through player UI/diagnostics.

## Acceptance

- Every returned index selects a legal option from the original prompt order; rollout
  and full search may use different heuristics without extra prompt enums or engine execution specialization.

- Replacement stays responsive during bounded AI work. Old results are discarded at the
  choice boundary and cannot answer a replacement request.

- Simulation skips snapshot/event builders and display waits, even when the live queue
  is full. Report its construction/search allocations instead of claiming all prompts
  are free.

- Live passing/play use the existing policy publication path and original option
  indices. Reuse task 10's generic multi-choice queue proof; do not force a
  five-AI-choice fixture into Hearts' human-turn action contract.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](40-hearts-simulation-ai.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Repeat a fixed passing and card-play decision, then restart during bounded search and
verify stale output is discarded.
