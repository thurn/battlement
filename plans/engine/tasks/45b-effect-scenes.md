# 45b. Complete native effect and pause demonstrations

[Task group 45](45-effects-failures-laboratory.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Animation](../motion.md)
- [Presentation timing](../presentation.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [45a: Close publication, cancellation, and recovery coverage
gaps](45a-failure-scenes.md) is integrated.

**Starting code:** Laboratory/inspector; effects/retention/pause; worker barriers;
generic failure surfaces.

## Implementation

1. Reuse motion-equivalence, draw-reflow, material-effects, attached-effects, and
   occurrence-delivery. Fill demonstrated gaps and make their controls/captions/reset
   usable; do not reproduce the driver's full permutation matrix.

2. Exercise material dissolve/reverse, separate text fade, independent overrides,
   persistent auras, projectile/trail retention, live/captured anchors, and simultaneous
   sound/burst labels.

3. Demonstrate typed Rust effect fallback plus optional RON values and
   captured-versus-current configuration behavior.

## Acceptance

- Sound/burst occurrences deduplicate through retry/delivery and remain isolated across
  pause/resume and session replacement.

- Native captures prove shader/text/particle behavior that fake observations cannot
  establish.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](45-effects-failures-laboratory.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Observe dissolve/text fade/attached effects, pause/resume, and native occurrence
delivery; reset and verify cleanup.
