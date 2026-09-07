# 37. Compose Hearts cards, hands, tricks, and inspection views

The Hearts display component renders a readable 3D table with stable cards and
independent inspection copies.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [World objects and input](../world.md)
- [Identity and state](../identity.md)
- [Animation](../motion.md)

**Prerequisite:** [Task 36: Implement fixed Hearts rules through the shared
context contract](36-hearts-rules.md) is integrated.

**Starting code:** Hearts shell/assets; world primitives/layouts; player cloned
state snapshots.

## Example

The root display owns world and UI composition; the displayed state snapshot
supplies its data:

```rust
(
    TableLayout::new().state(state),
    UiRoot::new().child(HeartsScoreboard::new().scores(state.scores())),
)
```

## Implementation

1. Create `HeartsDisplay` with reusable `CardView`, `Hand`, `Trick`,
   `CapturedPile`, `Seat`, and table components from the snapshot. Build
   front/back surfaces and independent hit regions in Rust.

2. Assign one stable UUID to each of the 52 cards from game/display data; never
   generate IDs during render. Preserve it across every deal, save/load, new
   match, hand/trick/pile transfer, and hidden presentation. Use distinct IDs
   only for simultaneous inspection or outcome-preview copies.

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

- Collecting and redealing all 52 cards preserves every UUID; offstage and
  face-down cards remain declared rather than being destroyed.

- An inspection copy has its own presentation identity and cannot collide with
  or move the primary card.

- Portrait/landscape native views keep the human hand and UI readable without
  overlapping active hit targets incorrectly.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Passing and play animations/actions are tasks 38-39; detailed drag/keyboard UX
is tasks 41-42.

## Manual QA

Load a deal, inspect human and opponent cards, resize during an event-driven
transfer, and check identity and readable layout in the inspector.
