# 12b. Bind queued prompt controls and independent menu updates

[Task group 12](12-command-queue-integration.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Presentation](../presentation.md)
- [Rules and choices](../execution.md)
- [Rules and session API](../interfaces.md)

**Prerequisite:** [12a: Submit snapshot output through ordered gameplay
batches](12a-snapshot-batches.md) is integrated.

**Starting code:** Reactant commit/app delivery; `BatchStart`; Unity's batch scheduler
and operation registry; existing input properties; world/UI fakes.

## Implementation

1. Order request-specific prompt controls and gameplay input changes with the game
   commands. Use distinct existing object IDs for different prompt targets and
   request-bound response handles. Old visible targets must not invoke new handlers.
   Cover pointer/key/controller paths and delayed delivery.

2. Route rerender commands affecting gameplay behind earlier game commands; keep
   independent menus and host hover offsets responsive. Test a settings update while
   multiple future snapshots are already submitted.

3. Stop/replacement cancels queued and running work through existing host cleanup. Host
   failure reports recovery without reverting accepted rules state. Reuse
   session/batch/command identity and failure messages.

## Acceptance

- Prompt controls become usable in command order; stale controls cannot answer a newer
  request. Local menus do not reveal queued future gameplay state.

- Stop/restart removes old queued commands and effects. A host failure after acceptance
  leaves accepted state available for reconstruction.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](12-command-queue-integration.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Open a menu with future snapshots queued, invoke an old prompt target, then stop/restart
and verify replacement state remains unchanged.
