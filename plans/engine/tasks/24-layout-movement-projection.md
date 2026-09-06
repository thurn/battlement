# 24. Animate layout movement with continuous retargeting

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Motion contract](../motion.md)
- [World contract](../world.md)
- [Identity contract](../identity.md)
- [Presentation contract](../presentation.md)

**Prerequisite:** [Task 23: Implement world Flexbox, Grid, fans, piles, and
arcs](23-world-layout.md) and all its required follow-ups must be integrated.

**Source roles:** World layouts; existing UI layout projection; unified Motion;
UUID movement/ref matching. Resolve these through source-map.md; its links track
the current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

Layout changes, bespoke sequences, and explicit UI/world projection preserve
visual continuity and arrive at current destinations.

## Implementation

1. Add root/default, inherited, and object movement policies resolved from the
   destination logical ancestry. Policies receive source/destination
   layout/pose, refs, and typed checkpoint changes when available.

2. Implement live layout destinations as native targets. Reflow preserves
   playback identity/arrival dependencies, spring velocity, and current
   displayed origin; duration tweens restart their configured duration.

3. Let a sequence own base placement while layout continues updating its pending
   destination. Return ownership continuously at arrival; hover remains an
   independent local offset and drag cannot steal required placement.

4. Implement the explicit camera/plane/rectangle projection policy for
   compatible UI/world visual transfers, with inactive host replacement and a
   retained transition representation.

5. Add draw-reflow and ui-world-transfer fixtures, initially driven by event
   animations rather than checkpoint gates.

## Acceptance

- A reveal step follows its own anchor while hand reflow changes only the
  pending destination; ready waits for actual arrival after the hand step
  starts.

- Repeated retargeting causes no pose jump and does not produce duplicate
  completion identities.

- Cross-root and portal UI moves retain identity and animate rendered geometry;
  UI/world transfer has continuous screen-space correspondence.

- Missing projection fails preparation; no implicit pixel/world conversion is
  invented.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Required checkpoint policy binding arrives in task 25. Hearts uses these
policies in tasks 37-39.

## Manual QA

Reflow a hand during reveal, flip, and arrival. Repeat an explicit UI/world
transfer with orthographic and perspective cameras while inspecting screen-space
geometry.
