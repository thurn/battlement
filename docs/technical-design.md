# Masonry Technical Design

Status: approved v1 implementation contract

This document is normative for v1. The Rust types under
`crates/masonry` are the source of truth for the boundary between a
Rust rules engine and its thin Unity client. The names, defaults, validation
rules, ordering, and failure behavior below are fixed. An implementation change
that alters any of them must update this document, the Rust types, and their
contract tests in the same commit. The Rust model is independent of its eventual
binary encoding.

## Masonry in one minute

Masonry is a thin Unity rendering and input client for turn-based games whose
rules engine is written in Rust. The Rust engine owns the rules and the
authoritative game state, including facts such as “piece P is on square B.” It
tells Masonry what to display and receives player input from Masonry; Masonry's
C# code is a Unity-facing wrapper rather than an independent game framework or
language-neutral SDK. In production, Unity reaches the Rust engine through a
native plugin. During development, the same engine may run as a synchronous
localhost HTTP service.
The [Transports](#transports) section describes both arrangements.

When Unity first connects, the rules engine sends a **snapshot**: a complete
description of the Unity content Masonry should construct, including loaded
scenes, game objects, transforms, cameras, lights, and enabled input
settings. A snapshot lets Masonry construct the current world without replaying
everything that happened earlier.
The [Snapshots and scene replacement](#snapshots-and-scene-replacement) section defines exactly what a snapshot contains and how Masonry applies one.

Each initial connection or reconnect begins a **session**, identified by a
**UUID** (universally unique identifier).

A normal turn follows this loop:

1. Unity connects to the rules engine.
2. The rules engine sends the current snapshot.
3. The player clicks a Unity object. Masonry sends an **action**—a MessagePack record
   of player input—to the rules engine. It includes the object's UUID, a globally
   unique identifier assigned by the rules engine.
   The [Pointer and keyboard input](#pointer-and-keyboard-input) section lists the supported actions.
4. The rules engine decides whether the click is legal and returns **commands**,
   MessagePack instructions that tell Masonry how to change Unity. A **batch** is one
   ordered set of commands. Masonry does not make the game-rule decision.
5. Masonry executes the commands against Unity. Some commands may animate their
   changes over time.

A batch can divide its commands into **parallel command groups**. Masonry
considers parallel command groups in list order, but it launches commands within
one group without waiting for earlier commands in that group to finish. Unity
calls still occur one at a time on Unity's main thread. A **blocking command**
prevents the next group from starting until that command finishes; a
nonblocking sound can continue while the next group begins.
The [Batch and parallel command group timing](#batch-and-parallel-command-group-timing)
section gives a complete timing example.

For example, consider a board-game piece at square A:

- The current snapshot says that piece UUID `P` is at A and may receive clicks.
- The player clicks P.
- Masonry sends `pointer.click(P)`.
- The rules engine accepts the move and returns “move P to B over 300 ms, and
  play `mygame/audio/piece-move`.”
- Masonry animates P from A to B over 300 ms. If Unity reconnects halfway
  through, the rules engine's new snapshot places P directly at the position it
  currently requires; it does not replay half of the animation or the sound.

That 300 ms movement is a **tween**: a gradual change from one displayed value
to another over a specified duration.
The [Animation, Animator, particles, and audio](#animation-animator-particles-and-audio) section defines the supported tween properties and timing options.

This is the central boundary: the rules engine decides what is true, while
Masonry turns that decision into Unity objects, animation, sound, and input
events.

## Responsibilities

Keeping game rules outside Masonry makes the boundary between the rules engine
and Unity explicit:

| Rules engine | Masonry Unity client |
|---|---|
| Own game rules and authoritative game state | Own the Unity objects it creates |
| Decide whether an action is legal | Raycast pointer input and report actions |
| Choose final positions and other values | Apply values and animate toward them |
| Decide which commands may overlap | Enforce the requested ordering and detect conflicts |
| Prepare complete snapshots | Construct Masonry-controlled Unity content from snapshots |
| Produce valid MessagePack | Deserialize commands and report execution failures |
| Prevent duplicate actions | Prevent duplicate command batches |

Masonry does not infer game rules, choose legal moves, or inspect arbitrary C#
properties. If a decision can live in the rules engine without harming responsiveness,
it belongs there.

## v1 scope

V1 focuses on turn-based 3D worlds. This table is a scope boundary: the right
column is not implemented by v1.

| Included in v1 | Deferred |
|---|---|
| Unity 6.5 and Universal Render Pipeline (URP) | Earlier Unity versions, Built-in pipeline, HDRP |
| Turn-based games | Real-time continuous state synchronization |
| Additive scenes loaded from runtime asset addresses | Loading the same scene address twice; WebGL |
| Empty objects, basic shapes, image quads, world-space TextMesh Pro (TMP) text | Runtime UI, Canvas, UI Toolkit |
| Prefabs and root-level supported components | Addressing arbitrary prefab children or scene objects |
| Standard cameras and lights | Cinemachine and pipeline-specific lighting |
| Transform, camera, light, text, image, and audio tweens | Arbitrary property tweening and spline paths |
| Unity Animator, particles, and audio | Advanced shader/material editing |
| Collider-based pointer input and discrete keyboard input | Dragging, scrolling, gestures, text entry |
| Colliders for selection | Rigidbody forces, joints, and physics game rules |
| Precompiled custom C# extensions | Downloaded or runtime-compiled C# |
| Native production and localhost HTTP development transports | Recorded-file and production network transports |

World-space TextMesh Pro text is treated as a 3D object, not as a general UI
system.

The reference project and package lock use Unity 6000.5.8f1, Linear color
space, and the editor-matched URP 17 and uGUI/TMP core packages. Registry
dependencies are pinned exactly to Input System 1.20.0, Addressables 4.0.1,
PrimeTween 1.4.11, MessagePack-CSharp 3.1.8, and Unity Test Framework 1.7.0.
Floating revisions and `@latest` documentation or package references are not
permitted. Upgrading any dependency requires the full release checks.

## End-to-end protocol example

A single click now illustrates the complete message flow. Examples use labeled
diagnostic notation for readability; they are not a second wire encoding.
MessagePack structs always include every field in Rust declaration order, even
when an example omits a defaulted field.
The [Rust protocol types](#rust-protocol-types)
section defines how those fields flow from the Rust source of truth into the
Unity client.

To keep the diagnostic examples compact, they omit values set to these v1 defaults:

- Game objects have Unity `activeSelf` set to true. Missing local position, rotation, and scale use
  zero, the identity quaternion, and one respectively.
- A sole content scene is primary. A snapshot with multiple content scenes must
  identify one as primary.
- Cameras are enabled and perspective, with a 60-degree field of view, 0.3 near
  clipping, and 1,000 far clipping.
- Pointer actions use the left mouse button and pointer ID 0.
- Batches start immediately.
- Commands are blocking unless marked otherwise.
- Animations have zero delay, zero duration, `inOutSine` easing, and no repeats.
  Zero duration applies the final value immediately.
- Animator commands use layer 0, no cross-fade, and normalized start time 0.
  Audio uses volume 1, pitch 1, and does not loop.
- A property write cancels an operation already controlling that property;
  `onConflict: "wait"` requests the alternative behavior.
- Optional lists are empty.

Fields without a safe default remain required. Examples include asset
addresses, target object UUIDs, and every command UUID.

### 1. Connect

Before sending game state, the rules engine needs to know which Unity build has
connected. Unity therefore reports its environment and every game-specific
command type compiled into the build. Each game-specific type is implemented by
a **custom handler**, a C# class such as the one behind
`mygame.character.flash`.
The [Custom C# code](#custom-c-code) section explains registration, execution, and failure handling.

```text
{
  "type": "masonry.connect",
  "platform": "macOS",
  "unityVersion": "6000.5.8f1",
  "screen": { "width": 2560, "height": 1440 },
  "customCommandTypes": ["mygame.character.flash"]
}
```

A native connect additionally includes `persistentDataPath` and
`streamingAssetsPath`; HTTP development connect omits them. Both are absolute
UTF-8 paths. Masonry sends no protocol-version or protocol-identity field.

### 2. Initial snapshot

To build the initial Unity world, the rules engine starts a session and sends
its first snapshot. The connect call returns a `masonry.response`; the first
element of its `messages` array is the snapshot shown below. Unity's
[Addressables 4.0.1](https://docs.unity3d.com/Packages/com.unity.addressables@4.0)
system loads scenes and assets identified by stable strings at runtime. The
snapshot declares every **prepared asset**: an Addressable scene, prefab,
material, texture, audio clip, or effect that Masonry must load and type-check
before any command may use it. Preparing assets in advance prevents an ordinary
command from unexpectedly starting an asset load.
The [Assets and Addressables](#assets-and-addressables) section covers their lifetime.

```text
{
  "type": "masonry.snapshot",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "preparedAssets": [
    { "address": "mygame/boards/forest", "kind": "scene" },
    { "address": "mygame/pieces/knight", "kind": "prefab" },
    { "address": "mygame/audio/piece-move", "kind": "audioClip" },
    { "address": "mygame/effects/dust", "kind": "particleEffect" }
  ],
  "scenes": [
    {
      "sceneId": "ca64d87d-33d9-4a19-be6e-597035312d01",
      "address": "mygame/boards/forest"
    }
  ],
  "objects": [
    {
      "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
      "kind": "prefab",
      "address": "mygame/pieces/knight",
      "parentScene": {
        "scene": "ca64d87d-33d9-4a19-be6e-597035312d01"
      },
      "pointerEvents": ["enter", "exit", "click"]
    },
    {
      "objectId": "8ff6f71c-6a74-41cf-8826-0e364abf9f97",
      "kind": "camera",
      "parentScene": {
        "scene": "ca64d87d-33d9-4a19-be6e-597035312d01"
      },
      "localTransform": {
        "position": { "x": 0, "y": 8, "z": -10 },
        "rotation": { "x": 0.25, "y": 0, "z": 0, "w": 0.97 }
      },
      "camera": {
        "fieldOfView": 50
      }
    }
  ],
  "inputCameraId": "8ff6f71c-6a74-41cf-8826-0e364abf9f97",
  "globalKeys": ["Escape"]
}
```

### 3. Player action

With the initial state visible, clicking the knight produces this action:

```text
{
  "actionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "type": "masonry.pointer.click",
  "payload": {
    "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
    "screenPosition": { "x": 1280, "y": 720 },
    "worldHit": { "x": 0.1, "y": 0.4, "z": 0.0 }
  }
}
```

### 4. Immediate response

Because the rules engine can decide this move quickly, it returns the response
in the same blocking call that carried the click. Delayed work instead arrives
through polling, as shown next. Returning immediate work this way avoids an
extra poll before a hover or click effect can begin.

Masonry supplies built-in command types such as
`masonry.transform.tweenWorldPosition` and
`masonry.audio.play`. Each built-in type is a **core command** implemented by
Masonry. The [Command execution and failures](#command-execution-and-failures)
section describes how command errors are reported.

```text
{
  "type": "masonry.response",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "messages": [
    {
      "type": "masonry.batch",
      "batchId": "c07f0804-6455-40a6-b0f0-5d1a3d87ea81",
      "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
      "causedByActionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
      "groups": [
        {
          "commands": [
            {
              "commandId": "7bbcb27e-f75b-4c63-bf86-ad1b0f6ee2cd",
              "type": "masonry.transform.tweenWorldPosition",
              "payload": {
                "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                "position": { "x": 4, "y": 0, "z": 2 },
                "durationMs": 300
              }
            },
            {
              "commandId": "b11cc056-12ad-4d57-8ea4-f988e5d24984",
              "type": "masonry.audio.play",
              "blocking": false,
              "payload": {
                "address": "mygame/audio/piece-move"
              }
            }
          ]
        },
        {
          "commands": [
            {
              "commandId": "50385228-4591-4b55-8bd9-bcb8521ee2e0",
              "type": "masonry.particle.spawn",
              "blocking": false,
              "payload": {
                "address": "mygame/effects/dust",
                "location": {
                  "gameObject": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03"
                },
                "lifetimeMs": 800
              }
            }
          ]
        }
      ]
    }
  ]
}
```

Masonry launches the move and sound without waiting for either to finish, and
starts the dust after the 300 ms move finishes. The sound does not delay the
dust because it is nonblocking. Unity calls within the first group still occur
one at a time on Unity's main thread. The rules engine remains responsible for
including P's current position in any future snapshot.

### 5. Later poll

If the rules engine does expensive work after the click, a later poll can return more
batches. `causedByActionId` connects the delayed result to the original input
for logs and performance measurements:

```text
{
  "type": "masonry.response",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "messages": [
    {
      "type": "masonry.batch",
      "batchId": "11ff68d6-293f-4192-9ea0-71d50d79e16b",
      "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
      "causedByActionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
      "groups": [
        {
          "commands": [
            {
              "commandId": "076f85ae-bc39-4be6-83bb-5793233092ac",
              "type": "masonry.animator.play",
              "blocking": false,
              "payload": {
                "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                "state": "Celebrate"
              }
            }
          ]
        }
      ]
    }
  ]
}
```

The rules engine is responsible for suppressing a delayed celebration when it
is no longer relevant. A later snapshot describes whatever Animator state the
rules engine wants after reconnecting; it does not resume this animation at its
previous playback time.

### 6. Batch failure

Masonry executes commands in order rather than simulating the entire batch in
advance. If a command cannot run because an object or asset is missing, a Unity
call throws, or a custom handler fails, Masonry stops that batch and reports
`masonry.batch.failed`. Commands that already ran are not rolled back.

This example reports a particle command whose asset was never prepared:

```text
{
  "type": "masonry.batch.failed",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "batchId": "0cb9b6d9-b6ee-4105-8afe-ee4ba5105b24",
  "commandId": "4a52e41e-0b60-4e00-8bc0-588165037b6f",
  "errorCode": "asset_not_prepared",
  "message": "mygame/effects/missing-spark was not in the prepared asset set"
}
```

## Commands and running operations

Commands change the Unity content currently displayed by Masonry. Masonry does
not maintain a second authoritative model of that content. The rules engine
owns the game state and must be able to produce a complete current snapshot.

A transform tween leaves the object at its final transform until another
command or snapshot changes it. Sounds and finite particle effects end according
to their command parameters. A snapshot cancels work still in progress and
constructs the content described by the rules engine; it does not resume the
previous playback position of a tween, sound, particle effect, or animation.

Masonry reports actions using the object UUID and hit position visible at the
time of input. The rules engine checks each action against its current game
state. If it rejects the action, it returns the correction the game needs, such
as a fresh snapshot or a short visual effect. Masonry has no generic
rejection UI in v1.

The rules engine must not enqueue work that it already knows is
obsolete. It serializes all outgoing batches in causal order, and each
transport preserves that order when sending them to Masonry.

Once Masonry starts a tween or effect, it tracks that running work as an
**operation**. Its UUID is the UUID of the command that started it. If a new
command writes the same canonical property, omission of `onConflict` cancels
the older operation and starts from the currently displayed value;
`onConflict: "wait"` delays the new command until the older operation ends. A
snapshot cancels running operations, then applies snapshot values directly.

For example:

```text
{
  "commandId": "565e76aa-b480-43c2-900b-1cb9d90e4602",
  "type": "masonry.transform.tweenLocalScale",
  "onConflict": "wait",
  "payload": {
    "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
    "scale": { "x": 1.1, "y": 1.1, "z": 1.1 },
    "durationMs": 80
  }
}
```

A blocking command with `onConflict: "wait"` remains incomplete while waiting
and while its own tween runs. A nonblocking command does not hold up its group,
but it still starts only after the conflicting operation ends. A snapshot may
cancel either one before it starts or finishes. A command using `wait` fails
when the existing operation repeats forever because it would never start.
`masonry.operation.cancel` succeeds as a no-op when its command UUID is known
but no longer running, and fails with `unknown_command` when that UUID was never
executed in the session. Masonry retains executed command UUIDs for the session.

All Masonry animations use unscaled time. They continue when Unity's
`Time.timeScale` is zero.

### Session and duplicate checks

Every snapshot, action, batch, and failure carries the session UUID.
A reconnect creates a new session UUID and clears both sides' duplicate-ID
histories. A message from another session is never executed.

| Incoming message | Masonry behavior |
|---|---|
| Duplicate batch UUID in the current session | Ignore it and log the duplicate |
| Different session UUID | Discard it and log an error |

## Batch and parallel command group timing

Parallel command groups are ordered by their blocking work, not by every effect
they start.

Example timeline:

```text
0 ms    Group 1 creates object A immediately.
Same frame, after Group 1 completes:
        Group 2 starts a blocking 300 ms move and a nonblocking 800 ms sound.
300 ms  Group 3 starts because Group 2's blocking move is done.
800 ms  The sound ends; it did not hold up Group 3.
```

A batch is complete when every group's blocking work is complete. Nonblocking
effects may still be running. Infinite effects must be nonblocking.

Each incoming batch says one of the following:

- `start: "now"` starts it as soon as scheduling permits. A hover response that
  changes a different property uses this mode.
- `start: "afterEarlierBlockingWork"` waits for blocking work in previously
  received batches. It does not wait for nonblocking sounds, particles, or
  infinite loops.

Only the rules engine knows whether two batches are related, so it chooses the start
mode. A conflicting write cancels the old operation unless the new command
explicitly says to wait.

If an earlier batch fails, a batch waiting with
`afterEarlierBlockingWork` fails before executing with
`earlier_batch_failed`. The failure propagates through consecutive dependent
batches. A later `start: "now"` batch is independent and may execute.

## Command execution and failures

Masonry deserializes the batch format and enforces basic safety limits before
scheduling it. These checks include:

- Required fields and finite numeric values
- Fixed size and count limits
- Session and duplicate batch UUID

When the batch and session IDs are available, an error at this stage is also
reported as `masonry.batch.failed`. A response or contained response message too
malformed to identify and order reliably is session-fatal rather than ignored.

Commands then execute in group and list order. Each command resolves its object,
scene, asset, component, and custom-handler references when it runs. Masonry
uses explicit lookup failures where Unity would otherwise silently accept a
missing reference, and it catches Unity and custom-handler exceptions. Every
such error reports `masonry.batch.failed`, identifies the command when known,
and stops the remaining commands in that batch. Earlier commands remain in
effect, and Masonry does not attempt rollback or simulate later commands in
advance.

For example, “create A, then move A” succeeds because the move resolves A after
the create command runs. “Create A, then move missing B” creates A and then
fails at the move. If a command depends on an earlier asynchronous command, the
rules engine must place it in a later group and make the earlier command
blocking.

A batch failure does not automatically request a snapshot. The authoritative
rules engine decides whether the reported failure requires corrective commands
or a replacement snapshot.

## Ownership and object identity

A Unity game keeps a **bootstrap scene** loaded for the life of the client so it
can host `MasonryRunner` and unrelated game-owned objects. Inside that scene,
Masonry creates a **persistent container** for game objects that must survive
content-scene changes. Each additively loaded content scene receives a **scene
container** for its Masonry-controlled objects.

An individually targetable object is a **game object**. Each game object has a
UUID and a `MasonryIdentity` component that registers it, unregisters it on
`OnDestroy`, and lets a pointer hit on a child collider find the game object. Objects
authored directly into a content scene load and unload with that scene, but v1
cannot target them individually.

Masonry therefore owns only objects it creates and scenes it loads. It never
scans or deletes unrelated objects in the bootstrap scene:

```text
Bootstrap scene
  MasonryRunner
  Masonry persistent container
    camera game object [UUID, MasonryIdentity]

Addressable content scene instance [scene UUID]
  Masonry scene container
    prefab instance root [object UUID, MasonryIdentity]
      authored child collider [no UUID]
  authored scene objects [loaded/unloaded with the scene; not targetable]
```

`MasonryIdentity` stores the UUID as a `System.Guid` rather than as an arbitrary
display name.

Unity's destroyed objects require special care: a destroyed
`UnityEngine.Object` can compare equal to `null` even while a C# reference still
exists. Masonry uses Unity-aware checks, does not rely on `?.` or `??` for these
objects, and checks references again after asynchronous waits.

Object and scene UUIDs come from the rules engine. A game-object UUID is not reused
after destruction within the same session. Static content uses Addressables
addresses rather than UUIDs. Command and action kinds use namespaced strings.

Masonry keeps every executed batch UUID for the session and ignores a duplicate
after logging it. The rules engine keeps every action UUID for the session. An exact
duplicate returns the cached response or reports no new work; the
action is never applied again. Reusing one action UUID with different MessagePack is an
error. This avoids an undefined retry window for commands with visible side
effects.

## Snapshots and scene replacement

A snapshot completely describes the Unity content that Masonry should
construct. It contains:

- Session UUID
- Complete prepared asset set
- Loaded content scenes and the primary scene
- Game objects, their parent scene, parent, kind, `activeSelf` state, and local
  transform
- Camera, light, material, image, text, and interaction values
- The stable Unity Animator state, persistent bool/int/float parameters, and
  speed that must be visible after reconnect or replacement

It does not contain custom-handler state, one-time sounds, particles, a hover
pulse, or progress through an attack animation.

`preparedAssets`, `scenes`, and `objects` are required lists. At least one scene
is required. `primarySceneId` may be omitted only when exactly one scene is
listed. `inputCameraId` is required and must name a camera GameObject declared
in that same snapshot's `objects` list; Masonry creates that object while
applying the snapshot. The GameObject must be active in the Unity hierarchy and
its Camera component must be enabled. This is distinct from Unity's active
Scene. `inputDisabled` defaults to false and `globalKeys` defaults to an empty
list.

Applying a snapshot may span frames while Masonry waits for asynchronous asset
or scene loads. Once those loads are ready, Masonry validates the decoded
snapshot, disables input, cancels operations, reconciles prepared assets and
additive scenes, destroys existing Masonry-created objects, and recreates the
snapshot objects in topological parent order without artificially slicing that
work across frames. The replacement is direct: Masonry does not retain a
complete old world, stage a second world, hide all controlled content, or
promise an atomic reveal. Input resumes after the new world is complete.

Prepared handles are reused when address and kind match. A content scene
instance is reused only when both scene UUID and address match. Its authored
objects retain their runtime state. A new scene UUID forces unload and reload,
even when its address is unchanged. Game-object instances are never reused
across a snapshot boundary, even when their UUID and kind match.
Because v1 cannot hold two instances of one scene address, a same-address reload
unloads the old instance before loading the replacement.

Messages after the snapshot in the ordered stream wait until it finishes. A
later snapshot waits in order rather than preempting the current replacement.
Snapshot validation or application failure disables input, cancels the
session's work, and permanently stops that session. Masonry does not roll back
or retry, so a failure after replacement begins may leave a partially replaced
world visible. Development builds log and display the diagnostic. A host-
requested reconnect may start a new session.

A normal scene-changing batch does not scan future commands to infer a
transition. The rules engine uses `masonry.input.setEnabled` before loading when
the old world must stop receiving input. `masonry.scene.setPrimary` disables
outgoing-scene pointer events during its atomic cutover and then restores the
configured input state.

One bootstrap scene persists for the life of the client. Content scenes must be
Addressable and load additively. Exactly one loaded content scene is primary.
V1 cannot load the same scene address twice at once.

Every game object has a parent-scene selection: the primary scene, a named
content scene, or the persistent container in the bootstrap scene. Omitting the
selection uses the primary scene.
Objects may only be parented within the same scene. Unloading a content scene
also removes its authored scene objects and every Masonry game object in that
scene. Those authored objects are considered part of the scene Masonry was
asked to load; Masonry still does not touch unrelated bootstrap objects.

Masonry loads and unloads authored objects inside a content scene, but v1
commands cannot target them individually. Targetable objects must be created by
Masonry or instantiated as prefabs.

## Game object types

V1 creates empty GameObjects, Unity's standard primitive shapes, image quads,
world text, standard cameras and lights, and Addressable prefab instances.

Prefab assets use stable Addressables strings such as
`mygame/characters/goblin`; each runtime copy has its own object UUID. Commands
look only for supported components on the prefab root, with at most one of each
supported component type. For example, `animator.play` targets the root
Animator. Child components and multiple independently controlled Animators need
custom C# in v1.

A click on a sword collider can walk upward to the goblin's
`MasonryIdentity` and emit the goblin UUID.

Snapshot entries use local position, quaternion rotation, and local scale.
Commands may move in local or world coordinates. A reparent command says
whether the object should stay at its current world position. Destroying a
game object also destroys its game-object descendants unless they were
reparented first.

Material support is intentionally small. Masonry may assign a prepared material
to all renderer slots or one slot on a supported root renderer. It does not edit
shader properties, keywords, or arbitrary material values. The built-in image
quad is a specific exception backed by a Masonry-owned URP material.

Image width and height are positive world units around a centered pivot.
`stretch` fills both dimensions, `contain` preserves texture aspect ratio and
leaves transparent space, and `cover` preserves aspect ratio while cropping
centered UVs. Texture filtering and wrapping come from the prepared texture.

When an image or text object enables face-camera behavior, Masonry updates it in
`LateUpdate` after tweens. Its local forward axis points toward the input camera
position and its local up aligns with the camera up vector projected onto the
facing plane, including camera roll. Coincident camera and object positions
retain the prior rotation. Rotation commands fail with
`property_controlled_by_billboard` while this behavior is enabled.

## Core data and command contract

The Rust protocol types are authoritative for serialization and machine
validation; this section is their human-readable inventory. UUIDs are nonzero UUID strings in
lowercase hyphenated form. All numeric values must be finite. Positions,
distances, and sizes use Unity world units, angles use degrees, normalized
values use `[0,1]`, and time fields are unsigned integer milliseconds. Colors
are linear `{r,g,b,a}` values with every component in `[0,1]`, except the
explicitly RGB-only image tint. Quaternions are
`{x,y,z,w}`, must have nonzero length, and are normalized by Masonry.

An object record has required `objectId` and `kind`, plus optional
`parentScene`, `parentId`, `active`, `localTransform`, and `pointerEvents`.
`parentScene` is a union of `"primaryScene"`, `{ "scene": sceneId }`, or
`"persistent"`, and defaults to `"primaryScene"`. Omitted `parentId` places the
object directly under its parent-scene container. `active` is Unity's
`activeSelf` value and defaults to true; `activeInHierarchy` may still be false
due to an inactive parent, and component `enabled` flags and Unity's active
Scene are separate states. The object graph must be acyclic and every parent
must have the same parent scene. Primary-scene selection is resolved when the
object is created; changing the primary scene later does not move existing
objects.

`kind` is exactly one of `empty`, `cube`, `sphere`, `capsule`, `cylinder`,
`plane`, `quad`, `image`, `text`, `camera`, `light`, or `prefab`. A prefab also
requires `address`; an image requires a prepared texture address, positive
width and height, and `fit` of `stretch`, `contain`, or `cover`; text requires
text content and a prepared TMP font address. Camera and light records contain
the complete component state listed below. Defaults are the end-to-end defaults
unless a row states otherwise.

Image state is `texture`, positive `width` and `height`, `fit` `stretch`, white
RGB `tint`, `opacity` 1, and `faceCamera` false. Image tint has no alpha channel;
opacity is its sole alpha control. Text state is `text`, `font`,
positive `size` 1, white `color`, horizontal `center`, vertical `middle`,
optional positive `wrapWidth` (absent disables wrapping), `richText` false,
and `faceCamera` false. Camera state is `enabled` true, `projection`
`perspective`, vertical `fieldOfView` 60, `orthographicSize` 5, `near` 0.3,
`far` 1000, `clearMode` `skybox`, and black `clearColor`. Light state is
`enabled` true, `lightType` `point`, white `color`, `intensity` 1, `range` 10,
`outerSpotAngle` 30, `innerSpotAngle` 0, and `shadows` `none`.

A primitive or prefab record may use `materials`, an ordered list of
`{slot, address}` records that assigns prepared materials by unique zero-based
root-renderer slot. A list avoids encoding numeric slots as MessagePack object-property
names and produces direct, strongly typed C# records. A prefab record may also
contain `animator` snapshot state for a supported Animator on its root. That
state requires `state` and may contain `layer` 0, `normalizedStartTime` 0,
`boolParameters`, `intParameters`, and `floatParameters` maps that default to
empty, and nonnegative `speed` 1. Triggers and playback progress are never
snapshot state. Preparation rejects more than one supported component of a
given type on the root.

Every command has required `commandId`, `type`, and `payload`. `blocking`
defaults to true. A command's UUID is also the UUID of any operation it starts.
Core property-writing commands accept `onConflict`; omission means `cancel`,
while `wait` waits for the existing operation. The shared custom-command
format does not contain `onConflict`; a game that needs conflict behavior for
a custom command defines it in that command's game-specific Rust payload type.
Waiting for an infinite operation fails. Immediate and tween writes use the
same conflict key. Destroying an object or applying a snapshot cancels affected
operations without consulting `onConflict`.

Conflict keys are object plus `position` (shared by local and world variants),
`rotation` (shared by local and world variants), `localScale`,
`camera.fieldOfView`, `camera.orthographicSize`, `light.color`,
`light.intensity`, `image.tint`, `image.opacity`, `text.color`, `text.size`, or
audio-play-command plus `volume`. Material slots are independent keys. A
projection switch cancels both camera projection-value keys. Reparenting
cancels position, rotation, and scale operations on the reparented root.
Different keys may animate concurrently.

The v1 core command union is exactly:

| Type | Payload and effect |
|---|---|
| `masonry.assets.replaceSet` | `assets`; atomically replace the complete prepared set after loading and validating additions |
| `masonry.scene.load` | `sceneId`, `address`, optional `makePrimary` false; load one prepared scene additively |
| `masonry.scene.unload` | `sceneId`; unload the non-primary scene and destroy its game objects |
| `masonry.scene.setPrimary` | `sceneId`; atomically call `SceneManager.SetActiveScene` and make the loaded scene Masonry's primary scene |
| `masonry.object.create` | `object`; create one complete object record; its UUID must be new in the session |
| `masonry.object.destroy` | `objectId`; destroy the game object and all game-object descendants |
| `masonry.object.setActive` | `objectId`, `active`; pass the value to `GameObject.SetActive`, changing `activeSelf` |
| `masonry.object.reparent` | `objectId`, nullable `parentId`, required `worldPositionStays`; a parent must share the object's parent scene, while null reparents to the existing parent-scene container and never changes `parentScene` |
| `masonry.transform.setLocalPosition` / `masonry.transform.setWorldPosition` | `objectId`, `position` |
| `masonry.transform.tweenLocalPosition` / `masonry.transform.tweenWorldPosition` | `objectId`, `position`, tween fields |
| `masonry.transform.setLocalRotation` / `masonry.transform.setWorldRotation` | `objectId`, `rotation` |
| `masonry.transform.tweenLocalRotation` / `masonry.transform.tweenWorldRotation` | `objectId`, `rotation`, tween fields; normalized shortest-arc spherical interpolation |
| `masonry.transform.setLocalScale` | `objectId`, `scale` |
| `masonry.transform.tweenLocalScale` | `objectId`, `scale`, tween fields |
| `masonry.renderer.setMaterial` | primitive or prefab `objectId`, `address`, optional zero-based `slot`; omission assigns every root-renderer slot using `sharedMaterials`; image/text renderers are excluded |
| `masonry.camera.setEnabled` | `objectId`, `enabled` |
| `masonry.camera.setPerspective` | `objectId`, `fieldOfView`; vertical FOV strictly between 1 and 179 |
| `masonry.camera.tweenFieldOfView` | `objectId`, `fieldOfView`, tween fields; camera must be perspective |
| `masonry.camera.setOrthographic` | `objectId`, positive `size` |
| `masonry.camera.tweenOrthographicSize` | `objectId`, positive `size`, tween fields; camera must be orthographic |
| `masonry.camera.setClipping` | `objectId`, positive `near`, `far` greater than `near` |
| `masonry.camera.setClear` | `objectId`, `clearMode` (`skybox`, `solidColor`, `depth`, or `nothing`) and `clearColor` when solid |
| `masonry.light.setEnabled` | `objectId`, `enabled` |
| `masonry.light.setType` | `objectId`, `lightType` (`directional`, `point`, or `spot`) |
| `masonry.light.setColor` / `masonry.light.tweenColor` | `objectId`, `color`, and tween fields for the tween variant |
| `masonry.light.setIntensity` / `masonry.light.tweenIntensity` | `objectId`, nonnegative `intensity`, and tween fields for the tween variant |
| `masonry.light.setRange` | `objectId`, positive `range`; valid for point and spot lights |
| `masonry.light.setSpotAngle` | `objectId`, `outerSpotAngle` in `(0,179)` and `innerSpotAngle` in `[0,outerSpotAngle]` |
| `masonry.light.setShadows` | `objectId`, `shadows` (`none`, `hard`, or `soft`) |
| `masonry.image.setTexture` | `objectId`, prepared texture `address` |
| `masonry.image.setSize` | `objectId`, positive `width`, positive `height`; also resizes its generated collider |
| `masonry.image.setFit` | `objectId`, `fit` (`stretch`, `contain`, or `cover`) |
| `masonry.image.setTint` / `masonry.image.tweenTint` | `objectId`, linear `{r,g,b}` `tint`, and tween fields for the tween variant |
| `masonry.image.setOpacity` / `masonry.image.tweenOpacity` | `objectId`, `opacity` in `[0,1]`, and tween fields for the tween variant |
| `masonry.image.setFaceCamera` | `objectId`, `enabled` |
| `masonry.text.setContent` | `objectId`, `text` |
| `masonry.text.setFont` | `objectId`, prepared TMP font `address` |
| `masonry.text.setSize` / `masonry.text.tweenSize` | `objectId`, positive `size`, and tween fields for the tween variant |
| `masonry.text.setColor` / `masonry.text.tweenColor` | `objectId`, `color`, and tween fields for the tween variant |
| `masonry.text.setAlignment` | `objectId`, horizontal (`left`, `center`, `right`, `justified`) and vertical (`top`, `middle`, `bottom`) alignment |
| `masonry.text.setWrapping` | `objectId` and optional positive `wrapWidth`; omission disables wrapping |
| `masonry.text.setRichText` | `objectId`, `enabled` |
| `masonry.text.setFaceCamera` | `objectId`, `enabled` |
| `masonry.animator.play` | `objectId`, `state`, optional nonnegative `layer` 0, optional `normalizedStartTime` in `[0,1]` 0, optional `waitMs` 0 |
| `masonry.animator.crossFade` | play fields plus positive `crossFadeMs` |
| `masonry.animator.setBool` | `objectId`, `parameter`, `value` |
| `masonry.animator.setInt` | `objectId`, `parameter`, 32-bit signed `value` |
| `masonry.animator.setFloat` | `objectId`, `parameter`, finite `value` |
| `masonry.animator.setTrigger` | `objectId`, `parameter` |
| `masonry.animator.setSpeed` | `objectId`, nonnegative `speed` |
| `masonry.particle.play` | `objectId`, optional `restart` false; recursively play the root and descendant particle systems |
| `masonry.particle.stop` | `objectId`, optional `clear` false; recursively stop the root and descendants |
| `masonry.particle.spawn` | prepared effect `address`; `location` union of `{ "gameObject": objectId }` or `{ "worldPosition": position }`; positive `lifetimeMs` |
| `masonry.audio.play` | prepared clip `address`, optional `volume` 1 in `[0,1]`, optional `pitch` 1 in `(0,3]`, optional `loop` false, optional `fadeInMs` 0 |
| `masonry.audio.stop` | `audioCommandId`, optional `fadeOutMs` 0 |
| `masonry.audio.setVolume` / `masonry.audio.tweenVolume` | `audioCommandId`, `volume` in `[0,1]`, and tween fields for the tween variant |
| `masonry.time.wait` | positive `durationMs`; always blocking |
| `masonry.operation.cancel` | `commandId`; cancel if running and otherwise succeed for any command already executed in this session |
| `masonry.input.setEnabled` | `enabled`; gate every pointer and key action |
| `masonry.input.setCamera` | enabled camera `objectId` |
| `masonry.input.setPointerEvents` | `objectId`, unique `events` drawn from `enter`, `exit`, `down`, `up`, `click` |
| `masonry.input.setGlobalKeys` | unique `keys` from the Rust protocol's W3C-code enum |

A tween variant accepts `durationMs` 0, `delayMs` 0, `easing` `inOutSine`, and
a `repeat` union that defaults to `"once"`. A bounded repeat uses
`{ "count": { "additionalTraversals": count, "mode": mode } }`; a forever
repeat uses `{ "forever": mode }`. The mode is `restart` or `pingPong`; `pingPong`
reverses each additional traversal; `restart` jumps to the captured start value
before moving forward. Delay applies only before the first traversal. A forever
operation must be nonblocking.
A zero-duration tween may not repeat. Easing is exactly `linear`, `inSine`,
`outSine`, `inOutSine`, `inQuad`, `outQuad`, `inOutQuad`, `inCubic`,
`outCubic`, `inOutCubic`, `inQuart`, `outQuart`, `inOutQuart`, `inQuint`,
`outQuint`, `inOutQuint`, `inExpo`, `outExpo`, `inOutExpo`, `inCirc`,
`outCirc`, `inOutCirc`, `inBack`, `outBack`, `inOutBack`, `inElastic`,
`outElastic`, `inOutElastic`, `inBounce`, `outBounce`, or `inOutBounce`.
Custom curves and easing parameters are not part of v1.

Standard cameras are controlled directly. Cinemachine, URP volumes and renderer
features, arbitrary shader properties, and components below a prefab root are
outside the core command union and require registered custom code.

## Assets and Addressables

Game content such as prefabs, textures, audio clips, and scenes cannot all be
loaded eagerly or referenced directly from Masonry's package. To instantiate
that content by the stable addresses supplied in MessagePack, Masonry relies on Unity
Addressables, introduced in the initial snapshot example. Masonry accesses
Addressables through an interface so tests can substitute in-memory asset
storage.

MessagePack refers directly to namespaced logical addresses. There is no separate
asset UUID manifest. Addresses are part of the content contract; they are not
CDN URLs, filesystem paths, or generated Unity GUIDs. Renaming one requires an
alias or a coordinated content update.

Each prepared entry includes its expected type. `kind` is exactly `scene`,
`prefab`, `particleEffect`, `material`, `texture`, `audioClip`, or `font`. An
address appears at most once in the set:

```text
{ "address": "mygame/pieces/knight", "kind": "prefab" }
```

Every snapshot contains the complete prepared set. An `assets.replaceSet`
command can change it later. Masonry loads and checks new
entries before releasing removed entries. A command cannot load an asset as a
side effect.

Example lifecycle:

1. An initial snapshot prepares `mygame/pieces/knight`.
2. A batch instantiates two knights.
3. A later batch destroys both knights, then replaces the prepared set without
   `mygame/pieces/knight`.
4. Removing the address before destroying the last live knight fails when the
   replace-set command runs.

The player contains one fixed catalog built with the coordinated release. That
catalog may refer to immutable HTTPS AssetBundles identified by the hashes in
the catalog. Masonry never checks for or installs a newer catalog. Addressables
may use its normal verified local cache; Masonry adds no download retry. A load
failure fails the snapshot or replace-set command that requested preparation.

Scene preparation downloads and verifies dependencies. Unity still constructs
and activates the scene during the scene-load command. That unavoidable Unity
work must be measured on representative scenes.

Masonry keeps each Addressables load handle—the Unity object used to retain and
later release a loaded asset—for as long as its address remains prepared.
Prepared prefabs are instantiated from that loaded asset instead of asking
Addressables to load again.

Temporary effect pooling is opt-in. A component on the effect prefab root
named `MasonryEffectPool` declares `maxInactiveCount` in `[1,128]`. Game
components that retain effect state implement `IMasonryPoolReset` with
`OnMasonryAcquire()` and `OnMasonryRelease()`; Masonry invokes every root
implementation in component order. Masonry also resets transform and recursively
stops and clears every ParticleSystem. A reset exception follows the spawning
command or running-operation failure rules. Masonry reuses inactive instances
through Unity's
[`ObjectPool<T>`](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Pool.ObjectPool_1.html),
clears the old object UUID, and resets transform and particle state. Arbitrary
prefabs are not pooled automatically because their scripts may retain state.

On Unity's low-memory warning, Masonry clears inactive pools and any cached
assets outside the prepared set. It does not release assets used by live
objects.

Unity's **IL2CPP** build pipeline compiles C# ahead of time when producing a
player, so Addressables and AssetBundles cannot add new C# types after that
player has been built. They can contain a prefab whose component type was
already compiled into the player. Custom code therefore ships in the game
project or another package installed through **Unity Package Manager (UPM)**
and is compiled into the player.
The [Distribution](#distribution) section describes that package boundary. See Unity's
[AssetBundle introduction](https://docs.unity3d.com/6000.0/Documentation/Manual/AssetBundlesIntro.html).

V1 pins `com.unity.addressables` to exactly 4.0.1. The manifest and lockfile
must contain that revision; floating package versions are forbidden.

## Pointer and keyboard input

To keep Unity from reporting input that the rules engine did not request, each
snapshot enables specific pointer events on specific objects and lists the
global keys enabled for the session. An object emits nothing unless its entry
enables that event. Pointer misses emit nothing in v1.

Masonry uses Unity's EventSystem, Input System UI module, and PhysicsRaycaster
with the enabled input camera. The closest physics hit blocks the ray. Masonry
walks upward from that collider to the nearest `MasonryIdentity`; if none is
found, or the identified root did not enable that event, it emits nothing and
does not search behind the collider. Primitive shapes receive their Unity
primitive collider only when they enable pointer events. An image receives a
centered BoxCollider matching its current width and height with depth 0.01
world units. Prefabs supply
authored colliders. Empty objects, cameras, lights, and world text receive no
automatic collider.

Mouse and touch use `masonry.pointer.enter`, `exit`, `down`, `up`, and `click`.
Each payload contains `objectId`, `pointerId`, screen position in pixels from
the bottom-left, and the world hit position; `exit` carries the last hit on the
object being exited. Button events additionally contain
`button` (`left`, `middle`, or `right`); touch uses `left`. Mouse pointer ID is
0; touch IDs are the stable positive IDs supplied by the Input System.

Within one input update, pointer IDs are processed in ascending order. For each
pointer, a target change emits `exit` for the old target and then `enter` for
the new target before a button transition. Press emits `down`. Release emits
`up` and then `click` only when press and release resolve to the same runtime
object and the press was not canceled. Moving away and back before release
still clicks. Disabling input, beginning a snapshot, losing application focus,
or destroying/deactivating the pressed object cancels the press without
synthetic `up` or `click` actions.

Actions are sent as soon as they occur, even while unrelated animations run.
For a hover scale effect, the rules engine may return a transform batch in the
action response. If that batch targets a property already being animated, it
must say whether to cancel or wait for the existing operation.

Enabled keys emit `masonry.key.down` and `masonry.key.up` once per physical
transition. Key repeat is suppressed. Identifiers are physical W3C
`KeyboardEvent.code` names, not layout-resolved text. V1 supports `Escape`,
`F1`-`F12`, `Backquote`, `Digit0`-`Digit9`, `Minus`, `Equal`, `Backspace`,
`Tab`, `KeyA`-`KeyZ`, `BracketLeft`, `BracketRight`, `Backslash`, `CapsLock`,
`Semicolon`, `Quote`, `Enter`, `ShiftLeft`, `ShiftRight`, `ControlLeft`,
`ControlRight`, `AltLeft`, `AltRight`, `MetaLeft`, `MetaRight`, `Comma`,
`Period`, `Slash`, `Space`, `ContextMenu`,
`Insert`, `Delete`, `Home`, `End`, `PageUp`, `PageDown`, all four arrow keys,
`PrintScreen`, `ScrollLock`, `Pause`, `NumLock`, `Numpad0`-`Numpad9`,
`NumpadDecimal`, `NumpadAdd`, `NumpadSubtract`, `NumpadMultiply`,
`NumpadDivide`, and `NumpadEnter`. Text, IME input, chords, and a held-key
stream are outside v1.

## Animation, Animator, particles, and audio

Commands describe animation in Masonry terms so MessagePack producers do not depend on
a C# library's enums or handles. The supported properties and complete tween
fields are defined in the core command contract above. Paths, custom curves,
parametric easing, and multi-revolution rotation are outside v1.

V1 pins `com.kyrylokuzyk.primetween` to exactly 1.4.11 through the npm registry
documented by [PrimeTween](https://github.com/KyryloKuzyk/PrimeTween). A
Masonry-owned adapter is the only code that calls PrimeTween. Every tween uses
unscaled time and is linked to its target so target destruction cancels it.

Animator commands target the root Unity Animator. Play and cross-fade specify
state name, layer, normalized start time, and an explicit `waitMs`. Animator
speed is separate persistent state. Masonry never infers group timing from
clips or transitions. A looping state uses zero wait and is nonblocking.

Particle commands play or stop a root ParticleSystem and all its descendant
systems, or spawn a prepared effect prefab at an object or world position. The
rules engine supplies the temporary effect lifetime. Root play has no inferred
end and must be nonblocking; spawned effects complete at `lifetimeMs` when
blocking.

Audio commands play prepared clips through Masonry-owned two-dimensional
AudioSources associated with the current input camera. V1 has no spatial audio,
world-position playback, object-attached playback, custom rolloff, or mixer
routing. A finite blocking play completes when the AudioSource stops; a loop
must be nonblocking. Stop with a fade completes when the fade finishes.
Changing the input camera re-associates live sources without restarting them.
Snapshots do not restart or resume audio.

## Custom C# code

Masonry never receives source code or an arbitrary method name in MessagePack. A game
compiles trusted handlers into the player and registers them during startup:

```csharp
Masonry.RegisterCommand<MyFlashPayload>(
    "mygame.character.flash",
    new FlashCommandHandler());
```

Game code depends on Masonry, while Masonry never takes a dependency on a
particular game's assembly. Explicit registration avoids scanning assemblies at
runtime and is safe for IL2CPP.

A custom handler runs on Unity's main thread. It receives cancellation,
logging, object lookup, prepared-asset lookup, and tween helpers. Masonry times
the call and converts exceptions thrown before it returns into batch failures.
The handler returns either completed or a tracked operation with completion and
cancellation. A blocking operation failure fails the waiting batch. A
nonblocking operation that fails after the batch advances reports
`masonry.operation.failed` with its session, batch, and command UUID; it cannot
retroactively stop commands. Invoking a handler occupies Unity's main thread
until the handler returns.

Handlers are trusted and may call Unity APIs directly. They must respond to
cancellation if they start work that outlives the call. Snapshots describe only
core Masonry-controlled content, so game code is responsible for cleaning up or
reconstructing any additional Unity state created by a custom handler.

Game code may emit a typed custom action through Masonry. It uses the configured
transport and receives a response like a pointer action. If it submits while a
response is being executed, the blocking transport call still occurs
immediately and its return is parsed immediately, but the parsed response waits
in the main-thread reentrancy deque until the current response or batch step
completes rather than being applied recursively. The outermost
response-processing call drains the deque before returning to polling. Custom
code does not call the native plugin directly.

V1 snapshots cover only the built-in Masonry content listed above. State owned
by custom handlers is outside the snapshot contract and must be reconstructed
or cleaned up by game code.

## Rust protocol types

The canonical built-in message and command definitions are Rust types in
`crates/masonry`. Rules engines depend on this crate and use its Serde types
directly. The domain declarations remain serialization-format neutral and do
not contain wire-only attributes or discriminator fields.

The public DTO fields remain directly constructible so rules engines can
assemble protocol messages incrementally. Strongly typed protocol UUIDs reject
the all-zero value while delegating their serialized representation to
`uuid::Uuid`. Numeric ranges, collection uniqueness, reference integrity, and
cross-field rules remain the responsibility of Rust-side validation rather than
fallible constructors on every DTO field.

A game defines custom command and action payloads as Rust types alongside its
rules engine. The custom payload types are explicit generic parameters; the core
crate does not provide a MessagePack value default or otherwise prescribe a dynamically
typed payload representation.

The wire encoding is MessagePack. Rust uses
`masonry::messagepack::{to_vec, from_slice}`, backed by `rmp-serde`'s compact
representation. Structs are arrays in declaration order, unit enum variants are
strings, data enum variants are single-entry maps, UUIDs are 16-byte binary
values in network byte order, and options are nil or their contained value.
There is no compression, typeless encoding, or generated projection.

Unity uses the handwritten, AOT-safe `Masonry.MessagePack` assembly backed by
MessagePack-CSharp 3.1.8. It reads and writes the same layout without annotating
the domain declarations. Game-owned command payloads, action payloads, and
error codes cross the boundary only through explicitly supplied
`IMessagePackFormatter<T>` implementations. Both implementations reject
unknown variants, malformed lengths, invalid UUIDs, overflow, truncation,
excessive nesting, and trailing bytes.

## Transports

Masonry reaches the same engine interface through a native production plugin or
a synchronous localhost HTTP development server. Both expose connect, generic
client-message submission, and nonblocking poll. Every successful connect,
submit, or nonempty poll returns the same `masonry.response` shape. Client
submissions block and happen immediately on Unity's main thread. Every returned
response is parsed there synchronously. When response processing is idle,
Masonry applies the parsed messages immediately. If a nested submission returns
while response or batch work is running, it appends the parsed return to a main-thread
reentrancy deque. The outermost processing call finishes the current work and
drains that deque in call order before returning. The deque exists only to
prevent recursive application; there is no background parser, cross-frame scheduler queue, or
response resequencing.

### Native plugin

Calling a rules engine compiled as a native plugin uses this exact C ABI. All
structs use the platform C ABI with normal alignment; `uint64_t` and `int32_t`
have their standard widths. `MasonryEngine` is incomplete and opaque.

```c
typedef struct MasonryEngine MasonryEngine;
typedef struct { uint8_t *data; uint64_t length; } MasonryBuffer;

int32_t masonry_engine_create(
    MasonryEngine **out_engine, MasonryBuffer *out_error);
void masonry_engine_destroy(MasonryEngine *engine);
int32_t masonry_connect(
    MasonryEngine *engine, const uint8_t *messagepack, uint64_t length,
    MasonryBuffer *out_buffer);
int32_t masonry_submit(
    MasonryEngine *engine, const uint8_t *messagepack, uint64_t length,
    MasonryBuffer *out_buffer);
int32_t masonry_poll(
    MasonryEngine *engine, MasonryBuffer *out_buffer);
void masonry_buffer_free(MasonryBuffer buffer);
```

Status values are `0` (`OK`), `1` (`NO_MESSAGE`), `2`
(`INVALID_ARGUMENT`), `3` (`ENGINE_ERROR`), and `4` (`PANIC`). Unknown status
values are fatal ABI errors. `OK` returns one MessagePack-encoded response;
`NO_MESSAGE` is valid only for poll and returns `{NULL,0}`; error statuses
return diagnostic UTF-8 text when available. `{NULL,0}` is the only empty
buffer. Every nonempty output is freed exactly once through
`masonry_buffer_free` in a C# `finally` block. Input bytes are borrowed only for
the duration of the call. Output allocation capacity is not part of the ABI.
All output pointers are required. Create sets `*out_engine` to null before work;
on `OK` it returns a nonnull handle and `{NULL,0}`, and on failure it leaves the
handle null. Connect, submit, and poll always initialize their output to
`{NULL,0}` before work. Destroying a null handle and freeing `{NULL,0}` are
no-ops; any other invalid pointer is caller error.

Creation produces one opaque engine instance. A Masonry client supports one
live instance, reuses it across explicit reconnects, and destroys it at player
shutdown. A repeated connect starts a new session, clears pending old-session
responses, and retains authoritative game state. Unity invokes connect,
submit, poll, and destroy serially on its main thread; calls on one handle are
non-reentrant. The engine may run internal workers and enqueue responses.
Poll returns immediately with one response or `NO_MESSAGE`; Unity polls exactly
once per frame while the session is active.

No native callback enters Unity. No C# exception or native panic crosses the
ABI; `masonry-native` catches Rust panics and returns `PANIC`. Unity validates
pointers and lengths before copying or allocating. The required library base
name is `masonry_rules`: `masonry_rules.dll` on Windows,
`libmasonry_rules.dylib` on macOS, `libmasonry_rules.so` on Android, and
`__Internal` for statically linked iOS exports.

V1 builds macOS universal (`arm64` and `x86_64`), Windows `x86_64`, iOS device
`arm64`, and Android `arm64-v8a`. Other architectures and platforms are outside
v1. `crates/masonry` contains the canonical Serde types and MessagePack codec.
The Unity package contains an independent handwritten implementation of the
same wire contract. `crates/masonry-native` contains
the ABI types, engine trait, panic containment, and reusable Rust adapter. A
supported native rules engine links these crates rather than independently
reimplementing the wire format or C ABI.

The host-architecture macOS player proof established the packaging procedure
without requiring a permanent smoke harness. The fixture dylib is staged at
`Assets/Plugins/macOS/libmasonry_rules.dylib` and marked compatible with the
macOS standalone target using the plugin importer's `AnyCPU` setting. A
host-only CPU label caused Unity's universal player build to omit the dylib;
`AnyCPU` packaged the host-built artifact correctly. Because that importer
setting does not prove which Mach-O slices are present, the proof separately
checked both the staged and packaged dylibs for the host architecture.

The packaged application must be tested without `DYLD_LIBRARY_PATH`, an Editor
search path, or a repository-root library copy. The player executable is
located through the app bundle's `CFBundleExecutable` value rather than inferred
from the requested `.app` name. With those constraints, the production C#
transport successfully completed create, connect, submit, poll, and destroy
against the Rust fixture, decoded recognizable MessagePack after each protocol
operation, and finished with no outstanding native output buffers.

### Localhost HTTP

Development HTTP is synchronous and mirrors the ABI:

- `POST /connect` accepts `masonry.connect` and returns a response.
- `POST /messages` accepts any client-message union member and returns a
  response, including an empty `messages` list when there is no immediate work.
- `GET /poll` returns immediately with one response or HTTP 204 when no message
  is ready. It is not a long poll and never runs through a Unity background
  request.

Unity uses the same exactly-once-per-frame poll schedule for HTTP and native
transports.

Requests and successful bodies use `application/msgpack`. HTTP 400
reports an invalid request and HTTP 500 reports an engine error; either may
include diagnostic text. Other status codes are transport failures. The client reuses one
persistent localhost connection and blocks Unity's main thread exactly like a
native call. Connect timeout is 2 seconds; submit and poll timeout is 100 ms.
Timeout, refusal, or connection failure stops the session without retry. The
host may explicitly reconnect after repairing or restarting the development
server.

## Failure and explicit reconnection

Masonry moves through these runtime states:

```text
Stopped --host connect/reconnect--> AwaitingSnapshot -> ApplyingSnapshot -> Running
Running --replacement snapshot--> ApplyingSnapshot -> Running
AwaitingSnapshot | ApplyingSnapshot | Running --fatal session error--> Stopped
```

### AwaitingSnapshot

Input is disabled while Masonry connects. A
valid snapshot for the current session moves the client to ApplyingSnapshot.
Messages for another session are discarded.

### Running

Input and new batches are accepted. If a command fails, Masonry stops the
remaining commands in that batch and reports `masonry.batch.failed`. Earlier
commands are not rolled back. Masonry remains in Running unless the rules engine
sends a replacement snapshot or a session-fatal failure stops it.

### ApplyingSnapshot

Masonry waits for required asynchronous loads, replaces its controlled Unity
content directly on the main thread, then resumes input. The partially replaced
world may be visible during replacement.

For owned operations, cancellation means:

- Tweens stop without firing completion behavior.
- Temporary particle and audio instances stop and return to their pools or are
  destroyed.
- Pending Addressable handles are released when safe.
- Masonry cannot react while a synchronous custom handler blocks Unity's main
  thread. Cancellation takes effect only after the handler returns or throws.

Asynchronous work started by custom code receives cancellation, but Masonry
cannot force game code to honor it.

Masonry does not roll back commands that ran before a batch failure. If the
rules engine sends a replacement snapshot, that snapshot becomes the correction
boundary.

Tests log the structured error and fail the current test. In Editor Play Mode,
Masonry logs the failure and throws on the main thread after reporting it.
Production reports the batch and command UUIDs, stops the failed batch, and does
not throw.

Transport failure, timeout, malformed response MessagePack, an unknown top-level
response message, or snapshot failure stops the session. Masonry disables input,
cancels owned operations, discards queued responses, and makes no automatic
retry. The native engine handle remains alive. The host may explicitly call
reconnect—for example after restarting the development HTTP server—which calls
connect again, creates a new session UUID, and requires a new snapshot.

Mobile resume stops the existing session and requires the host to reconnect.
The new snapshot does not replay sounds or animations from suspension.

The v1 hard limits are:

| Resource | Maximum |
|---|---:|
| UTF-8 bytes in one connect request, submitted client message, or response | 16,777,216 |
| UTF-8 bytes in one string | 65,536 |
| Response messages in one response | 256 |
| Parallel command groups in one batch | 256 |
| Commands in one parallel command group | 4,096 |
| Loaded content scenes | 32 |
| Game objects in one snapshot | 100,000 |
| Game-object hierarchy depth | 256 |
| Prepared assets | 16,384 |
| Queued responses awaiting main-thread application | 256 |
| Duration, delay, wait, effect lifetime, or fade | 86,400,000 ms |
| Finite tween repeat count | 10,000 |

The Rust domain model applies the 4,096-command limit to each parallel command group;
it defines no separate aggregate command-count limit for a batch. The response
byte limit still bounds the complete serialized batch.

Limits are fixed rather than game-configurable. Exceeding one is a validation
failure under the batch, snapshot, or session rules appropriate to that record.

## Runtime profiling and logging

Masonry does not implement a cooperative per-frame work budget. Every response
deserializes and applies in order on Unity's main thread after the 16 MiB limit
check, and native response memory is freed as soon as parsing finishes.
Asynchronous asset and scene loads may naturally span frames, but Masonry does
not split ordinary parsing, validation, or Unity object construction into a
cross-frame job system.

One repeatable development-player scenario exercises a pointer action, an
immediate response, and a tween while recording profiler markers and
allocations. It is a diagnostic smoke check, not a hardware-specific release
gate. Additional benchmarks are added in response to measured problems.

Coarse profiler markers cover:

- Masonry frame and poll work
- Action serialization and native or HTTP transport
- Response deserialization
- Response application
- Custom handlers

If a frame exceeds 16.67 ms and Masonry did work, Masonry emits one structured
slow-frame record with its measured contribution and relevant IDs, without
claiming it was the only cause. The profiler is the primary tool for deeper
diagnosis; more granular markers or scheduling are added only in response to a
measured problem.

Returned responses are applied in call order. Transport submissions are
blocking, immediate, and serialized on Unity's main thread. Response processing
finishes each return before the next frame's poll. A nested return waits in
the main-thread reentrancy deque only until the current response or batch step
finishes, then the outermost processing call drains it without recursive
application.

Logging uses one structured interface with Unity console output by default.
Games may add file, crash-reporting, or telemetry outputs. Records include
severity, stable event name, relevant session/action/batch/command/object IDs,
duration and payload bytes when relevant. Empty polls, successful high-frequency
pointer events, and raw MessagePack are not logged by default.

## Testing and release checks

Release checks cover these observable behaviors:

- A command failure reports `masonry.batch.failed`, stops the remaining commands
  in that batch, and does not roll back earlier commands.
- Group 3 starts after Group 2's blocking move, not after its nonblocking sound.
- A duplicate batch has no second effect for the rest of the session.
- Applying a snapshot produces the world described by the rules engine without
  resuming interrupted animations.
- Destroyed Unity objects are removed from the UUID registry; a later command
  targeting one reports a clear batch failure.
- An asset cannot be used before preparation or removed while a live object uses
  it.
- Child collider input emits the game object's UUID.
- Native response memory is freed before the transport returns, including after
  validation and managed-copy failures.
- HTTP connect, submit, and nonblocking poll requests execute synchronously on
  Unity's main thread and obey their fixed timeouts.
- HTTP 204 and native `NO_MESSAGE` have identical poll behavior.
- A custom-handler exception reports a batch failure and stops the rest of that
  batch.
- A late nonblocking custom-operation failure reports
  `masonry.operation.failed` without retroactively failing its batch.
- Pointer target transitions emit exit then enter before button transitions;
  click requires matching press and release object UUIDs.
- Key down/up uses physical W3C code names and suppresses repeats.
- Replacement snapshots recreate every game object, retain only matching
  prepared handles and scene instances, and process later batches afterward.
- A malformed response, transport failure, or snapshot failure stops the
  session and never retries automatically; explicit reconnect starts a new
  session on the existing native handle.

Generated protocol fixtures drive end-to-end Unity tests through both
transports. A test-only instant animation mode applies final values immediately
while preserving group order.

One host-platform smoke player verifies native linking and the common connect,
snapshot, action, batch, poll, and output-freeing path. The release does not
require a hardware or IL2CPP smoke matrix for every supported target.

Content checks are test helpers rather than an editor product.
They verify Addressables addresses and types, required root components, custom
handler registration, and protocol fixtures against the current project.

## Distribution

Masonry ships as a reusable package inside a Unity project that supplies
integration scenes and a small performance smoke fixture:

```text
Cargo.toml                    Rust workspace manifest
crates/masonry/               Canonical Rust protocol types
crates/masonry-native/        Rust engine adapter and native ABI
Packages/com.masonry.client/   Reusable package
Assets/                        Integration scenes and performance smoke fixture
docs/                          Design and installation documentation
```

Public C# types use the `Masonry` namespace. V1 consumers install a tagged Git
revision that pins the Rust crates and matching Unity package together. A game
keeps its handlers and other C# code in its own assembly or UPM package, so
upgrading Masonry does not require merging a fork. The Rust protocol model
remains independent of format-generated artifacts.

## Appendix: lessons from Dreamtides

Dreamtides is an existing Unity/Rust game whose native Unity bridge served as a
starting point for Masonry. Three parts of that implementation constrain this
design:

- Its command sequence already uses ordered groups whose members are launched
  without waiting for one another to finish.
- It has both MessagePack over a native C interface and a localhost development server.
- Its C# plugin wrapper allocates a fixed 10 MB response array for each call.
  Masonry instead returns an exact native-owned response buffer.
