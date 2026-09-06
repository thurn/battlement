# 22. Implement immutable sequences and completion-relative labels

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Motion contract](../motion.md)
- [Presentation contract](../presentation.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 21: Use shared Motion drivers for UI, world, and native
properties](21-shared-motion-drivers.md) and all its required follow-ups must be
integrated.

**Source roles:** Animation controls/sequences; host-neutral Motion driver;
protocol graph definitions; fake time driver. Resolve these through
source-map.md; its links track the current owner after crate moves. Inspect the
concrete caller and host/fake counterpart before editing.

## Result

Sequences coordinate locally executed tracks with fixed-time and
actual-completion dependencies.

## Implementation

1. Replace the fixed-duration-only sequence lowering with an immutable
   dependency graph containing targets, absolute/relative scheduling, completion
   edges, labels, and stable declaration order.

2. Add typed ref targets and live/captured anchor resolution. Reserve a playback
   handle during preparation and start only on commit.

3. Validate cycles, missing labels/targets, unsupported properties, infinite
   required dependencies, and accidental overlapping property writes before
   playback. Support explicit replacement for intentional overlap.

4. Execute prepared successor tracks and labels locally in Unity/fake without
   waiting for Rust callbacks. Emit correlated terminal and label events once
   per live generation.

5. Expose pause/resume/speed/stop consistently through scoped use_animate
   controls and update the public authoring example.

## Acceptance

- Concurrent and sequential steps start at their specified times; a
  completion-relative label waits for a deliberately delayed finite predecessor.

- Same-timestamp labels/occurrences preserve declaration order and precede
  terminal completion.

- Invalid graphs leave the previous presentation unchanged and start no track.

- Replacing a property interrupts the old owner; unrelated properties continue.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Live layout destinations become real in task 24, checkpoint gate binding in task
25, and sound/burst entries in task 26.

## Manual QA

Inspect a sequence with concurrent rotation, sequential movement, and a delayed
label; pause mid-step and verify successors remain locally scheduled.
