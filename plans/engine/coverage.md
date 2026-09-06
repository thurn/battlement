# Requirement checklist and implementation ownership

Use this checklist to verify that the finished engine covers its full design.
Each item states the behavior to build, the tasks that own it, and the
observation that demonstrates it. An item is not complete merely because a
similarly named API exists.

Related pages: [overview and task order](README.md), [test scenes](fixtures.md),
[validation](validation.md), and [sample migration](migration.md).

## Authoring and ownership

Games should be simple to register, and every layer should have one clear owner.
Read [architecture](architecture.md) and [API examples](interfaces.md).

- One `Game` trait owns state, view, action, `StateAnimation`, validation, and
  synchronous action application. Registration infers all associated types.

  **Tasks:** [02](tasks/02-rules-api-simulation.md),
  [11](tasks/11-accepted-action-runtime.md).

  **Verify:** Compile a choice-free game and a game with two typed choices.

- UI-only apps need no game state or worker; default movement needs no
  configuration.

  **Tasks:** [06](tasks/06-crate-boundaries.md),
  [13](tasks/13-mixed-logical-tree.md),
  [24](tasks/24-layout-movement-projection.md),
  [48](tasks/48-retire-adapters-final-audit.md).

  **Verify:** Run SettingsPanel and move an identified card with no movement
  setup.

- Reactant owns game presentation; Battlement provides generic Unity execution,
  with a one-way dependency including tooling and assemblies.

  **Tasks:** [06](tasks/06-crate-boundaries.md),
  [07](tasks/07-asset-tooling-boundary.md),
  [48](tasks/48-retire-adapters-final-audit.md).

  **Verify:** Build direct basic/ui samples without a Reactant dependency;
  inspect all dependency edges.

- One logical tree for UI and world props, hooks, context, events, and cleanup.

  **Tasks:** [13](tasks/13-mixed-logical-tree.md),
  [14](tasks/14-global-presentation-identity.md),
  [16](tasks/16-view-selectors-stores.md).

  **Verify:** One component contributes a world card and UI details with shared
  state.

- Useful existing implementations and builder patterns retained; API changes
  update all callers.

  **Tasks:** [06](tasks/06-crate-boundaries.md),
  [07](tasks/07-asset-tooling-boundary.md),
  [33](tasks/33-reactant-sample-migration.md),
  [34](tasks/34-chess-ui-migration.md),
  [48](tasks/48-retire-adapters-final-audit.md).

  **Verify:** Existing sample checks pass throughout extraction; authoring
  examples use new() plus setters.

## Rules, choices, cancellation, and saving

A worker computes privately while the display presents ordered immutable views.
Read [execution](execution.md) and [presentation](presentation.md).

- Run-local checkpoint IDs and fixed ordered animation indices, including a
  single animation at index zero.

  **Tasks:** [02](tasks/02-rules-api-simulation.md),
  [09](tasks/09-checkpoint-publication.md).

  **Verify:** Retry presentation without changing IDs or animation order.

- Immutable owned `Send` views, independent accepted state and worker copy;
  optional immutable sharing.

  **Tasks:** [02](tasks/02-rules-api-simulation.md),
  [09](tasks/09-checkpoint-publication.md),
  [11](tasks/11-accepted-action-runtime.md).

  **Verify:** Mutating the worker never changes a published or accepted value.

- At most one pending checkpoint; reserve before builders, including prompts and
  final output; no extra queue during preparation.

  **Tasks:** [09](tasks/09-checkpoint-publication.md),
  [10](tasks/10-typed-prompts.md), [12](tasks/12-host-transactions.md).

  **Verify:** With A visible and B pending, C builders have not run.

- Typed choices return directly on the synchronous stack; owned UI data is built
  only interactively.

  **Tasks:** [02](tasks/02-rules-api-simulation.md),
  [10](tasks/10-typed-prompts.md).

  **Verify:** Two different typed choices run through nested functions in both
  modes.

- One unchanged outstanding prompt; previous required presentation finishes
  first; invalid answers retain the request.

  **Tasks:** [10](tasks/10-typed-prompts.md),
  [25](tasks/25-checkpoint-motion-gates.md).

  **Verify:** Reject illegal/wrong-type answers; allow correction without
  restarting the request.

- Run/request IDs reject stale answers and output; changed legal sets get new
  requests.

  **Tasks:** [10](tasks/10-typed-prompts.md),
  [11](tasks/11-accepted-action-runtime.md),
  [43](tasks/43-hearts-save-resume.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Old handles cannot answer after replacement or restoration.

- Menus, inspection, and stores stay responsive during prompt and publication
  waits.

  **Tasks:** [10](tasks/10-typed-prompts.md),
  [11](tasks/11-accepted-action-runtime.md),
  [16](tasks/16-view-selectors-stores.md),
  [41](tasks/41-hearts-pointer-touch.md),
  [42](tasks/42-hearts-navigation-menus.md).

  **Verify:** Open settings while choosing and while waiting for animation.

- Cancellation on primitive entry, after capacity and builders, after waits,
  before returning answers, and before final output.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [09](tasks/09-checkpoint-publication.md), [10](tasks/10-typed-prompts.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Coordinate every position with fixture barriers; no new builder
  after cancellation is observed.

- Immediate run invalidation, wakeups without lost notifications, private unwind
  payload, and silent expected cancellation.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [04](tasks/04-threaded-webgl-proof.md),
  [09](tasks/09-checkpoint-publication.md), [10](tasks/10-typed-prompts.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Cancel a select/deselect cycle and a full publication queue.

- Builders finish before cancellation check; explicit checks in long
  calculations; detached shutdown; stopped only after cleanup.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [04](tasks/04-threaded-webgl-proof.md), [05](tasks/05-mobile-build-paths.md),
  [45](tasks/45-effects-failures-laboratory.md),
  [47](tasks/47-release-conformance.md).

  **Verify:** Release a held builder and observe destructors before
  worker-stopped while replacement stays responsive.

- Cancellation/valid-answer/normal-return races discard abandoned results;
  genuine panic reports failure only for its active run.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [10](tasks/10-typed-prompts.md), [11](tasks/11-accepted-action-runtime.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Test both race orders and an ordinary panic separately.

- Unwind stays inside Rust, guards released first, unknown inner payloads
  propagated, safe disposal documented.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [04](tasks/04-threaded-webgl-proof.md), [05](tasks/05-mobile-build-paths.md),
  [47](tasks/47-release-conformance.md).

  **Verify:** Nested real Unity release fixtures prove cleanup and catching;
  enforce compatible panic/runtime settings.

- Rules own Rust memory, not persistence/network/native handles; accepted state
  survives worker or required-animation failure.

  **Tasks:** [11](tasks/11-accepted-action-runtime.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [43](tasks/43-hearts-save-resume.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Restart from the last accepted state after failure.

- Final state accepted only after final checkpoint, required animation/label,
  and a qualifying rendered frame.

  **Tasks:** [11](tasks/11-accepted-action-runtime.md),
  [12](tasks/12-host-transactions.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [43](tasks/43-hearts-save-resume.md).

  **Verify:** Saving and next action remain disabled while any condition is
  missing.

- Persistence outside rules; ordered immutable saves; write failure does not
  undo accepted gameplay.

  **Tasks:** [11](tasks/11-accepted-action-runtime.md),
  [32](tasks/32-chess-cutover.md), [43](tasks/43-hearts-save-resume.md).

  **Verify:** Retry failed saves and restore the last durable state.

- Same synchronous simulation with lazy builders skipped, inline policy, no
  mandatory primitive allocation, no-op cancellation.

  **Tasks:** [02](tasks/02-rules-api-simulation.md),
  [40](tasks/40-hearts-simulation-ai.md),
  [46](tasks/46-performance-workloads.md).

  **Verify:** Allocation trace and optimized-code inspection through public
  entrypoints.

- Independent simulation states and shared immutable data; previews use the
  normal display path.

  **Tasks:** [02](tasks/02-rules-api-simulation.md),
  [40](tasks/40-hearts-simulation-ai.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Render a simulated outcome without changing the live game or
  exposing hidden cards.

## Identity, components, and world layout

Movement preserves a live component; removal ends its lifetime. Read
[identity](identity.md) and [world objects](world.md).

- id(Uuid) applies across UI/world roots and portals; keys remain
  sibling-scoped; UUIDs stable across renders.

  **Tasks:** [14](tasks/14-global-presentation-identity.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Same Card keeps hooks and refs after reparenting; equal local keys
  remain legal.

- Match before descendants; reject duplicate live UUIDs before commit; extract
  survivors before destroying old parents.

  **Tasks:** [14](tasks/14-global-presentation-identity.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Move out of a removed ancestor and reject duplicates across
  separate roots.

- New ancestry supplies context and event paths; changed effects clean up before
  replacement; compatible descendants survive.

  **Tasks:** [14](tasks/14-global-presentation-identity.md),
  [16](tasks/16-view-selectors-stores.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Moved counter retains state but reads new provider with one
  subscription.

- Component type changes reset hooks; incompatible hosts prepared before
  replacement; explicit UI/world projection.

  **Tasks:** [14](tasks/14-global-presentation-identity.md),
  [17](tasks/17-world-rendering-primitives.md),
  [24](tasks/24-layout-movement-projection.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Verify logical continuity, compatible visual reuse, and
  screen-space transfer.

- Committed absence unmounts immediately; aborted preparation does not; old
  exits have no hooks/input.

  **Tasks:** [15](tasks/15-incarnations-and-removal.md),
  [27](tasks/27-effect-exit-retention.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Remove and recreate the same UUID while its old visual exits.

- Separate mounted lifetimes; callbacks/refs target original lifetime; release
  resources after final retained use.

  **Tasks:** [15](tasks/15-incarnations-and-removal.md),
  [18](tasks/18-hit-regions-anchors.md),
  [27](tasks/27-effect-exit-retention.md), [28](tasks/28-inspection-replay.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Old completion cannot destroy replacement; projectile retains only
  original resources.

- Props complete without selectors; equal selectors skip evaluation; stable
  store version and queued writes.

  **Tasks:** [16](tasks/16-view-selectors-stores.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Compare props/selector scenes; render-time writes appear in a
  later whole update.

- Rust-built sprites, meshes, world text, groups, hit regions, anchors,
  camera/light hosts, and independent assets.

  **Tasks:** [13](tasks/13-mixed-logical-tree.md),
  [17](tasks/17-world-rendering-primitives.md),
  [18](tasks/18-hit-regions-anchors.md), [35](tasks/35-hearts-assets-shell.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Inspect Rust-composed card hierarchy, font/material dependencies,
  and native geometry.

- Normal/table/hidden/UI faces; rich text, badges, outlines, preview,
  conditional action buttons, and contained cards.

  **Tasks:** [17](tasks/17-world-rendering-primitives.md),
  [18](tasks/18-hit-regions-anchors.md), [37](tasks/37-hearts-card-layout.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Native scene shows mixed sprite/text ordering and
  context-dependent hit dimensions.

- Typed per-card material overrides; shared dissolve value plus text opacity;
  reverse dissolve.

  **Tasks:** [17](tasks/17-world-rendering-primitives.md),
  [21](tasks/21-shared-motion-drivers.md),
  [27](tasks/27-effect-exit-retention.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Changing one card never changes another or its shared asset.

- Typed Rust-created anchors; attached/live and captured-at-start effect
  targets.

  **Tasks:** [18](tasks/18-hit-regions-anchors.md),
  [26](tasks/26-effect-occurrences.md), [27](tasks/27-effect-exit-retention.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Move an anchor and compare following versus captured effects after
  removal.

- World Flexbox/Grid via Taffy; pure fan/pile/arc algorithms; native UI layout
  retains pixel units.

  **Tasks:** [23](tasks/23-world-layout.md),
  [24](tasks/24-layout-movement-projection.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Check all layout pairs on two planes and during reflow.

- Declared plane/extents and stable rest boxes/pivots; cached measurements
  before commit; localized invalidation.

  **Tasks:** [23](tasks/23-world-layout.md),
  [44](tasks/44-identity-composition-laboratory.md),
  [46](tasks/46-performance-workloads.md).

  **Verify:** Hover scale does not change layout; reject stale measurements;
  preserve geometry/facing by default.

- Display placement independent of rules zone; browser/deck order and
  draft/shop/quest-deck arrangements.

  **Tasks:** [23](tasks/23-world-layout.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Rearrange cards locally; confirm a typed decision only when the
  player accepts.

- Primary, inspection, and outcome-preview presentations have distinct UUIDs
  while sharing permitted game data.

  **Tasks:** [14](tasks/14-global-presentation-identity.md),
  [37](tasks/37-hearts-card-layout.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Show simultaneous views without moving or remounting the primary
  card.

## Input, animation, effects, and visible updates

The player interacts with the committed display, and one animation system
controls its properties. Read [world input](world.md), [animation](motion.md),
and [presentation](presentation.md).

- Click/hover/press/drag across domains; logical capture/bubble through portals;
  synchronous default prevention.

  **Tasks:** [19](tasks/19-unified-pointer-routing.md),
  [20](tasks/20-world-focus-touch.md), [41](tasks/41-hearts-pointer-touch.md),
  [42](tasks/42-hearts-navigation-menus.md).

  **Verify:** Use geometric input and observe one eligible target without
  reentrant Rust calls.

- UI blocks world unless passthrough; ordered modals; world
  layer/depth/stable-order hit priority.

  **Tasks:** [19](tasks/19-unified-pointer-routing.md),
  [44](tasks/44-identity-composition-laboratory.md).

  **Verify:** Overlap UI/world targets and nested modals, including exact-depth
  ties.

- Capture survives moves and ends on removal; touch IDs independent; visible
  focus and semantic keyboard/controller activation.

  **Tasks:** [19](tasks/19-unified-pointer-routing.md),
  [20](tasks/20-world-focus-touch.md), [41](tasks/41-hearts-pointer-touch.md),
  [42](tasks/42-hearts-navigation-menus.md).

  **Verify:** Cancel drag, remove focus, restore modal focus, and switch input
  modes safely.

- Only presented decision points change game state; local inspection/settings
  available throughout.

  **Tasks:** [10](tasks/10-typed-prompts.md),
  [11](tasks/11-accepted-action-runtime.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [41](tasks/41-hearts-pointer-touch.md),
  [42](tasks/42-hearts-navigation-menus.md).

  **Verify:** Input cannot start another action during required presentation.

- Stable render view/store version; inactive preparation can span frames;
  current display remains usable.

  **Tasks:** [12](tasks/12-host-transactions.md),
  [16](tasks/16-view-selectors-stores.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Delay assets and update settings without exposing partially
  prepared objects.

- Revalidate run/checkpoint/base generation/display revision before commit;
  discard obsolete resources and reserved playback.

  **Tasks:** [12](tasks/12-host-transactions.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Replacement or newer store state prevents the obsolete update from
  committing.

- Dependency-ordered native changes, handlers, refs, and animation installed
  together; acknowledge before dispatching new events.

  **Tasks:** [12](tasks/12-host-transactions.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Repeated delivery has one effect; input never sees a mismatched
  handler; unexpected apply failure stops session.

- Targets/transitions/variants/gestures/initial/exit/Motion values with
  inherited defaults and shared playback controls.

  **Tasks:** [21](tasks/21-shared-motion-drivers.md),
  [22](tasks/22-sequence-dependencies.md),
  [24](tasks/24-layout-movement-projection.md).

  **Verify:** Compare UI, world, layout, and sequence controls and
  reduced-motion behavior.

- Every changed layout pose has default/overridden movement; entry for
  new/restored objects; exits for removal.

  **Tasks:** [24](tasks/24-layout-movement-projection.md),
  [25](tasks/25-checkpoint-motion-gates.md).

  **Verify:** No-configuration movement works; equal poses finish immediately
  but still need a frame.

- One writer per property; sequences own placement, layout keeps destination
  current, hover uses separate offsets.

  **Tasks:** [21](tasks/21-shared-motion-drivers.md),
  [22](tasks/22-sequence-dependencies.md),
  [24](tasks/24-layout-movement-projection.md),
  [41](tasks/41-hearts-pointer-touch.md).

  **Verify:** Reject accidental overlap; permit explicit replacement; required
  draw cannot lose placement to drag.

- Actual-pose retargeting, spring velocity retained, fixed tween duration
  restarted, coordinate conversion through world space.

  **Tasks:** [21](tasks/21-shared-motion-drivers.md),
  [24](tasks/24-layout-movement-projection.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Repeated reflow has no jump and eventually arrives after
  stabilization.

- Each sequence step follows its declared target; later hand step resolves
  latest destination and returns control continuously.

  **Tasks:** [22](tasks/22-sequence-dependencies.md),
  [24](tasks/24-layout-movement-projection.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Reflow during reveal does not redirect reveal prematurely.

- Immutable sequences, concurrent/absolute/relative scheduling, labels after
  actual completion, declaration-order equal-time events.

  **Tasks:** [22](tasks/22-sequence-dependencies.md),
  [26](tasks/26-effect-occurrences.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Retargeted arrival delays dependent sound; absolute entries keep
  timestamps.

- Host-local sampling and prepared successors; Rust selects game behavior;
  reject cycles/missing labels/infinite required loops.

  **Tasks:** [21](tasks/21-shared-motion-drivers.md),
  [22](tasks/22-sequence-dependencies.md), [26](tasks/26-effect-occurrences.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** No per-step Rust callback is needed for prepared motion and
  effects.

- State-animation callbacks prepare once and commit once across retries;
  event-driven controls share machinery.

  **Tasks:** [22](tasks/22-sequence-dependencies.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [26](tasks/26-effect-occurrences.md).

  **Verify:** Rerendering starts no duplicate playback.

- Wait for all required movement by default; early label replaces only its
  movement requirement; cosmetic work independent.

  **Tasks:** [25](tasks/25-checkpoint-motion-gates.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Two cards contribute; choosing an early label does not retain a
  hidden arrival requirement.

- Rendering opportunity after satisfaction in current run/checkpoint/generation;
  bounded admission per frame.

  **Tasks:** [12](tasks/12-host-transactions.md),
  [25](tasks/25-checkpoint-motion-gates.md).

  **Verify:** Old/pre-completion frames cannot advance; empty checkpoints each
  receive a frame.

- Track has one completed/interrupted/failed outcome; unfinished requirements
  transfer on replacement; accepted satisfaction persists.

  **Tasks:** [21](tasks/21-shared-motion-drivers.md),
  [25](tasks/25-checkpoint-motion-gates.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Test event-before-replacement and event-after-replacement
  separately.

- Required failure abandons action with accepted-state recovery;
  stale/replay/inspection events excluded.

  **Tasks:** [25](tasks/25-checkpoint-motion-gates.md),
  [28](tasks/28-inspection-replay.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Cosmetic stop is harmless; required failure shows restart/exit.

- Material/light/emission/volume continuous properties; discrete sound/burst;
  projectile visuals and persistent aura children.

  **Tasks:** [21](tasks/21-shared-motion-drivers.md),
  [26](tasks/26-effect-occurrences.md), [27](tasks/27-effect-exit-retention.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Native and fake timelines agree on start order and resource
  lifetime.

- Unique run/checkpoint/animation/effect identity; duplicate names rejected;
  rerenders/retries/delivery deduplicated.

  **Tasks:** [26](tasks/26-effect-occurrences.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Two intended sounds use distinct identities; each plays once.

- Rust effect fallback/composition; optional typed RON constants; captured
  configuration; required dependencies prepared.

  **Tasks:** [26](tasks/26-effect-occurrences.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Change settings during playback; old playback keeps values and
  next one uses new values.

- Scoped cleanup plus retained exits/effects; seek capability reporting;
  delivered history preserved on seek/resume.

  **Tasks:** [27](tasks/27-effect-exit-retention.md),
  [28](tasks/28-inspection-replay.md), [29](tasks/29-presentation-inspector.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Resume emits only undelivered sounds; unsupported native seeking
  is visible.

- Explicit replay uses fresh session-unique IDs separate from live checkpoints;
  leaving inspection restores live playback.

  **Tasks:** [28](tasks/28-inspection-replay.md),
  [29](tasks/29-presentation-inspector.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** Each replay emits once without accepting an action or retaining
  leaked resources.

## Samples, tests, performance, and platform support

Complete the migrations and playable reference while preserving externally
observable behavior. Read [migration](migration.md), [Hearts](hearts.md), [test
scenes](fixtures.md), and [validation](validation.md).

- Tic-tac-toe: immediate human display, 100 ms AI delay,
  rules/messages/reset/seeds.

  **Tasks:** [01](tasks/01-behavior-baseline.md),
  [08](tasks/08-public-display-driver.md),
  [30](tasks/30-tictactoe-migration.md).

  **Verify:** Existing behavior passes before and after migration, including
  99/100 ms timing.

- Chess: paths/captures/castling/promotion, spawn/audio, AI, inputs/diagnostics,
  saving/reset.

  **Tasks:** [01](tasks/01-behavior-baseline.md),
  [08](tasks/08-public-display-driver.md),
  [31](tasks/31-chess-reactant-fixture.md), [32](tasks/32-chess-cutover.md).

  **Verify:** Compare both fixtures before switching default; remove the old
  command-building engine.

- Existing Reactant and implemented chess-ui gallery behavior; basic/ui stay
  direct Battlement.

  **Tasks:** [06](tasks/06-crate-boundaries.md),
  [07](tasks/07-asset-tooling-boundary.md),
  [33](tasks/33-reactant-sample-migration.md),
  [34](tasks/34-chess-ui-migration.md), [47](tasks/47-release-conformance.md).

  **Verify:** Retain native initial/changed/reset captures and current
  input/localization behavior.

- Hearts: complete fixed rules, hidden-information display, three AI opponents,
  all input modes.

  **Tasks:** [35](tasks/35-hearts-assets-shell.md),
  [36](tasks/36-hearts-rules.md), [37](tasks/37-hearts-card-layout.md),
  [38](tasks/38-hearts-passing.md), [39](tasks/39-hearts-play-scoring.md),
  [40](tasks/40-hearts-simulation-ai.md),
  [41](tasks/41-hearts-pointer-touch.md),
  [42](tasks/42-hearts-navigation-menus.md).

  **Verify:** Complete match, rare-rule deals, simultaneous passing, and
  phase-correct next-hand flow.

- Hearts persistence: initial accepted deal, passing and each play; ordered
  writes, durable native/WebGL storage, retry/resume.

  **Tasks:** [43](tasks/43-hearts-save-resume.md).

  **Verify:** New Game races with old saves; Exit flushes newest accepted state;
  corrupted data is explained.

- Public display scenarios with virtual interpolation, labels/effects, input,
  anchors, prompts, and worker barriers.

  **Tasks:** [08](tasks/08-public-display-driver.md),
  [29](tasks/29-presentation-inspector.md),
  [44](tasks/44-identity-composition-laboratory.md),
  [45](tasks/45-effects-failures-laboratory.md).

  **Verify:** No endpoint-only tween fake, private maps, guessed sleeps, or
  ignored audio substitutes for behavior.

- Native evidence for shaders, particles, fonts, audio, geometry, and input;
  actual threaded WebGL cleanup/storage.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [04](tasks/04-threaded-webgl-proof.md), [05](tasks/05-mobile-build-paths.md),
  [17](tasks/17-world-rendering-primitives.md),
  [26](tasks/26-effect-occurrences.md), [47](tasks/47-release-conformance.md).

  **Verify:** Real Unity release runs, not Rust-only or fake proof; retain exact
  reproducible inputs.

- Complete 300/500-card views, 30 layouts/tracks, fixed seeds/assets, concurrent
  AI, ten-minute warmed release captures.

  **Tasks:** [46](tasks/46-performance-workloads.md).

  **Verify:** Record counts/environment plus CPU/GPU/frame/latency/allocation
  distributions and missed targets.

- No redundant unchanged-tree serialization, global layout rebuilding,
  unconditional quadratic matching, or mandatory geometry streaming.

  **Tasks:** [16](tasks/16-view-selectors-stores.md),
  [23](tasks/23-world-layout.md), [29](tasks/29-presentation-inspector.md),
  [46](tasks/46-performance-workloads.md).

  **Verify:** Measure sparse updates and final swaps; keep ordinary motion
  host-local.

- Functional desktop native/threaded WebGL, preserved macOS/Windows support,
  iOS/Android builds and automated paths.

  **Tasks:** [03](tasks/03-native-cancellation.md),
  [04](tasks/04-threaded-webgl-proof.md), [05](tasks/05-mobile-build-paths.md),
  [47](tasks/47-release-conformance.md).

  **Verify:** Enforce actual threading and unwind compatibility; report each
  platform honestly.

- Physical iPhone 17/Galaxy S25 certification separate; numeric performance
  targets advisory; correctness and primitive overhead mandatory.

  **Tasks:** [05](tasks/05-mobile-build-paths.md),
  [46](tasks/46-performance-workloads.md),
  [47](tasks/47-release-conformance.md),
  [48](tasks/48-retire-adapters-final-audit.md).

  **Verify:** Explicit physical checklist and target misses, with no false
  completion claims.

## Manual QA

Select a requirement, open its assigned task and related design page, and repeat
the stated observation. For final review, play every migrated sample and Hearts,
then use the test scenes and inspector for timing, failure, and lifetime cases.
Verify separate evidence for public Rust behavior and actual Unity rendering.
