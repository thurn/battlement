# 24b. Preserve screen continuity across UI/world transfers

[Task group 24](24-layout-movement-projection.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [24a: Animate layout destinations with continuous
retargeting](24a-layout-motion.md) is integrated.

**Starting code:** World layouts; existing UI layout projection; unified Motion; UUID
movement/ref matching.

## Implementation

1. Implement the explicit camera/plane/rectangle projection policy for compatible
   UI/world visual transfers, with inactive host replacement and a retained transition
   representation.

2. Extend the existing identity-transfer scene with explicit orthographic and
   perspective camera/plane/rectangle mapping. A transition representation is limited to
   the transferred component; no arbitrary scene-copy system is required.

## Acceptance

- Cross-root and portal UI moves retain identity and animate rendered geometry; UI/world
  transfer has continuous screen-space correspondence.

- Missing projection fails render validation; no implicit pixel/world conversion is
  invented.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](24-layout-movement-projection.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Move one card across UI/world and portal roots; verify continuous screen geometry and
failure before mutation when projection data is absent.
