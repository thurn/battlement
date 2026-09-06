# 43. Save completed Hearts actions durably and resume them

Hearts resumes the latest durably saved accepted boundary across native and
WebGL sessions without replaying old effects.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 42: Finish Hearts keyboard/controller navigation and
menus](42-hearts-navigation-menus.md) and all its required follow-ups must be
integrated.

**Starting code:** Accepted-state notifications; generic persistent-data host
support; chess save integration; Hearts state.

## Example

Save completions must identify the exact state that reached durable storage:

```text
accepted state 8 queued; write for state 7 completes
state 8 is still not marked saved
state 8 write and durable flush finish
reload now restores state 8, with fresh prompt and effect identities
```

## Implementation

1. Serialize complete accepted Hearts state, including the initial NewGame deal,
   PRNG, and passing-cycle state, outside the worker. Use one serial writer with
   match ID and accepted-state sequence number; supersede queued older-match
   writes and prevent stale completions from acknowledging a newer save. Never
   serialize mutable worker state.

2. Use atomic temporary-write/replace on native. Add or reuse a generic browser
   durable-storage flush/acknowledgement capability so a WebGL save is not
   reported complete while only in memory.

3. Enable Continue only for a valid persisted state; restore the visible
   position without transient replay and then schedule from the accepted phase:
   passing, card play for the correct seat, or completed-match results. Every
   new prompt gets a fresh identity.

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

No save-format backward-compatibility scheme or cloud accounts. Physical-device
durability checks join separate certification.

## Manual QA

Save, close/reopen, resume into both human and AI turns, then repeat with an
injected write/flush failure and a corrupt save.
