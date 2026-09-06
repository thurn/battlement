# 43. Save Hearts explicitly and resume accepted state

An explicit Save captures an accepted state and durably restores it on native
and WebGL. V1 does not autosave or save implicitly on exit.

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

**Starting code:** GameHandle::accepted_state and explicit menu actions; generic
persistent-data host support; chess save integration; Hearts state.

## Example

Saving during animation captures the last completed action:

```text
accepted state 7; action 8 is still being displayed
Save -> capture an owned copy of state 7
write and durable flush complete -> report state 7 saved
reload -> restore state 7 with fresh prompt/effect identities
```

## Implementation

1. Add explicit Save to the menu. Capture game.accepted_state outside rules and
   serialize the complete state, including initial-deal, PRNG, and passing-cycle
   data. A busy session returns the previous accepted boundary. Never save
   mutable worker state or subscribe to acceptance for autosave.

2. Use one explicit write at a time and disable duplicate Save while pending.
   Native writes use temporary-write/atomic replace where supported. WebGL must
   acknowledge durable storage flush. Bind success/failure to that captured
   copy; later actions or New Game do not change what the write contains.

3. Continue loads valid state through App::start_game with a fresh context.
   Restore entry visuals without old transient replay, then schedule passing,
   play, or results from the accepted phase after entry presentation. Old
   response handles cannot act on restored prompts.

4. Keep New Game and completed actions from writing automatically. Exit does not
   capture a newer state. If an explicit write is already pending, normal Exit
   waits for it with retry/exit-without-finishing on failure. Forced exit
   restores the last durably acknowledged explicit save.

5. Expose write/flush failure and retry without invalidating in-memory play.
   Explain corrupt saves and offer New Game without silent overwrite. Use public
   fixture save services to observe exact captured/durable state.

## Acceptance

- No save occurs at startup, acceptance, New Game, or Exit without explicit
  Save. The initial deal and previous accepted state while busy can be saved.

- A new action completing during a write cannot be reported as saved by that
  earlier write. Duplicate Save cannot start concurrent writes.

- Native and WebGL reload the exact acknowledged deal, scores, phase, turn, and
  future seeded behavior. Fresh requests do not replay old effects.

- Write/flush failure preserves the prior durable save and playable state.
  Corrupt data is explained, and New Game alone does not overwrite it.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

No acceptance subscription, autosave, save-format migration framework, or cloud
accounts. Physical-device durability checks remain separate certification.

## Manual QA

Explicitly save at initial entry, during a human choice, and during trick
collection. Reload each captured boundary. Repeat with write/flush failure and
corrupt data; verify ordinary actions and Exit do not start an automatic save.
