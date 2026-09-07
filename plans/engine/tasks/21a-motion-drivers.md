# 21a. Share UI/world motion sampling and command operations

[Task group 21](21-shared-motion-drivers.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 20: Extend focus, controller navigation, and touch across
domains](20-world-focus-touch.md) is integrated.

**Starting code:** UI Motion sampler/timeline; Reactant targets/values; world property
writers; fake scheduling.

## Implementation

1. Extract host-neutral sampling, playback identity/generation, and property ownership
   from UI Motion. First supply existing UI writers and world transform writers, keeping
   direct sample behavior.

2. Preserve existing UI animate/initial/exit/variants/gesture semantics and MotionConfig
   inheritance. Ordinary host playback must not trigger Rust component evaluation.

3. Implement separate base transform and local gesture-offset composition. Make
   replacement start from actual displayed values and preserve spring velocity.

4. Extend the fake for these interpolation/control/outcome contracts. Reuse the temporal
   driver; do not create a second scheduling framework.

5. Expose running Motion as existing `IBattlementCommandOperation` instances, including
   finite/infinite behavior, completion, cancellation, and failures. Preserve generated
   command blocking flags; verify blocking waits and nonblocking continuation through
   the existing batch scheduler. Reuse generic low-level tween drivers where semantics
   match; preserve direct sample contracts.

## Acceptance

- One UI opacity and one world scalar with the same transition sample equivalently at
  intermediate times.

- Pause/resume/speed/stop and interrupted/completed/failed outcomes behave consistently
  across adapters.

- A hover offset composes with a moving base and returns smoothly to the latest
  underlying target.

- Existing UI motion regression tests and native captures remain valid, including
  reduced-motion behavior.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](21-shared-motion-drivers.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Compare UI opacity and a world scalar with the same transition; interrupt a spring and
compose hover with moving placement.
