# 24. Animate layout movement with continuous retargeting

Layout changes, bespoke sequences, and explicit UI/world projection preserve
visual continuity and arrive at current destinations.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 23: Implement world Flexbox, Grid, fans, piles, and
arcs](23-world-layout.md) and all its required follow-ups must be integrated.

**Starting code:** World layouts; existing UI layout projection; unified Motion;
UUID movement/ref matching.

## Example

An ordinary identified card should move without root movement configuration:

```rust
Hand::new().child(Card::new().id(card_id))
// Moving it on the next render uses the engine default spring.
Table::new().child(Card::new().id(card_id))
```

## Implementation

1. Supply the engine default spring movement with no required App setup. Add
   inherited and object overrides resolved from the destination logical
   ancestry. Policies receive source/destination layout/pose, refs, and typed
   state animations when available.

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
   animations through ordinary command operations.

## Acceptance

- A game with no `.movement()` or MotionConfig declaration moves identified
  objects automatically; inherited/object overrides still work.

- A reveal step follows its own anchor while hand reflow changes only the
  pending destination; the blocking operation waits for arrival after the hand
  step starts.

- Repeated retargeting causes no pose jump and does not produce duplicate
  completion identities.

- Cross-root and portal UI moves retain identity and animate rendered geometry;
  UI/world transfer has continuous screen-space correspondence.

- Missing projection fails render validation; no implicit pixel/world conversion is
  invented.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Snapshot movement-command generation arrives in task 25. Hearts uses these
policies in tasks 37-39.

## Manual QA

Reflow a hand during reveal, flip, and arrival. Repeat an explicit UI/world
transfer with orthographic and perspective cameras while inspecting screen-space
geometry.
