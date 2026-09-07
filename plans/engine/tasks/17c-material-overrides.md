# 17c. Add typed per-instance material overrides

[Task group 17](17-world-rendering-primitives.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Presentation timing](../presentation.md)
- [Identity and state](../identity.md)

**Prerequisite:** [17b: Integrate rich world text and mixed visual
ordering](17b-world-text-sorting.md) is integrated.

**Starting code:** World adapter; object protocol/builders; Unity world creation;
prepared assets; fake asset catalog.

## Implementation

1. Validate typed material parameter declarations against prepared assets and apply
   per-instance overrides without mutating the shared material. Expose these properties
   to later Motion adapters.

2. Create an early composed-card test scene with two cards using one shared material but
   different static parameters and hit-independent visual geometry.

## Acceptance

- Changing one card's material parameter does not change the other card or the source
  material.

- Required typed parameters are checked against prepared assets. Wrong/missing
  declarations fail before dependent playback; two cards sharing a material retain
  independent static values.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](17-world-rendering-primitives.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Change one card's material value and verify the other card and shared source remain
unchanged.
