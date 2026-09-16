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
   submit ordinary FlatBuffers batches. Release snapshot capacity when consumed;
   apply [bounded downstream admission](../presentation.md#bound-downstream-admission)
   before consuming further snapshots. No second host-acknowledged tree is needed.

2. Preserve the current `AfterEarlierBlockingWork` gameplay ordering across responses
   and separate asset preparation. Make independent menu/control classification explicit
   so ordinary delivery does not overwrite it. Use existing asset loading/admission. Empty output
   needs no command or frame wait.

3. Use existing tween operations and a simple UI fixture. Submit final output before
   accepting rules state, including empty output. Establish independent app/menu
   delivery using the existing batch categories. Input-target replacement belongs to
   12b.

4. Implement game-scoped cancellation in the existing scheduler as specified in
   [presentation](../presentation.md#rules-completion-and-failure), covering queued
   unstarted batches and active operations while preserving unrelated app/menu work.
   Handle existing binary failure messages and release retained buffers on every exit.

## Acceptance

- Rust renders/submits play, draw, and energy while Unity is paused. No Unity
  success/frame notification is required for consumer or worker progress.

- Resuming Unity preserves blocking order across batches while particles run.

- No-change output finishes locally; `TimeWait` supplies explicit pacing.

- Saturate a controlled downstream budget: final state remains Busy/unaccepted,
  further consumption stops, and the 32-slot FIFO backpressures builders. Free capacity
  and verify ordered exactly-once continuation. Oversized single output reports failure.

- Menus, resume, and cancellation remain deliverable at saturation. Stop/replacement
  cancels unstarted and running game work, releases its buffers, and preserves app/menu
  state and operations. No per-snapshot host completion handshake is introduced.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](12-command-queue-integration.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Hold Unity, submit play/draw/energy, resume, and verify queue order and final rules
acceptance independently.
