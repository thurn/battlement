# Animate properties, movement, and effects with one system

Reactant adapts Motion's component animation model to UI and Unity objects.
Components describe target values; transitions say how to reach them; sequences
coordinate those same animations. Unity advances prepared animation locally so
ordinary playback does not require Rust rendering on every frame.

Read this for property animation, layout movement, sequences, effects, and
inspection controls. Related pages: [world objects](world.md), [identity and
removal](identity.md), [checkpoint timing](presentation.md), and
[validation](validation.md). Existing implementation pointers and Motion
references are in the [source map](source-map.md).

## Start with target values

An **animation target** describes desired property values. A **transition**
selects spring behavior, duration, or easing for reaching them. These concepts
apply to UI opacity, world transforms, material parameters, light intensity,
particle emission rate, and audio volume.

For example, a button can become dim when disabled and enlarge on hover:

```rust
Button::new().text("Inspect")
    .animate(target().opacity(if enabled { 1.0 } else { 0.4 }))
    .while_hover(target().scale(1.05))
    .transition(Transition::spring())
```

Preserve the familiar authoring features across host kinds:

- `initial`, `animate`, and `exit` describe entry, current, and removal targets.
- `variants` name reusable targets and can coordinate descendants.
- Hover, tap, and drag supply interaction targets.
- `MotionConfig` supplies inherited defaults, overridable per object/property.
- Motion values drive host properties without a component render per sample.
- Controls pause, resume, change speed, stop, and inspect supported tracks.
- Reduced-motion behavior stays consistent with existing UI behavior.

Reuse existing spring/tween sampling and UI property writers. Extract shared
timing and control code for world/material/audio adapters; do not introduce a
separate world scheduler. The fake must implement the same observable behavior.

## Movement works without configuration

When an existing object's layout target changes, animate it using the engine's
default spring transition. No application `.movement()` setting is required.
Equal poses complete immediately. Applications may override defaults through
inherited `MotionConfig`, an object policy, or an authored sequence.

For example, keeping the UUID while moving a card between layouts is sufficient:

```rust
Hand::new().child(Card::new().id(card_id))
// In a later render:
Table::new().child(Card::new().id(card_id))
```

Resolve overrides from the new logical ancestry. A movement policy may select a
transition or construct a sequence using source/destination layouts and poses,
stable refs and anchors, typed state animations, and current configuration.
Unhandled moves fall back to the engine default.

UI layout supplies native target rectangles, including supported size changes.
World layouts supply world-space transforms. Initial connection, restoration,
and new mounts use entry behavior; removed objects use exit behavior.

## Separate placement from hover and drag

One effective animation controls each property. Layout normally controls base
placement. A custom sequence can temporarily take over that placement while
layout continues computing the eventual destination.

Hover lift and tilt are separate local offsets applied after base placement:

```rust
WorldGroup::new()
    .while_hover(target().local_offset_y(0.12))
    .child(CardFaces::new().card(card))
```

This allows a card to respond to hover while it travels. Gesture targets
temporarily override declarative targets and return smoothly to the latest
underlying value on release. Disjoint properties may animate concurrently.

Explicit controls claim their declared properties. Reject accidental overlapping
writes inside a sequence during preparation; permit them only with explicit
replacement. A new playback interrupts the former writer for those properties.
Required work must transfer its completion requirement or abandon the action.

Drag can take placement only when gameplay eligibility permits it. It cannot
steal a card from a required draw sequence. On release, move smoothly to the
latest valid destination; inspection and menus remain responsive throughout.

## Retarget from what is actually on screen

Position targets may be coordinates, typed anchors, or live layout destinations.
A **live layout destination** is a reference to the object's latest computed
layout target. Anchors explicitly choose follow-live or capture-at-start.

```rust
target().position(card.layout_destination())
target().position(reveal_anchor.follow())
target().position(reveal_anchor.capture_at_start())
```

When the target changes, the host starts from the displayed pose at the moment
it applies the update. Convert between layout spaces through world coordinates.
Preserve spring velocity; a fixed-duration tween restarts its configured
duration from the current pose. Reflow retains the playback ID and arrival
requirement.

During a reveal step, hand reflow changes the pending hand destination while the
card still follows its reveal anchor. The later placement step reads the latest
hand destination when it starts and continues to retarget during movement.
Repeated reflow can postpone arrival; once the layout stabilizes, it completes.
Returning control from a sequence to ordinary layout must not cause a jump.

## Coordinate a draw with a sequence

An **animation sequence** is immutable data describing animations, labels, and
timing dependencies. It uses the same targets, transitions, property ownership,
and controls as a single animation.

For example, rules can place a drawn card in the hand immediately, while display
code moves its existing face-down visual through reveal, flip, and placement:

```rust
let draw = AnimationSequence::new()
    .animate(card, target().position(reveal.follow()), quick_move())
    .label("reveal")
    .animate(card, target().rotation(face_up), flip_transition())
    .animate(card, target().position(card.layout_destination()), settle())
    .label("ready");
```

Use `use_animate` during component rendering to obtain scoped controls. Build
the sequence in a state-animation callback and require `ready` before the
checkpoint advances. The callback reserves a playback during preparation; commit
starts it.
[Presentation](presentation.md#tell-the-display-when-a-checkpoint-may-advance)
explains how retries avoid duplicate playback and how earlier labels work.

Event-driven animations use the same machinery without needing a checkpoint.
Rust constructs sequences and game-specific replacements. Unity executes the
prepared tracks, labels, sounds, and successors without calling back to Rust for
each step. Game-specific callbacks still run in Rust.

## Labels can follow actual completion

Support sequential steps, concurrent starts using `at`, absolute times, and
offsets relative to a label or another step. Known-duration transitions follow
the familiar Motion timeline behavior.

A label after movement that can retarget must follow actual arrival:

```text
move into hand: estimated 250 ms
at 100 ms: hand reflows, destination changes
"ready": emitted when the card reaches the new destination
sound scheduled at "ready": starts at that actual event
absolute entry at 250 ms: keeps its fixed timestamp
```

Reject cycles, missing labels/targets, unsupported properties, and required
dependencies on infinite cosmetic loops before playback. Same-time labels and
effects run in declaration order; final completion follows terminal entries.

Every playback has an ID and generation. Each track finishes once with
completed, interrupted, or failed. Stale events cannot satisfy current gameplay
requirements. Retargeting keeps the requirement; replacement explicitly moves
unfinished responsibility to its successor. Already accepted completion remains
satisfied. See [replacement
behavior](presentation.md#replace-animation-without-losing-required-work).

## Combine sounds, particles, and material effects

Continuous effect properties use ordinary targets. Starting a sound or emitting
a burst is a discrete event in a sequence. A projectile combines an instantiated
visual and normal movement. A persistent aura is a component child with ordinary
`animate` and `exit` behavior.

For example, attach reveal effects to the draw's label:

```rust
let draw = draw
    .play_sound(config.draw_sound).at("reveal")
    .emit(config.reveal_particles, card.spark_anchor()).at("reveal");
```

Game code selects effects using ordinary Rust. A chess move can choose a castle
sound, fall back to the piece sound or default, and separately add a check
sound:

```rust
let sound = castle_sound
    .or_else(|| config.pieces.get(&piece.kind))
    .unwrap_or(&config.default_move);
let mut sequence = movement.play_sound(sound).at("move");
if gives_check {
    sequence = sequence.play_sound(&config.check).at("arrived");
}
```

Required assets and targets must be prepared before playback. Omit optional
configuration while constructing the sequence; missing required assets fail
preparation. Active playbacks retain captured configuration, while later
playbacks use current values.

RON is optional data input for ordinary Rust configuration types. It can
populate constants, assets, and typed parameters; Rust retains conditions and
control flow:

```ron
(
    default_move_seconds: 0.25,
    draw_sound: "audio/draw",
    reveal_particles: "effects/reveal",
)
```

## Play each transient effect once

Rerendering must not play the same card sound again. A **transient occurrence**
is one intended sound, burst, or other one-time effect. Its identity combines
the action run, checkpoint, state-animation index zero, and stable effect name
within that event. Each publication has at most one semantic event; it may
produce many tracks and effects. Engine-assigned stable positions may supply
names when unambiguous.

For example, two sounds in one state animation need separate identities:

```text
(run 7, checkpoint 3, animation 0, "move-sound")
(run 7, checkpoint 3, animation 0, "check-sound")
```

Require unique effect names across all registrations and sequences for that
animation. Reject duplicates before commit. Retrying preparation or delivering a
request twice reuses the same identity. Deduplicate at both the Rust
registration layer and native start boundary. A new session renders current
state and emits transients only for subsequent state animations.

## Keep visuals for their last effect, then release them

Scoped controls stop ordinary animation when their component unmounts. Declared
exit animation and effects that still need visuals retain the prepared native
objects, anchors, and assets. Logical handlers and subscriptions detach at once.

For example, a removed card can dissolve while a projectile still follows an
anchor on its old visual. Those refs keep the old incarnation, even if the UUID
is mounted again. Release resources after their final retained use, not merely
when hooks disappear. [Identity](identity.md) defines this lifetime separation.

## Inspect and replay without changing game progress

Seeking samples supported visual tracks without replaying sounds and bursts that
have already occurred. Resume emits only occurrences not previously delivered.
Native capabilities must report seek support; unavailable particle rewinding
must be visible in the inspector rather than simulated inaccurately.

Explicit replay requests a fresh presentation of the sequence. It gets a new
session-unique replay ID separate from live checkpoint IDs:

```text
seek backward over reveal, then resume: do not repeat its delivered sound
explicitly replay the draw: play the sound once for this new replay
replay again: allocate another replay ID and play it once again
```

Neither seeking nor replay can advance a live checkpoint or mutate rules state.
Preserve the live playback state so leaving inspection returns to the current
game presentation. Repeated delivery within one replay is still deduplicated.

## Manual QA

Compare a UI property, world property, default layout move, and custom sequence
using the same transition settings. Omit application movement configuration.
Pause, slow, interrupt, and retarget each. Reflow a hand during reveal and check
that hover responds while `ready` waits for arrival. Seek across sound/burst
labels, resume, and explicitly replay. Remove a card during dissolve and an
attached projectile, then verify final resource cleanup.
