# 42. Finish Hearts keyboard/controller navigation and menus

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [World contract](../world.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 41: Finish Hearts pointer, touch, drag, and inspection
behavior](41-hearts-pointer-touch.md) and all its required follow-ups must be
integrated.

**Source roles:** Hearts controls; world focus/navigation; existing UI
modal/focus behavior. Resolve these through source-map.md; its links track the
current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

The complete Hearts flow works without pointer input and communicates legal
choices and focus visibly.

## Implementation

1. Map arrows/controller directions to human cards and the active UI scope. Use
   Enter/primary activation to select/confirm and Escape/back to close
   inspection/menu.

2. Implement visible focused/selected/illegal states independently. A focused
   illegal card remains inspectable but cannot submit a play.

3. Finish Resume/New Game/Exit menu flows, score/results focus order, and
   return-focus to the original card/control after a modal closes.

4. Ensure a new request or removed card chooses a deterministic eligible focus
   target without firing activation. Preserve existing sample input mappings.

## Acceptance

- A user can start, pass, play a hand, inspect cards, open/close menus, and
  handle results using keyboard or controller only.

- Opening a modal excludes table activation and closing it restores visible
  focus.

- Focus moves safely when its card is played/removed; stale activation cannot
  affect a new incarnation.

- Pointer-to-controller switching does not lose selected cards or trigger a
  duplicate answer.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Continue-from-disk is connected by task 43. The menu may expose it only when the
persistence capability exists.

## Manual QA

Complete passing and a trick without touching the pointer, switch input modes
mid-selection, and verify focus/selection remain distinct.
