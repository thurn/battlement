# 43. Implement durable accepted-boundary Hearts saves

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [Execution contract](../execution.md)
- [Presentation contract](../presentation.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 42: Finish Hearts keyboard/controller navigation and
menus](42-hearts-navigation-menus.md) and all its required follow-ups must be
integrated.

**Source roles:** Accepted-state notifications; generic persistent-data host
support; chess save integration; Hearts state. Resolve these through
source-map.md; its links track the current owner after crate moves. Inspect the
concrete caller and host/fake counterpart before editing.

## Result

Hearts resumes the latest durably saved accepted boundary across native and
WebGL sessions without replaying old effects.

## Implementation

1. Serialize complete accepted Hearts state, including the initial NewGame deal,
   PRNG, and passing-cycle state, outside the worker. Use one serial writer with
   match epoch/accepted-state sequence; supersede queued older-match writes and
   prevent stale completions from acknowledging a newer save. Never serialize
   mutable worker state.

2. Use atomic temporary-write/replace on native. Add or reuse a generic browser
   durable-storage flush/acknowledgement capability so a WebGL save is not
   reported complete while only in memory.

3. Enable Continue only for a valid persisted state; restore the visible
   position without transient replay and then issue fresh human prompt identity
   or schedule the next AI turn.

4. Show nonblocking save failure with retry; keep accepted in-memory state
   usable. Normal Exit asynchronously flushes the newest accepted state and
   offers retry/exit-without-saving on failure. Handle corrupted data with an
   explanation/New Game choice before replacing it.

5. Capture pending-save and durable-save observations through public fixture
   services for deterministic tests.

## Acceptance

- Autosave occurs after the initial accepted deal, accepted passing, and each
  accepted card action, never while required final presentation is pending.

- New Game followed by normal Exit/reload during its first passing prompt
  restores that new deal, including when an older match had a save/write
  pending. Forced termination before durable acknowledgement restores only the
  last durable state.

- Reload after a durable acknowledgement restores the same deal, scores, turn,
  and future seeded behavior on native and WebGL.

- Exiting during a later action restores the previous accepted save; old
  request/effect identities are not replayed.

- Restoration into PassingDue starts a fresh passing request; restoration into
  Playing starts the correct seat, and MatchComplete remains on results.

- Write/flush failure preserves in-memory play and surfaces retry; corrupt saves
  are not silently overwritten.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

No save-format backward-compatibility scheme or cloud accounts. Physical-device
durability checks join separate certification.

## Manual QA

Save, close/reopen, resume into both human and AI turns, then repeat with an
injected write/flush failure and a corrupt save.
