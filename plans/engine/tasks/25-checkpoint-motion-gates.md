# 25. Bind checkpoint registrations to Motion and rendered acceptance

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Minimum interfaces](../interfaces.md)

- [Presentation contract](../presentation.md)
- [Motion contract](../motion.md)
- [Execution contract](../execution.md)

**Prerequisite:** [Task 24: Animate layout movement with continuous
retargeting](24-layout-movement-projection.md) and all its required follow-ups
must be integrated.

**Source roles:** Application checkpoint admission; host transactions/frame
acknowledgement; sequence events; movement policies. Resolve these through
source-map.md; its links track the current owner after crate moves. Inspect the
concrete caller and host/fake counterpart before editing.

## Result

Typed checkpoint changes prepare exactly-once animations whose required
labels/completions pace gameplay and saving.

## Implementation

1. Add the checkpoint/change/registration-slot presentation-effect API. Evaluate
   it during preparation, reserve playback handles, and atomically install
   visible tree, playback registration, occurrences, and required contributions
   at commit.

2. Default gate contributions to required layout movement. Let a registration
   choose an earlier label for its own contribution while preserving other
   required contributors.

3. Implement permanent satisfaction and atomic successor rebinding for
   unfinished required work. Serialize event acceptance with replacement commits
   and reject stale playback generations.

4. Require a rendering opportunity at or after gate satisfaction before
   admitting another checkpoint or accepting final state. Keep cosmetic work
   independent and cap initial checkpoint commits to one per rendered frame.

5. Add explicit finite wait steps to sequences for presentation pacing without
   sleeping the rules worker; validate them as ordinary finite dependencies.

## Acceptance

- Rerender/preparation retry starts one animation registration; two simultaneous
  required movements both contribute to the gate.

- An earlier chosen label permits advancement while later cosmetic movement
  continues, but a pre-gate frame never counts.

- Retarget preserves an arrival dependency; replacement accepts old completion
  only if it was processed before rebinding.

- Required failure abandons the action and exposes accepted-state recovery;
  cosmetic stop does not.

- A final worker result enables saving only after its real gate and rendering
  acknowledgement.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Sound/burst occurrences are task 26 and replay isolation task 28. No fake-only
shortcut may satisfy the live gate.

## Manual QA

Exercise gate-replacement just before/after a label event, then step a final
checkpoint frame by frame and observe when saving becomes available.
