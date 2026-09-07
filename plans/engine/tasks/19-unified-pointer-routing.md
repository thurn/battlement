# 19. Unify UI/world hit testing, propagation, and modal capture

Pointer input reaches one eligible committed target with consistent logical
propagation and capture across host domains.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 18: Add independent hit regions and typed Rust-created
anchors](18-hit-regions-anchors.md) is integrated.

**Starting code:** Core event dispatch; Unity pointer/panel coordinators; world
hit regions; fake pointer helpers.

## Example

Check actual geometric targeting before testing event callbacks:

```text
UI menu overlaps world card -> click reaches menu only
UI decoration allows passthrough -> click may reach card
card moves to another parent while captured -> drag stays captured
object is hidden or destroyed -> emit capture loss once
```

## Implementation

1. Extend world callbacks for click/hover/press/drag using the same event/hook
   patterns as UI. Route capture/bubble through logical ancestry and preserve
   synchronous default prevention.

2. Implement UI blocking with explicit passthrough, ordered modal scopes, and
   world candidate ordering by interaction layer/depth/stable sibling order.

3. Use existing input subscription identity and refs bound to the entity UUID.
   Validate gameplay eligibility through status/prompt handles. Preserve capture
   across reparenting; emit capture loss on hiding or destruction and reject
   events generated for an inert or destroyed target.

4. Add geometric public-driver input rather than requiring tests to bypass
   arbitration with a target ID. Retain direct low-level helpers only for
   protocol-specific tests.

## Acceptance

- An overlapping UI control blocks the card unless passthrough is enabled;
  nested modals admit only their top applicable scope.

- Exact-depth world ties resolve deterministically and obey interaction-layer
  priority.

- A captured object can reparent without losing capture; hiding or destruction
  loses capture once, and showing a hidden object cannot resume the old drag.

- Portal capture/bubble order follows logical ancestry and default prevention
  returns synchronously.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

World keyboard/controller navigation is task 20. Hearts-specific eligibility and
drop destinations belong to task 41.

## Manual QA

Exercise mixed-input with overlapping UI/world objects, nested modals,
reparented capture, and removal during drag.
