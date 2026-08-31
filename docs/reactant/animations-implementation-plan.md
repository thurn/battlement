# Reactant Animations implementation plan

When a task is complete, append `[DONE]` to its task heading.

Status: implementation companion to [`animations.md`](animations.md).

This plan implements the approved Reactant Animations contract as a sequence of
focused screens in the existing Reactant sample. Each screen is both a public
API example and a validation surface. The technical design remains normative.
If this plan and the design disagree, the design wins.

The work does not port the settings mockup or add mockup-analysis
infrastructure. It builds small, deterministic specimens for the animation
families that the mockup requires.

## Related information

- [`animations.md`](animations.md) defines the authoring, sampling, lifecycle,
  performance, and manual validation contract implemented here.
- [`reactant-technical-design.md`](reactant-technical-design.md) defines the
  session, commit, snapshot, and Rust-to-Unity boundaries extended by motion.
- [`reactant-implementation-plan.md`](reactant-implementation-plan.md) records
  the completed Reactant runtime, reconciliation, event, hook, and geometry
  prerequisites.
- [`host-facades.md`](host-facades.md) defines the host-ownership,
  order-independent authoring, and private-lowering prerequisite.
- [Reactant Asset Generator implementation plan][asset-plan] describes an
  independent project whose tasks are not prerequisites here.
- The [settings mockup][settings-mockup] at commit
  `2451ea9cc6f76b356b1102ee37b82c478853122a` is the manual feature checklist.
- Motion's [test guidance][motion-test-guidance] separates pure numerical tests
  from browser tests for rendered animation, gestures, scroll, and layout.
- Motion's [layout group tests][motion-layout-tests] measure initial,
  intermediate, interrupted, and final geometry.
- Motion's [frozen-progress arc tests][motion-arc-tests] make a specific
  intermediate frame inspectable instead of racing wall-clock time.
- Motion's [drag momentum tests][motion-drag-tests] use realistic pointer
  sequences and verify visible post-release displacement.
- Motion's [presence layout tests][motion-presence-tests] inspect the actual
  rendered bounds of retained and reflowed elements.

[settings-mockup]:
  https://github.com/thurn/mockups/tree/2451ea9cc6f76b356b1102ee37b82c478853122a
[asset-plan]: asset-generator-implementation-plan.md
[motion-test-guidance]:
  https://github.com/motiondivision/motion/blob/1b037b0032578b52af94b06ff3920bfa0aaa5e36/AGENTS.md
[motion-layout-tests]:
  https://github.com/motiondivision/motion/blob/1b037b0032578b52af94b06ff3920bfa0aaa5e36/packages/framer-motion/cypress/integration/layout-group.ts
[motion-arc-tests]:
  https://github.com/motiondivision/motion/blob/1b037b0032578b52af94b06ff3920bfa0aaa5e36/packages/framer-motion/cypress/integration/transition-arc.ts
[motion-drag-tests]:
  https://github.com/motiondivision/motion/blob/1b037b0032578b52af94b06ff3920bfa0aaa5e36/packages/framer-motion/cypress/integration/drag-momentum.ts
[motion-presence-tests]:
  https://github.com/motiondivision/motion/blob/1b037b0032578b52af94b06ff3920bfa0aaa5e36/packages/framer-motion/cypress/integration/animate-presence-pop.ts

## Decisions and starting point

The repository already contains the Reactant runtime, host façades, hooks,
events, portals, refs, geometry, prepared textures, Unity UI Toolkit client,
fake clients, controlled Ditto motion, and Reactant sample. The façades own
order-independent properties, children, handlers, keys, refs, and portal
targets, and privately lower to `Ui`-prefixed protocol hosts. The repository
does not contain Reactant motion descriptors, samplers, animation authoring,
presence, motion values, layout projection, or an audio playhead source.

The following decisions govern implementation:

- Motion 13.1.1 supplies the supported numerical and behavioral baseline.
- Rust declares animation state. Unity owns continuous sampling, gestures,
  layout projection, motion-value graphs, and final property application.
- The Reactant sampler is authoritative. UI Toolkit transitions are optional
  internal optimizations proven equivalent by controlled-clock tests.
- Concrete host façades receive inherent Motion builders without introducing
  a host stage. A custom component opts in by forwarding one `MotionProps`
  value to exactly one stable façade.
- No user-facing animation macro, animated host duplicate, runtime CSS string,
  arbitrary selector string, or per-frame Rust callback is added.
- Time, scroll, pointer, geometry, and audio playhead values are Unity-local
  inputs to a closed serializable expression graph.
- The Reactant sample gains focused animation screens. It does not gain a
  mirrored settings gallery, declaration manifest, TypeScript analyzer, source
  hash, or generated coverage system.
- Every screen uses only public Reactant builders for animation. Sample-only
  observation code may read the resulting Unity presentation, geometry,
  lifecycle messages, and profiler counters, but may not drive animation by
  calling private C# motion APIs.
- Fast deterministic checks run in ordinary CI and add less than 30 seconds to
  its cached critical path. Renderer captures, exhaustive scenario matrices,
  player builds, reconnect sweeps, and performance runs stay on demand.
- Native macOS and desktop WebGL are release targets. Mobile constrains input,
  allocation, threading, memory, and renderer choices but has no release gate.
- Generated assets are optional ordinary texture inputs. No task depends on
  the Reactant Asset Generator implementation plan.

## Sample and validation contract

Animation correctness cannot be established by seeing an element move once.
Every animation screen therefore combines an interactive specimen with
repeatable measurements of the real rendered result.

### Shared screen controls

Task 01 adds a validation strip used by every animation screen. A screen may
hide controls that do not apply, but it must not invent a separate clock or
capture mechanism.

The strip provides:

- `Reset`, which restores a documented baseline, clears lifecycle history, and
  restores the screen's deterministic seed;
- `Trigger`, which performs the screen's primary state transition;
- `Play`, `Pause`, and `Replay` for ordinary observation;
- a controlled-clock selector and exact `Seek` or `Step` controls;
- real-time speed choices including `0.1x`, `0.25x`, `1x`, and `4x`;
- a reduced-motion override that can return to the platform setting;
- a reconnect action that reconstructs the active session when relevant; and
- an evidence action that captures the current checkpoint and exports the
  screen's validation record.

Controlled mode is the primary correctness mode. Slow real time is a visual
aid and must never replace exact checkpoints.

### Independent observations

The validation record compares expected values with independent observations.
It must not merely repeat the sampler's internal value and call that proof.

Depending on the property, a checkpoint records:

- resolved opacity, color, filter, clip, transform, or paint inputs applied to
  the Unity host;
- rendered bounds, layout bounds, or relative geometry;
- current motion-value sample and velocity when an explicit public
  subscription is part of the scenario;
- host presence, retained component state, generation identity, and ordered
  lifecycle events;
- active timeline, graph, projection, and allocation profiler counters; and
- screenshots of the baseline, checkpoint, and completed or interrupted state
  when appearance is material.

Each expected value comes from the design, a checked-in Motion 13.1.1
conformance vector, or simple geometry written in the scenario. The probe reads
the rendered presentation after the frame has completed. It does not use the
expected value as an input to the animation.

### Checkpoint rules

Every finite specimen defines named checkpoints. The minimum useful set is:

```text
baseline -> immediately after trigger -> exact midpoint -> just before end
         -> exact end -> one frame after end
```

The screen captures each checkpoint once after the relevant frame. A validator
must not poll until a desired result eventually appears because that can hide a
wrong intermediate value or late completion. Tolerances are property-specific,
small, authored with the expectation, and never widened merely to make a case
pass.

Each scenario defines its controlled-time quantum. Sampler-only cases default
to `1ms`; rendered-frame cases default to `16.666667ms`. “Just before end”
means one scenario quantum before the exact boundary. “One frame after end”
means one rendered-frame quantum after it.

Logical `t=0` is the admitted descriptor's start before applying its delay. The
first rendered observation samples that same logical time after post-layout
property application. Tween and duration-spring midpoint means halfway through
one active iteration, excluding delay and repeat delay. A repeated-schedule
midpoint is used only when a case names it explicitly.

Negative delay advances local time before the first observation. Finite tween
end is the normalized schedule's exact completion time, including repeats and
repeat delays. A physical spring or bounded inertia case uses explicit vector
times and defines end as the first sample satisfying its normative rest rule;
the exact terminal sample must equal the target. Infinite motion has no end
checkpoint and instead proves phase at named iteration boundaries.

Default comparison rules are part of the scenario schema:

- conformance scalar and velocity channels use absolute error at most `1e-5`;
- other applied scalar and decomposed transform channels use absolute error at
  most `1e-4` in their authored units;
- rendered positions and bounds use absolute error at most `0.5px`;
- normalized color channels use absolute error at most `1 / 255` in the
  interpolation color space;
- discrete values, lifecycle identities, event order, mount counts, and
  cleanup counts require exact equality;
- controlled logical boundary times require exact integer-microsecond equality;
  and
- real-player event delivery may trail its logical boundary by at most one
  rendered-frame quantum, while retaining the exact logical timestamp.

Paint and shader cases compare their typed applied parameters with these scalar,
color, and geometry rules. Screenshots are supporting visual evidence, not a
pixel-diff oracle. A scenario may tighten a tolerance. Widening one requires a
documented design change, not an implementation-local exception.

Every applicable screen also defines interruption checkpoints:

- retarget at the exact midpoint;
- reverse or replay before completion;
- remove or reconnect while active; and
- change reduced-motion policy while mounted.

The record includes the pre-interruption rendered value and the first value
after interruption. Continuity checks compare those observations directly.

Cross-cutting cases also verify:

- direct seek, repeated seek, backward seek, and step-by-step playback produce
  the same value at the same logical time;
- delay, keyframe, repeat, and completion boundaries choose the documented side
  of the boundary exactly;
- valid zero-duration transitions and zero-length keyframe segments choose the
  later value at the boundary and preserve `transition_end`, presence cleanup,
  and callback ordering;
- several retargets or commits in one rendered frame preserve the last visible
  presentation and deterministic lifecycle order;
- transform order, units, color space, and multi-property composition match the
  property catalog;
- property ownership is restored correctly when a higher animation layer ends
  or is canceled;
- invalid durations, non-finite values, unsupported shapes, and graph cycles
  are rejected atomically; and
- long-running repeated motion does not accumulate phase or numerical drift.

### Fast CI lane

The fast lane protects deterministic behavior on every change. It runs through
ordinary repository CI and its total added cached critical path must remain
below 30 seconds.

It contains:

- Rust codec, normalization, builder, fake-client, lifecycle, and graph tests;
- Unity EditMode sampler tests under a controlled clock;
- exact scalar, velocity, geometry, and lifecycle checkpoints for a small
  representative subset of each screen;
- allocation assertions that are cheap and stable in EditMode;
- registry checks proving every animation screen and focused case remains
  reachable; and
- one negative self-test proving the validator rejects a wrong expectation.

The fast lane does not build Release players, capture screenshots, run broad
device matrices, wait on wall-clock animation, or run the 30-second performance
scenarios. Prefer virtual time and direct single-frame observation so duration
does not grow with authored animation length.

### Heavy on-demand lane

The heavy lane validates the behavior most likely to fool an agent or pass in
a fake client while appearing broken in a player. Task 01 adds one explicit
local command or Unity Editor action that runs a selected screen, named case,
or the complete matrix.

It drives the same public controls as a reviewer and produces a human-readable
record containing:

- build target, player version, graphics backend, resolution, and commit;
- scenario name, seed, clock mode, and reduced-motion policy;
- expected and observed checkpoint values with tolerances;
- lifecycle and mount/unmount traces;
- failed continuity, ordering, geometry, allocation, or timing checks; and
- paths to the smallest useful screenshot set and profiler captures.

This lane includes real players, all checkpoints, rendered presentation probes,
pointer sequences, resize and scroll cases, reconnect and reduced-motion
sweeps, long-running loops, and performance profiling. Tasks 01–10 run their
focused heavy cases in the native macOS player. They also run WebGL when the
task changes a platform-specific path. Task 11 performs the complete native and
WebGL parity sweep, and Task 12 runs performance in both players. The lane stays
outside `./scripts/ci.py`.

Task 11 checks the complete controlled checkpoint matrix from Tasks 02–11 in
both players. Device interactions unavailable in WebGL are exercised through
their controlled synthetic cases. A checked-in platform-exception list may
describe input-source availability, but it may not relax sampler, presentation,
geometry, lifecycle, audio-playhead, cleanup, or reduced-motion parity.

Both lanes use the same scenario identities, baselines, checkpoint
expectations, and tolerance definitions. The fast lane selects a small subset;
it does not maintain a second set of expected values.

A heavy run is not accepted until the implementer has viewed the screen and
read its record. A green exit code or a final-state screenshot alone is
insufficient evidence.

## Task dependencies

There are no hard dependencies on Asset Generator Tasks 01–19.

| Task | Sample screen or infrastructure | Internal prerequisites |
|---|---|---|
| 01 | Shared validation infrastructure | None |
| 02 | Canonical animation sampling | Task 01 |
| 03 | Targets & Timelines | Task 02 |
| 04 | Physical Motion | Task 03 |
| 05 | Styles & Decorations | Task 03 |
| 06 | Variants & Orchestration | Tasks 03–05 |
| 07 | Presence & Lifecycle | Task 06 |
| 08 | Values, Time & Controls | Tasks 04, 06–07 |
| 09 | Gestures & Drag | Tasks 04, 06, 08 |
| 10 | Layout & Reorder | Tasks 07–09 |
| 11 | Composed Effects | Tasks 05–10 |
| 12 | Motion Performance | Task 11 |

The independence from the Asset Generator is intentional. Animation specimens
use styles, authored prepared textures, custom geometry, and shaders through
their ordinary Reactant paths. Generated textures work automatically if the
other project is complete, but neither project receives a special bridge.

## Task and testing conventions

Each task leaves the Rust workspaces and Unity project compiling. Public
behavior must work through an existing product boundary and its sample screen
before the task is marked done.

Use these boundaries:

- public Reactant builders rendered through `battlement-fake` and
  `battlement-ui-fake`;
- public animation protocol values serialized through existing message codecs;
- Unity EditMode tests running `MotionWorld` with a controlled clock;
- the existing Reactant sample with the shared validation strip; and
- native macOS and desktop WebGL Release players for heavy validation.

Unit tests cover pure numerical and protocol behavior. Real-renderer validation
covers opacity, transforms, layout, scroll, gestures, presence, and advanced
paint. A passing fake-client test does not prove those features visually work.

For every screen task:

- add the screen, navigation entry, stable specimen names, reset behavior, and
  focused validation cases in the same task as the public capability;
- make the baseline and expected checkpoint values visible to a reviewer;
- add a small representative case to the fast lane;
- run the full focused case in the heavy lane and perform its real-time manual
  walkthrough;
- include at least one deliberately interrupted case;
- verify the completed state and cleanup, not only motion in progress; and
- retain the heavy validation record as review evidence.

Do not add one unit test for every simple builder or mockup source line. Do not
expose private production state merely to make a test convenient. Public Rust
and C# APIs receive concise documentation.

Stage every intended change before running `./scripts/ci.py`. Measure the fast
lane after each task that materially expands it rather than waiting until the
end to discover a budget overrun. The complete implementation receives one
independent review because it is a major work item.

Before Task 01 changes the tree, record the current release commit, machine,
cache state, and median of three warmed unchanged-input CI runs. Every later CI
timing comparison identifies this commit, but Task 12's paired measurement is
the authoritative budget result.

## Task 01 — Build shared animation validation infrastructure [DONE]

**Prerequisites:** none. **Asset Generator prerequisites:** none.

Add the shared validation strip, rendered-presentation probe, lifecycle trace,
checkpoint capture, scenario registry, fast-case selector, and heavy on-demand
runner to the Reactant sample.

Define the scenario and evidence contract shared by both validation lanes:

- stable screen, case, checkpoint, and deterministic seed identities;
- expected scalar, velocity, paint, geometry, lifecycle, and cleanup values;
- explicit clock quantum and property-specific tolerances;
- build, player, renderer, resolution, platform, and commit metadata; and
- machine-readable results plus a concise human-readable report.

Use sample-local static and deliberately failing fixtures to prove the runner,
probe, report, and failure path before animation implementation exists. The
fixtures validate observation infrastructure; they are not product animation
APIs. Task 03 removes the passing placeholder specimen, but the deliberately
wrong expectation remains as a permanent validator self-test.

**Black-box acceptance:** the validation strip can reset, select a case, seek,
capture one observation per checkpoint, export evidence, and show a failed
expectation. Fixture-backed actions prove `Trigger`, `Play`, `Pause`, `Replay`,
speed selection, reduced-motion override, and reconnect dispatch through the
shared control path; later tasks prove their animation semantics. The fast
selector and heavy runner resolve the same case identity and expected data. A
malformed case, missing checkpoint, duplicate identity, or unexplained
tolerance is rejected.

**Fast CI:** run registry/schema checks, one passing observation fixture, and
one negative self-test proving a wrong expectation fails. Record the initial
incremental cached runtime.

**Heavy validation:** run the same fixtures in a native player. Verify the
report metadata, rendered observation, screenshot path, and failure explanation
are accurate, then inspect both the passing and intentionally failing reports.

**Evidence:** schema and registry results, fast-lane timing, and one passing
plus one intentionally failing native-player record.

## Task 02 — Implement animation data and canonical sampling [DONE]

**Prerequisites:** Task 01. **Asset Generator prerequisites:** none.

Define the typed values shared by Rust and Unity:

- exhaustive animation-property metadata and interpolation categories;
- `MotionStyle`, property keyframes, targets, transitions, repeats, and easing;
- normalized descriptors, stable slot identities, generations, and callbacks;
- lifecycle, playback, checkpoint, and controlled-clock protocol values; and
- renderer-capability validation for each supported value shape.

Implement the canonical Unity sampler for tweens, springs, inertia, discrete
segments, delays, negative delays, repetitions, seeking, and interruption. Use
the Motion 13.1.1 defaults and numerical contract from the design. Sample
physical generators from logical time rather than frame-dependent integration.

Add `MotionWorld` descriptor admission, generation-checked storage, clocks,
property writers, and pre-layout/post-layout PlayerLoop integration. A rejected
transaction leaves the previous host tree and motion state unchanged. Steady
sampling allocates no managed memory.

Create Motion 13.1.1 conformance vectors and normalized Release-player
PlayerLoop topology fixtures. Temporary local comparison code may derive
vectors, but no JavaScript analyzer or runtime tool becomes a product
dependency.

**Black-box acceptance:** protocol round trips preserve every descriptor field.
Controlled-clock Unity tests match conformance vectors, retarget from visible
presentation values, retain compatible spring velocity, process missed
boundaries in order, and perform no second synchronous panel layout update. The
sampler produces identical values for direct seek, repeated seek, backward
seek, and step-by-step playback.

Before Task 02 is complete, pin Motion 13.1.1 seek side effects in the normative
design and conformance vectors. The contract must say whether each direct,
repeated, and backward seek emits or suppresses start, repeat, update, and
completion events; tests assert that behavior as well as the sampled value.

**Fast CI:** run representative tween, spring, repeat, discrete, interruption,
seek-equivalence, invalid-value, and transaction-rejection vectors entirely
under virtual time.

**Heavy validation:** use a protocol fixture to run a known linear animation in
a native player at `0ms`,
`250ms`, `500ms`, `999ms`, and `1000ms`. Verify expected and observed values
are independently recorded and the screen remains frozen at each checkpoint.
Task 03 replaces this temporary protocol fixture with public authoring.

**Evidence:** codec and sampler results, conformance vectors, fast-lane timing,
an allocation-profiler capture, and the native-player checkpoint record.

## Task 03 — Add authoring and the Targets & Timelines screen [DONE]

**Prerequisites:** Task 02. **Asset Generator prerequisites:** none.

Add inherent Motion builders and private motion state to every Reactant host
façade. Every façade specialization retains its ordinary
properties, children, events, key, ref, portal-target, and Motion methods, so
authors may interleave them freely. Lower motion with the façade into the same
host node without adding a logical position or Unity UI Toolkit
`VisualElement`.

Implement initial, animate, exit, transition, typed scalar targets, keyframes,
property overrides, and `transition_end`. Add `MotionComponent` and
`MotionComponentExt`; forwarding must preserve one complete `MotionProps` value
and exactly one stable host façade. Applying forwarded props does not restrict
later host methods.

Build the **Targets & Timelines** screen with small side-by-side specimens for:

- linear and eased tweens with obvious numeric endpoints;
- multi-keyframe times and per-property transition overrides;
- delays, negative delays, finite repeats, reverse, and mirror;
- discrete and structured interpolation; and
- retargeting an active transform and color from their visible presentation.

Every specimen exposes its expected value at named checkpoints. The screen
uses public builders only and replaces Task 02's protocol fixtures and Task
01's static observation fixtures.

**Black-box acceptance:** public Rust tests prove one-host lowering, equivalent
output across materially different cross-category host-method orders that
preserve repeatable-layer order, target serialization, transition behavior,
property validation, and component forwarding without wrapper hosts. The
rendered probe detects a wrong midpoint, keyframe boundary, repeat direction,
final value, or `transition_end` application.

**Fast CI:** exercise one public tween, keyframe boundary, repeat direction,
same-frame multi-retarget, and `transition_end` case through fake clients and
controlled EditMode sampling.

**Heavy validation:** pause every specimen at its midpoint and end, then
retarget transform and color at their midpoint. Confirm no jump between the
last pre-retarget and first post-retarget rendered observation. Replay at
`0.1x` only after exact observations pass.

**Evidence:** public permutation-test results, fake-client host trees and motion
descriptors, fast-lane timing, and baseline/midpoint/interrupted/final captures
and records.

## Task 04 — Add physical generators and the Physical Motion screen

**Prerequisites:** Task 03. **Asset Generator prerequisites:** none.

Complete spring and inertia authoring, velocity handoff, constraints, playback
speed, seeking, pause, resume, reverse, complete, stop, and cancel semantics.
Keep logical-time sampling authoritative across long frames and dropped-frame
catch-up.

Build the **Physical Motion** screen with:

- spring specimens for duration/bounce and physical parameter forms;
- underdamped, critically damped, and overdamped motion with visible targets;
- spring interruption that preserves compatible velocity;
- unconstrained and bounded inertia; and
- playback controls whose lifecycle distinctions remain visible in an event
  trace.

The screen plots sparse checkpoint markers rather than a per-frame Rust graph.
The markers come from explicit controlled-clock observations.

**Black-box acceptance:** controlled tests cover convergence, rest thresholds,
velocity handoff, long-frame stability, inertia constraints, and playback
terminal behavior. The rendered specimen agrees with scalar and velocity
conformance vectors at named times.

**Fast CI:** sample representative spring and inertia vectors at a few exact
times, including interruption and one dropped-frame jump. Assert terminal event
distinctions without waiting on wall-clock time.

**Heavy validation:** interrupt an underdamped spring while it travels toward
and away from its target. Confirm the new motion begins at the rendered
position and keeps signed velocity. Pause inertia, step twice, resume, catch it
with a new target, and verify no old momentum resumes.

**Evidence:** conformance results, fast-lane timing, checkpoint records,
lifecycle traces, and captures showing overshoot, interruption, and completion.

## Task 05 — Add CSS animation and the Styles & Decorations screen

**Prerequisites:** Task 03. **Asset Generator prerequisites:** none.

Implement:

- typed CSS-style transitions and pseudo-state styles;
- reusable keyframe animations and CSS playback controls;
- keyed decoration layers for non-interactive chrome and particles;
- fill, direction, iteration, delay, pause, restart, and composition behavior;
  and
- property ownership validation across Motion, transitions, and animations.

Add property writers and typed styles for filter, rectangular clip, custom quad
deformation, polygon geometry, shadow, mask, shader, and prepared textures.
Task 11 composes them into mockup-required effects.

Build the **Styles & Decorations** screen with pseudo-state transitions, one
finite fill example, independently phased infinite loops, keyed burst
decorations, and one representative specimen for each advanced paint path.

**Black-box acceptance:** tests cover pseudo-state precedence, animation
identity, restart, pause, fill, composition, decoration identity, property
conflicts, interpolation, and renderer-capability rejection. Rendered probes
confirm intermediate resolved values rather than relying on callbacks.

**Fast CI:** exercise one pseudo-state transition, finite fill, keyed restart,
property conflict, advanced property interpolation, and rejection case under
controlled time.

**Heavy validation:** hold hover, press, and focus separately; rapidly
alternate them; restart keyed bursts before exit; and leave loops running
through several iterations. Confirm pseudo-state priority, no synchronized loop
restart, one cleanup per burst generation, and no invisible decoration blocking
input.

**Evidence:** public tests, fast-lane timing, resolved-presentation records,
lifecycle traces, and captures of pseudo-state, advanced paint, and burst
states.

## Task 06 — Add variants and the Variants & Orchestration screen

**Prerequisites:** Tasks 03–05. **Asset Generator prerequisites:** none.

Implement typed variant names, static and computed targets, custom data,
ordered variant lists, logical propagation, opt-out, child delay, stagger, and
before/after-child orchestration.

Build the **Variants & Orchestration** screen around a parent list whose
children display their start and completion order. Include directional route
variants, nested propagation, one opt-out child, stagger in both directions,
and computed variants using snapshotted custom data.

**Black-box acceptance:** public tests cover propagation, computed directional
variants, list priority, orchestration, custom-data snapshots, and cancellation.
The screen record proves actual child presentation and lifecycle order at exact
boundaries.

**Fast CI:** cover propagation, one stagger order, opt-out, custom-data
snapshot, and reversal with virtual time and ordered lifecycle assertions.

**Heavy validation:** trigger both directions, reverse midway, and change
custom data after motion begins. Confirm already-started targets use the
snapshot, staggered children do not begin early, and reversing does not leave
mixed variant layers active.

**Evidence:** descriptor facts, fast-lane timing, ordered event records,
checkpoint captures, and the screen's orchestration record.

## Task 07 — Add presence and the Presence & Lifecycle screen

**Prerequisites:** Task 06. **Asset Generator prerequisites:** none.

Add `AnimatePresence` with `Sync` and `Wait` modes. Retain exiting component
state, hooks, effects, hosts, and logical ancestry until all finite exit tracks
and manual holds finish. Snapshot custom data and exit targets when removal
begins. Reserve `PopLayout`; Task 10 makes it usable after projection exists.

Implement lifecycle callbacks and playback completion as the supported way to
sequence removal. Reconnect, cancellation, ancestor removal, and shutdown
follow the design's explicit terminal behavior.

Build the **Presence & Lifecycle** screen with a modal, routed panel, retained
counter state, manual hold, nested exit, `Sync`, and `Wait`. Make mount count,
unmount count, retained-state value, and ordered lifecycle events visible.

**Black-box acceptance:** tests cover retained state, automatic exit, manual
holds, completion callbacks, stale generations, cancellation, reconnect, and
one final unmount. Reserved `PopLayout` reports that projection is unavailable
instead of behaving partially.

**Fast CI:** drive `Sync`, `Wait`, retained state, one manual hold, reconnect,
and stale-generation cancellation with controlled time. Assert visible host
facts plus terminal callback and unmount counts.

**Heavy validation:** close and reopen the modal rapidly, navigate during exit,
release a manual hold after its animation completes, and reconnect during an
exit. Verify rendered presence independently of the event trace, then verify
exactly one terminal event and one unmount.

**Evidence:** fake-client host facts, controlled presence results, fast-lane
timing, lifecycle and mount records, and retained/interrupted/final captures.

## Task 08 — Add values, clocks, and the Values, Time & Controls screen

**Prerequisites:** Tasks 04, 06–07. **Asset Generator prerequisites:** none.

Implement stable typed motion values, velocity, derived ranges, springs, and a
closed serializable expression graph. Evaluate dirty nodes once per Unity frame
in topological order. Explicit subscriptions coalesce replaceable samples while
preserving lifecycle boundaries.

Add unscaled, scaled, controlled, and audio-playhead time sources. Introduce the
minimal stable `AudioPlayback` identity shared by stop, volume, and motion.
Pause or buffering freezes playhead time; seek, loop, and replacement are
velocity discontinuities.

Implement animation controls, typed scopes, sequences, selectors, and playback
operations.

Build the **Values, Time & Controls** screen with:

- one source feeding several derived hosts without Rust rerenders;
- range, color, length, filter, transform, and spring expressions;
- a scroll-independent controlled-time specimen;
- an audio-synchronized pulse with pause, buffer, seek, loop, replace, and
  stop controls; and
- a scoped sequence whose selected hosts and lifecycle order are visible.

**Black-box acceptance:** tests cover graph identity, cycles, dirty propagation,
shared bindings, spring retargeting, subscription coalescing, imperative
replacement, selector snapshots, clock behavior, and audio discontinuities.
The sample confirms that unsubscribed per-frame Rust traffic remains zero.

**Fast CI:** cover one shared graph, cycle rejection, dirty evaluation,
coalescing, selector snapshot, controlled clock, and audio pause/seek/loop
sequence without decoding or playing a long audio asset.

**Heavy validation:** compare hosts derived from the same source, pause each
clock, seek across a loop boundary, and replace audio while the pulse is active.
Confirm rendered values freeze or jump exactly as specified, velocity resets at
discontinuities, and lifecycle boundaries are never lost among coalesced
samples. Compare the motion playhead with the native audio playhead at start,
pause, seek, loop, and completion within the design's platform tolerance.

**Evidence:** graph results, transport counters, fast-lane timing,
audio-playhead traces, checkpoint records, and discontinuity captures.

## Task 09 — Add gestures and the Gestures & Drag screen

**Prerequisites:** Tasks 04, 06, 08. **Asset Generator prerequisites:** none.

Implement Unity-local hover, tap, focus, pan, drag, scroll, and in-view state.
Preserve pointer capture, device identity, thresholds, variant layers,
callbacks, reduced-motion behavior, and cancellation on reconnect.

Add drag constraints, direction locking, momentum, elastic bounds, external
drag controls, snap-to-cursor, snap-to-origin, and reorder prerequisites. Drive
continuous state through motion values rather than Rust events.

Build the **Gestures & Drag** screen with device-state indicators, threshold
guides, a constrained drag field, momentum catch-and-release, external drag
controls, a scroll-progress specimen, and an in-view specimen.

**Black-box acceptance:** controlled Unity tests cover mouse, pen, keyboard,
gamepad, and touch-compatible pointer behavior; thresholds; constraints;
momentum; scroll offsets; viewport progress; and cancellation. Geometry probes
verify visible displacement, not only callback payloads.

**Fast CI:** synthesize representative pointer sequences around the pan and
drag thresholds, one constrained momentum release, scroll progress, and
reconnect cancellation. Observe exact geometry at controlled frames.

**Heavy validation:** approach every threshold from below and above, release
with zero and high velocity, catch active momentum, resize constraints, scroll
both directions, and reconnect during pointer capture. Confirm the visible
element, motion values, and callbacks agree and no canceled gesture resumes.

**Evidence:** callback records, rendered geometry records, fast-lane timing,
pointer lifecycle traces, and threshold, constraint, and momentum captures.

## Task 10 — Add projection and the Layout & Reorder screen

**Prerequisites:** Tasks 07–09. **Asset Generator prerequisites:** none.

Implement layout projection around Reactant commits, including position-only,
size-only, combined projection, scale correction, scroll roots, layout roots,
shared layout IDs, groups, portals within one panel, and presence handoffs.
Retarget from currently visible projected bounds.

Complete `PopLayout` by moving exiting hosts into presence-owned projection
overlays. Add reorder helpers on top of gesture and projection behavior.

Build the **Layout & Reorder** screen with an expander, grid resize, shared tab
indicator, shared-element handoff, scroll-root case, reorder list, and
`PopLayout` removal. Each case displays known start and final geometry.

**Black-box acceptance:** controlled tests cover one-layout-pass projection,
scale correction, shared layout, interruption, presence handoff, portals, and
cross-panel rejection. Include nested projecting transforms and a child already
running a transform animation. Checkpoints compare actual rendered bounds with
start, midpoint, and final expectations.

**Fast CI:** cover one position, size, shared-layout, interruption,
`PopLayout`, and cross-panel case at frozen controlled checkpoints. Avoid real
resize loops or wall-clock waits.

**Heavy validation:** freeze each case at its midpoint, verify it is at neither
endpoint, then interrupt back toward the start. Reorder while a parent projects,
remove a `PopLayout` item, and resize a scroll root. Confirm continuity,
neighbor reflow, scale-corrected children, and final unprojected geometry.

**Evidence:** focused Unity results, fast-lane timing, geometry records,
lifecycle traces, and baseline/midpoint/interrupted/final captures.

## Task 11 — Validate composed effects in the Composed Effects screen

**Prerequisites:** Tasks 05–10. **Asset Generator prerequisites:** none.

Build the **Composed Effects** screen from the complex public capabilities used
by the pinned settings mockup:

- dropdown entrance, option staggering, selection flash, and exit completion;
- modal backdrop, panel keyframes, filter mixing, shine, and retained removal;
- directional tabs and routes, `PopLayout`, beams, scans, and clipping;
- button, checkbox, and slider bursts with keyed decorations and particles;
- pseudo-state transform, filter, shadow, border, and color transitions;
- ambient grid, particle, comet, and binding-indicator loops; and
- the audio-synchronized control pulse built from audio time and expressions.

Add native macOS and WebGL reduced-motion bridges and apply live policy changes
across targets, gestures, drag momentum, scroll-linked motion, layout
projection, and reusable animations. Add reconnect cases for phase
reconstruction and terminal cleanup.

Review the pinned mockup manually against the design's coverage ledger. Require
the local checkout to match the pinned commit or inspect the immutable GitHub
commit. Record every ledger entry as supported by a named public API and a
named sample specimen. Do not create an analyzer, manifest, source hash,
mirrored gallery, or per-declaration fixture project.

**Black-box acceptance:** every coverage-ledger entry maps to a public API and
sample case. Composed cases preserve targets, times, easing, repetition,
interruption, presence, and reduced-motion behavior. No case uses direct C#
animation calls or generated assets. Reconnect records show correct phase,
generation, and one terminal cleanup.

**Fast CI:** select one finite composition, one ambient loop, one live
reduced-motion change, and one reconnect case. Assert their exact controlled
checkpoints and cleanup without capturing screenshots or building players.

**Heavy validation:** run the Composed Effects screen and the complete
controlled checkpoint matrix from Tasks 02–11 in native and WebGL players with
motion enabled and reduced motion forced. Interrupt each finite family, leave
every ambient family running, and reconnect during entrance, loop, pause, drag,
and exit. Include audio-playhead checkpoints in both players. Inspect all
failed and borderline measurements; do not accept the work based on a montage
of final screenshots. Require observations to agree within shared tolerances,
except for input availability in the checked-in platform-exception list.

**Evidence:** completed coverage review, named records, focused test results,
fast-lane timing, lifecycle and reconnect traces, and useful player captures.

## Task 12 — Add the Motion Performance screen and complete validation

**Prerequisites:** Task 11. **Asset Generator prerequisites:** none.

Build the **Motion Performance** screen with the `transform-200`, `mixed-200`,
and mixed interaction scenarios from the design. Expose counters for motion CPU
time, frame pacing, active work, managed allocation, graph evaluation,
properties applied, and lifecycle traffic.

Exercise the screen in native macOS and desktop WebGL Release players. Keep the
full run in the heavy on-demand lane. It may be repeated for releases or
suspected regressions but is not added to ordinary CI.

Use the release commit and environment recorded before Task 01 as the baseline.
For the authoritative comparison, create isolated baseline and final worktrees
on the same otherwise-idle machine. Give each its own cache directory, prepared
from the same cache seed. Warm each worktree once without timing it, then run
the unchanged-input CI command in alternating final/baseline order three times
per revision. Compare medians of those three timed runs.

If the measured increase is within three seconds of the 30-second limit, repeat
the six alternating runs and use the median of all six results per revision.
The increase must remain strictly below 30 seconds. Record worktree commits,
cache seed, run order, individual times, medians, machine load, and tool
versions. Parallelize or remove redundant fast cases if the gate fails.

Finish public documentation, the Reactant feature ledger, sample navigation,
manual QA records, and the repository-mandated independent review.

**Black-box acceptance:** both reference profiles meet the design's CPU,
frame-pacing, and zero-allocation gates; cached ordinary CI grows by less than
30 seconds; public examples compile; every screen's focused record passes; and
every Manual QA item below has recorded evidence.

**Fast CI:** retain only a short virtual-time structural smoke case that proves
the scenario builds the intended host, graph, subscription, and timeline counts.
Do not assert real performance from EditMode or a shortened sample.

**Heavy validation:** run all three scenarios after five seconds of warm-up.
Inspect the complete 30-second sample rather than a favorable interval. Verify
counters return to baseline after leaving the screen and no lifecycle traffic
exists without explicit subscriptions or boundaries.

**Evidence:** native and WebGL profiler captures, recorded environments, CI
timing comparison, screen records, documentation results, independent review,
and the final Manual QA record.

## Completion criteria

Reactant Animations is complete when all twelve tasks are marked done and:

- every public animation family has a focused Reactant sample screen;
- every screen can be reset, controlled, observed, interrupted, and validated
  without private C# animation calls;
- the fast deterministic suite covers every screen and adds less than 30
  seconds to ordinary cached CI;
- the heavy on-demand runner validates real native and WebGL presentation,
  interactions, reconnect behavior, and performance;
- Motion targets, transitions, values, gestures, presence, and layout work
  through public Rust builders without per-frame Rust execution;
- supported sampling matches the Motion 13.1.1 numerical contract;
- rendered checkpoints prove intermediate and terminal presentation, not only
  descriptor construction or callback delivery;
- every mockup coverage-ledger entry has a named public Rust counterpart and
  sample specimen;
- audio-synchronized procedural motion uses the general time-source graph;
- native macOS and desktop WebGL meet the performance gates;
- no mockup-analysis or generated-asset dependency has been introduced; and
- the technical design, public documentation, sample, and validation records
  agree.

## Manual QA

Run the following from a staged tree. Use a native macOS Release player for the
complete interactive pass. Run the complete controlled checkpoint matrix in a
desktop WebGL Release player, then repeat WebGL-specific interactive and
performance items. Export the heavy validation record and read every failure
or tolerance warning.

1. Open each animation screen, press `Reset`, and verify its documented static
   baseline before triggering motion. A screen that starts in an unknown phase
   fails the pass.
2. Run **Targets & Timelines** under controlled time. Capture the baseline,
   exact midpoint, exact end, and one frame after end. Interrupt transform and
   color at the midpoint and compare the two continuity observations.
3. Run **Physical Motion** through overshoot and settling. Interrupt springs in
   both velocity directions; pause and catch inertia; distinguish complete,
   stop, and cancel in the lifecycle trace.
4. Run **Styles & Decorations** with hover, press, focus, and rapid changes.
   Restart bursts before cleanup and observe ambient loops for at least three
   iterations. Confirm keyed generations do not leak or synchronize.
5. Run **Variants & Orchestration** in both directions. Verify child start and
   completion order, opt-out, custom-data snapshots, reversal, and cancellation.
6. Run **Presence & Lifecycle** in `Sync` and `Wait`. Reopen during exit,
   release a manual hold, remove an ancestor, and reconnect during exit.
   Confirm visible retention, one terminal callback, and one unmount.
7. Run **Values, Time & Controls** with controlled, scaled, and audio time.
   Pause, buffer, seek, loop, replace, and stop audio. Confirm the pulse follows
   the playhead, discontinuities reset velocity, and unsubscribed motion sends
   no per-frame Rust traffic.
8. Run **Gestures & Drag** using mouse, keyboard, gamepad, pen, and
   touch-compatible pointer paths where supported. Exercise both sides of each
   threshold, constraints, momentum catch, scroll, in-view, and cancellation.
9. Run **Layout & Reorder** at frozen midpoints. Check rendered bounds, child
   scale correction, shared handoff, scroll roots, `PopLayout`, reorder, and
   interruption back toward the initial layout.
10. Run every **Composed Effects** specimen with motion enabled and reduced
    motion forced. Check target values, keyframe order, timing, easing, repeats,
    fill, final state, input continuity, and cleanup against the coverage
    ledger.
11. Toggle the native macOS preference and WebGL
    `prefers-reduced-motion` while specimens are mounted. Confirm live spatial
    suppression and retained opacity or color feedback without restarting
    unrelated tracks.
12. Disconnect and reconstruct during entrance, repetition, drag, pause, and
    exit. Confirm phase restoration, no replayed completed entrance, canceled
    pointer ownership, and exactly one final unmount.
13. Run `transform-200`, `mixed-200`, and the mixed interaction scenario on
    both reference profiles. Confirm CPU p95 below `4ms`, at least `59` average
    fps, interval gates, zero steady-state managed allocations, and the exact
    lifecycle traffic bound. Retain environment details and profiler captures.
14. Review exported records beside their screenshots. Reject any case where
    only the final frame is correct, an intermediate observation was retried
    until it passed, the event trace disagrees with rendered state, a tolerance
    is unexplained, or the screen cannot return to baseline after cleanup.
