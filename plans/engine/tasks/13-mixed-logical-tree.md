# 13. Render world and UI contributions from one logical tree

One stateful component can own a world hierarchy and a UI portal contribution
with shared hooks/context.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 12: Prepare native updates and acknowledge rendered
frames](12-host-transactions.md) and all its required follow-ups must be
integrated.

**Starting code:** Extracted tree construction and host adapter; application
roots; existing GameObject protocol.

## Example

A card component can share selection state between a world visual and a UI
portal:

```rust
(
    CardView::new().card(card),
    Portal::to(details_panel).child(CardDetails::new().card(card)),
)
```

## Implementation

1. Implement the world host adapter for empty groups and opaque prefab visuals
   using existing low-level host capability. Keep world host descriptions in the
   same proposed/committed logical tree as UI.

2. Add explicit world scene roots and UI document/portal attachment descriptors.
   Physical transforms/documents must not become the hook/context ownership
   hierarchy.

3. Lower mixed hosts through task 12's preparation/commit protocol. Ensure
   events/refs retain a logical owner even when native hosts are physically
   elsewhere.

4. Create a small mixed component fixture with one local counter, a world
   visual, and a UI details portal. Add an App authoring example that needs no
   manual command generation.

## Acceptance

- Updating a shared prop/context changes the world and UI outputs in one commit
  generation.

- Hooks run once per logical component, and portal events follow that
  component's logical ancestry.

- Adding/removing a UI contribution does not remount its sibling world component
  or vice versa.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Cross-parent UUID continuity is task 14. Rich world primitives arrive in tasks
17-18. Existing group/prefab rendering must work now.

## Manual QA

Interact with the world visual and its portal UI, update the shared counter, and
inspect a single shared logical ownership path.
