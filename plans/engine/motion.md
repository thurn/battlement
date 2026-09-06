# One Motion model for UI, world objects, and effects

Read this for transitions, property ownership, sequences, layout movement,
effects, or replay. See [presentation](presentation.md), [world](world.md), and
[identity](identity.md).

## Shared authoring and execution

Reactant describes typed targets, transitions, variants, gestures, layout
movement, Motion values, and immutable sequences. Unity evaluates one shared
playback model through domain-specific property writers. The fake independently
implements its observable contract.

Retain the existing useful spring/tween samplers and UI behavior. Extract
host-neutral timing, ownership, and scheduling from the UI-specific executor.
Keep UI property writing in its adapter and world/material/audio writing in
their adapters. Do not add a separate world animation scheduler.

~~~rust
WorldGroup::new()
    .animate(target().scale(1.0))
    .while_hover(target().local_offset_y(0.12))
    .transition(Transition::spring())
~~~

This is an illustrative API. MotionConfig supplies inherited defaults; objects
and individual properties can override them. Preserve initial, animate, exit,
variants, descendant orchestration, pause/resume, speed, stop, and
reduced-motion behavior. Native property sampling does not cause a Rust
component render.

## Property ownership

Each property has one effective owner across declarative targets, gestures, and
explicit controls. Layout owns base placement unless a sequence or permitted
drag claims it. Hover lift/tilt uses a separate local offset that composes after
base placement.

Gesture targets temporarily override declarative values and return to the latest
underlying target on release. Explicit sequences claim their declared property
sets. Preparation rejects accidental overlapping writes inside a sequence; an
explicit replacement annotation permits intentional overlap.

Starting replacement work interrupts previous property owners. Disjoint
properties can run concurrently. A required owner must transfer its gate
responsibility or take the explicit failure/abandonment path. Disable game drag
while required motion owns placement; hover/menu input remains responsive.

## Targets and retargeting

Position targets are fixed coordinates, typed anchors, or live layout
destinations. Anchor constructors explicitly select follow-live or capture-at-
start behavior. Layout destinations always resolve to the latest layout result.

Each step follows its own target: a reveal step follows its chosen reveal
anchor, even while the hand reflows. The later hand-placement step resolves the
latest hand destination when it starts, then retargets if that destination
changes.

Retarget from the actual displayed pose at host application time. Preserve
spring velocity. Restart a fixed-duration tween's configured duration from its
current pose. Convert across layout spaces through world coordinates. Reflow
does not replace the playback identity or its arrival dependency.

Equal poses complete immediately. Repeated reflow can delay arrival; stable
destinations must eventually complete. On sequence handoff back to layout, adopt
current pose/velocity so the object does not jump.

## Sequences and labels

Compile sequence data into typed tracks, discrete occurrences, and a dependency
graph executed locally by the host. Support sequential steps, absolute times,
relative offsets, concurrent starts, and named labels.

~~~rust
let sequence = AnimationSequence::new()
    .animate(card, target().position(reveal.follow()), quick())
    .label("reveal")
    .animate(card, target().rotation(face_up), flip())
    .animate(card, target().position(card.layout_destination()), settle())
    .label("ready");
~~~

Known-duration timing follows the familiar Motion timeline model.
Completion-relative successors wait for actual completion, including retargeted
arrival. Absolute-time entries keep their timestamps. A label after a retargeted
movement means actual arrival, not the original estimated duration.

Reject cycles, missing labels/targets, required dependencies on infinite loops,
and unsupported properties during preparation. Same-timestamp entries run in
declaration order; completion follows terminal labels and occurrences.

Every playback has an instance ID and generation. Each track terminates once
with completed, interrupted, or failed. Aggregate playback controls preserve
these outcomes; stale generations cannot affect live gates.

## Effects and occurrence identity

Continuous material parameters, light intensity, particle emission, and audio
volume are ordinary animatable properties. Sound starts and particle bursts are
discrete sequence entries. Projectiles combine a visual and normal movement.
Persistent auras are desired component children with ordinary animate/exit.

A transient slot is unique within its change record across all registrations and
sequences. Its live occurrence key is:

~~~text
(run ID, checkpoint ID, change index, stable effect slot)
~~~

Preparation retries, rerenders, and repeated delivery reuse this key. Two
intended sounds need different slots. Reject duplicate declarations before
commit. Deduplicate starts at the host as well as the Rust registration layer.

Prepare required assets and targets before starting anything. Optional missing
configuration is omitted by game Rust code; missing required assets fail
preparation. Effect selection/fallbacks remain ordinary Rust, not a host rule
language. RON may populate typed constants/assets but contains no control flow.
Active playbacks retain captured configuration; future playbacks use new values.

## Resource lifetime and inspection

Scoped controls stop ordinary work on unmount. Declared exits and effects
requiring visual retention transfer to frozen visual ownership. Their references
stay tied to the original incarnation. Release resources after their last use,
not merely when a component disappears.

Pause/seek samples supported visual tracks without replaying already delivered
sound/burst occurrences. Resume emits only undelivered occurrences. Native
capabilities report their seek support; do not claim particles can be rewound
when their executor cannot reproduce that state.

Explicit replay allocates a session-unique replay ID in a namespace separate
from live checkpoints. Slots remain stable within one replay; another replay
gets another ID. Replay/inspection events cannot satisfy or replace live gates.
Provide snapshot/restore of inspected playback state so leaving inspection
returns to the live presentation without changing game state.

## Manual QA

Use identical settings for a UI property, world property, default move, and
authored sequence. Compare pause, speed, interruption, and retargeting. Reflow a
hand during reveal: hover stays responsive and ready occurs only at arrival.
Seek around a sound/burst label, resume, then explicitly replay; verify
occurrence counts and live gate isolation.
