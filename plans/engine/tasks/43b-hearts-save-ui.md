# 43b. Connect explicit Hearts Save and Continue

[Task group 43](43-hearts-save-resume.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [43a: Provide native and WebGL durable save
operations](43a-durable-save-service.md) is integrated.

**Starting code:** GameHandle::accepted_state and explicit menu actions; generic
persistent-data host support; chess save integration; Hearts state.

## Implementation

1. Add explicit Save to the menu. Capture game.accepted_state outside rules and
   serialize the complete state, including initial-deal, PRNG, and passing-cycle data. A
   busy session returns the previous accepted boundary. Never save mutable worker state
   or subscribe to acceptance for autosave.

2. Use one explicit write at a time and disable duplicate Save while pending. Native
   writes use temporary-write/atomic replace where supported. WebGL must acknowledge
   durable storage flush. Bind success/failure to that captured copy; later actions or
   New Game do not change what the write contains.

3. Continue loads valid state through App::start_game with a fresh context. Restore
   entry visuals without old transient replay, then schedule passing, play, or results
   from the accepted phase after submitting entry rendering. Old response handles cannot
   act on restored prompts.

4. Keep New Game and completed actions from writing automatically. Exit does not capture
   a newer state. If an explicit write is already pending, normal Exit waits for it with
   retry/exit-without-finishing on failure. Forced exit restores the latest complete durable
   save, including a pending write that completed before its success message
   appeared. Never load a partial write; no visible-acknowledgement journal is
   needed.

5. Expose write/flush failure and retry without invalidating in-memory play. Explain
   corrupt saves and offer New Game without silent overwrite. Use public fixture save
   services to observe exact captured/durable state.

## Acceptance

- No save occurs at startup, acceptance, New Game, or Exit without explicit Save. The
  initial deal and previous accepted state while rules are busy can be saved. While
  Unity is paused, saving a later accepted action and reloading restores that logical
  result without replaying pending commands.

- A new action completing during a write cannot be reported as saved by that earlier
  write. Duplicate Save cannot start concurrent writes.

- Native and WebGL reload the exact acknowledged deal, scores, phase, turn, and future
  seeded behavior. Fresh requests do not replay old effects.

- Write/flush failure preserves the prior durable save and playable state. Corrupt data
  is explained, and New Game alone does not overwrite it.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](43-hearts-save-resume.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Save during a human prompt and paused trick hold; reload the accepted boundary, retry
failure, and confirm New Game/Exit create no implicit save.
