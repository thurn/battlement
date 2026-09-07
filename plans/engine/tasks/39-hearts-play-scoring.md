# 39. Complete Hearts card play, trick collection, and scoring

A complete Hearts match is playable with a simple legal AI policy and ordered
presentation of every trick and score change.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Animation](../motion.md)

**Prerequisite:** [Task 38: Connect Hearts passing prompts and simultaneous
transfers](38-hearts-passing.md) and all its required follow-ups must be
integrated.

**Starting code:** Hearts fixed rules and layouts; passing UI; required gate
API.

## Example

Choose the next action from the accepted phase, including after a new deal:

```rust
match accepted.phase {
    Phase::PassingDue => dispatch(Action::ResolvePassing),
    Phase::Playing => dispatch(Action::PlayTurn),
    Phase::MatchComplete => show_results(),
}
```

## Implementation

1. Use status-driven application scheduling when Ready, dispatching from the
   accepted phase: ResolvePassing for PassingDue, one PlayTurn for Playing, and
   no action for MatchComplete. Connect legal-card selection and Play
   confirmation to the resulting human prompt answer, not a second action
   dispatch. Keep illegal cards inspectable but disable illegal submission using
   prompt validation. Fault-injected active invalid replies panic.

2. Register card-play, full-trick hold, collection, score update, and next-hand
   deal sequences against their semantic checkpoint occurrences.

3. Require a finite readable full-trick presentation before collection, using a
   600 ms default hold captured in game presentation configuration. Acceptance
   remains after required collection/scoring/deal presentation.

4. Show hand totals, match totals, moon outcome, next-hand passing direction,
   and shared-win match results in native UI.

5. Route AI-owned prompts through the context to choose_with_policy immediately
   after enqueueing. Keep consecutive AI plays inside PlayTurn rather than
   dispatching once per AI card. Only a full 32-slot queue blocks their
   progress. UI dispatch stays at accepted boundaries; human prompt replies wait
   for their displayed snapshot. Menus remain usable.

## Acceptance

- A four-card trick is visibly complete before collection; the correct winner
  leads next.

- Score updates do not appear before their checkpoint; the last card's action
  includes trick/hand results before acceptance.

- Explicit fixtures cover moon scoring, next hand, reaching 100, and tied
  winners through the displayed result surface.

- Consecutive hands route through left/right/across/hold passing correctly; a
  last-card action dealing a pass-due hand cannot skip directly to card play.

- A complete scripted match uses the same rules path as interactive play with no
  direct state mutation by view code.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Replace the simple AI policy in task 40. Drag/full navigation and persistence
are tasks 41-43.

## Manual QA

Play one complete hand, inspect the last trick before collection, then run
moon/tied-result fixtures and verify the next-hand or match-end UI.
