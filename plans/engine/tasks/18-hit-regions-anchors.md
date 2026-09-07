# 18. Add independent hit regions and typed Rust-created anchors

Game components author collision geometry and effect attachment points
independently of visual meshes or prefab parts.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 17: Add sprites, meshes, world text, and material
overrides](17-world-rendering-primitives.md) and all its required follow-ups
must be integrated.

**Starting code:** World adapter/primitives; pointer host; refs; prepared
asset/host ownership.

## Example

The hit box and effect attachment point are explicit children, independent of
the artwork:

```rust
WorldGroup::new().children((
    BoxHitRegion::new().size(hit_size).center(hit_center),
    Anchor::new().reference(spark_origin).position(offset),
))
```

## Implementation

1. Implement BoxHitRegion and typed Anchor hosts with position/size/center
   setters. Attach them to logical/world owners and retain refs bound to the
   entity UUID until destruction cleanup.

2. Generate hit geometry and visual changes in the same render and order their
   ordinary commands together. Permit a Card
   face/context to change its collider dimensions through ordinary Rust props.

3. Expose typed anchor references and live/captured target descriptors for later
   movement/effects. Reject a wrong native kind or missing required anchor
   before submitting dependent commands.

4. Create a fixture with a moving parent, an offset anchor, and a marker showing
   its current world position. Do not look up named prefab children.

## Acceptance

- A collider size/center change follows the corresponding visual update through
  the existing command path; native clicks exercise the resulting hit area.

- An anchor follows its parent's transform and survives compatible ancestry
  moves.

- A hidden entity keeps its ref, while a destroyed entity's retained refs remain
  tied only to its terminal exit host and cannot target another object.

- Incorrect target kind or missing required ref fails before any visible effect
  starts.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Geometric/modal arbitration is task 19; effect playback at anchors is task 26.
This is explicitly not typed prefab binding.

## Manual QA

Move and rotate the parent while observing its anchor marker, change hit size,
then hide/show and finally destroy the parent while inspecting ref validity.
