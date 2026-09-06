# 45. Complete animation, cancellation, and failure test scenes

The test scenes reproduce animation, cancellation, cleanup, and failure behavior
through public scenarios, with Unity evidence for rendered effects.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Animation](../motion.md)
- [Presentation timing](../presentation.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 44: Complete component, layout, and input test
scenes](44-identity-composition-laboratory.md) and all its required follow-ups
must be integrated.

**Starting code:** Laboratory/inspector; effects/retention/replay; worker
barriers; generic failure surfaces.

## Example

Test a completion racing with replacement in both possible acceptance orders:

```text
old completion accepted before replacement -> requirement stays satisfied
replacement accepted first -> wait for successor completion
late old completion -> ignored
seek/replay completion -> never satisfies live gameplay
```

## Implementation

1. Complete motion-equivalence, draw-reflow, material-effects, attached-effects,
   occurrence-replay, prompt-cycle, cancellation, preparation, gate-replacement,
   and save-failure test scenes.

2. Exercise material dissolve/reverse, separate text fade, independent
   overrides, persistent auras, projectile/trail retention, live/captured
   anchors, and simultaneous sound/burst labels.

3. Add controlled publication/builder/prompt/answer/completion races and
   explicit long-computation cancellation points. Observe worker-stopped and
   fixture-owned drops without private channel assertions.

4. Cover superseded asset preparation, missing required targets, duplicate
   effect names, invalid graphs, required failure, early gate labels, and stale
   event generations.

5. Demonstrate typed Rust effect fallback plus optional RON values and
   captured-versus-current configuration behavior.

## Acceptance

- All named cancellation positions/races terminate cleanly and cannot alter a
  replacement display; real panic remains distinguishable.

- Sound/burst occurrences deduplicate through retry/delivery and remain isolated
  across inspection/replay.

- Required/cosmetic ownership, retarget/replacement, asset preparation, and
  resource release match their shared contracts.

- Native captures prove shader/text/particle behavior that fake observations
  cannot establish.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Sustained performance evidence is task 46. No numerical target can replace these
correctness scenarios.

## Manual QA

Use the inspector to walk through delayed preparation, draw reflow,
dissolve/recreate, seek/replay, prompt cancellation, and required-track failure.
