# Battlement fake client design

Status: proposed implementation contract

## How to read this contract

This document is normative for `battlement-fake`. **Must** and **must not** state
requirements. **Should** states the expected implementation unless repository
evidence makes it impractical. Examples illustrate requirements but do not
replace them.

The canonical `battlement` protocol types and their `Validate` implementations
remain authoritative for protocol shape and cross-field validity. The fake
must call `Validate::validate` for every snapshot and command before applying
fake-specific catalog, reference, or state checks. It must not copy that
validation logic into the new crate. This document is authoritative for the
fake-only choices that the protocol does not define: instant completion,
in-memory state, input helpers, journaling, panic behavior, and explicit
polling.

When implementation reveals a genuine conflict between this document and the
current protocol API, update this document in the same change. Do not silently
invent a compatibility layer or choose behavior from the production Unity
implementation that contradicts this contract.

## Summary

Battlement is a Unity rendering and input client for turn-based games whose
authoritative rules engine is written in Rust. In production, Unity connects to
the engine, receives a complete scene snapshot, sends player input back to the
engine, and executes the commands returned by the engine.

This document proposes `battlement-fake`, a Rust crate for testing those engines
without starting Unity. The fake owns a rules engine, applies its snapshots and
commands to an in-memory representation of Battlement-controlled objects, and lets
tests click objects or press keys. Tests can inspect the resulting world and the
commands that were executed.

The fake is intentionally not a Unity simulator. It favors speed and simple
failure behavior so games can run thousands of table-driven cases quickly.
Animations, waits, particles, and audio execute instantly. The original command
is retained for assertions, while state-changing commands immediately apply
their final state. Any invalid or unsupported behavior panics.

## Related information

- [Battlement technical design](technical-design.md) defines the production
  protocol, Unity client behavior, snapshots, command batches, and input model.
- [Battlement UI technical design](battlement-ui-technical-design.md) defines
  the proposed `battlement-ui-fake` state and event model that composes with
  this fake client.
- [Battlement implementation plan](implementation-plan.md) records the existing
  production implementation and test conventions.
- [`battlement`](../crates/battlement/src/lib.rs) contains the canonical Rust protocol
  types used by engines and clients.
- [`battlement_native::Engine`](../crates/battlement-native/src/engine.rs) is the
  typed rules-engine interface driven directly by the fake.

## Battlement background

The rules engine owns authoritative game state. For a chess game, that includes
facts such as which piece occupies A7, whose turn it is, and whether promotion
is legal. Battlement does not make those decisions. It owns the presentation state
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

The production client receives these values through JSON over a native
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

A **built-in collider** is the selectable shape Battlement creates automatically
for a primitive or image object. A prefab collider is not described by the wire
protocol and must be declared in the fake asset catalog.

## Goals

The crate must make these tests straightforward:

- Construct a game engine from an in-memory state or loaded save.
- Connect that engine to a fake Battlement client.
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

The fake models the semantic result of valid Battlement commands. It does not model
the visual or temporal process Unity uses to reach that result.

A position tween is observable as its original journaled `Command` and the
target object's immediate final position. The fake cannot report an
intermediate position or verify easing, rendering, or wall-clock completion;
production Unity tests own those behaviors. Every time-based operation in the
fake collapses immediately, including waits, fades, effect lifetimes, and
animator waits.

`time::ManualClock` is separate from command execution. An engine that accepts
an injected time source may read a clone of this clock so a test can advance
engine-authored deadlines deterministically before calling `FakeClient::poll`.
This does not add frame simulation or change the immediate handling of
time-based Battlement commands.

The fake does not implement:

- Unity rendering, physics, raycasting, meshes, shaders, or texture pixels.
- JSON, native plugin, or HTTP transport behavior.
- Frame-by-frame animation or intermediate property values.
- Concurrent operation scheduling or property-conflict waiting.
- Natural audio completion or temporary particle-effect lifetime.
- Game-specific custom commands or custom actions.
- Exact Unity exception messages or recoverable client failure reporting.
- Arbitrary child objects authored inside a prefab or content scene.

## Failure policy

Every constructor, mutating operation, engine-driving operation, and assertion
helper panics if it cannot do exactly what the test requested. Examples include
an engine returning `EngineError`, a malformed initial response, a missing
object, a broken parent relationship, an unknown asset, or an attempt to click
a non-clickable object. Read-only queries follow their documented `Option` or
iterator behavior instead of panicking merely because a component or object is
absent; `world_transform` is the exception and panics for an unknown object.

The crate will not introduce a fake-specific error enum, configurable failure
policies, `try_` variants, failure journals, or automatic submission of
`BatchFailed` and `OperationFailed` messages. A panic should include the
relevant session, batch, command, object, or asset identifier when one exists.
Tests that exercise production error recovery must use the real client or a
purpose-built protocol fixture.

Internal validation protects fake invariants and produces useful panics without
reproducing production validator branches or exact `CoreErrorCode` attribution.

## Crate boundary

The new package is named `battlement-fake` and imported as `battlement_fake`. It is a
workspace crate depending on `battlement` and `battlement-native`.

Its primary type is `client::FakeClient<E>`, where `E` implements:

- `battlement_native::Engine<Command = battlement::Command>`

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
    pub fn connect(
        engine: E,
        assets: impl Into<Arc<FakeAssetCatalog>>,
    ) -> Self;
    pub fn connect_clocked(
        make_engine: impl FnOnce(ManualClock) -> E,
        assets: impl Into<Arc<FakeAssetCatalog>>,
    ) -> (Self, ManualClock);
    pub fn connect_with(
        engine: E,
        assets: impl Into<Arc<FakeAssetCatalog>>,
        connect: Connect,
    ) -> Self;
    pub fn reconnect(&mut self);
    pub fn poll(&mut self);
    pub fn click(&mut self, object_id: ObjectId);
    pub fn click_at(&mut self, object_id: ObjectId, world_hit: Vector3);
    pub fn move_pointer(
        &mut self,
        object_id: Option<ObjectId>,
        input: PointerInput,
    );
    pub fn pointer_down(&mut self, object_id: ObjectId, input: PointerInput);
    pub fn pointer_up(&mut self, object_id: ObjectId, input: PointerInput);
    pub fn pointer_cancel(&mut self);
    pub fn drag_start(&mut self, object_id: ObjectId, input: PointerInput);
    pub fn drag_end(
        &mut self,
        object_id: ObjectId,
        input: PointerInput,
        world_position: Vector3,
    );
    pub fn key_down(&mut self, input: KeyInput);
    pub fn key_up(&mut self, input: KeyInput);
    pub fn world(&self) -> &FakeWorld;
    pub fn commands(&self) -> &[ExecutedCommand];
    pub fn clear_commands(&mut self);
    pub fn checkpoint(&self) -> CommandCheckpoint;
    pub fn assert_one_object_created_since(
        &self,
        checkpoint: CommandCheckpoint,
    ) -> ObjectId;
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

The optional manual clock has this independent API:

```rust
impl ManualClock {
    pub fn new(now: Instant) -> Self;
    pub fn now(&self) -> Instant;
    pub fn advance(&self, duration: Duration);
}
```

`connect_clocked` creates that standalone clock, passes a clone to the engine
factory, and returns the client and original clock together. Advancing the
clock does not poll the client or execute Battlement command timing.

The crate should use the following ownership boundaries:

- `lib.rs` declares the public `assets`, `client`, `journal`, `time`, and `world`
  modules. It does not re-export their contents.
- `assets.rs` owns `FakeAssetCatalog`, `FakePrefab`, and animator descriptors.
- `client.rs` owns the engine, session lifecycle, response processing, input
  state, polling, journal, and assertion helpers.
- `world.rs` owns `FakeWorld`, `FakeObject`, component state, hierarchy
  mutation, and public queries.
- `journal.rs` defines `ExecutedCommand`.
- `time.rs` owns the standalone `ManualClock` used by engines with injected
  time sources.
- Private command-execution and transform-math modules may be added when they
  keep the public modules comfortably below the repository's file-size limit.

`FakeClient` must retain exactly the state needed by this contract: the owned
engine; shared catalog; original `Connect`; current session ID; `FakeWorld`;
session-scoped admitted batch and executed command ID sets; the next action ID;
the hovered object and its last `PointerInput`; the pressed object, pointer ID,
and button; held keys; and the command journal. Do not add
transport state, clocks, task executors, background workers, or operation
lifecycle registries.

## Connection API

The common constructor accepts an engine and a shared asset catalog:

```rust
use std::sync::Arc;

use battlement_fake::assets::FakeAssetCatalog;
use battlement_fake::client::FakeClient;

let assets = Arc::new(FakeAssetCatalog::new());
let engine = ChessEngine::from_position(position);
let client = FakeClient::connect(engine, assets);
```

`connect` constructs a deterministic `Connect` with platform and Unity version
`battlement-fake`, a 1,920 by 1,080 physical-pixel screen, no custom command types,
and no persistent-data or StreamingAssets paths. An engine that branches on
connection metadata uses `connect_with` with an explicit value:

```rust
use battlement::{Connect, ScreenSize};

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

### Engine-call and response contract

All engine calls and response application occur before the public method
returns. There is one internal response-application path used by constructors,
`reconnect`, action submission, and `poll`; these callers must not implement
different message-ordering rules.

The path follows these steps:

1. Propagate any `EngineError` by panicking with the operation name and current
   session ID when one exists.
2. Require a nonzero response session ID. On initial connection or reconnect,
   adopt that ID; reconnect additionally requires it to differ from the prior
   session. Otherwise require it to equal the current session.
3. Process `Response::messages` once, in vector order. Do not sort, group, or
   defer messages.
4. Require every snapshot and batch to carry the response's session ID.
5. Apply each message completely before examining the next message.

The initial `Engine::connect` response must contain at least one message, and
message zero must be `ResponseMessage::Snapshot`. Later messages in that same
response may be snapshots or batches and follow the ordinary ordering rules.
Submit responses may contain no messages. A nonempty poll response follows the
same ordinary rules; `Engine::poll` returning `None` is a successful no-op.

No response is transactional. If a later message or command panics, mutations
and journal entries produced by earlier successfully applied work remain. This
only matters to a test that catches unwinding; the fake must not add rollback
machinery.

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

The complete catalog-builder surface is:

```rust
impl FakeAssetCatalog {
    pub fn new() -> Self;
    pub fn add_scene(&mut self, address: impl Into<SceneAddress>);
    pub fn add_prefab(&mut self, address: impl Into<PrefabAddress>, value: FakePrefab);
    pub fn add_particle_effect(&mut self, address: impl Into<ParticleEffectAddress>);
    pub fn add_material(&mut self, address: impl Into<MaterialAddress>);
    pub fn add_texture(&mut self, address: impl Into<TextureAddress>);
    pub fn add_textures<T>(&mut self, addresses: impl IntoIterator<Item = T>)
    where
        T: Into<TextureAddress>;
    pub fn add_audio_clip(&mut self, address: impl Into<AudioClipAddress>);
    pub fn add_font(&mut self, address: impl Into<FontAddress>);
}

impl FakePrefab {
    pub fn new() -> Self;
    pub fn with_material_slots(self, count: usize) -> Self;
    pub fn with_camera(self, initial: CameraState) -> Self;
    pub fn with_light(self, initial: LightState) -> Self;
    pub fn with_animator(self, animator: FakeAnimator) -> Self;
    pub fn with_particle_systems(self) -> Self;
    pub fn with_pointer_collider(self) -> Self;
}

impl FakeAnimator {
    pub fn new() -> Self;
    pub fn with_state(self, layer: u32, state: impl Into<String>) -> Self;
    pub fn with_bool_parameter(self, name: impl Into<String>) -> Self;
    pub fn with_int_parameter(self, name: impl Into<String>) -> Self;
    pub fn with_float_parameter(self, name: impl Into<String>) -> Self;
    pub fn with_trigger_parameter(self, name: impl Into<String>) -> Self;
}
```

All builders panic on duplicate declarations. `with_material_slots` requires a
positive count. The animator may declare multiple states on one layer.

Catalog construction is deliberately direct:

```rust
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};

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

Registration accepts the corresponding typed Battlement address through `impl
Into<AddressType>`. `add_prefab` additionally accepts a complete `FakePrefab`.
`FakePrefab::new` starts with no renderer, camera, light, animator, particles,
or collider. Its `with_` methods set those independent capabilities and return
the descriptor for chaining. The catalog exposes no mutation methods through
`&self`, so sharing it does not require a lock.

When a prefab object is created, catalog camera and light values become its
initial logical component state. Snapshot material assignments require a
catalog renderer and valid slots. A supplied snapshot `AnimatorState` requires
a catalog animator and must name only declared layers, states, and typed
parameters; an omitted animator state remains absent even when the catalog can
support one. A selected Battlement input camera must be the protocol's `Camera`
object kind, enabled, and active in the hierarchy; the authoritative
`Snapshot::validate` implementation rejects prefab objects in this role.
Snapshots may instead select Unity's scene-authored main camera, which the fake
world records without creating a fake object. A
prefab camera declared in the catalog is still available to camera commands
and `InputSetCamera`, which use fake component capabilities after protocol
validation. Any mismatch panics during object construction before the object
enters the world.

## In-memory world

`world::FakeWorld` contains only content controlled by Battlement. Authored objects
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

The fake computes world transforms by walking the parent chain. For parent `P`
and local `L`, world scale is componentwise `P.scale * L.scale`; world rotation
is normalized `P.rotation * L.rotation`; and world position is `P.position +
rotate(P.rotation, P.scale * L.position)`. A root object's world transform is
its local transform. World-space setters and reparenting with
`world_position_stays` use the inverse equations: subtract parent position,
rotate by the inverse parent rotation, divide componentwise by parent scale,
and multiply rotation by the inverse parent rotation. They panic when an
inverse needs a zero parent-scale component. Small private `f64` vector and
quaternion helpers are sufficient; do not add a transform cache or general
math dependency. Shear produced by rotated nonuniform scale need not match
Unity's decomposition exactly.

Destroying an object recursively removes its Battlement-controlled descendants.
Unloading a scene removes all Battlement-controlled objects placed in that scene.
Queries for removed objects return `None`; commands that target them panic.

`WorldTransform` is a public copyable value with public `position: Vector3`,
`rotation: Quaternion`, and `scale: Vector3` fields. The main world queries are:

```rust
pub fn object(&self, id: ObjectId) -> Option<&FakeObject>;
pub fn object_count(&self) -> usize;
pub fn images(&self) -> impl Iterator<Item = (&FakeObject, &ImageState)>;
pub fn texts(&self) -> impl Iterator<Item = (&FakeObject, &TextState)>;
pub fn children(
    &self,
    id: ObjectId,
) -> Option<impl Iterator<Item = &FakeObject>>;
pub fn world_transform(&self, id: ObjectId) -> WorldTransform;
pub fn scene(&self, id: SceneId) -> Option<&Scene>;
pub fn primary_scene_id(&self) -> SceneId;
pub fn audio(&self, play_command_id: CommandId) -> Option<&FakeAudio>;
```

`FakeObject` has these read-only methods: `id() -> ObjectId`, `parent_id() ->
Option<ObjectId>`, `scene_id() -> Option<SceneId>` (`None` means persistent
placement), `active_self() -> bool`, `active_in_hierarchy() -> bool`,
`local_transform() -> LocalTransform`, `kind() -> &GameObjectKind`, `image() ->
Option<&ImageState>`, `text() -> Option<&TextState>`,
`pointer_events() -> &[PointerEvent]`, `renderer_slot_count() -> Option<usize>`,
`material(u32) -> Option<&MaterialAddress>`, `camera() ->
Option<&CameraState>`, `light() -> Option<&LightState>`, `animator() ->
Option<&AnimatorState>`, and `particles_playing() -> Option<bool>`. A missing
component or material slot returns `None`.

`FakeAudio` has `address() -> &AudioClipAddress`, `volume() -> f64`, `pitch() ->
f64`, and `is_looping() -> bool`.
`FakeWorld` additionally has `scenes() -> impl Iterator<Item = &Scene>`,
`prepared_assets() -> &[PreparedAsset]`, `is_prepared(&PreparedAsset) -> bool`,
`input_enabled() -> bool`,
`input_camera_id() -> Option<ObjectId>`, `uses_main_camera() -> bool`, and
`global_keys() -> &[PhysicalKey]`.
Queries never mutate or lazily allocate state. An unknown object produces
`None` from `object` and `children`; `world_transform` panics with the object ID.
`FakeWorld` implements `Clone` and `PartialEq` so tests can compare complete
observable state before and after an interaction.

## Snapshot application

A snapshot is a complete replacement, not a patch. Application proceeds
synchronously:

1. Verify the response and snapshot session IDs agree and are nonzero.
2. Call `snapshot.validate()` and panic with its diagnostic on failure.
3. Verify every prepared address exists in the shared catalog under the exact
   category named by the protocol value.
4. Determine the primary scene using the already-validated snapshot rule.
5. Pre-size the object collection from the snapshot object count.
6. Move object descriptions into indexed fake objects and attach catalog
   capabilities to prefab instances.
7. Resolve scene placement, parent links, and child lists.
8. Install the selected input camera, input-enabled state, and global keys.

Validation panics at the first violation. There is no rollback because a
panicking unit test ends immediately.

A snapshot received later in the same session replaces prepared assets, scenes,
objects, logical audio records, and input state. It does not reset admitted
batch IDs, executed command IDs, the action-ID sequence, or the command journal;
those identities remain session-scoped. Reconnect starts a new session and
resets those session-scoped sets and sequences as described above.

Snapshot replacement also clears hovered-object, pressed-pointer, and held-key
state because those are client-device state tied to the replaced world and
input configuration. The new snapshot does not synthesize exit, cancel, or
key-up actions.

## Batch execution

A response may contain snapshot and batch messages. Messages are processed in
their listed order. Applying a later snapshot replaces the world before later
batches execute.

For each batch, first require the current session ID. Then check its batch ID.
If that ID is already admitted, ignore the entire duplicate without validating
or executing its groups. Otherwise require at least one group and at least one
command in every group, then admit the batch before executing its first
command. Duplicate suppression depends only on the batch ID; a retransmission
with different content is still ignored.

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

Every command ID must be unique within the session. For each command, call
`command.validate()`, check that its ID has not executed, check all
fake-specific preconditions, mutate the world, mark its ID executed, and move
it into the journal, in that order. A command that panics is neither marked
executed nor journaled.
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

`checkpoint()` returns an opaque journal position. The focused
`assert_one_object_created_since` helper scans subsequent `ObjectCreate`
commands and returns the sole created ID, or panics with all matching IDs. A
checkpoint is invalid after `clear_commands` shortens the journal past it.

The slice order is the complete execution order across responses. The indexes
describe placement within the containing batch and are always present. The
`assert_command` predicate receives `&Command`, not `&ExecutedCommand`; tests
that need session, batch, or group metadata inspect `commands()` directly.

There are no separate events for object creation, tween completion, particle
play, or audio stop. Their commands and resulting current state provide the two
useful observations without storing the same fact twice.

## Core command behavior

Every `CommandBody` variant must follow the behavior below. “Validate” always
includes `Command::validate` plus the referenced object's component, prepared
asset category, material-slot bounds, animator declaration, scene, or prior
command ID needed by that variant.

| Command family | Required observable result |
| --- | --- |
| Assets | `AssetsReplaceSet` atomically replaces the prepared-address set after all new entries pass catalog checks. Existing logical state may continue to name a removed address. |
| Scenes | Load inserts one new catalog-declared scene instance; unload rejects the primary scene and removes that scene's objects recursively; set-primary selects an already loaded scene. |
| Objects | Create inserts one fully validated object and its parent/scene links; destroy removes the target subtree; set-active changes `activeSelf`; reparent updates both parents' child lists and applies the requested transform-preservation rule. |
| Transforms | Local setters assign the supplied value. World setters compute the required local value from the current parent world transform. Tween variants use the instant rule below. |
| Renderer | Set-material replaces the named slot, or every declared slot when the payload selects all slots, using one prepared material. |
| Camera | Enable changes component state; disabling the selected input camera clears that selection. Projection setters replace the projection mode and its mode-specific value. Field-of-view and orthographic-size tweens use the instant rule. Clipping and clear commands replace their complete logical properties. |
| Light | Enable, type, range, spot angles, and shadows replace their named logical properties. Color and intensity setters assign immediately; their tweens use the instant rule. |
| Image | Texture, size, fit, and face-camera replace their named properties. Tint and opacity setters assign immediately; their tweens use the instant rule. |
| Text | Content, font, alignment, wrapping, rich-text, and face-camera replace their named properties. Size and color setters assign immediately; their tweens use the instant rule. |
| Animator | Play, cross-fade, parameters, trigger, and speed follow the logical rules in the animator section below. |
| Particles | Play, stop, and spawn follow the logical rules in the particle section below. |
| Audio | Play, stop, set-volume, and tween-volume follow the logical rules in the audio section below. |
| Time and cancellation | Wait and cancel follow the no-live-operation rules below. |
| Input | Set-enabled replaces the input gate; set-camera selects a valid active enabled camera; pointer-events and global-keys replace their complete deduplicated sets. Disabling input clears hover, press, and held-key state without submitting synthetic actions. |

This table is not permission to use a wildcard match arm. The executor must
name every current `CommandBody` variant so a protocol addition fails to
compile until its fake behavior and black-box test are added.

`Command::validate` owns protocol numeric and cross-field rules, including
non-finite values, rotations, clipping, repeat shape, and blocking constraints.
Fake-specific checks own only current-world and catalog facts: referenced IDs
exist, prepared assets are present with the right category, required components
and declared animator members exist, hierarchy mutations stay valid, and
material slots are in range. Do not add Unity-only size limits or duplicate
validation branches merely to improve panic wording.

## Instant tween behavior

All tween commands use the same instant rule as the production test adapter:

- Validate that the target component exists.
- Validate the basic repeat shape, including the prohibition on repeating a
  zero-duration tween.
- Determine the final interpolation factor.
- Apply the resulting property once.
- Journal the original command.

A one-traversal tween ends at the target. A finite restart tween also ends at
the target. For `TweenRepeat::Count`, total traversals equal
`1 + additional_traversals`. A finite ping-pong tween ends at the start after
an even total and at the target after an odd total. A forever tween applies the
target once and is then considered complete, for both repeat modes.

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
`AnimatorSetTrigger` validates the trigger name and journals the command but
does not retain trigger state, because triggers are transient and absent from
`AnimatorState` snapshots.

Particle play and stop commands update a boolean logical state on a target that
has catalog-declared particle systems. Restart and clear flags remain available
in the command journal. Particle spawn verifies the prepared address and target
location, then journals the command without creating a temporary fake object.

Audio play creates a small logical record keyed by its command ID. That record
retains address, requested volume, pitch, and loop flag. Audio volume commands
update that record. `AudioTweenVolume` captures the current volume as its start
and applies the same final-factor rule as every other tween; an even finite
ping-pong therefore restores the captured volume. Fade durations and natural
playback completion are not simulated.

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

The helper uses pointer ID zero, the center of the configured screen, the
object's current world position as its hit, the left button, mouse pointer
type, and no modifiers. It performs the logical hover, press, and release
sequence. It submits only action kinds selected in the object's
`pointer_events` list. Each synchronous engine response is fully applied before
the next selected action is submitted.

`click_at(object_id, world_hit)` performs the same semantic gesture with a
caller-supplied world-space hit. It retains the default pointer ID, screen
position, left button, mouse pointer type, and empty modifiers. This supports
boards, maps, and other objects whose behavior depends on the hit location
without requiring tests to recreate the pointer lifecycle.

The exact sequence is:

1. If another object is hovered and still exists, submit its selected
   `PointerExit` action using its last recorded pointer input.
2. If the requested object was not already hovered, make it current and submit
   its selected `PointerEnter`. If it was already hovered, update its recorded
   pointer input without emitting another enter.
3. Record an internal press on that object and submit its selected
   `PointerDown`.
4. After the exit, enter, and down submissions, recheck the target before the
   next gesture step. If it was destroyed, disabled, made inactive, or lost
   click selection, clear pointer state and panic because the requested
   semantic click cannot complete.
5. Submit the selected `PointerUp` and recheck the target again.
6. Submit `PointerClick`, which is always selected because `click` requires it,
   and fully apply its response. The target may become invalid now because the
   requested gesture has completed.
7. Clear the internal press. Retain hover only if response reconciliation left
   the target valid.

If a synchronous response changes only the target's pointer-event selection,
each later step uses the new selection. A response that disables click before
the final step prevents `PointerClick` and causes `click` to panic, because the
requested semantic gesture could not complete.

The lower-level methods share the same hovered and pressed pointer state.
`move_pointer` accepts an optional target, emits selected exit and enter actions
for a target change, and makes the new target current. Moving within the same
target updates the stored input but emits no action. Moving from `None` to
`None` is a no-op. `pointer_down` requires the target to be current, records
its object, pointer ID, and button as pressed, and emits `PointerDown` only when
selected. A second down replaces the recorded press after its preconditions
pass. `pointer_up` requires the target to be current, emits selected
`PointerUp`, emits selected `PointerClick` only when the recorded press has the
same object, pointer ID, and button, and always clears the press.
`pointer_cancel` clears only the press without emitting an action; it retains
the hovered target and last hit.

If a synchronous response invalidates the target during a lower-level method,
the fake clears hover and press state and returns after the actions already
sent. This is a normal device-state transition. Only the semantic `click`
wrapper promises a complete click and therefore panics on mid-gesture
invalidation.

For primitive and image objects, the automatic collider is present only while
the pointer-event set is nonempty; prefab collider capability remains the
catalog-declared value. After every command that changes objects or input,
reconcile device state without submitting actions. Disabling input clears hover,
press, and held keys.
Removing or making a hovered object inactive clears hover and any press on that
object. Removing or making only the pressed object inactive clears the press.
Replacing global keys removes held keys that are no longer enabled. Changing
pointer-event selection alone does not clear hover or press. These rules also
apply to scene unload and recursive object destruction.

`PointerInput` has `pointer_id: i32`, `screen_position: ScreenPosition`,
`world_hit: Vector3`, `button: PointerButton`, `modifiers: KeyModifiers`, and
`pointer_type: PointerType`. `KeyInput` has `physical_key: PhysicalKey` and
`modifiers: KeyModifiers`. Their component types are the shared physical-input
values also used by the UI fake; the world and UI coordinate-bearing wrappers
remain distinct. A non-null lower-level target
must exist, be active in the hierarchy, and have a collider. Input must be
enabled before every lower-level method. Pointer-event selection decides which
actions are emitted; absence of an optional enter, exit, down, or up selection
does not make an otherwise valid physical transition fail. A violated target or
state precondition panics. A supplied pointer ID must be nonnegative, world
input rejects `PointerButton::Other`, and all screen and world coordinates must
be finite; screen coordinates are not clamped to the configured screen
rectangle.

Key down and key up require input to be enabled and the input's `PhysicalKey`
to be present in `global_keys`. `key_down` records an unheld key before
submitting `KeyDown`; calling it for a held key is a no-op. `key_up` removes a
held key before submitting `KeyUp`; calling it for a key that is not held is a
no-op.
The submitted payload preserves the supplied modifier set. Held-key identity
depends only on `physical_key`, so modifier changes do not create another
transition for an already held key. Input-enabled and global-key preconditions
are checked before the held-key no-op check. No-op key calls do not consume
action IDs or call the engine.

Every submitted action receives a deterministic, nonzero UUID created with
`Uuid::from_u128`, beginning at one and increasing by one. The sequence restarts
whenever a new session is established, including reconnect. IDs are consumed
only for action bodies actually passed to `Engine::submit`; unselected pointer
events and physical-state no-ops consume none. This keeps histories and
`caused_by_action_id` values
reproducible without adding a configurable ID-provider abstraction.

Polling is always explicit. `connect`, `reconnect`, and the input helpers never
call `Engine::poll` or drain queued engine work. `poll()` performs exactly one
`Engine::poll` call and fully applies its response if one is returned. Tests
call it once for each queued response they expect, which keeps work returned
synchronously by `submit` distinguishable from work queued for polling.
`poll()` returns `()` whether the engine returns `None`, an empty response, or a
response with messages. Tests observe the difference through world and journal
state, not a fake-specific status value.

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

- `assert_object(ObjectId) -> &FakeObject` returns the requested object or
  panics.
- `assert_object_absent(ObjectId)` verifies that an ID is no longer present.
- `assert_object_kind(ObjectId, &GameObjectKind)` compares the complete current
  protocol kind state with an expected value.
- `assert_image(ObjectId, &ImageState)` compares complete image state and
  diagnoses objects of the wrong kind.
- `assert_text(ObjectId, &str)` compares visible text content and diagnoses
  objects of the wrong kind.
- `assert_local_transform(ObjectId, LocalTransform, f64)` and
  `assert_world_transform(ObjectId, WorldTransform, f64)` compare every vector
  component and quaternion orientation with the caller-supplied absolute
  tolerance. Quaternion `q` and `-q` must compare equal because they represent
  the same orientation.
- `assert_world_position(ObjectId, Vector3, f64)` is the position-only
  convenience used by command-focused tests.
- `assert_command(description, predicate)` finds a matching journal command and
  prints the journal when none matches.
- `assert_no_commands()` verifies that no commands ran after the caller last
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
the test calls `client.poll()` before making the assertions.

The game can create `ChessEngine` from a deserialized save, from a compact
in-memory position builder, or by mutating a reusable game-state template.
Those choices remain outside `battlement-fake`.

## Performance considerations

The fake should be fast because it does less work, not because it has complex
internals. Required practices are:

- Share an immutable catalog with `Arc`.
- Move snapshot objects and executed commands rather than clone them.
- Reserve object and journal collections from known input counts.
- Use direct synchronous calls without serialization or locks.
- Store only current world state and the compact command journal.
- Avoid allocating lifecycle records for instant operations.
- Let callers clear command history between table-driven cases.

Use ordinary standard-library maps and compute world transforms on demand. Do
not add custom hashers, arenas, copy-on-write worlds, transform caches, or
compiled snapshots without profiling a real slow suite and separating fake
costs from save parsing and engine work. The crate has no benchmark or fixed
wall-clock CI threshold. Large game suites should use the table-driven form
defined below and in-memory state builders rather than thousands of functions
or repeated parsing.

## Validation strategy

The test suite validates the public contract, not the chosen collection types
or private reducer boundaries. Except for focused transform-math tests, every
test must construct a `FakeClient`, drive public methods, and make assertions
through the public world, journal, or a fixture engine's external probe. A
refactor that preserves those observations should preserve the tests.

### Deterministic fixture engine

Put shared test support under `crates/battlement-fake/tests/support/`. Its
`ScriptedEngine` uses `ActionPayload = ()`, `ErrorCode = ()`, and `Command =
Command`, with an `Rc<RefCell<Probe>>` retained by the test. It owns:

- One queued connect response per expected connection.
- One expected `ClientMessage` and scripted response per expected submit.
- A FIFO of scripted poll results, including explicit `None` entries.
- A cloneable probe that records received `Connect` and `ClientMessage` values
  so the test can inspect them after the fake takes ownership of the engine.

Each engine method consumes exactly one scripted entry and panics on an
unexpected call, wrong submitted message, or exhausted script. This makes an
implicit extra poll, a missing pointer action, or incorrect call order fail at
the point of divergence. Fixture builders must use deterministic nonzero IDs
and provide the smallest valid catalog, scene, camera, and object graph needed
by the case. Tests must not sleep, inspect wall time, start threads, or depend
on hash-map iteration order.

### Required black-box test groups

Organize integration tests by observable behavior. File names may differ, but
all groups below are required:

1. **Connection and responses:** default and custom `Connect`, initial snapshot
   ordering, empty submit responses, `poll` returning `None`, one response per
   explicit `poll`, no implicit polling, message ordering, wrong-session panic,
   reconnect replacement, session-state reset, and journal retention.
2. **World and hierarchy:** snapshot construction; local and world transforms;
   active-in-hierarchy; create, recursive destroy, activate, reparent with both
   `world_position_stays` values; scene load, unload, and primary selection; and
   snapshot replacement.
3. **Command coverage:** at least one public-path case for every current
   `CommandBody` variant. Each state-changing case asserts both the journaled
   original command and the exact final world or logical-operation state. A
   command with no retained state asserts its journal entry and the relevant
   unchanged invariant. Group related variants into table-driven cases instead
   of one test function per variant.
4. **Temporal semantics:** once, finite restart, odd finite ping-pong, even
   finite ping-pong, and both forever modes. Exercise the shared factor rule on
   representative numeric, vector, quaternion, color, and audio-volume
   properties rather than duplicating every repeat case for every tween.
5. **Input:** exact enter/exit/down/up/click action order and payloads; no event
   for an unselected pointer action; same-target movement; mismatched press and
   release; pointer cancellation; mid-gesture synchronous responses; key
   filtering and held-key no-ops; deterministic action IDs; input disable; and
   explicit separation of submit responses from polled responses.
6. **Duplicate and logical-operation behavior:** duplicate-batch suppression,
   duplicate command rejection, audio play/volume/tween/stop, particle
   play/stop/spawn, animator state and parameters, transient triggers, wait,
   and known versus unknown cancellation targets.
7. **Representative panics:** invalid initial response, one rejected
   `Snapshot::validate`, one rejected `Command::validate`, catalog category
   mismatch, missing target, invalid hierarchy mutation, and non-clickable
   click. Assert the relevant identifier appears in the panic text when one is
   available. Do not duplicate the protocol crate's exhaustive validation
   matrix.
8. **End-to-end game scenario:** promote a pawn through two clicks, explicitly
   poll queued work, and assert its movement tween, promotion particle,
   destruction, queen creation, and final state.

A table-driven test stores named cases containing inputs and expected outputs,
then runs the same arrange/act/assert body for each row. Every row must carry a
case name that is included in assertion failures. Use this form for command
variants, tween modes, and large game-state suites; do not create thousands of
nearly identical Rust test functions.

### Test oracles and coverage rules

Every behavior assertion must use at least one of these explicit oracles:

- The fixture probe contains the exact outbound `Connect` or `ClientMessage`.
- A public world query equals the expected current state.
- The journal contains the exact original command and expected location
  metadata.
- A documented invalid request panics with identifying context.

Do not assert private field layouts, allocation counts, helper call counts, or
exact panic prose. Test identifiers and salient values, not the full sentence.
The exhaustive `CommandBody` match is the compile-time guard for new variants;
the implementation change that adds a match arm must also add its black-box
case before it is complete.

During implementation, run `cargo test -p battlement-fake` after each coherent
slice. Before committing a milestone, run `cargo fmt --all --check`,
`cargo clippy -p battlement-fake --all-targets -- -D warnings`, and
`cargo test -p battlement-fake`. Before final submission, run `./scripts/ci.py` so
the new crate is checked together with the complete Rust workspace and existing
Unity suite. CI must run the same `battlement-fake` tests; there is no separate
manual-only correctness suite.

This crate is non-rendering, so screenshots and video are neither useful nor
required. Production Unity tests remain the authority for rendering, physics,
actual scheduling, intermediate animation values, easing, prefab-authored
children, particle appearance, and audio timing.

## Proposed milestone breakdown

### Milestone 1: connection and world

Add the workspace member and public module skeleton first. Then implement the
shared asset catalog, scripted test engine, constructors, common response path,
snapshot application, object hierarchy, transforms, read-only queries, and
reconnect. Stop only when the connection/response and world/hierarchy
black-box groups pass. Do not add command stubs that claim success without
applying state.

### Milestone 2: commands and input

Add batch admission and the journal before individual command families. Add
command families in the order of the behavior table, running their
table-driven public-path cases as each family lands. Then add instant temporal
rules, semantic pointer and keyboard helpers, and explicit polling. Stop only
when every `CommandBody` variant has a passing black-box case and the complete
pawn-promotion scenario passes.

### Milestone 3: test ergonomics and completion

Add only the focused assertion helpers listed in this document. Complete
representative panic coverage and public API documentation, then run the
focused format, Clippy, and package-test commands. Run `./scripts/ci.py` last.
The implementation is ready for review only when the worktree is clean after
one Conventional Commit and the complete change is submitted through the
repository's Tollgate workflow.

## Definition of done

The fake-client implementation is complete only when all of the following are
true:

- `battlement-fake` is a workspace crate with no transport, Unity, async-runtime,
  or wall-clock waiting.
- The public API in this document compiles for an external integration test;
  no black-box test relies on private modules or engine accessors.
- Constructors, submit-driven input, reconnect, and `poll` all use one ordered
  response path with the documented session rules.
- Every applied snapshot and newly executed command passes through the protocol
  crate's `Validate` implementation before fake-specific execution.
- The executor has an explicit arm and black-box case for every current
  `CommandBody` variant.
- Every state-changing command is observable in both the journal and current
  fake state; transient commands have the exact observation described here.
- Input action order, payloads, ID allocation, physical state, synchronous
  response interleaving, and absence of implicit polling are covered.
- Reconnect and same-session snapshot replacement reset exactly the state
  listed in this contract and retain exactly the state listed here.
- Focused package checks and the complete `./scripts/ci.py` suite pass without
  warnings introduced by the new crate.
- Public items have concise documentation and all source files remain within
  the repository's size and Rust-style rules.
