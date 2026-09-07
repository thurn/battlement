# 25. Generate blocking and nonblocking animation commands from snapshots

A tree change supplies ordinary movement. A `StateAnimation` can select a custom
sequence. Both run through Battlement's existing command queue.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Presentation](../presentation.md)
- [Animation](../motion.md)
- [Rules and choices](../execution.md)

**Prerequisite:** [Task 24: Animate layout movement with continuous
retargeting](24-layout-movement-projection.md) is integrated.

**Starting code:** Task 12's snapshot-to-batch consumer; Motion command-operation
adapters; layout movement policies; Reactant event hooks and scoped controls.

## Example

```text
CardPlayed -> tree moves card to table -> blocking default movement
CardDrawn  -> custom reveal/flip/arrive -> blocking sequence
EnergyGained -> tree updates label     -> immediate command
particle trail                       -> nonblocking operation
```

## Implementation

1. Let components interpret each queued snapshot's typed `StateAnimation` using
   scoped animation controls and declared refs. This task owns the minimal
   facade authoring API that exposes the typed event during its consuming render;
   the rules/session surface stays as defined in interfaces.md. Collect commands with
   that render and consume the event once at submission. A local rerender must
   not start it again. No playback reservation/commit handshake is needed.
2. Emit default layout movement as blocking commands. A custom sequence owns
   its selected properties instead of also running default movement on them.
   Preserve unrelated parallel movement and nonblocking decoration.
3. Preserve command blocking flags through Reactant lowering. Connect the
   operation adapters from tasks 21-22 to the existing scheduler. Rust renders
   ahead; do not collect completion labels or add completion notifications.
4. Retarget active layout movement in place. Queue conflicting gameplay
   sequences after the active one; local hover/drag cannot cancel its blocking
   placement. Use existing replacement controls for cosmetic work.
5. Support authored finite waits through `TimeWait`. Finish equal-pose/no-change
   entries without a mandatory frame. Prompt controls follow earlier commands;
   final rules acceptance depends only on Rust consuming final publication.

## Acceptance

- Play, draw, and energy updates occur in order; rerendering repeats no event.
- Two parallel blocking movements both finish before the next entry's commands execute; particles
  can keep playing and menus respond throughout.
- Custom movement suppresses default movement only on its owned properties.
- Reflow delays completion until arrival at the new target without a jump.
- No-change entries require no frame receipt; explicit waits preserve pacing.
- Host failure preserves the latest accepted state even if Unity is behind;
  stopped-session reports cannot affect the replacement. Cosmetic stop is local.

Run the public scenarios, native queue checks, and staged aggregate CI specified
in [validation](../validation.md).

## Scope of this task

Task 26 adds sequence sound/burst entries and task 28 gameplay pause. They use
existing command/playback identities, not a parallel presentation protocol.

## Manual QA

Hold Unity while Rust renders play/draw/energy and accepts the action. Resume
playback with a particle loop running; check command order and prompt controls.
Resize during a draw and verify future state cannot jump ahead of queued work.
