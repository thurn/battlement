# Writing a game with Reactant

Reactant lets Rust components describe both a game's world objects and its UI.
The game supplies state and synchronous rules functions. A separate display
component reads immutable snapshots and describes the visible result. Unity
executes generic capabilities; it does not decide how Hearts or chess should
look or behave.

Read this when adding an application, separating crate responsibilities, or
working on component composition. Related pages: [overview](README.md), [API
examples](interfaces.md), [rules](execution.md), [world objects](world.md), and
[starting code](source-map.md).

## State, rules, and display are separate

Use ordinary game types and functions. There is no requirement to implement a
large `Game` trait just to start an application.

- `HeartsState` holds hands, tricks, scores, and the other rules data.
- `resolve` is a synchronous Rust function that changes a private state copy.
- `HeartsView` contains the information the human player may see.
- `HeartsDisplay` is the root component. It composes the 3D table and cards as
  well as UI menus, scores, and prompts.

For example, application registration can infer its types from state and
callbacks. This illustrates the API to build, rather than an existing API:

```rust
let game = Game::new()
    .state(saved_or_new)
    .snapshot(HeartsState::visible_to_south)
    .changes::<Change>()
    .validate(validate_action)
    .rules(resolve);
App::new().game(game).root(HeartsDisplay::new())
```

The snapshot callback is optional when cloning the whole state is appropriate.
Hearts supplies it because normal UI must never receive opponents' hidden cards.
`Game` here is a registration builder, not a game-defined trait or a second
state object. [API examples](interfaces.md) specifies inference and defaults.

Movement is optional configuration. With no `.movement()` call, an existing
object moves using the engine's default transition. A game can customize that
transition locally or through inherited `MotionConfig`. Entry and restoration
show the current state using entry behavior; they do not replay past actions.

A UI-only application needs no game registration or worker:

```rust
App::new().root(SettingsPanel::new())
```

## One component can produce UI and 3D objects

The **logical tree** is the component hierarchy that determines hook lifetime,
context lookup, and event propagation. Unity's transform hierarchy and UI
Toolkit's element hierarchy are physical destinations for its output.

For example, one card component can contribute a world visual and details in a
UI panel. Both read the same card data and selection context:

```rust
(
    CardVisual::new().card(card)
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

Store writes schedule another render. Each render sees one snapshot and one
stable store version. Ordinary host animation does not trigger a component
render every frame. Input handlers may synchronously prevent native defaults;
reconciliation happens afterward.

## Crate and Unity responsibilities

Reuse the existing runtime and animation code while separating UI-specific
implementation from shared component behavior. Keep one Cargo workspace and the
existing standalone sample workspaces.

| Owner | Responsibility |
| --- | --- |
| `reactant-core` | Component tree, hooks, context, stores, identity, refs, shared animation authoring, and host interfaces |
| `reactant-ui` | UI controls, UI properties, and UI Toolkit adapters using the shared runtime |
| `reactant-rules` | Synchronous execution, worker communication, choices, and simulation; no component or Unity dependency |
| `reactant` | Application registration, world components, layouts, checkpoint presentation, effects, and convenient reexports |
| `reactant-testing` | Public display scenarios using Battlement's fake host |
| Reactant asset libraries and CLI | Reactant asset declarations, generated paint, and related preparation |
| Battlement | Protocol, C ABI, generic Unity hosts, generic asset loading, and low-level fakes |
| Game Rust code | Rules, state, action validation, choices, AI, save format, display components, and effect selection |

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

## Asset preparation follows the same dependency direction

Battlement continues to build native plugins and Unity players and generate
Addressables identifiers. Reactant-specific asset generation moves to
`reactant-cli` and Reactant's Unity editor integration.

A sample can configure a preparation executable and argument list in
`sample.toml`. The generic builder executes it from the sample directory,
without a shell, before importing generated inputs. A nonzero result stops the
build and reports the failure. Reactant samples configure this hook to invoke
the checkout's Reactant CLI.

Generic Unity import helpers remain in Battlement. Reactant-specific preparation
uses those helpers from its own assembly; Battlement assemblies never import
Reactant assemblies. Repository CI may orchestrate both projects.

## Manual QA

Create a small component with a world visual and UI details in a portal. Change
a shared setting and verify both update. Move the component between parents and
check its retained state and new context. Also run a UI-only application and a
game with no movement configuration; both should work without extra setup.
