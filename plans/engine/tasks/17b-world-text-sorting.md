# 17b. Integrate rich world text and mixed visual ordering

[Task group 17](17-world-rendering-primitives.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Presentation timing](../presentation.md)
- [Identity and state](../identity.md)

**Prerequisite:** [17a: Compose sprites, meshes, cameras, and
lights](17a-world-hosts.md) is integrated.

**Starting code:** World adapter; object protocol/builders; Unity world creation;
prepared assets; fake asset catalog.

## Implementation

1. Adapt existing GameObjectKind::Text/TextState with world::Text reconciliation,
   prepared font dependencies, rich text, wrapping, alignment, tint, and opacity. Do not
   introduce another text host or protocol.

2. Implement group-relative mixed sprite/text ordering, rich text, wrapping, alignment,
   tint/opacity, explicit geometry scale/orientation, and independently selectable
   front/back children.

## Acceptance

- Sprites and rich text render in the declared order, including overlapping glyph/sprite
  regions in a native capture.

- Mesh scale/orientation and text wrapping match explicit props in fake observations and
  native geometry.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](17-world-rendering-primitives.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Inspect overlapping glyph/sprite regions, change text width/alignment, and verify
ordering with native rendered evidence.
