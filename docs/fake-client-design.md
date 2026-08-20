# Masonry fake client design

Status: proposed implementation contract

## Summary

Masonry is a Unity rendering and input client for turn-based games whose
authoritative rules engine is written in Rust. In production, Unity connects to
the engine, receives a complete scene snapshot, sends player input back to the
engine, and executes the commands returned by the engine.

This document proposes `masonry-fake`, a Rust crate for testing those engines
without starting Unity. The fake owns a rules engine, applies its snapshots and
commands to an in-memory representation of Masonry-controlled objects, and lets
tests click objects or press keys. Tests can inspect the resulting world and the
commands that were executed.

The fake is intentionally not a Unity simulator. It favors speed and simple
failure behavior so games can run thousands of table-driven cases quickly.
Animations, waits, particles, and audio execute instantly. The original command
is retained for assertions, while state-changing commands immediately apply
their final state. Any invalid or unsupported behavior panics.

## Related information

- [Masonry technical design](technical-design.md) defines the production
  protocol, Unity client behavior, snapshots, command batches, and input model.
- [Masonry implementation plan](implementation-plan.md) records the existing
  production implementation and test conventions.
- [`masonry`](../crates/masonry/src/lib.rs) contains the canonical Rust protocol
  types used by engines and clients.
- [`masonry_native::Engine`](../crates/masonry-native/src/engine.rs) is the
  typed rules-engine interface driven directly by the fake.

## Masonry background

The rules engine owns authoritative game state. For a chess game, that includes
facts such as which piece occupies A7, whose turn it is, and whether promotion
is legal. Masonry does not make those decisions. It owns the presentation state
that the engine asked Unity to construct.

A client connection begins with a `Connect` value. The engine responds with a
`Snapshot` containing loaded scenes, prepared assets, GameObjects, an input
camera, and input settings. A GameObject has a stable UUID, placement in a
scene, an optional parent, a local transform, enabled pointer events, and a
kind-specific component description.

After connecting, the client sends built-in actions such as pointer clicks and
key transitions. The engine responds with batches of commands. A batch contains
ordered groups, and each group contains commands that production Unity launches
in parallel. Commands create and destroy objects, mutate component properties,
start animations and effects, or change input settings.

The production client receives these values through MessagePack over a native
plugin or HTTP transport. A Rust unit test does not need to test that transport
on every game-state case. The fake therefore calls the typed `Engine` trait
directly and works with the same Rust values before serialization.

## Terminology

A **prepared asset** is an Addressables entry named in the current snapshot or
asset-replacement command. Preparation makes the address available to later
commands; the fake checks it against a test-owned catalog instead of loading a
Unity asset.

An **admitted batch** is a batch whose ID the client has accepted for the
current session. Recording admitted IDs lets the client ignore a retransmitted
batch instead of executing it twice.

A tween **traversal** is one trip from its captured start value to its target.
`Restart` repeats another start-to-target trip. `PingPong` reverses direction
on each additional traversal.

A **built-in collider** is the selectable shape Masonry creates automatically
for a primitive or image object. A prefab collider is not described by the wire
protocol and must be declared in the fake asset catalog.

## Goals

The crate must make these tests straightforward:

- Construct a game engine from an in-memory state or loaded save.
- Connect that engine to a fake Masonry client.
- Click a particular object by UUID or send a physical key transition.
- Poll once when a test expects queued engine work.
- Inspect the current fake GameObject hierarchy and component state.
- Assert that a particular tween, particle, audio, or animator command ran.
- Reconnect the same engine and verify snapshot replacement behavior.
- Reuse immutable fake asset descriptions across many test cases.

The default path must be small and allocation-conscious. It must not start
threads, serialize messages, wait on wall time, or simulate frames. Protocol
values should be moved into the fake wherever ownership allows.

## Intentional fidelity boundary

The fake models the semantic result of valid Masonry commands. It does not model
the visual or temporal process Unity uses to reach that result.

For example, a position tween is observable in two ways:

- Its original `Command` appears in the executed-command journal.
- The target object immediately has the tween's final position.

The fake cannot answer where the object would appear halfway through the tween.
It also cannot prove that PrimeTween uses the intended easing curve, that a
particle prefab renders correctly, or that an audio clip finishes at the right
wall-clock time. Those behaviors remain covered by the production Unity tests.

This boundary is similar to Masonry's existing test-only instant-animation
mode, but it is broader. In `masonry-fake`, every time-based operation collapses
immediately, including waits, fades, effect lifetimes, and animator waits.

The fake does not implement:

- Unity rendering, physics, raycasting, meshes, shaders, or texture pixels.
- MessagePack, native plugin, or HTTP transport behavior.
- Frame-by-frame animation or intermediate property values.
- Concurrent operation scheduling or property-conflict waiting.
- Natural audio completion or temporary particle-effect lifetime.
- Game-specific custom commands or custom actions.
- Exact Unity exception messages or recoverable client failure reporting.
- Arbitrary child objects authored inside a prefab or content scene.

## Failure policy

`masonry-fake` is test infrastructure. It does not need a production-style
recovery system.

Every public operation panics if it cannot do exactly what the test requested.
Examples include an engine returning `EngineError`, a malformed initial
response, a missing object, a broken parent relationship, an unknown asset, or
an attempt to click a non-clickable object.

The crate will not introduce a fake-specific error enum, configurable failure
policies, `try_` variants, failure journals, or automatic submission of
`BatchFailed` and `OperationFailed` messages. A panic should include the
relevant session, batch, command, object, or asset identifier when one exists.
Tests that exercise production error recovery must use the real client or a
purpose-built protocol fixture.

Internal validation should protect the fake's invariants and produce useful
panics. It should not reproduce every production validator branch or exact
`CoreErrorCode` attribution.

## Crate boundary

The new package is named `masonry-fake` and imported as `masonry_fake`. It is a
workspace crate depending on `masonry` and `masonry-native`.

Its primary type is `client::FakeClient<E>`, where `E` implements:

- `masonry_native::Engine<Command = masonry::Command>`

The engine may use any `ActionPayload` and `ErrorCode` types because the fake
only constructs the built-in `ClientMessage::Action` variant. Custom commands
are excluded, so the engine's command type must be the core `Command` union.

The fake takes ownership of the engine. It does not expose `engine()` or
`engine_mut()` afterward. Tests should prepare authoritative state before
connecting and assert only state observable at the client boundary.

The principal public signatures are:

```rust
impl<E> FakeClient<E>
where
    E: Engine<Command = Command>,
{
    pub fn connect(engine: E, assets: Arc<FakeAssetCatalog>) -> Self;
    pub fn connect_with(
        engine: E,
        assets: Arc<FakeAssetCatalog>,
        connect: Connect,
    ) -> Self;
    pub fn reconnect(&mut self);
    pub fn frame(&mut self);
    pub fn click(&mut self, object_id: ObjectId);
    pub fn move_pointer(
        &mut self,
        object_id: Option<ObjectId>,
        input: PointerInput,
    );
    pub fn pointer_down(&mut self, object_id: ObjectId, input: PointerInput);
    pub fn pointer_up(&mut self, object_id: ObjectId, input: PointerInput);
    pub fn pointer_cancel(&mut self);
    pub fn key_down(&mut self, key: KeyCode);
    pub fn key_up(&mut self, key: KeyCode);
    pub fn world(&self) -> &FakeWorld;
    pub fn commands(&self) -> &[ExecutedCommand];
    pub fn clear_commands(&mut self);
    pub fn assert_command(
        &self,
        description: &str,
        predicate: impl Fn(&Command) -> bool,
    );
}
```

The assertion methods described later are additional inherent methods on
`FakeClient`. No public method in the initial crate is asynchronous or returns
`Result`.

## Connection API

The common constructor accepts an engine and a shared asset catalog:

```rust
use std::sync::Arc;

use masonry_fake::assets::FakeAssetCatalog;
use masonry_fake::client::FakeClient;

let assets = Arc::new(FakeAssetCatalog::new());
let engine = ChessEngine::from_position(position);
let client = FakeClient::connect(engine, assets);
```

`connect` constructs a deterministic default `Connect` value with:

- Platform `masonry-fake`.
- Unity version `masonry-fake`.
- A documented default screen size.
- No custom command types.
- No persistent-data or StreamingAssets paths.

The default screen is 1,920 by 1,080 physical pixels. An engine that branches
on connection metadata can use `connect_with` and supply an explicit `Connect`
value:

```rust
use masonry::{Connect, ScreenSize};

let connect = Connect::new(
    "test-platform",
    "test-unity",
    ScreenSize {
        width: 1920,
        height: 1080,
    },
);
let client = FakeClient::connect_with(engine, assets, connect);
```

Both constructors call `Engine::connect` synchronously. The response must be
for one nonzero session and its first message must be a snapshot. The fake
applies that snapshot and then applies later messages in response order. Any
engine or snapshot failure panics before a client is returned.

`reconnect()` calls `Engine::connect` again with the same connection metadata.
It discards the current world, input state, admitted batch IDs, and executed
command IDs before applying the new initial snapshot. The command journal is
retained and every journal entry carries its session ID, so a test can inspect
both sessions or call `clear_commands()` before reconnecting.

## Fake asset catalog

Unity Addressables resolve strings such as `chess/pieces/queen` into assets.
The protocol records the address and expected asset category, but it cannot
describe all components inside a prefab. Rust therefore needs a small test-owned
catalog for facts that Unity would discover while loading assets.

`assets::FakeAssetCatalog` is immutable while a client uses it and is normally
shared through `Arc`. A test suite can construct one catalog for the game and
reuse it for every case.

The catalog stores marker entries for:

- Content scenes.
- Materials.
- Textures.
- TextMesh Pro fonts.
- Audio clips.
- Particle-effect prefabs.

These entries prove only that an address exists with the expected category.
They do not contain load handles, file data, durations, dimensions, pool sizes,
or reference counts.

Prefab entries additionally describe root capabilities needed by core
commands:

- Optional renderer material-slot count.
- Optional root camera state.
- Optional root light state.
- Optional animator description.
- Whether the hierarchy contains any particle systems.
- Whether the hierarchy has a collider suitable for pointer input.

An animator description lists accepted states by layer and accepted parameter
names by type. It need not describe clip curves, transitions, playback length,
or child Animators.

Catalog construction is deliberately direct:

```rust
use masonry_fake::assets::{FakeAssetCatalog, FakePrefab};

let mut assets = FakeAssetCatalog::new();
assets.add_scene("chess/board");
assets.add_material("chess/materials/white-piece");
assets.add_particle_effect("chess/effects/promotion");
assets.add_prefab(
    "chess/pieces/queen",
    FakePrefab::new()
        .with_material_slots(1)
        .with_pointer_collider(),
);
```

Registration methods panic on duplicate addresses. Snapshot and command
application panic when an address is absent or registered under the wrong
category. The catalog does not attempt to model failed asynchronous loads.

Registration accepts the corresponding typed Masonry address through `impl
Into<AddressType>`. `add_prefab` additionally accepts a complete `FakePrefab`.
`FakePrefab::new` starts with no renderer, camera, light, animator, particles,
or collider. Its `with_` methods set those independent capabilities and return
the descriptor for chaining. The catalog exposes no mutation methods through
`&self`, so sharing it does not require a lock.

## In-memory world

`world::FakeWorld` contains only content controlled by Masonry. Authored objects
inside a loaded scene are not individually represented because the protocol
cannot target them.

The world tracks:

- Loaded scene instances and the primary scene ID.
- Current prepared asset addresses.
- Objects indexed by `ObjectId`.
- Each object's parent, children, and owning scene placement.
- `activeSelf` and computed active-in-hierarchy state.
- Local position, rotation, and scale.
- Kind-specific mutable component state.
- The selected input camera and enabled global keys.
- Whether pointer and keyboard input are enabled.

Object records are initialized from `GameObject` protocol values. Primitive,
image, text, camera, and light objects derive their capabilities from their
kind. Prefab instances combine snapshot state with their catalog description.

The fake computes a world transform by walking the parent chain when queried or
when a world-space command requires it. It uses small internal vector and
quaternion helpers with `f64`, matching the protocol representation. It does
not add a general math dependency or a cache-invalidation system before
measurements demonstrate that either is necessary.

Reparenting with `world_position_stays` preserves the current world transform
using ordinary transform composition and decomposition. Pathological cases
involving shear from rotated, nonuniformly scaled ancestors need not reproduce
Unity's floating-point decomposition exactly.

Destroying an object recursively removes its Masonry-controlled descendants.
Unloading a scene removes all Masonry-controlled objects placed in that scene.
Queries for removed objects return `None`; commands that target them panic.

The main query signatures are `object(ObjectId) -> Option<&FakeObject>`,
`children(ObjectId) -> impl Iterator<Item = &FakeObject>`, and
`world_transform(ObjectId) -> WorldTransform`. Component queries live on
`FakeObject` and return `Option<&State>` when the component is not guaranteed by
the object's protocol kind. Queries never mutate or lazily allocate state.

## Snapshot application

A snapshot is a complete replacement, not a patch. Application proceeds
synchronously:

1. Verify the response and snapshot session IDs agree and are nonzero.
2. Verify prepared assets exist in the shared catalog.
3. Verify scenes are unique and determine the primary scene.
4. Pre-size the object collection from the snapshot object count.
5. Move object descriptions into indexed fake objects.
6. Resolve scene placement and parent relationships.
7. Reject duplicate IDs, missing parents, cross-scene parents, and cycles.
8. Verify the input camera exists, is active, and has an enabled camera.
9. Install input-enabled state and global keys.

Validation panics at the first violation. There is no rollback because a
panicking unit test ends immediately.

A snapshot received later in the same session replaces prepared assets, scenes,
objects, logical audio records, and input state. It does not reset admitted
batch IDs, executed command IDs, the action-ID sequence, or the command journal;
those identities remain session-scoped. Reconnect starts a new session and
resets those session-scoped sets and sequences as described above.

## Batch execution

A response may contain snapshot and batch messages. Messages are processed in
their listed order. Applying a later snapshot replaces the world before later
batches execute.

The fake records batch IDs seen in the current session. A duplicate batch is
ignored. A new batch must use the current session and contain nonempty groups
and commands.

Groups execute in order. Commands within a group also execute in list order
because Rust calls are synchronous, but their recorded group index preserves
the production declaration that they launched together.

All commands complete before the next command is considered. Consequently:

- Blocking and nonblocking flags do not change fake execution.
- `BatchStart::AfterEarlierBlockingWork` is immediately satisfied after earlier
  instant commands finish.
- Property conflict policies have no observable effect.
- No operation registry or scheduler is needed.

The executor must exhaustively match `CommandBody`. Adding a new core command to
the protocol will therefore fail compilation until the fake defines its final
state effect.

Every command ID must be unique within the session. After successful execution,
the command is moved into the journal. A command that panics is not journaled.
There is no batch rollback: mutations and journal entries from earlier commands
remain if a later command panics. An executor should validate one command before
mutating its target when that is straightforward, but it does not need a
transaction mechanism for a test that is already unwinding.

## Executed-command journal

The journal is the only history retained by the fake. It replaces a richer
event or operation-lifecycle system.

Each `journal::ExecutedCommand` contains:

- Session ID.
- Batch ID.
- Zero-based group index.
- Zero-based command index within that group.
- The original `Command` value.

The executor borrows the command while changing the world and then moves it
into the journal. It does not clone payload strings, nested GameObjects, or
other protocol data.

`commands()` returns the journal as a slice. `clear_commands()` drops all
entries. Tests running many cases through one process should clear history once
earlier commands are no longer relevant.

The slice order is the complete execution order across responses. The indexes
describe placement within the containing batch and are always present. The
`assert_command` predicate receives `&Command`, not `&ExecutedCommand`; tests
that need session, batch, or group metadata inspect `commands()` directly.

There are no separate events for object creation, tween completion, particle
play, or audio stop. Their commands and resulting current state provide the two
useful observations without storing the same fact twice.

## Core command behavior

World and component commands apply their natural final state:

- Asset replacement swaps the prepared address set after checking catalog
  membership. It does not reproduce Unity asset leases or reject removal of an
  address still named by current logical state.
- Scene commands load, unload, or select the primary scene.
- Object commands create, destroy, activate, and reparent objects.
- Transform setters apply local or world position and rotation, or local scale.
- Renderer commands update one or all declared material slots.
- Camera and light commands update the corresponding logical component.
- Image and text commands update their complete logical properties.
- Animator parameter commands update stored parameters and speed.
- Input commands update input availability, camera, pointer-event selections,
  and global keys.

The fake need only validate conditions necessary to produce coherent state. It
should not copy all bounds, size limits, and wording from the Unity validators.
Obvious property errors such as non-finite transforms, a missing component, or
an invalid material slot panic.

## Instant tween behavior

All tween commands use the same instant rule as the production test adapter:

- Validate that the target component exists.
- Validate the basic repeat shape, including the prohibition on repeating a
  zero-duration tween.
- Determine the final interpolation factor.
- Apply the resulting property once.
- Journal the original command.

A one-traversal tween ends at the target. A finite restart tween also ends at
the target. A finite ping-pong tween ends at the start after an even total
number of traversals and at the target after an odd total number. A forever
tween applies the target once and is then considered complete.

Delay, duration, and easing remain present in the journal for assertions but do
not affect execution time. The fake does not implement easing functions.

For example:

```rust
client.assert_command(
    "pawn moves to A8 with a tween",
    |command| matches!(
        &command.body,
        CommandBody::TransformTweenWorldPosition(value)
            if value.payload.object_id == pawn_id
                && value.payload.position == square_a8
    ),
);
client.assert_world_position(pawn_id, square_a8, 0.000_001);
```

## Animator, particle, and audio behavior

Animator play and cross-fade commands immediately make the requested state the
logical current state. Animator waits and cross-fade duration are retained only
in the command journal. Parameter and speed changes persist on the fake object.
The catalog is used to reject unknown layers, states, or parameters.

Particle play and stop commands update a boolean logical state on a target that
has catalog-declared particle systems. Restart and clear flags remain available
in the command journal. Particle spawn verifies the prepared address and target
location, then journals the command without creating a temporary fake object.

Audio play creates a small logical record keyed by its command ID. That record
retains address, requested volume, pitch, and loop flag. Audio volume commands
update that record. Fade durations and natural playback completion are not
simulated.

Stopping audio removes its logical record after validating that the play
command is still active. A later volume or second stop command for that ID
panics. Every snapshot replacement and reconnect clears all logical audio
records. Tests assert completed stops through the command journal rather than a
retained stopped instance.

`TimeWait` validates its basic shape and otherwise does nothing. It cannot hold
up a later group.

`OperationCancel` is a no-op when its target command ID has already executed.
It panics when the command ID is unknown. There are never live operations to
cancel.

## Input model

The fake provides semantic UUID-targeted input rather than raycasting screen
coordinates through camera and collider geometry.

`click(object_id)` requires:

- An active session with input enabled.
- An existing object active in the hierarchy.
- `PointerEvent::Click` enabled on that object.
- A built-in collider or a prefab collider declared in the catalog.

The helper uses pointer ID zero, the center of the configured screen, and the
object's current world position as its hit. It performs the logical hover,
press, and release sequence. It submits only action kinds selected in the
object's `pointer_events` list. Each synchronous engine response is fully
applied before the next selected action is submitted.

The exact sequence is:

1. If another object is hovered and still exists, submit its selected
   `PointerExit` action.
2. Make the requested object current and submit its selected `PointerEnter`.
3. Record an internal press on that object and submit its selected
   `PointerDown`.
4. Recheck the target after every synchronous response. If it was destroyed,
   disabled, made inactive, or lost click selection, clear pointer state and
   panic because the requested semantic click cannot complete.
5. Submit the selected `PointerUp` and recheck the target again.
6. Submit `PointerClick`, which is always selected because `click` requires it.
7. Clear the internal press while leaving the target hovered.

If a synchronous response changes only the target's pointer-event selection,
each later step uses the new selection. A response that disables click before
the final step prevents `PointerClick` and causes `click` to panic, because the
requested semantic gesture could not complete.

The lower-level methods share the same hovered and pressed pointer state.
`move_pointer` accepts an optional target, emits selected exit and enter actions
for the transition, and makes the new target current. `pointer_down` requires
the target to be current, records it as pressed, and emits `PointerDown` only
when selected. `pointer_up` requires the target to be current, emits selected
`PointerUp`, emits selected `PointerClick` when the press target still matches,
and clears the press. `pointer_cancel` clears the press without emitting an
action.

If a synchronous response invalidates the target during a lower-level method,
the fake clears hover and press state and returns after the actions already
sent. This is a normal device-state transition. Only the semantic `click`
wrapper promises a complete click and therefore panics on mid-gesture
invalidation.

`PointerInput` has `pointer_id: i32`, `screen_position: ScreenPosition`,
`world_hit: Vector3`, and `button: PointerButton`. A non-null lower-level target
must exist, be active in the hierarchy, and have a collider. Input must be
enabled before every lower-level method. Pointer-event selection decides which
actions are emitted; absence of an optional enter, exit, down, or up selection
does not make an otherwise valid physical transition fail. A violated target or
state precondition panics.

Key down and key up require input to be enabled and the physical `KeyCode` to be
present in `global_keys`. Repeated key-down calls without an intervening key-up
do not emit repeated actions.

Every submitted action receives a deterministic, nonzero UUID created with
`Uuid::from_u128`, beginning at one and increasing by one. The sequence restarts
on connection. This keeps histories and `caused_by_action_id` values
reproducible without adding a configurable ID-provider abstraction.

Input helpers do not call `Engine::poll`. `frame()` performs exactly one poll
and applies its response if one is returned. This lets tests distinguish work
returned synchronously by `submit` from work queued for polling.

## Queries and assertions

The fake exposes ordinary read-only queries so games are not forced into a
crate-specific assertion language:

- Lookup an object by ID.
- Enumerate an object's direct children.
- Read local and computed world transforms.
- Read active-in-hierarchy state.
- Read logical component, particle, and audio state.
- Inspect executed commands in launch order.

A small set of assertion helpers provides better panic messages for common
tests:

- `assert_object` returns the requested object or panics.
- `assert_object_absent` verifies that an ID is no longer present.
- `assert_object_kind` compares a current kind with an expected value.
- Transform assertions compare values with an explicit caller-supplied
  tolerance.
- `assert_command(description, predicate)` finds a matching journal command and
  prints the journal when none matches.
- `assert_no_commands` verifies that no commands ran after the caller last
  cleared the journal.

Do not add assertion macros, golden snapshot serialization, matcher-builder
types, or a second expectation-recording system.

## Example promotion test

The intended complete test reads like this:

```rust
let assets = Arc::new(chess_fake_assets());
let engine = ChessEngine::from_save(promotion_save());
let mut client = FakeClient::connect(engine, assets);

client.click(pawn_id);
client.click(square_a8_id);

client.assert_command("pawn movement tween", |command| {
    matches!(
        &command.body,
        CommandBody::TransformTweenWorldPosition(_)
    )
});
client.assert_command("promotion particle", |command| {
    matches!(&command.body, CommandBody::ParticleSpawn(_))
});
client.assert_object_absent(pawn_id);
client.assert_object_kind(queen_id, &expected_queen_kind);
```

If the engine queues promotion work instead of returning it from the click,
the test calls `client.frame()` before making the assertions.

The game can create `ChessEngine` from a deserialized save, from a compact
in-memory position builder, or by mutating a reusable game-state template.
Those choices remain outside `masonry-fake`.

## Performance considerations

The fake should be fast primarily because it does less work, not because it has
complex optimized internals.

Required implementation practices are:

- Share an immutable catalog with `Arc`.
- Move snapshot objects and executed commands rather than clone them.
- Reserve object and journal collections from known input counts.
- Use direct synchronous calls without serialization or locks.
- Store only current world state and the compact command journal.
- Avoid allocating lifecycle records for instant operations.
- Let callers clear command history between table-driven cases.

The first implementation should use ordinary standard-library maps and compute
world transforms on demand. It should not introduce custom hashers, generational
arenas, copy-on-write worlds, transform caches, or compiled snapshot templates
without measurements showing that straightforward code is too slow.

The crate will not include a benchmark or a fixed wall-clock CI threshold.
Whole-suite performance also depends on each game's save format, engine setup,
snapshot size, and command generation. If a real game suite is slow, profiling
must separate those costs from fake-client execution before the fake is made
more complicated.

Games with thousands of meaningful states should consider table-driven tests
and in-memory state builders instead of creating thousands of separate Rust
test functions or repeatedly parsing large save files. This is guidance rather
than a responsibility of the fake crate.

## Automated validation

Tests should use the public fake-client API and small fixture engines. They
should avoid testing private reducers or helpers directly unless a numerical
transform helper needs focused coverage.

Required black-box scenarios are:

- Connect and apply a representative snapshot.
- Promote a pawn through two clicks and assert its tween, particle command,
  destruction, queen creation, and final state.
- Reconnect and prove that the old world is replaced.
- Create, reparent, activate, and recursively destroy object hierarchies.
- Load, unload, and select scenes.
- Exercise pointer selection, keyboard filtering, deterministic action IDs,
  synchronous submit responses, and explicit polling.
- Exercise every core command family through combined scenarios.
- Verify once, restart, odd ping-pong, even ping-pong, and forever tween final
  factors.
- Target logical audio records with volume and stop commands.
- Play and stop catalog-declared particle systems.
- Suppress duplicate batches and reject duplicate command IDs.

Panic tests should be sparse. Cover only central invariants such as a malformed
initial response, a missing object, an unknown asset, and a non-clickable click.
Do not reproduce the production client's exhaustive invalid-input matrix.

The complete repository verification remains `./scripts/ci.py`, including Rust
formatting, Clippy, tests, and existing Unity checks. This non-rendering change
does not require screenshot or video evidence.

## Proposed milestone breakdown

### Milestone 1: connection and world

Add the workspace crate, shared asset catalog, engine connection, snapshot
application, object hierarchy, transforms, read-only queries, and reconnect.
Validate the result with one representative snapshot fixture.

### Milestone 2: commands and input

Add synchronous batch execution, exhaustive core command handling, instant
temporal rules, semantic pointer and keyboard helpers, explicit polling, and
the move-only command journal. Validate the pawn-promotion flow.

### Milestone 3: test ergonomics and completion

Add the focused assertion helpers, remaining command-family scenarios, crate
documentation, formatting and lint cleanup, and full repository CI. Submit the
single completed change through the repository's Tollgate workflow.

## Manual QA

Read this document without relying on prior project knowledge and confirm that
the rules engine, production Unity client, snapshot, batch, action, and fake
client roles are understandable.

Walk through the catalog and pawn-promotion examples and confirm every type and
method is defined by the proposed public API. Verify that the examples never
depend on elapsed time or intermediate animation values.

Confirm that every failure path described by the fake panics and that the
design contains no configurable recovery mode, fake error hierarchy, or client
failure-reporting subsystem.

Confirm that a tween is asserted through its original command and final state,
not through simulated frames. Confirm that particle and audio rendering or
lifetime behavior is explicitly left to Unity tests.

Finally, run the Markdown link check and repository CI before submitting the
implementation for review.
