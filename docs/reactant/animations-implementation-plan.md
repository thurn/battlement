# Reactant Animations implementation plan

When a task is complete, append `[DONE]` to its task heading.

Status: implementation companion to [`animations.md`](animations.md).

This plan implements the approved Reactant Animations contract without porting
the settings mockup or adding mockup-analysis infrastructure. The technical
design is normative. If this plan and the design disagree, the design wins.

## Related information

- [`animations.md`](animations.md) defines the authoring, sampling, lifecycle,
  performance, and manual validation contract implemented here.
- [`reactant-technical-design.md`](reactant-technical-design.md) defines the
  session, commit, snapshot, and Rust-to-Unity boundaries extended by motion.
- [`reactant-implementation-plan.md`](reactant-implementation-plan.md) records
  the completed Reactant runtime, reconciliation, event, hook, and geometry
  prerequisites.
- [Reactant Asset Generator implementation plan][asset-plan] describes an
  independent project whose tasks are not prerequisites here.
- The [settings mockup][settings-mockup] at commit
  `2451ea9cc6f76b356b1102ee37b82c478853122a` is the manual feature checklist.

[settings-mockup]:
  https://github.com/thurn/mockups/tree/2451ea9cc6f76b356b1102ee37b82c478853122a
[asset-plan]: asset-generator-implementation-plan.md

## Decisions and starting point

The repository already contains the Reactant runtime, host primitives, hooks,
events, portals, refs, geometry, prepared textures, Unity UI Toolkit client,
fake clients, controlled Ditto motion, and Reactant sample. It does not contain
Reactant motion descriptors, samplers, animation authoring, presence, motion
values, layout projection, or an audio playhead source.

The following decisions govern implementation:

- Motion 13.1.1 supplies the supported numerical and behavioral baseline.
- Rust declares animation state. Unity owns continuous sampling, gestures,
  layout projection, motion-value graphs, and final property application.
- The Reactant sampler is authoritative. UI Toolkit transitions are optional
  internal optimizations proven equivalent by controlled-clock tests.
- Concrete hosts receive sealed Motion builders. A custom component opts in by
  forwarding one `MotionProps` value to exactly one stable host.
- No user-facing animation macro, animated host duplicate, runtime CSS string,
  arbitrary selector string, or per-frame Rust callback is added.
- Time, scroll, pointer, geometry, and audio playhead values are Unity-local
  inputs to a closed serializable expression graph.
- The settings mockup is reviewed manually. There is no TypeScript analyzer,
  source manifest, mirrored gallery, source hash, or automated coverage system.
- The settings screen is not ported. Focused tests and sample cases exercise
  complex animation families where they provide useful evidence.
- Native macOS and desktop WebGL are release targets. Mobile constrains input,
  allocation, threading, memory, and renderer choices but has no release gate.
- The full performance screen is manual or explicitly triggered. Ordinary
  cached CI may grow by no more than 30 seconds.
- Generated assets are optional ordinary texture inputs. No task below depends
  on the Reactant Asset Generator implementation plan.

## Cross-plan dependency contract

There are no hard dependencies on Asset Generator Tasks 01–19.

| Animations task | Internal prerequisites | Asset Generator prerequisites |
|---|---|---|
| Task 01 — Animation data and sampling | None | None |
| Task 02 — Reactant authoring | Task 01 | None |
| Task 03 — Variants and presence | Task 02 | None |
| Task 04 — Motion values and time sources | Tasks 01–03 | None |
| Task 05 — Gestures and layout animation | Tasks 02–04 | None |
| Task 06 — Verify mockup-required effects | Tasks 03–05 | None |
| Task 07 — Performance and completion | Task 06 | None |

The independence is intentional:

- Asset Generator Tasks 01–15 implement declaration parsing, discovery,
  browser rendering, and generated output. Animations invoke none of them.
- Asset Generator Task 16 merges generated textures into Reactant snapshots.
  Animations already accept ordinary prepared textures, so it is not required.
- Asset Generator Task 17 imports and validates generated textures in Unity.
  Animation behavior is testable with styles, authored prepared textures,
  custom geometry, and shaders.
- Asset Generator Task 18 integrates generation into authoring and samples.
  The animation sample contains no generated declaration requirement.
- Asset Generator Task 19 completes that project's CI and release evidence.
  It has no bearing on animation correctness or release evidence.

If both projects are complete, generated textures work automatically through
the ordinary prepared-texture path. Neither project receives a special bridge.

## Task and testing conventions

Each task leaves the Rust workspaces and Unity project compiling. Public
behavior must work through an existing product boundary before the task is
marked done.

Use these boundaries:

- Public Reactant builders rendered through `battlement-fake` and
  `battlement-ui-fake`.
- Public animation protocol values serialized through the existing message
  codecs.
- Unity EditMode tests running `MotionWorld` with a controlled clock.
- The existing Reactant sample and Ditto scenario controls.
- Native macOS and desktop WebGL Release players for final manual performance.

Tests focus on complex behavior. Do not add one test for every simple builder
or every mockup source line. Do not expose private implementation state merely
to make a test convenient.

Stage every intended change before running `./scripts/ci.py`. Public Rust and
C# APIs receive concise documentation. The complete implementation receives one
independent review because it is a major work item.

## Task 01 — Implement animation data and canonical sampling

**Prerequisites:** none. **Asset Generator prerequisites:** none.

Define the typed values shared by Rust and Unity:

- the exhaustive animation-property metadata and interpolation category;
- `MotionStyle`, property keyframes, targets, transitions, repeats, and easing;
- normalized descriptors, stable slot identities, generations, and callbacks;
- lifecycle, playback, checkpoint, and controlled-clock protocol values; and
- renderer-capability validation for each supported value shape.

Implement the canonical Unity sampler for tweens, springs, inertia, discrete
segments, delays, negative delays, repetitions, seeking, and interruption.
Use the Motion 13.1.1 defaults and numerical contract from the design. Sample
physical generators from logical time rather than frame-dependent integration.

Add `MotionWorld` descriptor admission, generation-checked storage, clocks,
property writers, and pre-layout/post-layout PlayerLoop integration. A rejected
transaction must leave the previous host tree and motion state unchanged.
Steady sampling must allocate no managed memory.

Property writers include the mockup-required filter, rectangular clip, custom
quad deformation, polygon geometry, shadow, mask, shader, and prepared-texture
paths. Task 02 exposes them through typed styles and decorations; Task 06 only
verifies their composed behavior.

Create the Motion 13.1.1 conformance vectors and normalized Release-player
PlayerLoop topology in this task. Numeric vectors may be derived with temporary
local comparison code, but no JavaScript analyzer or runtime tool becomes a
product dependency. Capture the PlayerLoop topology from both reference player
profiles and check in only the stable fixtures needed by controlled tests.

**Black-box acceptance:** public protocol round trips preserve every descriptor
field. Controlled-clock Unity tests match the conformance vectors, retarget from
visible presentation values, retain compatible spring velocity, process missed
boundaries in order, and perform no second synchronous panel layout update.

**Evidence:** Rust codec results, focused Unity sampler results, representative
Motion conformance vectors, and an allocation-profiler capture.

## Task 02 — Add Reactant animation authoring

**Prerequisites:** Task 01. **Asset Generator prerequisites:** none.

Add the sealed `MotionHostExt` stage to every eligible Reactant host after
primitive properties, children, and events. Lower every adapter into the same
host node without adding a logical position or Unity `VisualElement`.

Implement:

- initial, animate, exit, and transition builders;
- typed scalar targets, keyframes, property overrides, and `transition_end`;
- typed CSS-style transitions and pseudo-state styles;
- reusable keyframe animations and their CSS playback controls;
- keyed decoration layers for non-interactive chrome and particles; and
- property ownership validation across Motion, transitions, and animations.

Add `MotionComponent` and `MotionComponentExt`. The component author receives
one complete `MotionProps` value and forwards it unchanged to exactly one stable
host. Missing, duplicate, or identity-changing forwarding is a developer error.

**Black-box acceptance:** public Rust tests prove host flattening, builder-stage
ordering, target serialization, pseudo-state precedence, animation restart and
fill behavior, decoration identity, property conflicts, and correct component
forwarding without wrapper hosts.

**Evidence:** public test results, compile-fail diagnostics for illegal builder
orders, and fake-client host trees showing unchanged hierarchy.

## Task 03 — Add variants and presence

**Prerequisites:** Task 02. **Asset Generator prerequisites:** none.

Implement typed variant names, static and computed targets, custom data,
ordered variant lists, logical propagation, opt-out, child delay, stagger, and
before/after-child orchestration.

Add `AnimatePresence` with `Sync` and `Wait` modes. Retain exiting component
state, hooks, effects, hosts, and logical ancestry until all finite exit tracks
and manual holds finish. Snapshot custom data and exit targets when removal
begins. Reserve the `PopLayout` public value here, but Task 05 makes it usable
after layout projection exists.

Implement lifecycle callbacks and playback completion as the supported way to
sequence removal. Mockup-style timeout duplication must not be necessary.
Reconnect, cancellation, ancestor removal, and runtime shutdown follow the
design's explicit terminal behavior.

**Black-box acceptance:** public tests cover propagation, computed directional
variants, orchestration, `Sync`, `Wait`, retained state, automatic exit, manual
holds, completion callbacks, stale generations, cancellation, and one final
unmount after reconnect. Selecting reserved `PopLayout` reports that layout
projection support is not installed rather than behaving partially.

**Evidence:** fake-client descriptor and host facts, ordered lifecycle event
records, and controlled presence scenario results.

## Task 04 — Add motion values and time sources

**Prerequisites:** Tasks 01–03. **Asset Generator prerequisites:** none.

Implement stable typed motion values, velocity, derived ranges, springs, and a
closed serializable expression graph. Expressions cover the arithmetic, clamp,
wrap, modulo, minimum, maximum, power, exponential, color, length, filter, and
transform operations required by the public design.

Evaluate dirty graph nodes once per Unity frame in topological order. A graph
may feed multiple hosts without duplicate Rust work. Explicit subscriptions
coalesce replaceable samples while preserving lifecycle boundaries.

Add `MotionTimeSource` for unscaled, scaled, controlled, and audio playhead
time. Introduce the minimal stable `AudioPlayback` identity shared by existing
audio stop and volume operations and by `MotionWorld`. Pausing or buffering
freezes playhead time; seeking, looping, and replacement are discontinuities
that do not carry velocity.

Implement animation controls, typed scopes, sequences, selectors, and playback
operations after the underlying values, named variants, and lifecycle messages
work publicly.

**Black-box acceptance:** tests cover graph identity, cycles, dirty propagation,
shared bindings, spring retargeting, subscription coalescing, imperative
replacement, typed selector snapshots, scaled and controlled clocks, and audio
pause, seek, loop, stop, and reconnect behavior.

**Evidence:** public Reactant and protocol results, Unity graph tests, and a
focused audio-playhead expression trace.

## Task 05 — Add gestures and layout animation

**Prerequisites:** Tasks 02–04. **Asset Generator prerequisites:** none.

Implement Unity-local hover, tap, focus, pan, drag, scroll, and in-view state.
Preserve pointer capture, device identity, gesture thresholds, variant layers,
callbacks, reduced-motion behavior, and input cancellation on reconnect.

Add drag constraints, direction locking, momentum, elastic bounds, external
drag controls, and reorder helpers. Drive continuous state through motion
values rather than Rust events.

Implement layout projection around Reactant commits, including position-only,
size-only, combined projection, scale correction, scroll roots, layout roots,
shared layout IDs, groups, portals within one panel, and presence handoffs.
Retarget from the currently visible projected bounds.

Complete `PopLayout` by moving exiting hosts into presence-owned projection
overlays. Add native macOS and WebGL reduced-motion bridges and apply live
policy changes across targets, gestures, drag momentum, scroll-linked motion,
layout projection, and reusable animations. Controlled tests synthesize touch
pointer events; physical touch hardware is not a manual release prerequisite.

**Black-box acceptance:** controlled Unity tests cover mouse, pen, keyboard,
gamepad, and touch-compatible pointer behavior; pan and drag thresholds;
momentum and constraints; scroll and viewport progress; one-layout-pass
projection; shared layout; interruption; presence handoff; and cross-panel
rejection.

**Evidence:** focused Unity results, public callback and motion-value records,
and Reactant sample state captures for gesture and layout flows.

## Task 06 — Verify every mockup-required animation family

**Prerequisites:** Tasks 03–05. **Asset Generator prerequisites:** none.

Review `~/Documents/mockups` at the pinned commit against the coverage ledger in
the design. Before review, require `git rev-parse HEAD` in that checkout to
equal `2451ea9cc6f76b356b1102ee37b82c478853122a`. If the local checkout is
absent or different, inspect that immutable GitHub commit without modifying the
user's checkout. This is a manual engineering review, not a source-analysis
tool or per-declaration fixture project.

Exercise the complex public capabilities needed by:

- dropdown entrance, option staggering, selection flash, and exit completion;
- modal backdrop, panel keyframes, filter mixing, shine, and retained removal;
- directional tab and route variants, `PopLayout`, beams, scans, and clipping;
- button, checkbox, and slider bursts with keyed decorations and particles;
- pseudo-state transitions for transform, filter, shadow, border, and color;
- infinite ambient grid, particle, comet, and binding-indicator animations;
- shared exit clocks and reduced-motion fallbacks; and
- the audio-synchronized control pulse built from audio time and expressions.

Use focused Rust tests or existing Reactant sample cases where they help prove
complex behavior. Use styles, custom geometry, shaders, or ordinary prepared
textures for static chrome. Do not port the settings screen and do not create a
manifest, analyzer, gallery framework, source hash, or coverage generator.

**Black-box acceptance:** the task evidence records every coverage-ledger entry
as supported by a named public API and records any focused test or sample case
used for complex behavior. Exercised cases preserve targets, times, easing,
repetition, interruption, presence, and reduced-motion behavior. No case
requires direct C# animation calls or generated assets.

**Evidence:** the completed manual ledger review, focused public test results,
and the smallest useful set of Reactant sample captures.

## Task 07 — Complete performance and release validation

**Prerequisites:** Task 06. **Asset Generator prerequisites:** none.

Add one Reactant manual performance screen containing the `transform-200`,
`mixed-200`, and mixed interaction scenarios defined by the design. Expose the
profiler counters needed to measure motion CPU time, frame pacing, active work,
managed allocation, and lifecycle traffic.

Exercise the screen in native macOS and desktop WebGL Release players. Run it
manually before completion and retain the environment details and profiler
captures. Keep the full run outside ordinary CI; an explicitly triggered job
may repeat it for releases or suspected regressions.

Record the release commit immediately before Task 01 as the project baseline.
Measure cached ordinary CI on that commit and on the final staged Task 07 tree
using the same machine and cache configuration. Warm each tree once, then
compare the median wall time of three unchanged-input runs. Parallelize or
remove redundant checks if the full project's added critical path exceeds 30
seconds. Finish public documentation, the Reactant feature ledger, the sample
entry, Manual QA, and the repository-mandated independent review.

**Black-box acceptance:** both reference profiles meet the design's CPU,
frame-pacing, and zero-allocation gates; cached ordinary CI grows by no more
than 30 seconds; public examples compile; and every Manual QA item passes.

**Evidence:** native and WebGL profiler captures, recorded environments, CI
timing comparison, documentation results, independent review, and Manual QA
record.

## Completion criteria

Reactant Animations is complete when all seven tasks are marked done and:

- Motion targets, transitions, values, gestures, presence, and layout work
  through public Rust builders without per-frame Rust execution;
- supported sampling matches the Motion 13.1.1 numerical contract;
- every mockup coverage-ledger entry has an obvious public Rust counterpart;
- audio-synchronized procedural motion uses the general time-source graph;
- native macOS and desktop WebGL meet the performance gates;
- ordinary cached CI remains within the 30-second added-time budget;
- no mockup-analysis or generated-asset dependency has been introduced; and
- the technical design, public documentation, and sample agree.

## Manual QA

The technical design's `Manual QA` section is authoritative. Complete every
item from a staged tree. At minimum:

1. Exercise every mockup animation family with motion enabled and reduced
   motion forced.
2. Interrupt entrances, exits, springs, layout projection, and gestures from
   visible intermediate states.
3. Verify modal and routed content remain mounted until completion and unmount
   exactly once.
4. Pause, buffer, seek, loop, and stop audio while observing the shared pulse.
   Confirm the visual value follows the playhead without per-frame Rust traffic.
5. Exercise mouse, keyboard, gamepad, pen, and touch-compatible pointer paths
   where supported by the reference hardware.
6. Disconnect and reconstruct during entrance, repetition, drag, pause, and
   exit.
7. Run the three manual performance scenarios in native macOS and desktop
   WebGL, retain the captures, and confirm every numeric gate.
