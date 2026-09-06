# 28. Implement safe seek, resume, and explicit presentation replay

Developers can inspect supported visual tracks and replay an occurrence without
changing game state or live checkpoint progress.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [Presentation timing](../presentation.md)
- [Test scenes](../fixtures.md)

**Prerequisite:** [Task 27: Retain exits, anchors, and effects after logical
unmount](27-effect-exit-retention.md) and all its required follow-ups must be
integrated.

**Starting code:** Playback controls; delivered-effect history; checkpoint gate
evaluator; retained visual ownership.

## Example

Seeking and explicit replay must produce different sound behavior:

```text
sound already played -> seek before its label -> resume: no second sound
explicit replay: one new sound
repeat delivery of that replay: no duplicate sound
none of these operations advances a live game checkpoint
```

## Implementation

1. Add capability-reported seeking for supported visual tracks and preserve the
   delivered sound/burst history when sampling earlier/later times.

2. Resume live playback with only undelivered occurrences eligible. Save/restore
   the live inspected playback state so inspection cannot masquerade as normal
   timeline advancement.

3. Allocate a session-unique replay identity in a separate namespace; preserve
   effect names within one replay and allocate a new ID for another replay.

4. Reject inspection/replay events in the live gate evaluator. Keep replay
   resource references independent from live/exit owners.

5. Expose unsupported native seek capabilities explicitly rather than resetting
   particles and claiming equivalent replay.

## Acceptance

- Seeking backward/forward across a delivered sound does not play it again;
  resuming emits a future undelivered sound once.

- Two explicit replays each play their occurrences once, while repeated delivery
  within one replay is deduplicated.

- Seeking past ready or replaying a completion cannot advance a live checkpoint
  or accept/save a game action.

- Stopping replay releases only its retained resources.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

User-facing developer controls are task 29; test the public playback/inspection
API here.

## Manual QA

Pause before a reveal sound, seek around it, resume, then replay twice. Observe
occurrence counts and an unchanged live checkpoint gate.
