# 25. Start checkpoint animations and wait before advancing

Each checkpoint starts its declared animations once. The next checkpoint and
saving wait for the required completion or label and a rendered frame.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [API examples and defaults](../interfaces.md)

- [Presentation timing](../presentation.md)
- [Animation](../motion.md)
- [Rules and choices](../execution.md)

**Prerequisite:** [Task 24: Animate layout movement with continuous
retargeting](24-layout-movement-projection.md) and all its required follow-ups
must be integrated.

**Starting code:** Application checkpoint admission; host transactions/frame
acknowledgement; sequence events; movement policies.

## Example

A state-animation callback chooses which event allows this checkpoint to
advance; playback still begins only after preparation commits:

```rust
let checkpoint = use_checkpoint::<StateAnimation>();
let animate = use_animate();
checkpoint.on_animation(card_ref.scoped_name("draw"), move |animation, cx| {
    if let StateAnimation::CardDrawn(id) = animation {
        if *id == card_id {
            let playback = animate.start(draw_sequence(card_ref, reveal_ref));
            cx.require(playback.reached("ready"));
        }
    }
});
```

## Implementation

1. Add callbacks for typed state animations, identified by a stable name within
   each animation. Provide scoped names based on stable presentation
   identity and resolve declared refs before evaluating the callback. Evaluate
   it during preparation, reserve playback handles, and atomically install
   visible tree, playback registration, occurrences, and required contributions
   at commit.

2. Default gate contributions to required layout movement. Let a registration
   choose an earlier label for its own contribution while preserving other
   required contributors.

3. Keep an already accepted completion satisfied. When replacing unfinished
   animation, update its requirement to the successor playback and label in the
   same commit. Serialize event acceptance with replacement commits and reject
   stale playback generations.

4. Require a rendering opportunity at or after gate satisfaction before
   admitting another checkpoint or accepting final state. Keep cosmetic work
   independent and cap initial checkpoint commits to one per rendered frame.

5. Add explicit finite wait steps to sequences for presentation pacing without
   sleeping the rules worker; validate them as ordinary finite dependencies.

## Acceptance

- Rerender/preparation retry starts one animation registration; two simultaneous
  required movements both contribute to the gate.

- An earlier chosen label permits advancement while later cosmetic movement
  continues, but a pre-gate frame never counts.

- Retarget preserves an arrival dependency; replacement accepts old completion
  only if it was processed before rebinding.

- Required failure abandons the action and exposes accepted-state recovery;
  cosmetic stop does not.

- A final worker result enables saving only after its real gate and rendering
  acknowledgement.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Sound/burst occurrences are task 26 and replay isolation task 28. No fake-only
shortcut may satisfy the live gate.

## Manual QA

Exercise gate-replacement just before/after a label event, then step a final
checkpoint frame by frame and observe when saving becomes available.
