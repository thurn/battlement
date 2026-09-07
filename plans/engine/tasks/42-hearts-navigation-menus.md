# 42. Finish Hearts keyboard/controller navigation and menus

The complete Hearts flow works without pointer input and communicates legal
choices and focus visibly.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 41: Finish Hearts pointer, touch, drag, and inspection
behavior](41-hearts-pointer-touch.md) and all its required follow-ups must be
integrated.

**Starting code:** Hearts controls; world focus/navigation; existing UI
modal/focus behavior.

## Example

Selection, focus, and legality are independent visible states:

```text
focus an illegal card -> it remains inspectable
activate Play -> cannot submit that card
open/close menu -> restore focus without losing selection
switch to pointer -> do not submit a duplicate answer
```

## Implementation

1. Map arrows/controller directions to human cards and the active UI scope. Use
   Enter/primary activation to select/confirm and Escape/back to close
   inspection/menu.

2. Implement visible focused/selected/illegal states independently. A focused
   illegal card remains inspectable but cannot submit a play.

3. Finish Resume/New Game/Exit menu flows, score/results focus order, and
   return-focus to the original card/control after a modal closes.

4. Ensure a new request or card that leaves the hand chooses a deterministic
   eligible focus target without firing activation. Preserve existing sample
   input mappings.

## Acceptance

- A user can start, pass, play a hand, inspect cards, open/close menus, and
  handle results using keyboard or controller only.

- Opening a modal excludes table activation and closing it restores visible
  focus.

- Focus moves safely when its card leaves the hand or becomes hidden; stale
  activation cannot answer a later request after the same card is shown again.

- Pointer-to-controller switching does not lose selected cards or trigger a
  duplicate answer.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Continue-from-disk is connected by task 43. The menu may expose it only when the
persistence capability exists.

## Manual QA

Complete passing and a trick without touching the pointer, switch input modes
mid-selection, and verify focus/selection remain distinct.
