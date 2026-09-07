# 26. Schedule sounds, particles, and attached effects

Sequences start prepared sound/burst/effect work locally once per intended
occurrence, with explicit following or captured attachment points.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Presentation timing](../presentation.md)

**Prerequisite:** [Task 25: Generate blocking and nonblocking animation commands from snapshots](25-snapshot-animation-commands.md) and all its required follow-ups must
be integrated.

**Starting code:** Unified Motion graph; asset preparation; Unity particle/audio
services; typed anchors; fake occurrence history.

## Example

Sound and particles are prepared sequence entries scheduled at a label:

```rust
let draw = draw
    .play_sound(config.draw_sound).at("reveal")
    .emit(config.reveal_particles, card.spark_anchor()).at("reveal");
```

## Implementation

1. Add prepared sound-start and particle-burst entries plus persistent effect
   children. Resolve targets/assets during preparation; select live-following or
   captured positions explicitly.

2. Consume semantic events once at snapshot submission. Reuse existing
   batch/command duplicate suppression and playback/sequence-entry delivery
   history. Two intended effects are separate entries; require no new
   run/checkpoint/effect identity or registration-name API.

3. Execute occurrence crossings locally alongside labels/tracks in declaration
   order. Expose public audio/effect occurrence observations in fake/native
   diagnostics.

4. Add ordinary Rust configuration/fallback helpers and optional typed RON
   loading in a neutral fixture. Capture configuration per playback; changing
   config affects only future starts.

5. Compose a projectile visual from the existing world host and Motion path; no
   game-specific command opcode or C# script.

## Acceptance

- Two intended sounds at one label are separate entries; batch redelivery and
  ordinary rerenders produce no extra occurrences.

- Optional absent configuration omits an effect, while a missing required asset
  prevents all dependent playback.

- Live and captured anchor effects diverge correctly when the target moves.

- Simultaneous dissolve/light/audio properties compose through disjoint property
  ownership; shared property conflicts are detected.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Retention after logical unmount is task 27 and pause/resume is task 28.
Developer seek/replay is outside v1.

## Manual QA

Trigger a sequence with sound and burst at reveal, move its anchor, and inspect
exact occurrence order/counts and captured versus following effects.
