# 23. Implement world Flexbox, Grid, fans, piles, and arcs

World arrangements compute stable target poses from declared planes and rest
measurements.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Animation](../motion.md)
- [Identity and state](../identity.md)

**Prerequisite:** [Task 22: Build sequences with labels that follow actual
completion](22-sequence-dependencies.md) and all its required follow-ups must be
integrated.

**Starting code:** World host descriptions; existing UI layout concepts;
prepared rest bounds and geometry observations.

## Example

World layout receives world-unit bounds and a declared plane:

```rust
WorldFlex::new()
    .plane(table_plane)
    .extent((12.0, 3.0))
    .gap(0.1)
    .children(cards)
```

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Animating to these destinations and UI/world transfer projection is task 24.
Hearts-specific hand spacing is task 37.

## Manual QA

Resize a fan/grid extent and a nested card box; inspect target positions, rest
bounds, and stable mesh facing.
