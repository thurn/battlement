# 18. Add independent hit regions and typed Rust-created anchors

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [World contract](../world.md)
- [Identity contract](../identity.md)
- [Presentation contract](../presentation.md)

**Prerequisite:** [Task 17: Add sprites, meshes, world text, and material
overrides](17-world-rendering-primitives.md) and all its required follow-ups
must be integrated.

**Source roles:** World adapter/primitives; pointer host; refs; prepared
asset/host ownership. Resolve these through source-map.md; its links track the
current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

Game components author collision geometry and effect attachment points
independently of visual meshes or prefab parts.

## Implementation

1. Implement BoxHitRegion and typed Anchor hosts with position/size/center
   setters. Attach them to logical/world owners and correlate prepared ref
   resolution with incarnation identity.

2. Keep hit geometry updates atomic with the visible commit. Permit a Card
   face/context to change its collider dimensions through ordinary Rust props.

3. Expose typed anchor references and live/captured target descriptors for later
   movement/effects. Reject a wrong native kind or missing required anchor
   during preparation.

4. Create a fixture with a moving parent, an offset anchor, and a marker showing
   its current world position. Do not look up named prefab children.

## Acceptance

- A collider size/center change agrees with the committed visible generation.

- An anchor follows its parent's transform and survives compatible ancestry
  moves.

- A ref from an old incarnation cannot target its replacement, and retained refs
  remain tied to their old host lease.

- Incorrect target kind or missing required ref fails before any visible effect
  starts.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Geometric/modal arbitration is task 19; effect playback at anchors is task 26.
This is explicitly not typed prefab binding.

## Manual QA

Move and rotate the parent while observing its anchor marker, change hit size,
then remove/recreate the parent and inspect ref validity.
