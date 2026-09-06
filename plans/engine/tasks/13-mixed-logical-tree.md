# 13. Render world and UI contributions from one logical tree

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Architecture contract](../architecture.md)
- [Identity contract](../identity.md)
- [Presentation contract](../presentation.md)

**Prerequisite:** [Task 12: Add prepared host commits and rendered-frame
acknowledgement](12-host-transactions.md) and all its required follow-ups must
be integrated.

**Source roles:** Extracted tree construction and host adapter; application
roots; existing GameObject protocol. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

One stateful component can own a world hierarchy and a UI portal contribution
with shared hooks/context.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Cross-parent UUID continuity is task 14. Rich world primitives arrive in tasks
17-18. Existing group/prefab rendering must work now.

## Manual QA

Interact with the world visual and its portal UI, update the shared counter, and
inspect a single shared logical ownership path.
