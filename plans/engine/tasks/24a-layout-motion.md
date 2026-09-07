# 24a. Animate layout destinations with continuous retargeting

[Task group 24](24-layout-movement-projection.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 23: Implement world Flexbox, Grid, fans, piles, and
arcs](23-world-layout.md) is integrated.

**Starting code:** World layouts; existing UI layout projection; unified Motion; UUID
movement/ref matching.

## Implementation

1. Supply the engine default spring movement with no required App setup. Add inherited
   and object overrides resolved from the destination logical ancestry. Policies receive
   source/destination layout/pose, refs, and typed state animations when available.

2. Implement live layout destinations as native targets. Reflow preserves playback
   identity/arrival dependencies, spring velocity, and current displayed origin;
   duration tweens restart their configured duration.

3. Let a sequence own base placement while layout continues updating its pending
   destination. Return ownership continuously at arrival; hover remains an independent
   local offset and drag cannot steal required placement.

4. Extend the draw-reflow scene through normal command operations. Snapshot
   semantic-event lowering is task 25; use the existing event-driven controls here.

## Acceptance

- A game with no `.movement()` or MotionConfig declaration moves identified objects
  automatically; inherited/object overrides still work.

- A reveal step follows its own anchor while hand reflow changes only the pending
  destination; the blocking operation waits for arrival after the hand step starts.

- Repeated retargeting causes no pose jump and does not produce duplicate completion
  identities.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](24-layout-movement-projection.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Reflow during reveal and arrival, then return placement to layout while hover remains
smooth.
