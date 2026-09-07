# 15. Destroy components while keeping unfinished exit visuals

Hiding preserves an entity and its UUID; committed absence destroys logical
state immediately while an explicitly retained terminal visual finishes.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Identity and state](../identity.md)
- [Animation](../motion.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 14: Keep UUID identity across parents, roots, and
portals](14-global-presentation-identity.md) and all its required follow-ups
must be integrated.

**Starting code:** Presence; refs; effect cleanup; command delivery; UUID index.

## Example

A hidden entity remains the same object, while a destroyed fixture can finish a
terminal exit without remaining logically mounted:

```text
card A: hide transition starts; hooks and ref remain; input is disabled
card A: shown again; reuse one visual and reverse/retarget the transition
effect target B: destroyed; hooks and input are gone; terminal visual exits
```

## Implementation

1. Treat visibility as presentation state rather than tree absence. Hidden
   entities retain their UUID, hooks, subscriptions, refs, and compatible native
   objects while disabling input and visibility. Showing one reconciles the same
   component and retargets, cancels, or reverses unfinished hide work.

2. On committed absence, detach handlers/subscriptions, drop hook state, retire
   the UUID for this presentation runtime, and retain the existing host
   representation. Do not retain a live component closure to implement exit
   visuals. Native removal stays ordered after earlier queued uses; logical
   unmount must not cancel those earlier movements.

3. Create a reference-counted retained visual/anchor/asset ownership record.
   Existing finite UI exits or an explicit fixture-held retained visual
   reference can demonstrate retention before unified world effects exist.

4. Reject duplicate and retired UUID declarations before visible mutation. Drop
   late input for destroyed targets, and use existing request, subscription,
   playback, and session identities for asynchronous callback validation.

## Acceptance

- Hide/show during an unfinished transition preserves hook state, ref identity,
  and one compatible native visual while input remains unavailable when hidden.

- A committed absence cleans up logical state immediately, and reuse of its UUID
  rejects the complete update even while its terminal visual remains.

- Logical subscriptions clean up at destruction, and retained native resources
  release exactly once after the last retained use.

- An abandoned render that omits the object does not unmount the committed
  component. Render a move and destruction ahead with Unity paused; playback
  must still move the object before its queued native removal.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

World exit sequences and projectile retention use this ownership in task 27. Do
not claim those effects are implemented yet.

## Manual QA

Hide/show an entity during its transition and verify one identity and visual are
preserved. Hold a terminal visual reference for a separate destroyed object,
attempt to reuse its UUID, then release the reference and verify rejection and
cleanup.
