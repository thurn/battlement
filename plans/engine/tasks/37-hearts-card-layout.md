# 37. Compose Hearts cards, hands, tricks, and inspection views

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [World contract](../world.md)
- [Identity contract](../identity.md)
- [Motion contract](../motion.md)

**Prerequisite:** [Task 36: Implement the fixed Hearts rules through the generic
executor](36-hearts-rules.md) and all its required follow-ups must be
integrated.

**Source roles:** Hearts shell/assets; world primitives/layouts; player snapshot
projection. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

The actual Hearts snapshot renders a readable 3D table with stable cards and
independent inspection copies.

## Implementation

1. Create reusable Card, Hand, Trick, CapturedPile, Seat, and Table components
   from the snapshot. Build front/back surfaces and independent hit regions in
   Rust.

2. Assign stable deal-lifetime presentation UUIDs from game/display data; never
   generate IDs during render. Reuse identities for hand/trick/pile transfer and
   use distinct IDs for inspection copies.

3. Implement South's fan, opponent back-facing hands, center trick positions,
   captured piles, scores, and active-seat/passing indicators. Preserve card
   geometry while adapting spacing/framing to viewport extents.

4. Add keyboard/hover visual hooks through generic engine input, and explicit
   enlarged inspection using the same card view data. Do not reveal an
   opponent's face in either display.

## Acceptance

- All 52 cards have correct visible ownership and hidden-information treatment
  after a deal.

- Moving an explicit fixture card between hand/trick/pile retains its component
  state/ref/UUID.

- An inspection copy has its own presentation identity and cannot collide with
  or move the primary card.

- Portrait/landscape native views keep the human hand and UI readable without
  overlapping active hit targets incorrectly.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Passing and play animations/actions are tasks 38-39; detailed drag/keyboard UX
is tasks 41-42.

## Manual QA

Load a deal, inspect human and opponent cards, resize during an event-driven
transfer, and check identity and readable layout in the inspector.
