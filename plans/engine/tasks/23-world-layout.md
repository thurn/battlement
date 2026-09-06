# 23. Implement world Flexbox, Grid, fans, piles, and arcs

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [World contract](../world.md)
- [Motion contract](../motion.md)
- [Identity contract](../identity.md)

**Prerequisite:** [Task 22: Implement immutable sequences and
completion-relative labels](22-sequence-dependencies.md) and all its required
follow-ups must be integrated.

**Source roles:** World host descriptions; existing UI layout concepts; prepared
rest bounds and geometry observations. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

World arrangements compute stable target poses from declared planes and rest
measurements.

## Implementation

1. Integrate Taffy for world Flexbox/Grid using explicit world-unit extents and
   authored/rest-metadata boxes. Preserve UI Toolkit ownership of UI layout.

2. Define the pure custom-layout interface over ordered IDs/boxes/parameters and
   implement fan, pile, and arc algorithms with deterministic output.

3. Cache optional native rest measurements with request identity and invalidate
   only dependent ancestors when they change. Do not use animated current bounds
   as layout input.

4. Expose live layout destination handles for each child, plus explicit
   orientation/scaling rules. Add fixed-plane and nested-layout fixtures with
   known expected positions.

## Acceptance

- Flex/Grid/fan/pile/arc targets match deterministic fixture geometry on two
  orthogonal planes.

- Changing one subtree's box recomputes its dependent layouts without rebuilding
  unrelated arrangements.

- A deferred measurement prevents an unmeasured pose commit; a stale measurement
  cannot overwrite a newer result.

- Animating a visual's scale does not change rest-layout positions unless its
  authored layout box also changes.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Animating to these destinations and UI/world transfer projection is task 24.
Hearts-specific hand spacing is task 37.

## Manual QA

Resize a fan/grid extent and a nested card box; inspect target positions, rest
bounds, and stable mesh facing.
