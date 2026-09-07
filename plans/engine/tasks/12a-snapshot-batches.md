# 12a. Submit snapshot output through ordered gameplay batches

[Task group 12](12-command-queue-integration.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Presentation](../presentation.md)
- [Rules and choices](../execution.md)
- [Rules and session API](../interfaces.md)

**Prerequisite:** [Task 11: Start game sessions, accept actions, and expose
recovery](11-accepted-action-runtime.md) is integrated.

**Starting code:** Reactant commit/app delivery; `BatchStart`; Unity's batch scheduler
and operation registry; existing input properties; world/UI fakes.

## Implementation

1. Consume snapshot entries in order, reconcile against the last rendered tree, and
   submit ordinary batches. Release snapshot capacity when consumed; do not wait for
   Unity or retain a second host-acknowledged tree.

2. Preserve `AfterEarlierBlockingWork` on gameplay batches across responses. Update app
   delivery so asset preparation does not overwrite that dependency. Use existing asset
   loading and admission behavior; local menu batches remain independent. Empty output
   needs no command or frame wait.

3. Use existing tween operations and a simple UI fixture. Submit final output before
   accepting rules state, including empty output. Establish independent app/menu
   delivery using the existing batch categories. Input-target replacement belongs to
   12b.

4. Keep the session stop/failure behavior from task 11 connected to existing host
   cleanup, so even this first integrated slice can abandon its queued batches.

## Acceptance

- Rust renders/submits play, draw, and energy while Unity is paused. No Unity
  success/frame notification is required for consumer or worker progress.

- Resuming Unity preserves blocking order across batches while particles run.

- No-change output finishes locally; `TimeWait` supplies explicit pacing.

- Stop/replacement cancels the integrated fixture's queued output through existing
  cleanup; no host completion handshake is introduced.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](12-command-queue-integration.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Hold Unity, submit play/draw/energy, resume, and verify queue order and final rules
acceptance independently.
