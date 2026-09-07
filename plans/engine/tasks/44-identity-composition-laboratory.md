# 44. Complete component, layout, and input test scenes

The test scenes exercise rich card composition and identity changes that Hearts'
simple card artwork does not demonstrate.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Identity and state](../identity.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 43: Save Hearts explicitly and resume accepted
state](43-hearts-save-resume.md) and all its required follow-ups must be
integrated.

**Starting code:** Reactant laboratory/inspector; world primitives; UI/world
projection; public display driver.

## Example

Exercise richer composition than a playing-card texture:

```text
card: artwork + frame + rich text + badges + outline + action button
open browser: change pile to grid without changing rules location
show simulation preview: independent presentation, live state unchanged
move across parents: preserve hooks, update context and event path
```

## Implementation

1. Complete identity-transfer, duplicate-identity, visibility-transition,
   ui-world-transfer, mixed-input, stores, composed-card, and contained-layout
   test scenes. Also add card-browser, card-selection-scenes, and
   simulation-preview as specified in fixtures.md.

2. Use neutral synthetic rich text, badges, outline visuals, face variants, UI
   preview, nested cards, and conditional action controls. Add a reusable
   generic host capability if a fixture exposes a real engine gap.

3. Generate transfer cases for every configured source/destination layout pair,
   including reflow and removal of the old ancestor. Include active-root and
   portal moves and separate inspection identities.

4. Assert both state/ref retention and changed ancestry/context through public
   fixture output. Validate material isolation and changed hit geometry in
   native captures.

5. Keep test scene reset deterministic and assert cleanup rather than
   accumulating retained objects across selections.

## Acceptance

- Browser/deck-order, draft/shop/quest-deck selection, and simulated outcome
  previews work through shared rules/display APIs. Local rearrangement and
  previews leave live rules state unchanged until a valid choice is committed.

- Every configured layout-pair transfer has public-driver coverage and
  representative native evidence.

- Rich card faces, badges/text/outline ordering, contained layout, conditional
  controls, and independent inspection are visibly correct.

- Duplicate rejection and single-visual hide/show identity pass across UI/world
  roots; destroyed UUID reuse is rejected.

- Native geometry confirms screen-space continuity, context-dependent hit
  regions, and resize/reorientation behavior.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Effect/replay/failure matrix completion is task 45. This task cannot replace
rich composed cards with Hearts texture-only cards.

## Manual QA

Select each completed test scene and perform its visible exercise/reset. Inspect
a rich card moving across domains and reversing an unfinished hide transition.
