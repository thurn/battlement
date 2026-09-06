# 15. Separate logical unmount from retained visual lifetime

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Identity contract](../identity.md)
- [Motion contract](../motion.md)
- [Presentation contract](../presentation.md)

**Prerequisite:** [Task 14: Preserve UUID identity across parents, roots, and
portals](14-global-presentation-identity.md) and all its required follow-ups
must be integrated.

**Source roles:** Presence; refs; effect cleanup; host transactions; UUID index.
Resolve these through source-map.md; its links track the current owner after
crate moves. Inspect the concrete caller and host/fake counterpart before
editing.

## Result

Removal destroys logical state immediately while an explicitly retained old
visual remains isolated from a new mount with the same UUID.

## Implementation

1. Allocate mounted incarnation identities and include them in refs, host
   events, and retained ownership. Validate them on every callback reaching a
   live object.

2. On committed absence, detach handlers/subscriptions, drop hook state, and
   freeze the prepared host representation. Do not retain a live component
   closure to implement exit visuals.

3. Create a reference-counted retained visual/anchor/asset ownership record.
   Existing finite UI exits or an explicit fixture-held visual lease can
   demonstrate retention before unified world effects exist.

4. Allow a new live incarnation with the same UUID while an older visual lease
   exists. Ensure final lease release cannot destroy the new host.

## Acceptance

- Remove/recreate shows fresh hook state and a new incarnation while the old
  visual can remain.

- Old input, capture callbacks, and effect completions cannot mutate or destroy
  the new object.

- Logical subscriptions clean up at removal, and retained native resources
  release exactly once after the last lease.

- An abandoned preparation that omits the object does not unmount the committed
  component.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

World exit sequences and projectile retention use this ownership in task 27. Do
not claim those effects are implemented yet.

## Manual QA

Hold an old visual lease, remove/recreate the UUID, interact with the new
object, then release the old lease. Verify isolated state and cleanup.
