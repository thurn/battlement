# Build a Rust component engine for Unity

Reactant currently provides React-like UI components in Battlement. This work
extends that component system to describe an entire turn-based game: its 3D
objects, menus, animations, effects, and interaction. Game rules stay in Rust.
Unity renders the result and supplies reusable input, animation, and audio
capabilities.

The central change is that a game describes what should be visible instead of
manually sending Unity a sequence of object commands. A card declared in a hand
moves to the table when its next declaration puts it there. Its UUID preserves
its component state and visual identity during that move. UI and 3D objects
share props, hooks, context, and event propagation.

Each game implements `Game` with state, actions, a shared prompt enum, semantic
animation events, and a domain-specific context. `App::start_game` supplies the
interactive connection and returns a session handle. `Game::execute` runs on a
worker as ordinary synchronous Rust. It can publish a state snapshot, ask a
player to choose a card, and continue after that response. The display sequences
snapshots and animation while menus stay responsive. A 32-slot pending queue
lets live AI choices run ahead; only a full queue delays their publication. MCTS
calls the same rules with a simulation context and an index-returning policy.

The [rules/session contract](interfaces.md) and its [compiling
sketch](interfaces.md#complete-contract-sketch) are the complete public contract
for this part of the system. The topic pages and tasks below use that contract;
a compiling placeholder is not a working engine.

## A small example

A **checkpoint** is an immutable logical clone of `Game::State`, optionally
paired with one `StateAnimation` event or an active prompt. The snapshot says
what is visible; display code interprets the event to select movements, sounds,
and effects. The display presents checkpoints in order. For example, a draw
should become visible before the energy it grants changes on screen:

```rust
draw_card(state);
cx.present(state, || StateAnimation::CardDrawn(card));
gain_energy(state);
cx.present(state, || StateAnimation::EnergyGained(1));
```

Interactive publication calls `Game::logical_clone`; simulation skips both the
copy and the lazy animation builder. Prompt data owns its choices, and policies
inspect a borrowed `Cow` wrapper in the same enum used for owned display data.
Each implementation task must prove the public behavior, including typed human
responses and policy indices.

The display component uses the current snapshot to describe both domains:

```rust
(
    Hand::new().cards(state.hand()),
    UiRoot::new().child(EnergyLabel::new().amount(state.energy())),
)
```

The display can animate the drawn card before showing the next checkpoint.
Hover, inspection, and settings remain usable while that animation runs. [Rules
and choices](execution.md#from-dispatch-to-accepted-state) defines the complete
dispatch-to-acceptance sequence; [animation](motion.md) shows the corresponding
draw sequence.

## What this implementation includes

The deliverable is a working engine, migrated samples, and a playable Hearts
sample that exercises the engine as an application.

- **One component runtime:** shared state and identity for UI Toolkit elements
  and Unity world objects, including movement between parents and portals.
- **Rust-built visuals:** sprites, meshes, world text, hit regions, attachment
  points, layouts, material effects, and card hierarchies composed in Rust.
- **One animation system:** property changes, layout movement, gestures,
  sequences, audio, and particles share timing and playback controls. Ordinary
  movement has an engine default; applications need not configure it.
- **Synchronous rules:** intermediate state snapshots, typed choices, safe
  cancellation, and final state acceptance after presentation finishes.
- **Simulation:** the same rules with inline choice policies and no mandatory
  allocation or display work in execution primitives; owned prompt construction
  and policy work are measured separately.
- **Sample migration:** preserve tic-tac-toe, chess, the Reactant laboratory,
  and the implemented chess-ui gallery. Keep basic and ui as direct Battlement
  examples. Preserve player-visible tests, replacing assertions about obsolete
  commands with assertions about the behavior those commands produced.
- **Hearts:** one human and three AI players, a 3D table, full mouse/touch and
  keyboard/controller input, scoring, and explicit durable save/load. V1 has no
  autosave.
- **Validation tools:** a public display test driver, a presentation inspector,
  focused interactive examples, and fixed 300/500-card performance workloads.

Reactant depends on Battlement in this repository. Battlement supplies generic
Unity execution and must not depend on Reactant, including in build tools. Chess
may keep its opaque piece prefabs. Rust builds the Hearts cards and the richer
card-composition examples from primitives. Typed access to prefab parts and
humanoid root motion are not included in this implementation.

Desktop native and threaded desktop WebGL must pass functional validation. Add
reproducible iOS and Android build and validation paths. Physical iPhone 17 and
Galaxy S25 certification is tracked separately. The numerical performance
targets in [test scenes and performance](fixtures.md) guide measurement and
improvement; missed targets must be reported, but do not waive correctness or
change the workload.

## Where to read next

The topic documents explain the design. A numbered task is the assignment for
its part of the implementation; it links the relevant topics and starting code.

| When should you read this? | Document |
| --- | --- |
| Understanding what game code looks like and who owns what | [Architecture](architecture.md) |
| Implementing or simplifying the rules API | [Rules and session API](interfaces.md) |
| Working on rules, choices, cancellation, simulation, or saving | [Execution](execution.md) |
| Updating visible objects or waiting for animation before advancing | [Presentation](presentation.md) |
| Moving components, preserving refs, or removing objects | [Identity and state](identity.md) |
| Building transitions, sequences, effects, or replay controls | [Animation](motion.md) |
| Adding world visuals, layouts, or input | [World objects](world.md) |
| Migrating a sample or its tests | [Migration](migration.md) |
| Implementing the playable card game | [Hearts](hearts.md) |
| Building interactive examples, inspector tools, or benchmarks | [Test scenes and performance](fixtures.md) |
| Checking requirements against their tasks and evidence | [Coverage checklist](coverage.md) |
| Finding the current implementation | [Source map](source-map.md) |
| Starting a task or validating a completed one | [Workflow](workflow.md) and [validation](validation.md) |

## Implementation order

Execute the tasks below serially. Each task builds on all earlier completed
ones. Keep existing callers working when an API changes; a later sample
migration is not permission to leave that sample unable to compile.

Improve API ergonomics and add reusable capabilities when the assigned behavior
needs them. Keep the examples and affected task pages consistent with those
improvements. Do not remove requirements or change game behavior to fit an
implementation shortcut.

Tasks 03–05 establish working platform support, including actual worker
cancellation through Rust/Unity. Task 35 needs access to the KayKit EXTRA asset
archive; earlier engine work does not depend on those assets.

### Execution foundations

1. [Establish behavioral baselines and classify existing
   tests](tasks/01-behavior-baseline.md)
2. [Define the typed rules API and prove the simulation
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
9. [Publish immutable checkpoints through a 32-slot
   queue](tasks/09-checkpoint-publication.md)
10. [Implement typed interactive prompts and validated
    responses](tasks/10-typed-prompts.md)
11. [Start game sessions, accept actions, and expose
    recovery](tasks/11-accepted-action-runtime.md)
12. [Prepare native updates and acknowledge rendered
    frames](tasks/12-host-transactions.md)

### Unified objects, input, and motion

13. [Render world and UI contributions from one logical
    tree](tasks/13-mixed-logical-tree.md)
14. [Keep UUID identity across parents, roots, and
    portals](tasks/14-global-presentation-identity.md)
15. [Remove components while keeping unfinished exit
    visuals](tasks/15-incarnations-and-removal.md)
16. [Add stable state selectors and queued display
    stores](tasks/16-view-selectors-stores.md)
17. [Add sprites, meshes, world text, and material
    overrides](tasks/17-world-rendering-primitives.md)
18. [Add independent hit regions and typed Rust-created
    anchors](tasks/18-hit-regions-anchors.md)
19. [Unify UI/world hit testing, propagation, and modal
    capture](tasks/19-unified-pointer-routing.md)
20. [Extend focus, controller navigation, and touch across
    domains](tasks/20-world-focus-touch.md)
21. [Animate UI, world objects, and effects with shared
    drivers](tasks/21-shared-motion-drivers.md)
22. [Build sequences with labels that follow actual
    completion](tasks/22-sequence-dependencies.md)
23. [Implement world Flexbox, Grid, fans, piles, and
    arcs](tasks/23-world-layout.md)
24. [Animate layout movement with continuous
    retargeting](tasks/24-layout-movement-projection.md)

### Presentation and existing sample migrations

25. [Start checkpoint animations and wait before
    advancing](tasks/25-checkpoint-motion-gates.md)
26. [Schedule sounds, particles, and attached
    effects](tasks/26-effect-occurrences.md)
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
36. [Implement fixed Hearts rules through the shared context
    contract](tasks/36-hearts-rules.md)
37. [Compose Hearts cards, hands, tricks, and inspection
    views](tasks/37-hearts-card-layout.md)
38. [Connect Hearts passing prompts and simultaneous
    transfers](tasks/38-hearts-passing.md)
39. [Complete Hearts card play, trick collection, and
    scoring](tasks/39-hearts-play-scoring.md)
40. [Choose Hearts moves by simulating possible
    hands](tasks/40-hearts-simulation-ai.md)
41. [Finish Hearts pointer, touch, drag, and inspection
    behavior](tasks/41-hearts-pointer-touch.md)
42. [Finish Hearts keyboard/controller navigation and
    menus](tasks/42-hearts-navigation-menus.md)
43. [Save Hearts explicitly and resume accepted
    state](tasks/43-hearts-save-resume.md)

### Complete coverage and integration

44. [Complete component, layout, and input test
    scenes](tasks/44-identity-composition-laboratory.md)
45. [Complete animation, cancellation, and failure test
    scenes](tasks/45-effects-failures-laboratory.md)
46. [Measure complete-card workloads and repair structural
    hotspots](tasks/46-performance-workloads.md)
47. [Validate native, threaded WebGL, and mobile
    builds](tasks/47-release-conformance.md)
48. [Remove transitional machinery and audit the finished
    architecture](tasks/48-retire-adapters-final-audit.md)

## Manual QA

Open the task you are implementing and follow its linked examples and source
pointers. You should be able to explain the intended user interaction and the
observations that prove it works. For final validation, play Hearts and each
migrated sample, then exercise cancellation, object movement, and animation
replacement in the test scenes using the inspector.
