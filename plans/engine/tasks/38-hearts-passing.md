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
views](37-hearts-card-layout.md) and all its required follow-ups must be
integrated.

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
   selected. Validate on the worker; invalid/stale answers preserve the prompt
   and show feedback.

3. Answer AI-owned passing prompts after presentation with the deterministic
   policy until task 40, using the private controller observation and unchanged
   pre-exchange hands. Publish all four transfers together with ordered semantic
   records. Do not expose AI decision payloads to UI props.

4. Animate the pass through movement policies and a required arrival label. Keep
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

Final card-play flow is task 39, advanced AI task 40, and durable autosave task
43. Accepted-state notifications must already occur correctly.

## Manual QA

Select/deselect three cards, inspect one, open/close a menu, confirm the pass,
and watch simultaneous arrivals and prompt advancement.
