# Reactant game engine implementation plan

This is the execution package for replacing Battlement's UI-centric Reactant
integration with a unified Rust component game engine. It is written for serial
implementation by Luna subagents: select one numbered task, read its linked
contracts, implement its concrete acceptance scenarios, and integrate it before
starting the next.

The target combines synchronous worker-owned rules, immutable presented
checkpoints, a shared UI/world logical tree, application-wide UUID identity, and
one host-executed Motion model. Battlement remains the generic Unity execution
layer. Reactant and Battlement stay in this repository with a one-way
Reactant-to-Battlement dependency.

## Authority and fixed delivery contract

This package translates the high-level Reactant game-engine proposal and the
resolved design decisions into implementation requirements. It is standalone:
implementors do not need the original Downloads file or the planning
conversation. Read shared contracts as normative behavior; code examples
illustrate intended authoring and may improve within those contracts.

- Implement the full proposed v1 engine with neutral fixtures and a playable
  Hearts reference. No Dreamtides repository, port, code, or assets are in
  scope.
- Keep basic and ui as direct Battlement examples. Migrate tictactoe, chess,
  reactant, and currently implemented chess-ui content while preserving
  behavior.
- Rework or delete tests that assert implementation details. Preserve meaningful
  external guarantees and prove replacement assertions before sample migration.
- Chess may retain opaque visual prefabs. Hearts cards use Rust-composed hosts.
  Typed prefab-part binding and humanoid root motion remain outside v1.
- Rules may expose generic mode/policy parameters. One synchronous
  implementation serves interactive play and statically dispatched
  allocation-free primitives in simulation.
- Hearts is a fixed-rule four-player game: one human, three bounded simulation
  AI opponents, angled 3D tabletop, full input support, autosave and resume. Its
  exact rules/UX/defaults are in [Hearts](hearts.md).
- Desktop native and actual threaded WebGL functional validation are required.
  Add mobile support and reproducible validation paths; physical iPhone 17 and
  Galaxy S25 certification is a separate follow-up.
- Keep the 300/500-card benchmarks and sustained captures. Report the proposal's
  performance numbers as targets, not completion gates. Correctness and the
  simulation primitive contract remain mandatory.
- Implementors may improve APIs and add reusable engine capabilities needed by
  their task. Changes to agreed semantics, scope, or acceptance need a user
  decision. Update the owning contract and dependent callers when APIs evolve.

## Reading guide

Start with this document, then open only the selected task and relevant shared
contracts. Each rule has one authoritative home; task acceptance specializes it.

| When should you read this? | Document |
| --- | --- |
| Starting or handing off any task | [Serial workflow](workflow.md) and [validation](validation.md) |
| Finding existing code or following moved modules | [Source map](source-map.md) |
| Choosing crate/API/host ownership | [Architecture and authoring](architecture.md) |
| Implementing game/choice traits, endpoint records, host messages | [Minimum interfaces](interfaces.md) |
| Rules, choices, simulation, cancellation, saving | [Execution](execution.md) |
| Preparation, commit identity, frames, required gates | [Presentation](presentation.md) |
| UUID moves, refs, context, removal, stores | [Identity](identity.md) |
| Animation, effects, property owners, sequencing, replay | [Motion](motion.md) |
| Primitives, layouts, projection, hit testing/input | [World](world.md) |
| Existing samples or test rewrites | [Migration](migration.md) |
| Hearts rules, assets, AI, input, persistence | [Hearts](hearts.md) |
| Inspector, neutral specimens, performance fixtures | [Fixtures](fixtures.md) |
| Checking that all proposal requirements have an owner | [Coverage](coverage.md) |

## Task order

The sequence is strictly serial. Every task depends on all earlier integrated
tasks, even when its page highlights only the immediate predecessor. Task
numbers are review/integration boundaries, not permission to leave unmentioned
behavior broken.

Each task supplies source roles, ordered work, exact observable acceptance,
named deferrals, and manual QA. The source map resolves roles to actual files
and must be updated when files move. A task changing an API updates all current
callers immediately; later sample tasks migrate behavior, not broken imports.

Tasks 03-05 are real platform work, not speculative design spikes that can be
declared successful from compiler flags. Missing toolchain or asset
prerequisites must be reported concretely. Task 35 needs access to the KayKit
EXTRA archive; earlier engine tasks do not.


### Execution foundations

1. [Establish behavioral baselines and classify existing
   tests](tasks/01-behavior-baseline.md)
2. [Define the typed rules API and prove the simulation fast
   path](tasks/02-rules-api-simulation.md)
3. [Prove native worker cancellation and Rust
   cleanup](tasks/03-native-cancellation.md)
4. [Prove cancellation in the threaded WebGL release
   path](tasks/04-threaded-webgl-proof.md)
5. [Prepare iOS and Android release validation
   paths](tasks/05-mobile-build-paths.md)
6. [Extract shared Reactant core, UI layer, and
   facade](tasks/06-crate-boundaries.md)
7. [Move Reactant asset preparation out of
   Battlement](tasks/07-asset-tooling-boundary.md)
8. [Add public display driving and deterministic virtual
   time](tasks/08-public-display-driver.md)
9. [Implement immutable checkpoint publication and
   backpressure](tasks/09-checkpoint-publication.md)
10. [Implement typed interactive prompts and validated
    answers](tasks/10-typed-prompts.md)
11. [Integrate action admission, accepted state, and failure
    surfaces](tasks/11-accepted-action-runtime.md)
12. [Add prepared host commits and rendered-frame
    acknowledgement](tasks/12-host-transactions.md)

### Unified objects, input, and motion

13. [Render world and UI contributions from one logical
    tree](tasks/13-mixed-logical-tree.md)
14. [Preserve UUID identity across parents, roots, and
    portals](tasks/14-global-presentation-identity.md)
15. [Separate logical unmount from retained visual
    lifetime](tasks/15-incarnations-and-removal.md)
16. [Add stable snapshot selectors and queued display
    stores](tasks/16-snapshot-selectors-stores.md)
17. [Add sprites, meshes, world text, and material
    overrides](tasks/17-world-rendering-primitives.md)
18. [Add independent hit regions and typed Rust-created
    anchors](tasks/18-hit-regions-anchors.md)
19. [Unify UI/world hit testing, propagation, and modal
    capture](tasks/19-unified-pointer-routing.md)
20. [Extend focus, controller navigation, and touch across
    domains](tasks/20-world-focus-touch.md)
21. [Use shared Motion drivers for UI, world, and native
    properties](tasks/21-shared-motion-drivers.md)
22. [Implement immutable sequences and completion-relative
    labels](tasks/22-sequence-dependencies.md)
23. [Implement world Flexbox, Grid, fans, piles, and
    arcs](tasks/23-world-layout.md)
24. [Animate layout movement with continuous
    retargeting](tasks/24-layout-movement-projection.md)

### Presentation and existing sample migrations

25. [Bind checkpoint registrations to Motion and rendered
    acceptance](tasks/25-checkpoint-motion-gates.md)
26. [Schedule sound, particles, and attached effects on the shared
    clock](tasks/26-effect-occurrences.md)
27. [Retain exits, anchors, and effects after logical
    unmount](tasks/27-effect-exit-retention.md)
28. [Implement safe seek, resume, and explicit presentation
    replay](tasks/28-inspection-replay.md)
29. [Build the reusable presentation
    inspector](tasks/29-presentation-inspector.md)
30. [Migrate tic-tac-toe through the unified rules and display
    path](tasks/30-tictactoe-migration.md)
31. [Port chess board composition and move presentation in a
    fixture](tasks/31-chess-reactant-fixture.md)
32. [Complete chess application integration and remove the old
    engine](tasks/32-chess-cutover.md)
33. [Migrate the existing Reactant laboratory to the unified
    APIs](tasks/33-reactant-sample-migration.md)
34. [Migrate the currently implemented chess UI
    gallery](tasks/34-chess-ui-migration.md)

### Playable Hearts reference

35. [Prepare Hearts assets and its 3D sample
    shell](tasks/35-hearts-assets-shell.md)
36. [Implement the fixed Hearts rules through the generic
    executor](tasks/36-hearts-rules.md)
37. [Compose Hearts cards, hands, tricks, and inspection
    views](tasks/37-hearts-card-layout.md)
38. [Connect Hearts passing prompts and simultaneous
    transfers](tasks/38-hearts-passing.md)
39. [Complete Hearts card play, trick collection, and
    scoring](tasks/39-hearts-play-scoring.md)
40. [Add bounded information-respecting Hearts
    simulations](tasks/40-hearts-simulation-ai.md)
41. [Finish Hearts pointer, touch, drag, and inspection
    behavior](tasks/41-hearts-pointer-touch.md)
42. [Finish Hearts keyboard/controller navigation and
    menus](tasks/42-hearts-navigation-menus.md)
43. [Implement durable accepted-boundary Hearts
    saves](tasks/43-hearts-save-resume.md)

### Complete coverage and integration

44. [Complete identity, composition, layout, and input laboratory
    cases](tasks/44-identity-composition-laboratory.md)
45. [Complete effects, preparation, cancellation, and gate laboratory
    cases](tasks/45-effects-failures-laboratory.md)
46. [Measure complete-card workloads and repair structural
    hotspots](tasks/46-performance-workloads.md)
47. [Run final native, threaded-WebGL, and mobile build
    conformance](tasks/47-release-conformance.md)
48. [Remove transitional machinery and audit the finished
    architecture](tasks/48-retire-adapters-final-audit.md)

## Manual QA

A coordinator should hand one task to a fresh reader with only this package and
the repository. The reader must locate the starting code, explain the fixed
contracts, and identify the exact behavior/tests that make the task complete.
For the final engine, play Hearts and the migrated samples, then use the
laboratory/inspector to exercise identity, failure, cancellation, and timing
cases.
