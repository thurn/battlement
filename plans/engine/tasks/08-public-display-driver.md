# 08. Add public display driving and deterministic virtual time

Scenarios can independently control worker progress, presentation time, and
rendered frames through a public test surface.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Validation](../validation.md)
- [Sample migration](../migration.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 07: Move Reactant asset preparation out of
Battlement](07-asset-tooling-boundary.md) and all its required follow-ups must
be integrated.

**Starting code:** World fake; UI fake; existing game tests; native batches and
tween execution.

## Example

Separate time advancement from a rendered frame and worker synchronization:

```rust
display.dispatch(move_card);
display.wait_for_render_submission();
display.advance_time(Duration::from_millis(125));
display.object(card_id).assert_position(halfway);
display.advance_frame();
```

## Implementation

1. Create reactant-testing around FakeClient with documented input,
   observations, wait, advance_time, advance_frame, and finite settle
   operations. Preserve the distinction between virtual time and worker
   scheduling.

2. Extend the world fake's low-level operation scheduling to represent finite
   tween interpolation and waits, instead of jumping to endpoints. Observe
   audio/effect occurrences through public history.

3. Add event-driven lifecycle/barrier waits with bounded hang timeouts. Keep
   fixture fault injection in explicit builder/service objects. A driver must
   run the real exported/public engine path, never call private rules directly.

4. Complete task 01's deferred timing-assertion rewrites against the
   still-unmigrated samples, comparing native evidence when changing fake timing
   exposes prior inaccuracies.

5. Keep direct samples working through explicit settle behavior; do not
   fabricate command compatibility or advance time inside a click helper.

## Acceptance

- A 250 ms move has an observable intermediate pose at 125 ms; TimeWait delays
  its successor; duplicate audio delivery creates one occurrence.

- Worker barriers do not advance virtual time, advance_time does not invent a
  rendered frame, and settle stops at unanswered prompts/infinite cosmetic work.

- The original tic-tac-toe 99/100 ms and chess capture/spawn timing guarantees
  are now expressed as observable behavior and pass before migration.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

New Motion features extend this driver in their own tasks. Task 12 connects
snapshot consumption to existing queue delivery without a native completion
wait. Frame stepping remains a visual test control, not a prerequisite for
advancing every snapshot.

## Manual QA

Step an existing chess animation with controlled time and compare intermediate
position/effect ordering to a native capture. Verify a never-completing fixture
yields a useful timeout.
