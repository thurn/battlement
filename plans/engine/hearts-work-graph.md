# Hearts native implementation and introspection graph

Epic: **hv-a8o**. Planning/delivery bead: **hv-qn6**. Asset-scope revision: **hv-faq**.
Visual-pass planning: **hv-dz8**.

[Design and contracts](hearts.md) · [Blind React reference](hearts-react-reference.md)

## Execution hold

**hv-a8o.1** is the execution-authorization gate and depends on the planning bead.
The user clarified that the present task is detailed planning and filing only.
The epic, gate, all implementation/introspection tasks, and final acceptance are
natively deferred without expiry. Closing hv-qn6 does not release implementation.
After a later explicit execution instruction, record it in the gate, resolve any
design questions, and release statuses deliberately while retaining dependencies.
Do not infer permission from elapsed time, an approved direction, or task creation.

## Dependency contract

The serial backbone is AUTHORIZE → H01 → R01 → … → H29 → R29 →
V01 → RV01 → V02 → RV02 → V03 → RV03 → H30 → R30 → ACCEPT.
Every implementation (H or V) has a dedicated introspection (R or RV), which
blocks the next implementation. Each has its own native bead.
Keys identify assignments; native IDs are the handles for claiming and evidence.
Split oversized work before execution and preserve the implementation/review chain.
Required repair beads and their own introspection block the parent review.

Every R/RV task must also file concrete native follow-up beads for larger-scope
architecture, tooling, or library findings, or attach evidence to matching
existing work. Record actual IDs, problem evidence, scope/non-goals, outcome,
acceptance, validation, priority, project, origin, prerequisites and discovery
links to the exposing implementation/review. Broader follow-ups stay independently
deferred unless explicitly authorized; they block Hearts only when a stated
acceptance criterion requires them. Completion notes identify each disposition
or explicitly record no broader findings. Prose TODOs are not filed follow-ups.

The [design's introspection contract](hearts.md#introspection-reuse-and-delivery)
owns the five mandatory questions and scope-expansion rules. Each R/RV bead includes
those requirements and a task-specific focus. Do not mark an R/RV complete merely
because a follow-up was filed when its fix is required for current acceptance.

All H/R and V/RV assignments and follow-up dispositions obey the
[existing asset boundary](hearts.md#existing-asset-boundary). Asset production or
acquisition is excluded except simple deterministic primitive geometry. Use
existing-pool substitutions for thematic gaps; retain required engine work.

## Assignment index

| Key | Implementation | Introspection | Prerequisite | Assignment |
| --- | --- | --- | --- | --- |
| [H01](#h01) | hv-a8o.2 | hv-a8o.3 | hv-a8o.1 | Establish the Hearts workflow and platform baseline |
| [H02](#h02) | hv-a8o.4 | hv-a8o.5 | hv-a8o.3 | Repair measured warm CI and tool workflow bottlenecks |
| [H03](#h03) | hv-a8o.6 | hv-a8o.7 | hv-a8o.5 | Implement Hearts cards, deterministic deals, and legal choices |
| [H04](#h04) | hv-a8o.8 | hv-a8o.9 | hv-a8o.7 | Implement Hearts transitions, scoring, and private observations |
| [H05](#h05) | hv-a8o.10 | hv-a8o.11 | hv-a8o.9 | Add revision-guarded Reactant reducer sessions |
| [H06](#h06) | hv-a8o.12 | hv-a8o.13 | hv-a8o.11 | Expose a bounded cancellable computation lane |
| [H07](#h07) | hv-a8o.14 | hv-a8o.15 | hv-a8o.13 | Expose presentation-settled receipts and scoped recovery |
| [H08](#h08) | hv-a8o.16 | hv-a8o.17 | hv-a8o.15 | Add use_task and Hearts controller composition |
| [H09](#h09) | hv-a8o.18 | hv-a8o.19 | hv-a8o.17 | Add asynchronous atomic native persistence |
| [H10](#h10) | hv-a8o.20 | hv-a8o.21 | hv-a8o.19 | Add acknowledged IndexedDB persistence for desktop WebGL |
| [H11](#h11) | hv-a8o.22 | hv-a8o.23 | hv-a8o.21 | Prepare Hearts owned assets and a runnable sample shell |
| [H12](#h12) | hv-a8o.24 | hv-a8o.25 | hv-a8o.23 | Compose the forest scene and reference camera |
| [H13](#h13) | hv-a8o.26 | hv-a8o.27 | hv-a8o.25 | Compose stable cards, hand fans, and captured destinations |
| [H14](#h14) | hv-a8o.28 | hv-a8o.29 | hv-a8o.27 | Implement passing, pointer selection, and drag-to-play |
| [H15](#h15) | hv-a8o.30 | hv-a8o.31 | hv-a8o.29 | Implement keyboard/controller navigation and modal focus |
| [H16](#h16) | hv-a8o.32 | hv-a8o.33 | hv-a8o.31 | Add lifecycle-owned audio and ambient playback |
| [H17](#h17) | hv-a8o.34 | hv-a8o.35 | hv-a8o.33 | Make ambient particles deterministic and lifecycle-owned |
| [H18](#h18) | hv-a8o.36 | hv-a8o.37 | hv-a8o.35 | Animate complete Hearts deals, passes, and tricks |
| [H19](#h19) | hv-a8o.38 | hv-a8o.39 | hv-a8o.37 | Implement bounded information-consistent AI sampling |
| [H20](#h20) | hv-a8o.40 | hv-a8o.41 | hv-a8o.39 | Implement fair rollout opponents through shared rules |
| [H21](#h21) | hv-a8o.42 | hv-a8o.43 | hv-a8o.41 | Complete match screens, contextual teaching, and settings |
| [H22](#h22) | hv-a8o.44 | hv-a8o.45 | hv-a8o.43 | Integrate autosave, recovery, and foreground/background lifecycle |
| [H23](#h23) | hv-a8o.46 | hv-a8o.47 | hv-a8o.45 | Finish portrait, safe-area, and cross-input ergonomics |
| [H24](#h24) | hv-a8o.48 | hv-a8o.49 | hv-a8o.47 | Add reproducible native iOS device delivery |
| [H25](#h25) | hv-a8o.50 | hv-a8o.51 | hv-a8o.49 | Add reproducible native Android delivery |
| [H26](#h26) | hv-a8o.52 | hv-a8o.53 | hv-a8o.51 | Validate desktop WebGL Hearts compatibility |
| [H27](#h27) | hv-a8o.54 | hv-a8o.55 | hv-a8o.53 | Migrate chess music to shared audio lifecycle |
| [H28](#h28) | hv-a8o.56 | hv-a8o.57 | hv-a8o.55 | Migrate a focused chess opponent path to shared tasks |
| [H29](#h29) | hv-a8o.58 | hv-a8o.59 | hv-a8o.57 | Refine Hearts art, audio, and measured performance |
| [V01](#v01) | hv-a8o.63 | hv-a8o.64 | hv-a8o.59 | Compare the scene to the reference and improve it — pass 1 |
| [V02](#v02) | hv-a8o.65 | hv-a8o.66 | hv-a8o.64 | Compare the scene to the reference and improve it — pass 2 |
| [V03](#v03) | hv-a8o.67 | hv-a8o.68 | hv-a8o.66 | Compare the scene to the reference and improve it — pass 3 |
| [H30](#h30) | hv-a8o.60 | hv-a8o.61 | hv-a8o.68 | Assemble final automated evidence and mobile review packet |

## Detailed assignments

These are bounded assignments, not a license for oversized commits. Read only
the relevant implementation and skills before executing. Each task inherits the
design contracts, repository validation/review policy, and certified delivery.
Its completion is followed by the paired introspection before dependent work.

### H01

**Establish the Hearts workflow and platform baseline** — hv-a8o.2; review hv-a8o.3.

**Scope and interface:** Inspect actual engine capabilities, owned deck/forest/audio inputs, generated-asset routes, current full warm CI, and native mobile delivery prerequisites. Record runner/cache conditions, exact CI handle/source, existing asset/license inventory, and iOS signing/Android SDK/install matrix. Identify usable existing-pool substitutions after listening/inspection; do not file asset-production or acquisition prerequisites. Do not treat a simulator as a physical-device proof.

**Acceptance:** Retained stage timings and a concrete bottleneck list distinguish warm changed-source work from cold setup and queueing. All 52 faces/back and forest sources are accounted for. Unknown signing/device prerequisites have a named user/tooling owner. Subsequent CI repairs are bounded from evidence.

**Validation:** Existing CI supervisor/performance report, build and asset metadata inspection, native mobile toolchain preflight; run no substitute ad hoc CI copies.

**Introspection focus:** Which baseline facts were difficult to discover? Which CLI diagnostics, timing reports, or asset inventory utilities would remove repeated investigation?

### H02

**Repair measured warm CI and tool workflow bottlenecks** — hv-a8o.4; review hv-a8o.5.

**Scope and interface:** Repair measured bottlenecks under the [five-minute CI contract](hearts.md#five-minute-ci-contract). Optimize shared setup, exact-input caching, concurrency and test selection; consolidate or remove redundant, obsolete or low-value tests with an explicit risk and coverage rationale. Retain replay evidence and required behavior contracts. Continue focused repair and remeasurement without building another orchestration layer.

**Acceptance:** Observed failures have focused reproduction and a fix. Full representative warm changed-source sample, engine and host validation meets 300 seconds, with queue/cold costs reported separately. An over-budget passing run leaves H02 active for optimization, not blocked or complete; neither retry counts nor diagnosis time boxes justify stopping the epic. Dependent features wait while prerequisite repair continues. Only the concrete external-dependency or explicit-pause exceptions in the linked contract permit a stopped handoff.

**Validation:** Focused regression for changed tooling, then staged full validation with timings and exact source; test cache invalidation if caching changes.

**Introspection focus:** Why did the workflow exceed the budget or obscure failure? Remove repeated setup/manual retries and show the before/after timing and coverage rationale.

### H03

**Implement Hearts cards, deterministic deals, and legal choices** — hv-a8o.6; review hv-a8o.7.

**Scope and interface:** Create the domain-only sample rules workspace/module boundary. Implement stable rank/suit identities, seats, zones, seeded Fisher–Yates with resumable PRNG state, pass direction, legal pass/card choices, and readable rejection reasons. Keep all host/audio assets out of rules.

**Acceptance:** A seeded deal conserves 52 cards and 13 per hand. Fixed tests cover two-clubs opening, follow suit, forced first-trick penalties, unbroken/all-hearts restrictions, queen exception, and invalid intentions without state mutation. Serialization round-trip preserves random continuation.

**Validation:** Sample-manifest black-box tests using explicit hands/deals and deterministic scripted inputs; no Unity build needed for these pure contracts.

**Introspection focus:** How naturally can a React developer express pure state and derive legal actions? Identify reusable validation/selector needs without moving Hearts rules into Reactant.

### H04

**Implement Hearts transitions, scoring, and private observations** — hv-a8o.8; review hv-a8o.9.

**Scope and interface:** Add SubmitPass, PlayCard, NextHand and shared live/no-op presentation sinks. Commit passing atomically; expose complete-trick before collection/scoring checkpoints. Add immutable HumanView/AiObservation projections and known-pass/void tracking. Complete hand/match scoring and shared wins.

**Acceptance:** All pass cycles, full hands, moon/100-point/tie cases and conservation pass. Simulation and live sinks reach identical final state while simulation allocates no display snapshots. Changing inaccessible hands leaves observations unchanged. Pending pass references do not duplicate ownership.

**Validation:** Explicit rare-deal black-box scenarios, live/no-op equivalence, projection privacy and seeded multi-hand replay.

**Introspection focus:** Compare transition plus events to the blind reducer. Remove library-driven prompt/context boilerplate while keeping observations and scoring game-owned.

### H05

**Add revision-guarded Reactant reducer sessions** — hv-a8o.10; review hv-a8o.11.

**Scope and interface:** Build use_game_reducer and a typed adapter over the existing Game/session/publication path. Expose stable handle, lazy session initializer, accepted/presented observations, readiness, and session/revision guarded dispatch. Reuse queue capacity and final-output submission acceptance.

**Acceptance:** Started/busy/stale/rejected outcomes are unambiguous. Intermediate outputs never accept state; final submission advances state/revision exactly once. Old callbacks cannot mutate replacement sessions. Bounded checkpoint construction and downstream backpressure work without a second queue.

**Validation:** Public display-driver tests for initialization, duplicate/stale/busy input, capacity, submission failures, replacement, and accepted-versus-presented state.

**Introspection focus:** Does this feel like useReducer plus effects? Compare caller setup to the blind hook and eliminate adapter/session plumbing that remains in game code.

### H06

**Expose a bounded cancellable computation lane** — hv-a8o.12; review hv-a8o.13.

**Scope and interface:** Generalize existing worker primitives for one component-owned computation lane separate from rules actions. Add cooperative cancellation tokens, result delivery, failure reporting, and bounded active/latest-replacement ownership. Respect finite native/WebGL worker resources.

**Acceptance:** At most one active Hearts computation plus latest replacement exists. Cancellation is immediate at ownership boundary and cleanup cooperates at deterministic work checkpoints. No UI joins, orphaned threads, or rules-slot starvation. Old results are ignored.

**Validation:** Worker behavior tests including long bounded jobs, replace/cancel/fail/unmount, observer lifecycle and finite-pool platform proof.

**Introspection focus:** Which lifecycle responsibilities correspond to React effect cleanup? Ensure callers do not hand-build generation flags, thread registries, or result mailboxes.

### H07

**Expose presentation-settled receipts and scoped recovery** — hv-a8o.14; review hv-a8o.15.

**Scope and interface:** Add public settled observation keyed to session/revision over current motion/output ownership. Distinguish output submission from native completion, and include blocking card movement while excluding infinite ambience. Handle pause, failed output, replacement and recovery from accepted state.

**Acceptance:** Next gameplay action cannot race unfinished presentation. Submission is not completion; cancelled/failed receipts never unblock a replacement incorrectly. Pause retains the same occurrence. Recovery renders accepted state without old transient events. Failure before final output retains prior state; failure after acceptance retains newly accepted state/save eligibility. Recovery never rolls accepted rules backward.

**Validation:** Public-driver/host tests for completion ordering, nonblocking ambient effects, pause/restart/failure, duplicate acknowledgments and no timer polling. Cover host failure both before and after final output acceptance.

**Introspection focus:** Why does game code need to reason about rendering acknowledgments? Make sequencing declarative and eliminate arbitrary sleeps/booleans from the controller.

### H08

**Add use_task and Hearts controller composition** — hv-a8o.16; review hv-a8o.17.

**Scope and interface:** Expose keyed use_task idle/pending/ready/failed states and cleanup over H06. Compose use_hearts with reducer handle, view selectors and observation-keyed task scheduling. Use a deterministic legal stub policy until rollout AI lands. Guard dispatch against session/revision and presented readiness.

**Acceptance:** Unrelated rerenders do not restart work; disabled/paused keys cancel. Secret pass submission does not invalidate an unchanged acting observation. Results are revalidated at dispatch. Menus remain responsive, and derived legal state is not copied through effects.

**Validation:** Component/public-driver tests for dependency changes, state ownership, cancellation/retry, stale results, and full scripted hand with stub opponents.

**Introspection focus:** Compare hook/component tree directly with useHearts/useComputerTurn. Remove local orchestration state the library can safely own.

### H09

**Add asynchronous atomic native persistence** — hv-a8o.18; review hv-a8o.19.

**Scope and interface:** Introduce typed async persistence state and one ordered save owner per slot. Implement immutable request capture, coalescing, cross-session ordering, nonblocking completion, same-directory temp write, file synchronization, atomic replacement, and directory sync where supported.

**Acceptance:** Loading/pending/committed/error are distinct. Old writes/completions cannot replace newer saves or contaminate new-session UI. Failed writes preserve prior valid data; retry targets latest desired state. Render/UI threads do not perform blocking file I/O.

**Validation:** Fault-injected backend and filesystem tests at write/sync/replace boundaries, coalescing/session races, termination before/after commit, and absent/corrupt loads.

**Introspection focus:** Would a React developer need a manual write queue or filesystem handle? Keep durable semantics reusable and caller-facing state simple.

### H10

**Add acknowledged IndexedDB persistence for desktop WebGL** — hv-a8o.20; review hv-a8o.21.

**Scope and interface:** Implement the browser backend through the shared host bridge with asynchronous initialization/read and transactional full-record replacement. Deliver success only after IndexedDB transaction completion; use the same typed hook and ordered slot semantics as native.

**Acceptance:** Reload restores the latest complete commit. Denial/quota/transaction failures are recoverable and preserve prior data. New-game/old-write races match native semantics. No in-memory filesystem acknowledgment is mistaken for durability.

**Validation:** Focused bridge tests and desktop-browser transaction/reload/failure checks selected through web contracts; retained exact build evidence.

**Introspection focus:** Is platform behavior hidden behind one understandable hook without hiding pending/failure states? Remove browser-specific persistence glue from sample code.

### H11

**Prepare Hearts owned assets and a runnable sample shell** — hv-a8o.22; review hv-a8o.23.

**Scope and interface:** Create standalone Unity/Reactant sample configuration and generated asset declarations. Import existing KayKit forest/card meshes/textures and existing fonts with source/license inventory; map 52 faces and one back. Configure imports/materials without repainting, remodeling, new shaders, or external asset acquisition; only simple primitive geometry is permitted as a new visual asset. Add native review fixtures and root component with initial/restore entrypoints.

**Acceptance:** Sample builds/runs through repository tools. Card mapping is complete and verified. Hearts has no runtime dependency on chess. Generated inputs reproduce from declarations; a native shell capture and reset fixture are retained.

**Validation:** Asset/addressable validation, native build and deterministic shell capture, face/back inventory assertions.

**Introspection focus:** What boilerplate does creating a Reactant app/assets still require? Improve reusable scaffolding or diagnostics rather than writing a Hearts-specific build script.

### H12

**Compose the forest scene and reference camera** — hv-a8o.24; review hv-a8o.25.

**Scope and interface:** Build Rust scene composition from existing forest assets and card meshes, allowing only simple primitive surfaces/markers where needed. Establish fixed camera, light/shadow/material defaults and quiet central clearing. Compose seat labels/score placeholders with reference-matched scale and palette.

**Acceptance:** Native landscape and portrait framing show readable cards and no occlusion/hit overlap. Side-by-side review against supplied reference records discrepancies and resolves material composition problems. Do not introduce gameplay C# or camera shake.

**Validation:** Retained native captures and direct reference comparison; asset/render errors absent; inspect depth order and light/shadow quality.

**Introspection focus:** Are scene/camera/light/material declarations as composable as a React scene tree? Improve general authoring where current ownership forces imperative setup.

### H13

**Compose stable cards, hand fans, and captured destinations** — hv-a8o.26; review hv-a8o.27.

**Scope and interface:** Privately map domain CardId to opaque random session tokens and render those tokens through existing WorldLayout destination identity; project faces only when visible. Add hands, trick, captured piles, own-face/opponent-back presentation, independent hit regions, inspection copy identity, and responsive fan measurement.

**Acceptance:** Zone changes preserve the original card component/ref identity; inspection copies use distinct identities. Counts remain correct and private faces never appear. Layout/retargeting uses the existing registry, with readable baseline portrait and landscape. Hidden card props/events/materials contain no rank/suit, reversible identity, or face texture; reveal adds face data to the same token.

**Validation:** Public layout/identity contracts plus native hand/trick/pile captures and resize/reparent scenarios. Inspect projected checkpoints/events/material assignments for privacy as well as rendered output.

**Introspection focus:** Where do list keys, refs, layout and card state differ from React composition? Extend existing Fan/Layout utilities instead of another identity system.

### H14

**Implement passing, pointer selection, and drag-to-play** — hv-a8o.28; review hv-a8o.29.

**Scope and interface:** Keep selection/inspection local and drive intentions through the reducer handle. Add three-card passing confirmation, select-then-confirm play, explicit touch Inspect, legal drop region, invalid/cancelled drag return, and input suppression while a command is pending.

**Acceptance:** Pass enables at exactly three and exchanges once. Click/tap/drag never double-dispatch; one activation cannot hit newly mounted confirmation. Capture loss returns to current layout, and illegal cards remain inspectable with a reason.

**Validation:** Native semantic/pointer/touch scenarios for passing, selection, confirm, invalid drop, capture loss, stale UI, and inspection.

**Introspection focus:** How much gesture/selection bookkeeping belongs in reusable controls? Compare local useState/callbacks to current Rust authoring and remove duplicated event glue.

### H15

**Implement keyboard/controller navigation and modal focus** — hv-a8o.30; review hv-a8o.31.

**Scope and interface:** Add arrows/D-pad/stick card focus, Enter/primary select-then-confirm, Escape/back cancellation, visible distinct focus/selection, semantic card labels, overlay trapping and focus restoration. Maintain input-method parity and preserve selection across menus.

**Acceptance:** A full pass/trick can be completed without pointer input. Illegal cards remain inspectable. Modal input cannot reach cards; closing returns focus safely. Reorientation/removal does not strand focus or expose hidden hands.

**Validation:** Native keyboard/controller/accessibility scenarios for all actions, modal restoration, card removal, and mixed input.

**Introspection focus:** Can domain/world focus use the same mental model as React controls? Consolidate reusable roving focus and focus-reveal behavior without sample-specific navigation loops.

### H16

**Add lifecycle-owned audio and ambient playback** — hv-a8o.32; review hv-a8o.33.

**Scope and interface:** Build reusable audio hook/components over existing playback handles: enabled/paused, gain, looping, track replacement/crossfade and cleanup. Audition and map existing NotJam clips to deal/pass/selection/landing/collection/results; choose existing music and tune timing/gain/crossfades. No recording, composition, synthesis, downloads, or custom sound assets. Literal paper/card and woodland sounds are not required: use existing cues, and omit the ambient layer if no suitable existing loop is available.

**Acceptance:** Rerenders/volume changes do not duplicate/restart playback; replacement crossfades and unmount/background cleans up. One-shots attach to occurrence IDs. Required music and SFX have separate working controls and provenance; expose an ambience control only if a suitable existing loop is used. Record source mappings, listening evidence and substitutions; thematic audio gaps do not block completion or authorize new assets.

**Validation:** Playback command/lifecycle contracts plus native listening evidence for timing, crossfade, pause and repeated mount/unmount.

**Introspection focus:** Compare effect cleanup to chess manual handles/counters and React audio ownership. Ensure the API removes actual lifecycle bookkeeping.

### H17

**Make ambient particles deterministic and lifecycle-owned** — hv-a8o.34; review hv-a8o.35.

**Scope and interface:** Expose reusable ambient/burst ownership over existing particle commands and motion clock. Supply deterministic seeds/time for Ditto, scoped pause/cleanup, reduced-motion behavior, and finite one-shot lifetime. Reuse existing particle assets/shaders with simple emitter/tint/scale configuration; simple primitive particles may supplement them for ambient, heart-breaking and result accents. No custom textures, detailed meshes, artistic shaders, or elaborate bespoke effects. Leaf/heart imagery is optional; use sparse neutral particles and text/color emphasis instead.

**Acceptance:** Controlled-time captures reproduce. Infinite ambience does not block readiness; pause/unmount/restart and reduced motion cleanly control effects. Bursts fire once and do not obscure card faces. Source mappings and any primitive geometry are recorded; no requirement depends on unavailable thematic assets.

**Validation:** Seed/time and lifecycle contracts, controlled native midpoints, repeated replacement and reduced-motion captures.

**Introspection focus:** Are particles ordinary declarative children/effects? Remove host-specific ownership/timing assumptions from the game and improve deterministic tooling where necessary.

### H18

**Animate complete Hearts deals, passes, and tricks** — hv-a8o.36; review hv-a8o.37.

**Scope and interface:** Compose event-driven deal/pass/play/hold/collect/score sequences using H07 and H16–H17 and stable world layout. Bind sounds/particles to actual sequence milestones. Apply documented initial timings and retarget during resize without issuing new occurrences.

**Acceptance:** A complete trick is readable before collection; scoring follows correct visual order. Rerender/resize never replay effects. Reduced motion preserves outcome order. Pause resumes exact motion and next action waits for settled receipt.

**Validation:** Native controlled midpoint/end captures for deal/pass/play/collect, reference comparison, synchronized listening, pause/reorientation/reduced-motion scenarios.

**Introspection focus:** Can a React developer compose animation from state/events without hand-authored command bookkeeping? Remove repeated sequence orchestration into shared idioms.

### H19

**Implement bounded information-consistent AI sampling** — hv-a8o.38; review hv-a8o.39.

**Scope and interface:** Build observation-only hidden-deal construction using finite randomized bipartite matching over unknown cards/hand slots with known-card and void constraints. Add separate per-seat resumable AI streams, common candidate samples and deterministic ordering. Check cancellation at assignment boundaries.

**Acceptance:** Every sampled deal satisfies card counts, known owners and void constraints. Infeasibility is an invariant failure, not a hidden-state fallback. Fixed observation/AI seed is invariant to inaccessible true hands. Cancelled work commits no random advancement.

**Validation:** Explicit constraint fixtures including late-hand/highly constrained cases, finite-bound assertions, privacy invariance, deterministic replay/cancellation.

**Introspection focus:** What generic cancellable-computation affordances are missing? Keep Hearts inference domain-owned while improving library task ergonomics.

### H20

**Implement fair rollout opponents through shared rules** — hv-a8o.40; review hv-a8o.41.

**Scope and interface:** Evaluate legal plays on 32 sampled deals and eight shortlisted passes on common samples/seeds. Use shared transitions and actor-specific simulated observations, deterministic legal heuristics, ordinary moon scoring and mean added penalties. Wire observation-keyed use_task results into guarded dispatch. Task results carry intention, observation/stream identity and resulting stream state; validate and commit intention plus stream advancement together at final acceptance.

**Acceptance:** A full match runs with one fair AI level. Actual hidden hands/submitted passes never enter sampling. Decisions are reproducible across resume; cancellation/stale results do not advance state/streams. UI remains responsive; workload changes have measurements. Rejection/cancellation/pre-acceptance failure preserves prior per-seat PRNG state; late playback failure preserves the accepted advancement.

**Validation:** Shared-rule rollout equivalence, hidden-hand invariance, seeded decision fixtures, complete-match tests and native responsiveness/cancellation evidence. Exercise envelope/PRNG acceptance atomically across stale observation, old stream, output failure and late playback failure.

**Introspection focus:** Compare useComputerTurn with the actual Rust call site. Remove worker/result/readiness glue while leaving policy and evaluation in the sample.

### H21

**Complete match screens, contextual teaching, and settings** — hv-a8o.42; review hv-a8o.43.

**Scope and interface:** Compose scoreboard/turn/passing prompts, hearts-broken announcements, hand/moon/match/shared-win results, Next Hand, New Game confirmation, pause/settings/rules and first-use contextual help. Apply coherent forest visual language and settings controls.

**Acceptance:** Every phase has a clear action/explanation. No accidental next-hand dispatch after match end. Settings apply without losing state; no secret cards leak. Input methods, larger text and focus trapping work throughout.

**Validation:** Native fixtures for every screen/phase, tied/moon results, first-use guidance, settings and focus; representative full match playthrough.

**Introspection focus:** Compare the component tree with the blind sketch. Simplify reusable controls/overlays and avoid a monolithic second UI controller.

### H22

**Integrate autosave, recovery, and foreground/background lifecycle** — hv-a8o.44; review hv-a8o.45.

**Scope and interface:** Connect accepted revisions to async durable storage; save initial deal and partial passes. Add loading/Continue, corruption/errors/retry, New Game replacement, controlled exit, pause/background audio/task handling and restore without event backlog.

**Acceptance:** Save status distinguishes accepted from committed. Old writes cannot overwrite new sessions. Reopen resumes valid committed state and randomness without replayed audio/particles. Background flush is best effort and unacknowledged-loss behavior is clear.

**Validation:** Native and desktop-browser end-to-end save/reload/failure races; lifecycle during search, drag, trick motion and pending write; termination boundary tests.

**Introspection focus:** Does the sample compose a few hooks like React, or manually manage persistence/task/lifecycle ordering? Move reusable ordering into engine ownership.

### H23

**Finish portrait, safe-area, and cross-input ergonomics** — hv-a8o.46; review hv-a8o.47.

**Scope and interface:** Refine native phone layouts: recompose perimeter seats, pannable human fan, automatic focus reveal, text scaling and safe areas. Resolve gesture conflict between fan pan and card drag, preserving explicit select/confirm and inspect behavior.

**Acceptance:** Thirteen human ranks remain readable/inspectable on portrait devices. Reorientation during pass/play preserves selection/identity/progress. Pan versus drag intent is unambiguous, controls reachable, and controller focus remains visible.

**Validation:** Native portrait/landscape/safe-area fixtures, pan/drag/controller focus, large text and in-flight rotation captures.

**Introspection focus:** Which adaptive-layout, gesture arbitration and focus utilities should be reusable? Eliminate device-specific conditionals scattered through game components.

### H24

**Add reproducible native iOS device delivery** — hv-a8o.48; review hv-a8o.49.

**Scope and interface:** Extend existing simulator/build identity tooling for real iOS target compilation, SDK/architecture identity, user provisioning/signing route, installation and lifecycle fixture access. Keep reusable build support in tooling, not Hearts shell scripts.

**Acceptance:** Produce a device-compatible signed build through supplied credentials and exact identity/install instructions. Missing user credentials are recorded external prerequisites, not silently replaced by a simulator or unsigned-project success claim.

**Validation:** Build identity/cache invalidation tests and native iOS build/install/lifecycle smoke when credentials available; retain logs and user handoff steps.

**Introspection focus:** What did native delivery force the app author to know? Generalize target/signing diagnostics and cache keys without weakening exact-build evidence.

### H25

**Add reproducible native Android delivery** — hv-a8o.50; review hv-a8o.51.

**Scope and interface:** Connect existing Android host hooks to shared target/build identity/tool resolution. Build ARM64 Rust plugin and installable APK with reproducible SDK/NDK/Unity inputs, development signing and install/log collection instructions.

**Acceptance:** Installable APK, exact build identity and native lifecycle fixture route exist. Tool/ABI failures are actionable. No ad hoc sample build fork or runtime dependency on chess; cache invalidates relevant Android inputs.

**Validation:** Build identity tests, Android build/emulator smoke and user physical-device packet; retain exact logs and signing classification.

**Introspection focus:** Which Android steps can shared tools own? Remove repeated CLI paths, manual plugin copying and opaque platform prerequisites.

### H26

**Validate desktop WebGL Hearts compatibility** — hv-a8o.52; review hv-a8o.53.

**Scope and interface:** Add/extend risk-selected browser contracts for Hearts shaders/materials, input/controller, audio activation, IndexedDB resume, startup/reload and finite task-worker admission. Use desktop browser checks only for browser-specific risks.

**Acceptance:** Desktop WebGL plays/resumes a complete representative hand with correct rendering and nonblocking AI. Audio unlock and storage failures behave coherently. Phone browser support/public deployment remain excluded.

**Validation:** Retained desktop-browser compatibility evidence tied to exact build, with selected automated checks registered in web contracts.

**Introspection focus:** What browser-specific work leaks into game code? Improve host capability/error abstractions and avoid duplicate native/browser gameplay implementations.

### H27

**Migrate chess music to shared audio lifecycle** — hv-a8o.54; review hv-a8o.55.

**Scope and interface:** Replace focused chess music handle/crossfade/volume/cleanup bookkeeping with H16 APIs. Preserve music selection, timing and player behavior; do not redesign menus or broaden the migration.

**Acceptance:** Before/after call sites demonstrate less lifecycle boilerplate. Existing music behavior and native evidence remain valid; no duplicate playback across menu/restart/volume changes.

**Validation:** Focused chess playback contracts and native listening/regression scenario with repeated lifecycle transitions.

**Introspection focus:** Does a second real caller expose Hearts-specific assumptions? Correct the shared API and both call sites before closing.

### H28

**Migrate a focused chess opponent path to shared tasks** — hv-a8o.56; review hv-a8o.57.

**Scope and interface:** Adopt use_task for a bounded asynchronous chess opponent path while preserving chess legality, difficulty/selection behavior and existing accepted-state semantics. Remove superseded task-generation/result-lifetime glue only in that path.

**Acceptance:** Chess search remains off the UI thread; pause/restart/unmount cancel or invalidate results correctly. Evidence compares React-shaped hook ownership across both games without forcing chess onto the entire Hearts reducer model.

**Validation:** Focused opponent stale-result/cancellation tests plus native move/restart responsiveness regressions.

**Introspection focus:** Is use_task genuinely game-independent? Fix second-caller friction rather than preserving two subtly different cancellation frameworks.

### H29

**Refine Hearts art, audio, and measured performance** — hv-a8o.58; review hv-a8o.59.

**Scope and interface:** Iterate assembled scene/input/motion/audio against the reference and native captures through composition, lighting, mix and parameter tuning of the existing pool. Do not expand into asset creation or acquisition; use the documented substitutions. Measure frame cost, batching/materials/shadows/particles and AI responsiveness on reproducible builds. Provide user-facing physical performance instrumentation for the selected device classes.

**Acceptance:** Resolved clipping/readability/occlusion/timing issues and documented remaining user-sign-off items. Decoration is reduced before clarity. Required sound roles and controlled effects use existing assets or permitted simple geometry, with thematic substitutions documented; no claim of physical 60fps without user evidence.

**Validation:** Native landscape/portrait controlled captures, actual listening, full-hand profiling/warmup distributions and final reference comparison.

**Introspection focus:** Which iteration cycles are too slow or opaque? Improve capture/profiling/asset feedback utilities and remove sample workarounds exposed by polish.

### Sequential reference refinement

These three passes each ask: **compare the latest native Hearts scene with
`docs/hearts-kaykit-forest-reference.png` and try to make it look more similar.**
They run after H29/R29 and before H30 final evidence. Each uses the previous
pass's delivered scene, not the original baseline or its stale discrepancy list.

For each pass, capture a representative full-hand landscape state and choose the
one to three most visible remaining discrepancies. Adjust camera/framing, card
scale/fans, clearing/foliage placement, palette, lighting/shadows, or HUD intrusion
as the comparison warrants. Keep the change bounded and use the existing asset
pool; simple primitive geometry remains the only new asset exception. Preserve
the approved KayKit court art, gameplay, readability, and performance.

Retain the reference and comparable native before/after captures with reproducible
state, viewport, clocks and seeds. Explain why the result is closer and hand off
remaining discrepancies. Check representative portrait framing and affected
interaction/motion, and run repository-required validation. Do not force portrait
to copy the landscape layout. If no worthwhile in-scope improvement remains,
retain fresh comparison evidence and a concrete no-change rationale instead of
manufacturing churn. A pass cannot substitute for final user aesthetic approval.

Each RV answers all five mandatory introspection questions and files actual
follow-up beads for larger architecture/library/tooling findings. Fixes required
for current acceptance still block closure; broader follow-ups stay deferred.
Focus on declarative scene/layout tuning and the capture/comparison feedback
cycle. Do not create custom-asset production work. Each review blocks the next
pass; H30 refreshes evidence after the final pass and its review.

### V01

**Compare the scene to the reference and improve it — pass 1** — hv-a8o.63; review hv-a8o.64 (RV01).

Prerequisite: **hv-a8o.59**. Execute the comparison/improvement contract
above against the latest delivered scene; retain before/after evidence and
remaining discrepancies for the next task.

### V02

**Compare the scene to the reference and improve it — pass 2** — hv-a8o.65; review hv-a8o.66 (RV02).

Prerequisite: **hv-a8o.64**. Execute the comparison/improvement contract
above against the latest delivered scene; retain before/after evidence and
remaining discrepancies for the next task.

### V03

**Compare the scene to the reference and improve it — pass 3** — hv-a8o.67; review hv-a8o.68 (RV03).

Prerequisite: **hv-a8o.66**. Execute the comparison/improvement contract
above against the latest delivered scene; retain before/after evidence and
remaining discrepancies for the next task.

### H30

**Assemble final automated evidence and mobile review packet** — hv-a8o.60; review hv-a8o.61.

**Scope and interface:** Run final risk-selected rules/engine/native/browser coverage, retain exact source/build evidence, exercise representative warm sample/engine/host CI budgets, and prepare Android/iOS installation and user test instructions. Audit all required engine changes and closed introspections.

**Acceptance:** Automated gates pass within defined budget with honest cold/queue reporting. Evidence covers complete match, visual/audio and runtime lifecycle. Mobile packet contains exact build identities and expected outcomes; user-owned final acceptance remains open.

**Validation:** Full staged validation plus scoped acceptance matrix; graph/evidence audit; no reuse after relevant inputs change.

**Introspection focus:** Did the project improve the library and workflow in measurable ways? Consolidate concrete before/after authoring/timing evidence and resolve remaining justified engine fixes.

## User-owned final acceptance

**hv-a8o.62** depends on **hv-a8o.61** and remains deferred for user sign-off.
Collect physical mobile results and final aesthetic approval against exact builds.
The agent delivers native iOS/Android builds, install instructions, deterministic
fixtures and performance instrumentation. The user owns iPhone 12 / Pixel 6 class
testing and final 60 fps/performance and aesthetic sign-off. Missing signing or
device access is an explicit external prerequisite, not a simulator substitute.
Failed acceptance produces bounded repair/introspection work with verified edges.
Closing the final implementation task is not equivalent to closing this gate or
the epic.

## Superseded assignments

Former engine tasks 35–43, including their lettered leaves, redirect here.
The general lower-level engine plan remains useful grounding for other callers;
its old Hearts context/prompt, explicit-save, or minimal-mobile requirements do
not override this design. Native statuses and evidence are the source of truth
for progress; this document is the assignment map, not a running progress log.
