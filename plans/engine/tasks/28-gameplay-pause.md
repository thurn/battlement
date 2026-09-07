# 28. Pause gameplay presentation while workers continue

A menu can freeze visible gameplay without blocking rules execution or local UI.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Validation](../validation.md)

## Read before implementing

- [Pause contract](../motion.md#pause-gameplay-presentation-without-stopping-rules)
- [Queued presentation](../presentation.md)

**Prerequisite:** [Task 27](27-effect-exit-retention.md) is integrated.

**Starting code:** Existing Motion playback controls, batch scheduler operations,
TimeWait, independent UI delivery, and task 12's game command classification.

## Implementation

1. Expose app-owned game-presentation pause/resume without extending Game or its
   rules/session API. Reuse existing playback clocks and operation controls.
2. Pause active gameplay animations and finite waits; prevent later gameplay
   commands from executing. Pause control must bypass the blocked gameplay
   queue. Continue asset preparation, workers, snapshot consumption, and local
   menu commands. The logical accepted state may advance while visible state is
   frozen. Ordinary queue limits still apply.
3. Resume from the paused time with undelivered occurrences still pending; do
   not replay sounds/bursts or charge paused wall time. Do not override an
   independently paused Motion track when the game clock resumes. Apply existing native particle/audio pause
   controls where supported; already emitted sound is not undone.
4. Keep stop/replacement effective while paused. A late menu close from the old session
   cannot resume or otherwise affect a replacement presentation.

## Acceptance

- Pause a moving card and a finite wait; poses and remaining presentation time
  remain fixed while a worker completes and submits subsequent output.
- A menu opens, animates, and accepts input during pause. Resume preserves command
  order and emits each pending transient once.
- Stop while paused cancels queued/running work and permits a responsive new game.

Reuse the queue/occurrence fixtures and add one focused native pause scenario;
run affected checks and staged aggregate CI from [validation](../validation.md).

## Scope of this task

No developer seek/replay, scene copy, or timeline editor. Hearts menu integration
is task 42; this task supplies the reusable presentation control.

## Manual QA

Pause during movement and a trick hold, operate a local menu, resume, then stop
while paused. Verify the worker remained independent of presentation time.
