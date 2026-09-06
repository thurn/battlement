# 27. Retain exits, anchors, and effects after logical unmount

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Identity contract](../identity.md)
- [Motion contract](../motion.md)
- [World contract](../world.md)

**Prerequisite:** [Task 26: Schedule sound, particles, and attached effects on
the shared clock](26-effect-occurrences.md) and all its required follow-ups must
be integrated.

**Source roles:** Incarnation/visual lease ownership; scoped controls; generic
effects; asset lifetime. Resolve these through source-map.md; its links track
the current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

Dissolves and projectiles finish on retained old visuals without keeping logical
components alive or affecting a replacement incarnation.

## Implementation

1. Transfer declared exit tracks and retaining effects from scoped component
   ownership to task 15's frozen visual leases.

2. Track host/material/font/anchor dependencies until their final exit/effect
   use. Keep anchors attached to the original incarnation while effects finish.

3. Resolve required work on unmount by finishing it, rebinding to a successor,
   or explicit abandonment. Never mark interruption as successful gate
   completion.

4. Implement neutral dissolve/reverse-dissolve with a shared material Motion
   value and separate text opacity, plus a projectile that outlives its source
   component.

## Acceptance

- Logical handlers/subscriptions detach immediately while the dissolve remains
  visible.

- A new component with the same UUID has fresh state and independent
  materials/effects while the old exit continues.

- Multiple retaining effects release the host only after the last one ends, with
  no leaked assets after reset.

- Required exit work follows its gate contract; a stale completion cannot
  satisfy the replacement's gate.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

The full inspector/replay UI is task 29; this task exposes the observations
needed to test retention.

## Manual QA

Remove/recreate a dissolving card, interact with the new one, and watch an old
anchored projectile complete. Inspect resource counts after both lifetimes end.
