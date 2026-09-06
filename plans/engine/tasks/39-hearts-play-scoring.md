# 39. Complete Hearts card play, trick collection, and scoring

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [Execution contract](../execution.md)
- [Presentation contract](../presentation.md)
- [Motion contract](../motion.md)

**Prerequisite:** [Task 38: Connect Hearts passing prompts and simultaneous
transfers](38-hearts-passing.md) and all its required follow-ups must be
integrated.

**Source roles:** Hearts fixed rules and layouts; passing UI; required gate API.
Resolve these through source-map.md; its links track the current owner after
crate moves. Inspect the concrete caller and host/fake counterpart before
editing.

## Result

A complete Hearts match is playable with a simple legal AI policy and ordered
presentation of every trick and score change.

## Implementation

1. Dispatch from the accepted phase: ResolvePassing for PassingDue, one
   PlayTurn for Playing, and no action for MatchComplete. Connect legal-card
   selection and Play confirmation to the resulting human
   prompt answer, not a second action dispatch. Keep illegal cards inspectable
   but reject submission with public feedback.

2. Register card-play, full-trick hold, collection, score update, and next-hand
   deal sequences against their semantic checkpoint occurrences.

3. Require a finite readable full-trick presentation before collection, using a
   600 ms default hold captured in game presentation configuration. Acceptance
   remains after required collection/scoring/deal presentation.

4. Show hand totals, match totals, moon outcome, next-hand passing direction,
   and shared-win match results in native UI.

5. Route AI-owned prompts to automatic answers only after their snapshot is
   presented. Keep action dispatch at accepted boundaries and gameplay input
   gated while earlier presentation is required. Menus remain usable.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Replace the simple AI policy in task 40. Drag/full navigation and persistence
are tasks 41-43.

## Manual QA

Play one complete hand, inspect the last trick before collection, then run
moon/tied-result fixtures and verify the next-hand or match-end UI.
