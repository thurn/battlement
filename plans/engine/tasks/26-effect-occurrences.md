# 26. Schedule sound, particles, and attached effects on the shared clock

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Motion contract](../motion.md)
- [World contract](../world.md)
- [Presentation contract](../presentation.md)

**Prerequisite:** [Task 25: Bind checkpoint registrations to Motion and rendered
acceptance](25-checkpoint-motion-gates.md) and all its required follow-ups must
be integrated.

**Source roles:** Unified Motion graph; asset preparation; Unity particle/audio
services; typed anchors; fake occurrence history. Resolve these through
source-map.md; its links track the current owner after crate moves. Inspect the
concrete caller and host/fake counterpart before editing.

## Result

Sequences start prepared sound/burst/effect work locally with stable occurrence
identity and typed attachment semantics.

## Implementation

1. Add prepared sound-start and particle-burst entries plus persistent effect
   children. Resolve targets/assets during preparation; select live-following or
   captured positions explicitly.

2. Use run/checkpoint/change-index/effect-slot identity across registrations and
   host delivery. Reject duplicate declared slots and deduplicate delivered
   starts.

3. Execute occurrence crossings locally alongside labels/tracks in declaration
   order. Expose public audio/effect occurrence observations in fake/native
   diagnostics.

4. Add ordinary Rust configuration/fallback helpers and optional typed RON
   loading in a neutral fixture. Capture configuration per playback; changing
   config affects only future starts.

5. Compose a projectile visual from the existing world host and Motion path; no
   game-specific command opcode or C# script.

## Acceptance

- Two intended sounds at one label use different slots; duplicate delivery of
  either produces one occurrence.

- Optional absent configuration omits an effect, while a missing required asset
  prevents all dependent playback.

- Live and captured anchor effects diverge correctly when the target moves.

- Simultaneous dissolve/light/audio properties compose through disjoint property
  ownership; shared property conflicts are detected.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Retention after logical unmount is task 27 and seek/replay occurrence history is
task 28.

## Manual QA

Trigger a sequence with sound and burst at reveal, move its anchor, and inspect
exact occurrence order/counts and captured versus following effects.
