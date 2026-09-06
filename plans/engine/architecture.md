# Writing a game with Reactant

Reactant lets Rust components describe both a game's world objects and its UI.
A game implements one `Game` trait that names its state, actions, player-visible
view, state animations, and synchronous action method. A separate display
component reads the immutable view and describes the visible result. Unity
executes generic capabilities; it does not decide how Hearts or chess should
look or behave.

Read this when adding an application, separating crate responsibilities, or
working on component composition. Related pages: [overview](README.md), [API
examples](interfaces.md), [rules](execution.md), [world objects](world.md), and
[starting code](source-map.md).

## The Game trait owns the rules contract

The application registers one game value. Its `Game` implementation associates
all types that the engine would otherwise have to register through unrelated
builder calls. This example is the API to build, not an existing API:

```rust
App::new()
    .game(HeartsGame::new(saved_or_new))
    .root(HeartsDisplay::new())
```

`HeartsGame` owns the accepted `HeartsState` supplied at registration and
implements `Game`. The associated types make their roles explicit:

- `HeartsState` is the complete rules state: all hands, tricks, scores, and
  saved random-number-generator state.
- `Action` is a request from the display, such as playing a card. The engine
  infers it from the `Game::Action` associated type for `HeartsGame`.
- `HeartsView` is immutable data the south player may see. It excludes the
  contents of opponents' hands and is the value read by display components.
- `StateAnimation` describes how a newly published view should be presented,
  such as `CardPlayed` or `TrickCollected`. It is not the action type and is not
  an automatically computed state diff.
- `Prompt` is public choice data. `ControllerPrompt` separately carries any
  owned private observation needed by an application-controlled AI job.
- `HeartsDisplay` is the root component. It reads `HeartsView` and composes the
  table, menus, scores, prompts, and individual `CardView` components.

This abridged shape shows its central responsibilities:

```rust
trait Game: Sized + Send + 'static {
    type State: Send + 'static;
    type View: Send + 'static;
    type Action: Send + 'static;
    type StateAnimation: Send + 'static;

    fn logical_clone(state: &Self::State) -> Self::State;
    fn view(state: &Self::State) -> Self::View;
    fn validate_action(state: &Self::State, action: &Self::Action)
        -> Result<(), ActionRejection>;
    fn apply_action<M: ExecutionMode<Self>>(
        state: &mut Self::State, action: Self::Action,
        cx: &mut Executor<Self, M>,
    );
    fn final_state_animations(
        _state: &Self::State,
    ) -> Vec<Self::StateAnimation> { Vec::new() }
}
```

The canonical trait, including initial-state and choice types, is in [API
examples](interfaces.md#the-game-owns-its-associated-types). `validate_action`
has an accept-all default. The engine calls these methods; the application does
not register or manually sequence separate callbacks.

`HeartsView` is game-wide presentation data, while `CardView` is one component
that renders a card from that data. Game-wide data types use the `View` suffix
because they implement `Game::View`; component names describe what they render.

The full dispatch, worker, checkpoint, and acceptance order is specified in
[action execution](execution.md#from-dispatch-to-accepted-state).

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

Store writes schedule another render. Each render sees one game view and one
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
- `reactant-rules` owns execution, worker communication, generic choice
  machinery, and simulation. It has no component or Unity dependency.
- `reactant` owns application registration, world components, layouts,
  checkpoint presentation, effects, and convenient reexports.
- `reactant-testing` owns public display scenarios using Battlement's fake host.
- Reactant asset libraries and the CLI own Reactant asset declarations,
  generated paint, and related preparation.
- Battlement owns the protocol, C ABI, generic Unity hosts, generic asset
  loading, and low-level fakes.
- Game Rust code owns rules, state, action validation, choice specifications and
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
