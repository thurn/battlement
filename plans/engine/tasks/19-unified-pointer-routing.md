# 19. Unify UI/world hit testing, propagation, and modal capture

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [World contract](../world.md)
- [Identity contract](../identity.md)
- [Presentation contract](../presentation.md)

**Prerequisite:** [Task 18: Add independent hit regions and typed Rust-created
anchors](18-hit-regions-anchors.md) and all its required follow-ups must be
integrated.

**Source roles:** Core event dispatch; Unity pointer/panel coordinators; world
hit regions; fake pointer helpers. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

Pointer input reaches one eligible committed target with consistent logical
propagation and capture across host domains.

## Implementation

1. Extend world callbacks for click/hover/press/drag using the same event/hook
   patterns as UI. Route capture/bubble through logical ancestry and preserve
   synchronous default prevention.

2. Implement UI blocking with explicit passthrough, ordered modal scopes, and
   world candidate ordering by interaction layer/depth/stable sibling order.

3. Carry commit/incarnation identity on input. Preserve capture across
   reparenting; emit capture loss on removal and reject stale target events.

4. Add geometric public-driver input rather than requiring tests to bypass
   arbitration with a target ID. Retain direct low-level helpers only for
   protocol-specific tests.

## Acceptance

- An overlapping UI control blocks the card unless passthrough is enabled;
  nested modals admit only their top applicable scope.

- Exact-depth world ties resolve deterministically and obey interaction-layer
  priority.

- A captured object can reparent without losing capture; removal loses capture
  once and the new incarnation receives no old drag events.

- Portal capture/bubble order follows logical ancestry and default prevention
  returns synchronously.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

World keyboard/controller navigation is task 20. Hearts-specific eligibility and
drop destinations belong to task 41.

## Manual QA

Exercise mixed-input with overlapping UI/world objects, nested modals,
reparented capture, and removal during drag.
