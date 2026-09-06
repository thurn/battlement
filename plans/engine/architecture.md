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
- `HeartsDisplay` is a component that builds the scene from the displayed
  snapshot and active prompt. `CardView` is a component, not a game-state type.

Display components can read full state, but must render opponents' cards as
backs and keep hidden values out of normal player UI/inspection. Policies
likewise receive state; the game's search code owns hidden-state randomization
and information-safe heuristics. Reactant supplies no separate observation or
controller-message types.

Rules use one `present` event per checkpoint and typed `choose` responses. The
[dispatch sequence](execution.md#from-dispatch-to-accepted-state) defines worker
ownership, bounded publication, and final acceptance. The display owns the queue
and decides when required animation is complete; rules may compute ahead.

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
  checkpoint presentation, effects, and convenient reexports.
- `reactant-testing` owns public display scenarios using Battlement's fake host.
- Reactant asset libraries and the CLI own Reactant asset declarations,
  generated paint, and related preparation.
- Battlement owns the protocol, C ABI, generic Unity hosts, generic asset
  loading, and low-level fakes.
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
