# 29. Build the reusable presentation inspector

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Fixtures contract](../fixtures.md)
- [Motion contract](../motion.md)
- [Presentation contract](../presentation.md)
- [Execution contract](../execution.md)

**Prerequisite:** [Task 28: Implement safe seek, resume, and explicit
presentation replay](28-inspection-replay.md) and all its required follow-ups
must be integrated.

**Source roles:** Public display observations; Reactant UI; playback inspection;
performance counters. Resolve these through source-map.md; its links track the
current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

A developer can inspect the unified runtime through a reusable UI without
leaking game-private state or changing live rules.

## Implementation

1. Add a hidden-by-default inspector mount with checkpoint/prompt,
   UUID/incarnation, layout targets/current transforms, property owners,
   playbacks/labels/effects, preparation/commit/frame identity, and worker
   lifecycle panels.

2. Connect pause, speed, one-frame advance, supported seek, and explicit replay
   to the public inspection API. Keep live gameplay gates isolated.

3. Expose timing/allocation/publication counters with opt-in expensive
   observations; normal animation must not stream every transform to Rust.

4. Add stable specimen selection/reset controls to the existing Reactant
   laboratory. Label later unavailable specimens explicitly and make selection
   deterministic.

5. Distinguish game-visible snapshot fields from fixture-only diagnostics; the
   normal Hearts inspector may not reveal opponent hands.

## Acceptance

- The inspector explains which required contribution is delaying a checkpoint
  and which generation owns a property.

- Pause/step/replay controls obey task 28's gate/occurrence isolation and show
  unavailable seek controls honestly.

- Reset abandons old runs, releases fixture resources, and starts one clean
  specimen.

- Disabling optional geometry observations eliminates their per-frame reporting.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Hearts content starts at task 35. Additional specimen coverage is added by tasks
44-45 rather than fabricated now.

## Manual QA

Use the inspector alone to diagnose delayed preparation, a pending prompt, a
retargeted movement, and an abandoned worker.
