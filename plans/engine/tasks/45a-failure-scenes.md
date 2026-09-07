# 45a. Close publication, cancellation, and recovery coverage gaps

[Task group 45](45-effects-failures-laboratory.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Animation](../motion.md)
- [Presentation timing](../presentation.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 44: Complete component, layout, and input test
scenes](44-identity-composition-laboratory.md) is integrated.

**Starting code:** Laboratory/inspector; effects/retention/pause; worker barriers;
generic failure surfaces.

## Implementation

1. Reuse prompt-cycle, cancellation, asset-loading, snapshot-queue, and save-failure
   fixtures from their owner tasks. Identify missing boundaries before adding cases; do
   not rebuild the engine's existing failure matrices.

2. Add controlled publication/builder/prompt/answer/completion races and bounded
   ordinary computation released to a helper/return boundary. Verify immediate public
   Stopped and later worker-stopped after fixture-owned drops, without pretending
   computation was forcibly interrupted.

3. Cover delayed/failed asset commands, missing targets, batch redelivery, rerenders
   without replay, invalid graphs, blocking failure, no-change entries, explicit waits,
   local UI during animation, and old-session messages. Pause Unity while rules finish,
   then inject a host failure without reversing state.

## Acceptance

- All named cancellation positions/races terminate cleanly and cannot alter a
  replacement display; real panic remains distinguishable.

- Blocking/nonblocking commands, in-place retargeting, asset dependencies, and resource
  release match their shared contracts.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](45-effects-failures-laboratory.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Use controlled builders and a held queue to exercise any uncovered cancellation
boundary, then verify replacement and save-failure recovery.
