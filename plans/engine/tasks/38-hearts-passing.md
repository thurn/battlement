# 38. Connect Hearts passing prompts and simultaneous transfers

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [Execution contract](../execution.md)
- [Presentation contract](../presentation.md)
- [Motion contract](../motion.md)

**Prerequisite:** [Task 37: Compose Hearts cards, hands, tricks, and inspection
views](37-hearts-card-layout.md) and all its required follow-ups must be
integrated.

**Source roles:** Hearts rules/Card components; typed prompt API; checkpoint
registration. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

A human can choose and confirm a three-card pass while all players' transfers
present as one coherent exchange.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Final card-play flow is task 39, advanced AI task 40, and durable autosave task
43. Accepted-state notifications must already occur correctly.

## Manual QA

Select/deselect three cards, inspect one, open/close a menu, confirm the pass,
and watch simultaneous arrivals and prompt advancement.
