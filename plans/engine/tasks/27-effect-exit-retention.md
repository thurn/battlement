# 27. Retain exits, anchors, and effects after logical unmount

Dissolves and projectiles finish on retained old visuals without keeping logical
components alive or allowing the destroyed UUID to identify another object.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Identity and state](../identity.md)
- [Animation](../motion.md)
- [World objects and input](../world.md)

**Prerequisite:** [Task 26: Schedule sounds, particles, and attached
effects](26-effect-occurrences.md) is integrated.

**Starting code:** Retained visual reference ownership; scoped controls; generic
effects; asset lifetime.

## Example

An effect retains the native resources it still needs, not the logical entity:

```text
destroy fixture target: detach handlers and subscriptions immediately
dissolve and projectile continue on its old native objects
dissolve ends: projectile still retains its old anchor
projectile ends: release the last retained resources
```

## Implementation

1. Transfer declared exit tracks and retaining effects from scoped component
   ownership to task 15's retained native visuals.

2. Track host/material/font/anchor dependencies until their final exit/effect
   use. Keep anchors attached to the destroyed visual while effects finish.

3. Keep blocking exit operations alive on retained visuals until they finish.
   A stop cancels them through existing host cleanup; it cannot accept the
   abandoned action. Cosmetic exits remain nonblocking.

4. Implement neutral dissolve/reverse-dissolve with a shared material Motion
   value and separate text opacity, plus a projectile that outlives its source
   component.

## Acceptance

- Logical handlers/subscriptions detach immediately while the dissolve remains
  visible.

- Retained visuals keep their original native handles; stale callbacks or
  cleanup from the old session cannot affect replacement resources. Destroyed-ID
  reuse within a lifetime is unsupported, not a retirement-registry feature.

- Multiple retaining effects release the host only after the last one ends, with
  no leaked assets after reset.

- Blocking exits delay their batch; cosmetic exits do not. Old cleanup cannot
  release objects belonging to a new mount.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

The small diagnostic inspector is task 29; this task exposes the observations
needed to test retention.

## Manual QA

Destroy a dissolving fixture object while an anchored projectile continues.
Replace the session, deliver a stale callback, and inspect resource counts
after every retained use ends.
