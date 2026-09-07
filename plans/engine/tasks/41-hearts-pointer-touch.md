# 41. Finish Hearts pointer, touch, drag, and inspection behavior

Mouse and touch users can pass, play, inspect, and cancel interactions without
accidental rules changes.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [World objects and input](../world.md)
- [Animation](../motion.md)

**Prerequisite:** [Task 40: Choose Hearts moves by simulating possible
hands](40-hearts-simulation-ai.md) is integrated.

**Starting code:** Hearts card/choice UI; unified pointer arbitration; world
capture and movement ownership.

## Example

An unsuccessful drag changes presentation but never commits a card play:

```text
drag a legal card outside the central play region
release or lose capture
return smoothly to the latest hand destination
keep the same unanswered prompt and rules state
```

## Implementation

1. Implement select-then-Play/second-activation confirmation for clicks/taps,
   and legal drag-to-center confirmation for card play.

2. Give drag explicit base-placement ownership only at a valid decision point.
   Invalid drop, capture loss, or cancellation returns the card to the latest
   hand destination without dispatching an action.

3. Keep hover/focus lift as an independent local offset during required
   movement. Use the explicit Inspect control for selected touch cards rather
   than relying on hover.

4. Ensure menus block table targets, preserve local selection, and restore
   interaction coherently after close. Handle viewport changes during capture
   and movement.

5. Add public-driver and native touch/mouse scenarios using geometric
   arbitration, not forced target IDs.

## Acceptance

- One valid drop submits one play; outside drops and removed/canceled capture
  submit none.

- Dragging during a required pass/draw cannot steal placement, while hover/menu
  input remains responsive.

- Selection/inspection never mutate rules and do not duplicate transient effects
  on rerender.

- Portrait/landscape reorientation during drag preserves target eligibility and
  smooth return.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Full keyboard/controller game navigation is task 42; no new rules or AI changes.

## Manual QA

Play and pass with mouse and touch, drag outside the table, open a modal while
selected, and resize during capture.
