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

Test rendering ahead, native queue order, and session cancellation:

```text
two blocking card moves + nonblocking particles -> wait for both cards
Unity paused; Rust finishes                    -> later commands queued, state accepted
stop/restart with queued old commands           -> replacement stays unchanged
inspect/replay another playback                -> live batch stays paused
```

## Implementation

1. Complete motion-equivalence, draw-reflow, material-effects, attached-effects,
   occurrence-replay, prompt-cycle, cancellation, asset-loading, snapshot-queue,
   and save-failure test scenes.

2. Exercise material dissolve/reverse, separate text fade, independent
   overrides, persistent auras, projectile/trail retention, live/captured
   anchors, and simultaneous sound/burst labels.

3. Add controlled publication/builder/prompt/answer/completion races and bounded
   ordinary computation released to a helper/return boundary. Verify immediate
   public Stopped and later worker-stopped after fixture-owned drops, without
   pretending computation was forcibly interrupted.

4. Cover delayed/failed asset commands, missing targets, batch redelivery,
   rerenders without replay, invalid graphs, blocking failure, no-change entries,
   explicit waits, local UI during animation, and old-session messages. Pause
   Unity while rules finish, then inject a host failure without reversing state.

5. Demonstrate typed Rust effect fallback plus optional RON values and
   captured-versus-current configuration behavior.

## Acceptance

- All named cancellation positions/races terminate cleanly and cannot alter a
  replacement display; real panic remains distinguishable.

- Sound/burst occurrences deduplicate through retry/delivery and remain isolated
  across inspection/replay.

- Blocking/nonblocking commands, in-place retargeting, asset dependencies, and
  resource release match their shared contracts.

- Native captures prove shader/text/particle behavior that fake observations
  cannot establish.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Sustained performance evidence is task 46. No numerical target can replace these
correctness scenarios.

## Manual QA

Use the inspector to walk through delayed asset loading, draw reflow,
dissolve/recreate, seek/replay, prompt cancellation, and required-track failure.
