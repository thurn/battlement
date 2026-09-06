# 41. Finish Hearts pointer, touch, drag, and inspection behavior

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [World contract](../world.md)
- [Motion contract](../motion.md)

**Prerequisite:** [Task 40: Add bounded information-respecting Hearts
simulations](40-hearts-simulation-ai.md) and all its required follow-ups must be
integrated.

**Source roles:** Hearts card/choice UI; unified pointer arbitration; world
capture and movement ownership. Resolve these through source-map.md; its links
track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

Mouse and touch users can pass, play, inspect, and cancel interactions without
accidental rules changes.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Full keyboard/controller game navigation is task 42; no new rules or AI changes.

## Manual QA

Play and pass with mouse and touch, drag outside the table, open a modal while
selected, and resize during capture.
