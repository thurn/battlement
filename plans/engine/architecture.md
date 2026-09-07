# Writing a game with Reactant

Reactant lets Rust components describe a game's world objects and UI. A game
implements `Game`, supplies a domain-specific `GameContext`, and uses the same
synchronous rules for interactive execution and simulation. Display components
read immutable clones of the full state and a shared prompt enum.

Read the [complete rules/session API](interfaces.md) and [compiling
sketch](interfaces.md#complete-contract-sketch) for the contract.
Related pages: [overview](README.md), [execution](execution.md), [world
objects](world.md), and [starting code](source-map.md).

## The app owns the live session

Use the existing app/component setup, then start a new or loaded game:

```rust
let game = app.start_game::<HeartsGame>(initial_state, |connection| {
    HeartsContext {
        human_player,
        mode: HeartsMode::Interactive { connection, policy: HeartsPolicy },
    }
});
```

`start_game` creates the connection, constructs the context, and attaches the
session to the display and hooks. It returns a cloneable `GameHandle` for
`dispatch`, `accepted_state`, `status`, and `stop`. Calling it again replaces
the old session. No separate registration, attachment, or acceptance callback is
required. Starting alone does not execute rules.

The roles are:

- `HeartsGame` names associated types and implements `logical_clone`,
  `is_legal_action`, and `execute` as static methods.
- `HeartsState` holds rules data. Logical clones serve as accepted/worker state
  and immutable snapshots for display; there is no separate view type.
- `HeartsContext` owns the interactive/simulation mode and domain data such as
  policies or RNGs. It routes human versus AI choices in a live game.
- `HeartsPrompt<'a>` wraps concrete choice structs in `Cow`: borrowed for policy
  calls, owned for display. There is no separate reference enum.
  `PresentedPrompt<HeartsPrompt<'static>>` adds a request-bound response handle
  for UI.
- `HeartsAnimation` describes what happened. Display registrations translate
  that event into movements, sound, and particles.
- `HeartsDisplay` is a component that builds the scene from each queued
  snapshot and its prompt. `CardView` is a component, not a game-state type.

Display components can read full state, but must render opponents' cards as
backs and keep hidden values out of normal player UI/inspection. Policies
likewise receive state; the game's search code owns hidden-state randomization
and information-safe heuristics. Reactant supplies no separate observation or
controller-message types.

Rules use one `present` event per checkpoint and typed `choose` responses. The
[dispatch sequence](execution.md#from-dispatch-to-accepted-state) defines
worker ownership, bounded publication, and final acceptance. The Rust consumer
renders queued snapshots and submits ordinary ordered Battlement batches
without waiting for Unity. Battlement waits for blocking operations before
executing subsequent commands.

Movement needs no configuration. Existing objects use an engine default
transition, customizable locally or through inherited `MotionConfig`. Entry and
restoration show current state without replaying past transient events.

A UI-only application uses the existing `App::ui`/root setup without a game
session or worker. Keep basic and ui as direct Battlement examples.

## One component can produce UI and 3D objects

The **logical tree** is the component hierarchy that determines hook lifetime,
context lookup, and event propagation. Unity's transform hierarchy and UI
Toolkit's element hierarchy are physical destinations for its output.

For example, one card component can contribute a world visual and details in a
UI panel. Both read the same card data and selection context:

```rust
(
    CardView::new().card(card)
        .on_click(move |_| selection.inspect(card.id)),
    Portal::to(details_panel)
        .child(CardDetails::new().card(card)),
)
```

An explicit UI root or portal selects the native UI container. It does not
create a second Reactant state tree. Moving a component changes its context and
event ancestry without losing compatible state; see [identity](identity.md).

Props and context must be complete ways to pass data. Selector hooks are an
optional optimization: a score label can subscribe only to the score and avoid
reevaluation when the hand changes. Local menus and inspection use hooks or
stores, rather than mutating the rules worker's state.

Store writes schedule another render. Each render sees one game snapshot and one
stable store version. Ordinary host animation does not trigger a component
render every frame. Input handlers may synchronously prevent native defaults;
reconciliation happens afterward.

## Crate and Unity responsibilities

Reuse the existing runtime and animation code while separating UI-specific
implementation from shared component behavior. Keep one Cargo workspace and the
existing standalone sample workspaces.

- `reactant-core` owns the component tree, hooks, context, stores, identity,
  refs, shared animation authoring, and host interfaces.
- `reactant-ui` owns UI controls, UI properties, and UI Toolkit adapters using
  the shared runtime.
- `reactant-rules` owns execution, worker communication, generic typed response
  machinery, and connection mechanics. It has no component or Unity dependency.
- `reactant` owns application registration, world components, layouts,
  snapshot-to-batch integration, effects, and convenient reexports.
- `reactant-testing` owns public display scenarios using Battlement's fake host.
- The Reactant-owned `rt` command owns general project build, run, authoring,
  Ditto, plugin, Addressables, and Reactant asset workflows. Reactant asset
  libraries own declarations, generated paint, and related preparation.
- Battlement owns the protocol, C ABI, generic Unity hosts, generic asset
  loading, command scheduling and operation completion, and low-level fakes.
- Game Rust code owns rules, state, action validation, owned prompt data and
  policies, AI, saves, display components, and effect selection.

The dependency goes from Reactant to Battlement. This includes Rust crates,
Unity assemblies, tests, and build tools. Core cannot depend on the facade or UI
authoring crate. UI adapters implement core's host interface; Battlement may
contain generic UI protocol properties without importing Reactant.

Move useful implementations instead of introducing parallel runtimes. Remove the
old `battlement-reactant` package when callers move to the new crates. Tasks
that change imports must update all existing callers immediately.

## A shared interface for native objects

A **host** is a native object controlled by a Reactant declaration, such as a
Unity sprite or a UI Toolkit label. Core needs enough information to create,
update, attach, and release a host without knowing every native property type.

The host interface must describe:

- Its native kind and whether an existing object can be reused.
- Required assets and other dependencies that must be ready before display.
- Typed properties and permitted child attachments.
- Supported input events and typed references.

Keep type erasure at the heterogeneous tree boundary. Do not serialize game
state or game-specific decisions into Unity. World groups, sprites, meshes,
text, hit regions, anchors, cameras, lights, and opaque prefabs use this shared
interface. UI remains UI Toolkit; world Flexbox and Grid use Taffy.

An opaque prefab is loaded as a visual beneath a Reactant-owned root. Chess can
continue using existing piece models this way. It does not introduce named-part
lookup, typed prefab binding, or game-specific C# callbacks. Hearts and the
richer card test scenes must construct their visual children in Rust.

## One CLI owns project workflows

`rt` is the only public command-line entry point. It belongs to Reactant and
works from explicit project inputs rather than knowledge of Battlement's
repository layout. Its general commands include:

```text
rt build --project path/to/game
rt run --project path/to/game
rt author --project path/to/game
rt ditto --config path/to/game/ditto.toml gallery
rt addressables check --project path/to/game
rt plugin inspect path/to/game.app
```

`rt build` prepares Reactant assets, builds the Rust application plugin and
Unity player, and reports the output. `rt run` invokes that same build operation
before launching the native player or serving the Web build; normal Cargo and
Unity incremental behavior may avoid unchanged work. `build` and `run` preserve
the existing `--web` and `--release` choices. Web runs accept `--port`,
defaulting to 8000. `rt author` prepares the same project inputs, opens Unity,
and enters Play mode. Typed identifier generation and checking live under
`rt addressables`. Native-plugin inspection, installation, restoration, and
verification remain under `rt plugin` with their existing macOS application and
signing scope. Reactant asset discovery, generation, checking, and preview
remain under `rt assets`.

`rt ditto` supports both Reactant projects and direct Battlement fixtures. A
Ditto player's configuration explicitly declares whether it uses Reactant:

```toml
[player]
reactant = true
unity_project = "."
```

When `reactant` is true, `rt` resolves `unity_project` relative to the Ditto
configuration, loads that project's `reactant.toml`, and runs the same asset
preparation used by `rt build` before calling the generic Ditto library. When it
is false, `rt` skips Reactant preparation. The generic Ditto library performs
player build and execution in both cases without depending on Reactant crates.

For `build`, `run`, `author`, and `assets`, `--project` identifies the Unity
project root and defaults to the current directory. That root contains
`reactant.toml`, whose minimum project contract is:

```toml
[project]
application = "Card Table"
scene = "Assets/Scenes/CardTable.unity"
manifest-path = "rules/Cargo.toml"
```

`application` and `scene` are required. `manifest-path` defaults to
`rules/Cargo.toml`; it names the Rust application plugin for either a game or a
UI-only Reactant app. Paths in the file resolve from the project root. Explicit
command flags override the corresponding file values. Target, profile, output,
and Web-server options remain command flags because they describe an invocation,
not the project. `addressables` needs only an explicit Unity project path and
`plugin` uses explicit application/library inputs; neither command requires
`reactant.toml`. `ditto` uses its explicit configuration to decide whether to
load Reactant metadata. The CLI must not enumerate sample names, assume a
`samples/` directory, default to chess, or contain any other
Battlement-checkout policy.

Repository convenience belongs in [justfile](../../justfile). Its recipes map
sample names and repository defaults to explicit `rt` invocations. For example:

```text
just sample reactant --web
# delegates to rt run --project samples/reactant --web
```

Direct Battlement examples such as basic and ui remain independent of Reactant.
Only `justfile` selects their names and repository defaults. Lower-level scripts
must accept explicit project paths and options, and must not contain their own
sample registry. These fixtures do not justify a second public CLI.

Battlement owns reusable native-plugin, Unity-player, Addressables, and import
mechanics. `rt` composes those lower-level operations with Reactant preparation,
so the dependency still points from Reactant to Battlement. Reactant-specific
Unity preparation uses generic import helpers from its own assembly;
Battlement assemblies never import Reactant assemblies. A preparation or build
failure stops the enclosing `rt` command and reports the failed operation.

The existing `battlement-cli` package is renamed and moved to Reactant ownership
as the `rt` package. It declares one binary, also named `rt`; the
`cargo-battlement` binary is removed. The `battlement-ditto` package becomes
library-only. Reusable implementations from both packages remain library code
called by `rt`, not alternate public entry points.

## Manual QA

Create a small component with a world visual and UI details in a portal. Change
a shared setting and verify both update. Move the component between parents and
check its retained state and new context. Also run a UI-only application and a
game with no movement configuration; both should work without extra setup.
