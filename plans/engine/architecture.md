# Engine architecture and authoring contract

Read this when changing crate ownership, the application API, host composition,
or the boundary between game rules and presentation. Start with the [entry
point](README.md), then the [source map](source-map.md).

## Purpose and ownership

Reactant is a Rust-authored component engine for both world objects and UI.
Battlement executes reusable Unity capabilities. A game supplies rules, views,
semantic changes, and presentation policies in Rust. Unity authors assets and
executes rendering, input, motion, and audio; it does not implement Hearts or
other game-specific presentation decisions.

Keep both projects in this repository. The dependency direction applies to
runtime code, build tools, tests, and Unity editor code, not just Cargo names.

| Owner | Responsibility |
| --- | --- |
| reactant-core | Logical tree, hooks, context, stores, identity, host-neutral refs and commits, shared Motion authoring |
| reactant-ui | UI controls, UI-specific properties and native document adapters |
| reactant-rules | Synchronous rules executor, interactive worker and simulation adapters; no Unity or component dependency |
| reactant | Application, world components, layouts, checkpoint presentation, effects, facade reexports |
| reactant-testing | Public display driver and scenario observations above Battlement fakes |
| Reactant asset crates and CLI | Generated paint, asset declarations and Reactant-specific preparation |
| Battlement crates and Unity package | Protocol, C ABI, host execution, generic asset loading and low-level fakes |
| Game Rust code | State, actions, prompts, legal choices, AI policy, persistence format and effect selection |

These are the planned crate names. Use a single Cargo workspace with standalone
sample workspaces as today. The facade may depend on core, UI, and rules; core
must not depend on the facade or UI authoring crate. The UI adapter implements
the core host interface using Battlement UI protocol types. Protocol types in
Battlement may describe UI properties without introducing a Reactant dependency.

Move existing useful code. Do not keep a second component runtime or introduce
permanent compatibility aliases for battlement-reactant. Task 06 performs the
mechanical caller migration; later tasks migrate behavior to the new services.

## Public application shape

A UI-only application must remain easy to author without a rules worker. A game
adds an accepted state, an action executor, and a presented-snapshot provider.
Do not make hooks, UI components, or local settings depend on a GameState type.

The intended author experience is illustrated below; these are design examples,
not existing compilable APIs:

~~~rust
App::new()
    .scene(assets::TABLE)
    .game(HeartsGame::new().state(saved_or_new))
    .movement(Transition::spring())
    .root(HeartsTable::new())
~~~

- Builders use argument-free new() and setters on newly introduced or migrated
  Reactant authoring surfaces. Keep direct Battlement APIs outside this rule.
- Props and context are complete authoring paths. Selector hooks are optional.
- Components return mixed world and UI contributions from one logical tree.
  Explicit UI roots or portals determine native UI attachment.
- Event handlers dispatch actions or update display stores; they do not mutate
  the worker's game state.
- Store writes enqueue later work. Every render reads a stable store version and
  one immutable presented snapshot.
- Keep native default prevention synchronous; defer ordinary reconciliation.
- Public gameplay dispatch reports Busy, Invalid(reason), or Started(run_id)
  without exposing worker internals. Started means admission, not final state
  acceptance; validate actions against accepted state before forking a worker.
- A headless simulation can render its result through the same display API.

## Host abstraction

Replace the UI-only host field in the render tree with a typed host description
and domain adapter. Keep logical identity and hooks above native host kinds.

A core host description must provide stable kind/compatibility information,
prepared dependencies, properties, child attachment rules, and event/ref
capabilities. Use ordinary traits and typed descriptors, with erasure only at
the heterogeneous tree boundary. Do not serialize game types into Unity.

World groups, sprites, meshes, text, hit regions, anchors, and opaque prefabs
are host kinds. UI remains UI Toolkit. World Flexbox/Grid uses Taffy; custom
world layouts are pure Rust algorithms. Neither domain borrows the other's
units.

An opaque prefab is a visual asset under a Reactant-owned root. It may expose
the existing generic whole-object host capabilities. Do not add named-part
lookup, typed prefab binding, or game-specific C# callbacks. Hearts cards must
be composed in Rust; chess may retain its existing opaque visual prefabs.

## Tooling direction

Move Reactant asset generation out of battlement-cli. Add an optional generic
sample preparation command represented as an executable plus argument array in
sample.toml. Execute it from the sample directory without a shell, before
building/importing generated inputs, and propagate a nonzero exit status.
Reactant samples configure this to invoke the checkout's reactant-cli.

Keep generic Addressables generation, plugin builds, and Unity player packaging
in Battlement. Split Reactant-specific Unity preparation from generic asset
import helpers; no Battlement assembly may reference a Reactant assembly. The
repository CI orchestrator may invoke both projects.

## Initiative and boundaries

Implementors should simplify public APIs, replace unnecessary framework
plumbing, and add reusable engine features needed by their assigned scenarios.
Update callers, examples, the owning shared contract, and later task pointers in
the same task when an API changes.

Changing agreed observable semantics, omitting a feature, expanding to another
game/repository, or weakening acceptance requires a user decision. Do not freeze
illustrative spellings at the expense of ergonomics. Do preserve the specified
ownership, ordering, lifetime, and simulation guarantees.

No Dreamtides code, assets, prefab audit, or repository work belongs here.
Prefab binding and humanoid root motion are outside this implementation.

## Manual QA

Build a tiny component with a world visual and a UI portal contribution. Change
a shared setting and verify both update through the same context. Build a
UI-only sample without constructing a game state or starting a rules worker.
