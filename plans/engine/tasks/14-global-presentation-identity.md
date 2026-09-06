# 14. Preserve UUID identity across parents, roots, and portals

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Identity contract](../identity.md)
- [Presentation contract](../presentation.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 13: Render world and UI contributions from one logical
tree](13-mixed-logical-tree.md) and all its required follow-ups must be
integrated.

**Source roles:** Core tree construction/matching; keys; hooks/context; portal
adapter; native reparent planning. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

A UUID-bearing component can move across logical ancestry without losing its
compatible state and refs.

## Implementation

1. Add id(Uuid) to components and both host domains while retaining
   sibling-scoped key(). Construct an application-wide proposed UUID index and
   reject all duplicate live declarations before visible mutation.

2. Match UUID positions before descendant reconciliation. Detach/extract moved
   nodes before disposing unmatched old ancestors; do not clone hook storage
   into two live positions.

3. Recompute context and event paths from new ancestry, including moves across
   active roots and UI portal destinations. Clean up changed effects before
   mounting their replacements.

4. Preserve compatible native handles and reset hook storage only for a changed
   component type or genuine committed absence. Extend public inspector
   observations for presentation identity without exposing private maps.

## Acceptance

- A counter/ref-bearing component moves from a soon-to-be-deleted parent to
  another root with state intact.

- Duplicate UUIDs in different UI/world roots reject the entire proposal and
  leave the previous display unchanged.

- New provider values and capture/bubble paths apply after the move, with
  exactly one active subscription.

- Ordinary equal sibling keys in separate parents remain legal and do not become
  global identities.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Old/new incarnation overlap on removal is task 15; animated cross-domain
projection is task 24. This task proves logical continuity with static
compatible hosts.

## Manual QA

Move the fixture among two layouts and a portal while changing the provider.
Verify preserved state, updated ancestry, and atomic duplicate rejection.
