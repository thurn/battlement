# 22. Build sequences with labels that follow actual completion

Sequences coordinate locally executed tracks with fixed-time and
actual-completion dependencies.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 21: Animate UI, world objects, and effects with shared
drivers](21-shared-motion-drivers.md) and all its required follow-ups must be
integrated.

**Starting code:** Animation controls/sequences; host-neutral Motion driver;
protocol graph definitions; fake time driver.

## Example

A label after a variable-duration move follows actual completion, whereas an
absolute-time entry retains its timestamp:

```text
move starts; nominal duration is 250 ms
destination changes at 100 ms
"arrived" occurs at actual arrival, even if later than 250 ms
entry at absolute 250 ms still runs at 250 ms
```

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Live layout destinations become real in task 24, checkpoint gate binding in task
25, and sound/burst entries in task 26.

## Manual QA

Inspect a sequence with concurrent rotation, sequential movement, and a delayed
label; pause mid-step and verify successors remain locally scheduled.
