# 12. Connect snapshot rendering to the existing command queue

Render queued snapshots ahead of playback and append ordinary ordered batches.
Unity's existing queue determines when their commands execute.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Presentation](../presentation.md)
- [Rules and choices](../execution.md)
- [Rules and session API](../interfaces.md)

**Prerequisite:** [Task 11: Start game sessions, accept actions, and expose
recovery](11-accepted-action-runtime.md) and all its required follow-ups must be
integrated.

**Starting code:** Reactant commit/app delivery; `BatchStart`; Unity's batch
scheduler and operation registry; existing input properties; world/UI fakes.

## Example

```text
Unity paused on play animation
Rust renders draw snapshot and energy snapshot, submits both batches
Unity resumes: finishes play, then draw, then updates energy
```

## Implementation

1. Consume snapshot entries in order, reconcile against the last rendered tree,
   and submit ordinary batches. Release snapshot capacity when consumed; do not
   wait for Unity or retain a second host-acknowledged tree.
2. Preserve `AfterEarlierBlockingWork` on gameplay batches across responses.
   Update app delivery so asset preparation does not overwrite that dependency.
   Use existing asset loading and admission behavior; local menu batches remain
   independent. Empty output needs no command or frame wait.
3. Order request-specific prompt controls and gameplay input changes with the
   game commands. Use distinct existing object IDs for different prompt targets
   and request-bound response handles. Old visible targets must not invoke new
   handlers. Cover pointer/key/controller paths and delayed delivery.
4. Route rerender commands affecting gameplay behind earlier game commands;
   keep independent menus and host hover offsets responsive. Test a settings
   update while multiple future snapshots are already submitted.
5. Stop/replacement cancels queued and running work through existing host
   cleanup. Host failure reports recovery without reverting accepted rules
   state. Reuse session/batch/command identity and failure messages.

## Acceptance

- Rust renders/submits play, draw, and energy while Unity is paused. No Unity
  success/frame notification is required for consumer or worker progress.
- Resuming Unity preserves blocking order across batches while particles run.
- Prompt controls become usable in command order; stale controls cannot answer
  a newer request. Local menus do not reveal queued future gameplay state.
- No-change output finishes locally; `TimeWait` supplies explicit pacing.
- Stop/restart removes old queued commands and effects. A host failure after
  acceptance leaves accepted state available for reconstruction.

Run public scenarios, native queue/input checks, and staged aggregate CI from
[validation](../validation.md).

## Scope of this task

Use existing tween operations for this slice. Tasks 21-22 adapt Motion to the
same command operation interface; task 25 generates layout/semantic animation
commands. No scheduler, batch-completion notification, or frame protocol is added.

## Manual QA

Pause host playback, dispatch play/draw/energy, and inspect queued work. Resume
and observe ordering. Open a menu, change a setting, test an old prompt target,
then stop/restart with commands still queued.
