# 15. Remove components while keeping unfinished exit visuals

Removal destroys logical state immediately while an explicitly retained old
visual remains isolated from a new mount with the same UUID.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Identity and state](../identity.md)
- [Animation](../motion.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 14: Keep UUID identity across parents, roots, and
portals](14-global-presentation-identity.md) and all its required follow-ups
must be integrated.

**Starting code:** Presence; refs; effect cleanup; host transactions; UUID
index.

## Example

A removed object and its replacement have separate mounted lifetimes:

```text
A, lifetime 1: exits visually; hooks and input are gone
A, lifetime 2: mounts with fresh hooks while old visual remains
old exit finishes: destroy lifetime 1 only
```

## Implementation

1. Allocate mounted incarnation identities and include them in refs, host
   events, and retained ownership. Validate them on every callback reaching a
   live object.

2. On committed absence, detach handlers/subscriptions, drop hook state, and
   freeze the prepared host representation. Do not retain a live component
   closure to implement exit visuals.

3. Create a reference-counted retained visual/anchor/asset ownership record.
   Existing finite UI exits or an explicit fixture-held retained visual
   reference can demonstrate retention before unified world effects exist.

4. Allow a new live incarnation with the same UUID while an older retained
   visual reference exists. Ensure releasing the final retained reference cannot
   destroy the new host.

## Acceptance

- Remove/recreate shows fresh hook state and a new incarnation while the old
  visual can remain.

- Old input, capture callbacks, and effect completions cannot mutate or destroy
  the new object.

- Logical subscriptions clean up at removal, and retained native resources
  release exactly once after the last retained use.

- An abandoned preparation that omits the object does not unmount the committed
  component.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

World exit sequences and projectile retention use this ownership in task 27. Do
not claim those effects are implemented yet.

## Manual QA

Hold an old retained visual reference, remove/recreate the UUID, interact with
the new object, then release the old retained reference. Verify isolated state
and cleanup.
