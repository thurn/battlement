# Neutral engine laboratory and reference workloads

Read this when adding engine scenarios, inspector demonstrations, or performance
workloads. See [validation](validation.md), [motion](motion.md), and
[Hearts](hearts.md).

## Harness and public observation

Retain samples/reactant as the focused laboratory and add numbered selectable
engine specimens to its existing structure. Hearts is the cohesive playable
example; the laboratory isolates contracts that Hearts does not naturally show.

Each specimen has a stable selector, reset, seed/deal input, and a short visible
caption stating what it proves. A placeholder for a later task must be labelled
unavailable. Tests must not claim that an unavailable specimen proves behavior.

The public display driver drives actions/input, advances presentation time and
frames, and observes rendered objects, text, transforms, layout destinations,
effects, prompts, and lifecycle states. It must not expose private reconciler
maps or mutable rules internals as assertions.

Provide public started, builder-entered, and worker-stopped synchronization
barriers for deterministic fixtures. Fault injection belongs to explicit fixture
services/builders, not hidden sleeps or production-only test branches.

## Required specimen set

Each row identifies a distinct observable fixture. Task ownership is in
[coverage](coverage.md).

| Selector | Demonstration |
| --- | --- |
| identity-transfer | Stateful card moves across hand, pile, grid, fan, portal, and active roots |
| duplicate-identity | Duplicate live UUID rejected without changing the previous frame |
| incarnation-exit | Remove/recreate the same UUID during an old exit; isolated state/input/effects |
| ui-world-transfer | Explicit orthographic and perspective projection with screen-space continuity |
| motion-equivalence | Same transition on UI, world, layout, and sequence |
| draw-reflow | Reveal/flip/live hand arrival, reflow, responsive hover, completion-relative ready |
| mixed-input | UI blocking/passthrough, nested modals, touch capture, keyboard/controller focus |
| composed-card | Faces, hidden face, rich text, badges, outlines, preview, conditional action button |
| contained-layout | Nested cards and resized rest bounds without animated-bound feedback |
| material-effects | Independent overrides, shared dissolve value, text fade and reverse dissolve |
| attached-effects | Live/captured anchors, trails, projectile, light/audio properties and retention |
| occurrence-replay | Sound/burst deduplication, same-time order, seek/resume and isolated replay |
| prompt-cycle | Select/deselect choices, invalid/stale answers and responsive settings |
| cancellation | Publication, builder, prompt, answer/completion races and ordinary computation |
| preparation | Delayed assets, superseded proposal, atomic host swap and missing required asset |
| gate-replacement | Early labels, multiple required contributors, retarget/replace/failure races |
| stores | Equal selectors suppress reevaluation; writes during render appear on the next revision |
| save-failure | Accepted in-memory state survives persistence failure and restores correctly |

The composed-card specimen is neutral artwork plus synthetic labels and badges.
It must include richer content than a single Hearts texture; no Dreamtides
assets or game logic. Include normal/table/hidden/UI representations, alternate
hit regions, and contained-card children. These validate the proposed primitive
capabilities independently of playing-card simplicity.

## Inspector

Implement a reusable developer-facing presentation inspector, mounted as
Reactant UI and hidden by default in player flows. Show current checkpoint and
prompt, UUID/incarnation, layouts/current transforms, property owners,
playbacks, labels, effects, preparation identity, frame acknowledgements, run
cancellation and worker-stopped states, plus timing/allocation/publication
counters.

Pause, slow playback, advance one frame, inspect seek support, and explicitly
replay a selected sequence. Inspection cannot dispatch gameplay or satisfy a
live gate accidentally. Normal Hearts inspection must not reveal private hands;
developer fixture state is explicitly separate.

## Performance fixtures

Define fixed 300-card and 500-card populations across 30 layouts. Each card is a
complete neutral composed view, not one quad. Record the actual node, text,
material, and renderer counts as part of the immutable fixture definition.

Include sparse changes, all-layout reflow, 30 simultaneous movement/effect
playbacks, prompts, menu/hover input, and concurrent Hearts-style AI load. Use
fixed seeds/assets and record AI work/worker counts.

Retain the proposal's values as reporting targets:
- Rust render/layout/reconcile p95: 2 ms.
- Total Reactant/Battlement main-thread CPU p95: 4 ms.
- Mean frame rate: 59 FPS; p99 frame interval: 18.34 ms.
- Maximum attributable warmed frame: 33.34 ms.
- Local hover/menu response: two rendered frames.

These numbers are not completion gates. Neither the 300-card nor 500-card
workload may be reduced to obtain favorable measurements. Report misses,
bottlenecks, and concrete follow-ups. Correctness failures still block
completion.

Record GPU time, allocations, queue depth, state-to-visible latency, preparation
cost and final-swap cost separately. Capture release builds after warmup for at
least ten minutes with device/OS, resolution, graphics settings, refresh cap,
browser version, and worker count. Use a recorded desktop machine for threaded
WebGL. Physical iPhone 17/Galaxy S25 certification is separate.

## Manual QA

Select and reset every specimen, inspect its stated contract, then exercise its
failure/replacement path. Run the fixed workload while opening menus and
prompts. Use the inspector to distinguish long preparation from a slow final
commit, and verify performance reporting does not suppress or hide missing
frames.
