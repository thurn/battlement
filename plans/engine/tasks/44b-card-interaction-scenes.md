# 44b. Complete small card browser, choice, and preview scenes

[Task group 44](44-identity-composition-laboratory.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Test scenes](../fixtures.md)
- [Identity and state](../identity.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [44a: Finish demonstrated identity and composition
gaps](44a-composition-scenes.md) is integrated.

**Starting code:** Reactant laboratory/inspector; world primitives; UI/world projection;
public display driver.

## Implementation

1. Complete card-browser, card-selection-scenes, and simulation-preview from fixtures.md
   with fixed synthetic content. Reuse one composition/choice surface where possible;
   these are small scripted interactions, not another game.

2. Provide stable selection/reset and public outcomes for browse/reorder cancel/confirm,
   draft/shop/quest-deck selection, and a hypothetical outcome. Use distinct preview IDs
   and keep live rules unchanged until a valid committed choice.

## Acceptance

- Browser/deck-order, draft/shop/quest-deck selection, and simulated outcome previews
  work through shared rules/display APIs. Local rearrangement and previews leave live
  rules state unchanged until a valid choice is committed.

- Existing scenes may prove multiple interactions. Add coverage only for missing
  local-placement, typed-choice, or preview isolation behavior.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](44-identity-composition-laboratory.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Browse/reorder, cancel and confirm one choice, then show a hypothetical outcome without
changing the live game.
