# 38. Connect Hearts passing prompts and simultaneous transfers

A human can choose and confirm a three-card pass while all players' transfers
present as one coherent exchange.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Animation](../motion.md)

**Prerequisite:** [Task 37: Compose Hearts cards, hands, tricks, and inspection
views](37-hearts-card-layout.md) is integrated.

**Starting code:** Hearts rules/Card components; typed prompt API; checkpoint
registration.

## Example

Selection remains local until one valid three-card answer is submitted:

```text
select two cards -> Pass disabled
select third -> Pass enabled
open/close menu -> selection retained
Pass -> validate answer, then present all four players' transfers together
```

## Implementation

1. Use a local display store for selected cards. Toggle selection through
   click/tap activation and expose count, direction, and Pass enabled state in
   native UI.

2. Submit one typed answer only when exactly three distinct owned cards are
   selected. Validate against the actual request; active invalid replies panic
   and ended-request replies are ignored. UI prevents illegal submission.

3. Have `HeartsPolicy::owner` classify AI passing so `ExecutionMode` invokes
   choose_with_policy immediately after enqueueing, using the deterministic
   policy until task 40 and unchanged pre-exchange hands. Publish all transfers
   with one semantic exchange event.
   Display code hides AI choices even though the shared prompt enum contains
   them.

4. Animate the pass through movement policies producing blocking commands. Keep
   menus and inspection usable while the worker waits or cards move.

5. Reset selection appropriately when the prompt/request changes, not on
   unrelated rerenders or menu opening.

## Acceptance

- Zero/two/four selections cannot confirm; three valid selections submit one
  answer.

- All left/right/across/hold phases reach the correct hands without exposing
  received cards before the committed exchange.

- Reselecting/opening menus does not duplicate playback or lose the current
  selection.

- Abandoning the prompt or transfer rejects its late answer/effects and leaves a
  replacement game responsive.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Card-play flow is task 39, advanced AI task 40, and explicit durable save/load
is task 43. Status and accepted_state access must already work; no autosave.

## Manual QA

Select/deselect three cards, inspect one, open/close a menu, confirm the pass,
and watch simultaneous arrivals and prompt advancement.
