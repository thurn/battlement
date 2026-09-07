# 14. Keep UUID identity across parents, roots, and portals

A UUID-bearing component can move across logical ancestry without losing its
compatible state and refs.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Identity and state](../identity.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 13: Render world and UI contributions from one logical
tree](13-mixed-logical-tree.md) and all its required follow-ups must be
integrated.

**Starting code:** Core tree construction/matching; keys; hooks/context; portal
adapter; native reparent planning.

## Example

Keep the UUID on the component whose hooks must survive the parent change:

```rust
Hand::new().child(Card::new().id(card_id))
// Later render:
Table::new().child(Card::new().id(card_id))
```

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

- Duplicate UUIDs in different UI/world roots reject the entire rendered update
  and leave the previous display unchanged.

- New provider values and capture/bubble paths apply after the move, with
  exactly one active subscription.

- Ordinary equal sibling keys in separate parents remain legal and do not become
  global identities.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Hidden-versus-destroyed lifetime behavior is task 15; animated cross-domain
projection is task 24. This task proves logical continuity with static compatible
hosts.

## Manual QA

Move the fixture among two layouts and a portal while changing the provider.
Verify preserved state, updated ancestry, and atomic duplicate rejection.
