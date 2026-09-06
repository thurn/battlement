# 27. Retain exits, anchors, and effects after logical unmount

Dissolves and projectiles finish on retained old visuals without keeping logical
components alive or affecting a replacement incarnation.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Identity and state](../identity.md)
- [Animation](../motion.md)
- [World objects and input](../world.md)

**Prerequisite:** [Task 26: Schedule sounds, particles, and attached
effects](26-effect-occurrences.md) and all its required follow-ups must be
integrated.

**Starting code:** Incarnation/retained visual reference ownership; scoped
controls; generic effects; asset lifetime.

## Example

An effect retains the native resources it still needs, not the logical card:

```text
remove card: detach handlers and subscriptions immediately
dissolve and projectile continue on old native objects
dissolve ends: projectile still retains its old anchor
projectile ends: release the last retained resources
```

## Implementation

1. Transfer declared exit tracks and retaining effects from scoped component
   ownership to task 15's retained native visuals.

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

The full inspector/replay UI is task 29; this task exposes the observations
needed to test retention.

## Manual QA

Remove/recreate a dissolving card, interact with the new one, and watch an old
anchored projectile complete. Inspect resource counts after both lifetimes end.
