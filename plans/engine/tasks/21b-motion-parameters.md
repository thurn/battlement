# 21b. Drive material, light, particle, and audio properties

[Task group 21](21-shared-motion-drivers.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [21a: Share UI/world motion sampling and command
operations](21a-motion-drivers.md) is integrated.

**Starting code:** UI Motion sampler/timeline; Reactant targets/values; world property
writers; fake scheduling.

## Implementation

1. Add typed writers for material scalar, light intensity, particle emission, and audio
   volume using 21a's shared driver and existing native property hosts. Task 26 adds
   discrete sound/burst sequence entries, not another continuous driver.

2. Extend fake observations and one existing native material/effects scene. Validate
   independent property ownership and prepared-asset parameter types; do not repeat the
   full UI/world control matrix.

## Acceptance

- Each property follows its declared transition and remains isolated per instance. Reuse
  21a's pause/replacement tests and add native checks only for new property-writer
  behavior.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](21-shared-motion-drivers.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Change independent material/light/audio values on the same prepared timeline and inspect
native output.
