# 29. Build the reusable presentation inspector

A developer can inspect the unified runtime through a reusable UI without
leaking game-private state or changing live rules.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Animation](../motion.md)
- [Presentation timing](../presentation.md)
- [Rules and choices](../execution.md)

**Prerequisite:** [Task 28: Implement safe seek, resume, and explicit
presentation replay](28-inspection-replay.md) and all its required follow-ups
must be integrated.

**Starting code:** Public display observations; Reactant UI; playback
inspection; performance counters.

## Example

The inspector should answer why gameplay is waiting in plain terms:

```text
checkpoint 12: visible
waiting for card A: draw sequence, label "ready"
latest target: hand position updated after resize
worker: waiting to publish next checkpoint
```

## Implementation

1. Add a hidden-by-default inspector mount with checkpoint/prompt,
   UUID/incarnation, layout targets/current transforms, property owners,
   playbacks/labels/effects, preparation/commit/frame identity, and worker
   lifecycle panels.

2. Connect pause, speed, one-frame advance, supported seek, and explicit replay
   to the public inspection API. Keep live gameplay gates isolated.

3. Expose timing/allocation/publication counters with opt-in expensive
   observations; normal animation must not stream every transform to Rust.

4. Add stable test scene selection/reset controls to the existing Reactant
   laboratory. Label later unavailable test scenes explicitly and make selection
   deterministic.

5. Distinguish game-visible view fields from fixture-only diagnostics; the
   normal Hearts inspector may not reveal opponent hands.

## Acceptance

- The inspector explains which required contribution is delaying a checkpoint
  and which generation owns a property.

- Pause/step/replay controls obey task 28's gate/occurrence isolation and show
  unavailable seek controls honestly.

- Reset abandons old runs, releases fixture resources, and starts one clean test
  scene.

- Disabling optional geometry observations eliminates their per-frame reporting.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Hearts content starts at task 35. Additional test scene coverage is added by
tasks 44-45 rather than fabricated now.

## Manual QA

Use the inspector alone to diagnose delayed preparation, a pending prompt, a
retargeted movement, and an abandoned worker.
