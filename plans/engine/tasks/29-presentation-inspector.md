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

**Prerequisite:** [Task 28: Pause gameplay presentation while workers continue](28-gameplay-pause.md) is integrated.

**Starting code:** Public display observations; Reactant UI; playback status; existing native diagnostics.

## Example

The inspector should answer why gameplay is waiting in plain terms:

```text
latest rendered snapshot: energy increased
Unity command queue: waiting for card A to arrive
latest target: hand position updated after resize
worker: action complete; Unity still playing queued commands
```

## Implementation

1. Add a hidden-by-default diagnostic panel using existing observations: safe
   current prompt, worker status, queued/blocking work, and a selected object's
   UUID, visibility, layout target, and displayed position. Label Rust state as
   potentially ahead of playback. Game code supplies safe labels; do not dump
   full states or hidden Hearts hands.
2. Reuse the laboratory's existing selector/reset controls. Reset through the
   ordinary fixture entrypoint and show unavailable scenes honestly.
3. Fetch detailed poses only for the selected object while the panel is open.
   Disable optional per-frame reporting when it closes.

## Acceptance

- The panel explains a blocked move and a waiting worker without revealing game
  secrets or changing rules/presentation progress.
- Reset abandons the old run and releases its resources through existing cleanup.
- Closing the panel removes optional per-frame pose reporting.

Reuse the relevant scene observations and obtain one native panel capture; run
staged aggregate CI from [validation](../validation.md).

## Scope of this task

No generic resource browser, timeline editor, scene cloning, seek/replay UI, or
allocation dashboard. Performance instrumentation belongs to task group 46.
Hearts' enlarged card inspection remains ordinary game UI.

## Manual QA

Use the inspector alone to diagnose delayed asset loading, a pending prompt, a
retargeted movement, and an abandoned worker.
