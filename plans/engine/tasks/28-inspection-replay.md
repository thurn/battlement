# 28. Implement safe seek, resume, and explicit presentation replay

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Motion contract](../motion.md)
- [Presentation contract](../presentation.md)
- [Fixtures contract](../fixtures.md)

**Prerequisite:** [Task 27: Retain exits, anchors, and effects after logical
unmount](27-effect-exit-retention.md) and all its required follow-ups must be
integrated.

**Source roles:** Playback controls; occurrence journal; checkpoint gate
evaluator; retained visual ownership. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

Developers can inspect supported visual tracks and replay an occurrence without
changing game state or live checkpoint progress.

## Implementation

1. Add capability-reported seeking for supported visual tracks and preserve the
   delivered sound/burst journal when sampling earlier/later times.

2. Resume live playback with only undelivered occurrences eligible. Save/restore
   the live inspected playback state so inspection cannot masquerade as normal
   timeline advancement.

3. Allocate a session-unique replay identity in a separate namespace; preserve
   slots within one replay and allocate a new ID for another replay.

4. Reject inspection/replay events in the live gate evaluator. Keep replay
   resource leases independent from live/exit owners.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

User-facing developer controls are task 29; test the public playback/inspection
API here.

## Manual QA

Pause before a reveal sound, seek around it, resume, then replay twice. Observe
occurrence counts and an unchanged live checkpoint gate.
