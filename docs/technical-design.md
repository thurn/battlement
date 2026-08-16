# Masonry Technical Design

Status: v1 design proposal

Unless a section is marked **Open question**, it records an agreed design
direction. Names in JSON examples are illustrative until the schema and C#
generation prototype is complete.

## Masonry in one minute

Masonry is a Unity rendering and input client for turn-based games. A separate
program owns the rules and the authoritative game state, including facts such
as “piece P is on square B.” This program is the **rules engine**: it tells
Masonry what to display and receives player input from Masonry. In production,
Unity will usually reach the rules engine through a native plugin. During
development, the rules engine may run as a localhost HTTP service.
The [Transports](#transports) section describes both arrangements.

When Unity first connects, the rules engine sends a **snapshot**: a complete
description of the Unity content Masonry should construct, including loaded
scenes, runtime objects, transforms, cameras, lights, and enabled input
settings. A snapshot lets Masonry construct the current world without replaying
everything that happened earlier.
The [Snapshots and scene replacement](#snapshots-and-scene-replacement) section defines exactly what a snapshot contains and how Masonry applies one.

Each initial connection or reconnect begins a **session**, identified by a
**UUID** (universally unique identifier).

A normal turn follows this loop:

1. Unity connects to the rules engine.
2. The rules engine sends the current snapshot.
3. The player clicks a Unity object. Masonry sends an **action**—a JSON record
   of player input—to the rules engine. It includes the object's UUID, a globally
   unique identifier assigned by the rules engine.
   The [Pointer and keyboard input](#pointer-and-keyboard-input) section lists the supported actions.
4. The rules engine decides whether the click is legal and returns **commands**,
   JSON instructions that tell Masonry how to change Unity. A **batch** is one
   ordered delivery of commands. Masonry does not make the game-rule decision.
5. Masonry executes the commands against Unity. Some commands may animate their
   changes over time.

A batch can divide its commands into **groups**. Masonry considers groups in
list order, but it launches commands within one group without waiting for
earlier commands in that group to finish. Unity calls still occur one at a time
on Unity's main thread. A **blocking command** prevents the next group from
starting until that command finishes; a nonblocking sound can continue while
the next group begins.
The [Batch and group timing](#batch-and-group-timing) section gives a complete timing example.

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
| Produce valid JSON | Deserialize commands and report execution failures |
| Prevent duplicate actions | Prevent duplicate command batches |

Masonry does not infer game rules, choose legal moves, or inspect arbitrary C#
properties. If a decision can live in the rules engine without harming responsiveness,
it belongs there.

## v1 scope

V1 focuses on turn-based 3D worlds and deliberately leaves general UI and
real-time simulation for later. This table separates the included product
surface from deferred work; later sections define the detailed commands.

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
| Native, localhost HTTP, and recorded-file transports | Production network transports |

World-space TextMesh Pro text is treated as a 3D object, not as a general UI
system.

## End-to-end protocol example

A single click now illustrates the complete message flow. Field names may
change during prototyping, but the behavior should not.
The [Schemas for built-in and game-specific messages](#schemas-for-built-in-and-game-specific-messages) section defines how those fields become a language-neutral contract.

To keep messages compact, examples omit properties set to these v1 defaults:

- Runtime objects are active. Missing local position, rotation, and scale use
  zero, the identity quaternion, and one respectively.
- A sole content scene is primary. A snapshot with multiple content scenes must
  identify one as primary.
- Cameras are enabled and perspective, with a 60-degree field of view, 0.3 near
  clipping, and 1,000 far clipping.
- Pointer actions use the left mouse button and pointer ID 0.
- Batches start immediately and do not request completion notification.
- Commands are blocking unless marked otherwise.
- Animations have zero delay, zero duration, `easeInOut` easing, and no repeats.
  Zero duration applies the final value immediately.
- Animator commands use layer 0, no cross-fade, normalized start time 0, and
  speed 1. Audio uses volume 1, pitch 1, and does not loop.
- Optional lists are empty.

Fields without a safe default remain required. Examples include asset
addresses, target object UUIDs, and `onConflict` when another animation already
controls the same property.

### 1. Connect

Before sending game state, the rules engine needs to know which Unity build has
connected. Unity therefore reports its environment and every game-specific
command type compiled into the build. Each game-specific type is implemented by
a **custom handler**, a C# class such as the one behind
`mygame.character.flash`.
The [Custom C# code](#custom-c-code) section explains registration, execution, and failure handling.

```json
{
  "message": "connect",
  "platform": "macOS",
  "unityVersion": "6000.5.3f1",
  "screen": { "width": 2560, "height": 1440 },
  "colorSpace": "linear",
  "customCommandTypes": ["mygame.character.flash"]
}
```

A production connect message may also include persistent-data and
streaming-assets paths for a native rules engine. Masonry does not send a
protocol-version field.

### 2. Initial snapshot

To build the initial Unity world, the rules engine starts a session and sends
its first snapshot. Unity's
[Addressables](https://docs.unity3d.com/Packages/com.unity.addressables@latest)
system loads scenes and assets identified by stable strings at runtime. The
snapshot declares every **prepared asset**: an Addressable scene, prefab,
material, texture, audio clip, or effect that Masonry must load and type-check
before any command may use it. Preparing assets in advance prevents an ordinary
command from unexpectedly starting an asset load.
The [Assets and Addressables](#assets-and-addressables) section covers their lifetime.

```json
{
  "message": "snapshot",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "preparedAssets": [
    { "address": "mygame/boards/forest", "kind": "scene" },
    { "address": "mygame/pieces/knight", "kind": "prefab" },
    { "address": "mygame/audio/piece-move", "kind": "audioClip" },
    { "address": "mygame/effects/dust", "kind": "particleEffectPrefab" }
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
      "sceneId": "ca64d87d-33d9-4a19-be6e-597035312d01",
      "pointerEvents": ["enter", "exit", "click"]
    },
    {
      "objectId": "8ff6f71c-6a74-41cf-8826-0e364abf9f97",
      "kind": "camera",
      "sceneId": "ca64d87d-33d9-4a19-be6e-597035312d01",
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
  "globalKeys": ["escape"]
}
```

### 3. Player action

With the initial state visible, clicking the knight produces this action:

```json
{
  "actionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "type": "masonry.pointer.click",
  "payload": {
    "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
    "worldHit": { "x": 0.1, "y": 0.4, "z": 0.0 }
  }
}
```

### 4. Action response

Because the rules engine can decide this move quickly, it returns the commands in the
same call that carried the click. A batch returned directly in an action's
response is an **action-response batch**; delayed work instead arrives through
polling, as shown next. Returning immediate work this way avoids an extra poll
before a hover or click effect can begin.

Masonry supplies built-in command types such as `masonry.transform.move` and
`masonry.audio.play`. Each built-in type is a **core command** implemented by
Masonry. The [Command execution and failures](#command-execution-and-failures)
section describes how command errors are reported.

```json
{
  "message": "actionResponse",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "batches": [
    {
      "batchId": "c07f0804-6455-40a6-b0f0-5d1a3d87ea81",
      "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
      "causedByActionId": "28dfd8ca-4908-4bb8-86d7-5775d271fced",
      "notifyOnCompletion": true,
      "groups": [
        {
          "commands": [
            {
              "commandId": "7bbcb27e-f75b-4c63-bf86-ad1b0f6ee2cd",
              "type": "masonry.transform.move",
              "payload": {
                "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                "worldPosition": { "x": 4, "y": 0, "z": 2 },
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
                "atObjectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
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

```json
{
  "message": "pollResponse",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "batches": [
    {
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

```json
{
  "type": "masonry.batch.failed",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "batchId": "0cb9b6d9-b6ee-4105-8afe-ee4ba5105b24",
  "commandId": "4a52e41e-0b60-4e00-8bc0-588165037b6f",
  "errorCode": "asset_not_prepared",
  "message": "mygame/effects/missing-spark was not in the prepared asset set"
}
```

Successful completion is reported only when `notifyOnCompletion` was true. In
that case, the message identifies the session and batch:

```json
{
  "type": "masonry.batch.completed",
  "sessionId": "94fa422b-301d-442d-b9a7-10ea54318e78",
  "batchId": "c07f0804-6455-40a6-b0f0-5d1a3d87ea81"
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
transport preserves that order when delivering them to Masonry.

Once Masonry starts a tween or effect, it tracks that running work as an
**operation**. An operation has a UUID when a later command may cancel it. If
two operations target the same object property, the newer command must say
whether it cancels the older operation or waits for it. Omitting that choice is
an error. A snapshot cancels running operations, then applies the snapshot
values directly.

For example:

```json
{
  "type": "masonry.transform.scale",
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

All Masonry animations use unscaled time. They continue when Unity's
`Time.timeScale` is zero.

### Session and duplicate checks

Every snapshot, action, batch, completion, and failure carries the session UUID.
A reconnect creates a new session UUID and clears both sides' duplicate-ID
histories. A message from another session is never executed.

| Incoming message | Masonry behavior |
|---|---|
| Duplicate batch UUID in the current session | Ignore it and log the duplicate |
| Different session UUID | Discard it and log an error |

## Batch and group timing

Groups are ordered by their blocking work, not by every effect they start.

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
mode. A conflicting write fails unless the new command explicitly says to
cancel or wait for the old operation.

## Command execution and failures

Masonry deserializes the batch envelope and enforces basic safety limits before
scheduling it. These checks include:

- Required fields and finite numeric values
- Configured size and count limits
- Session and duplicate batch UUID

When the batch and session IDs are available, an error at this stage is also
reported as `masonry.batch.failed`. An envelope too malformed to identify is
logged as a protocol error and ignored.

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
Masonry creates a **persistent container** for runtime objects that must survive
content-scene changes. Each additively loaded content scene receives a **scene
container** for its Masonry-controlled objects.

An individually targetable object is a **runtime object root**. Each root has a
UUID and a `MasonryIdentity` component that registers it, unregisters it on
`OnDestroy`, and lets a pointer hit on a child collider find the root. Objects
authored directly into a content scene load and unload with that scene, but v1
cannot target them individually.

Masonry therefore owns only objects it creates and scenes it loads. It never
scans or deletes unrelated objects in the bootstrap scene:

```text
Bootstrap scene
  MasonryRunner
  Masonry persistent container
    runtime camera [UUID, MasonryIdentity]

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

Object and scene UUIDs come from the rules engine. A runtime object UUID is not reused
after destruction within the same session. Static content uses Addressables
addresses rather than UUIDs. Command and action kinds use namespaced strings.

Masonry keeps every executed batch UUID for the session and ignores a duplicate
after logging it. The rules engine keeps every action UUID for the session. An exact
duplicate returns the cached action response or reports no new work; the
action is never applied again. Reusing one action UUID with different JSON is an
error. This avoids an undefined retry window for commands with visible side
effects.

## Snapshots and scene replacement

A snapshot completely describes the Unity content that Masonry should
construct. It contains:

- Session UUID
- Complete prepared asset set
- Loaded content scenes and the primary scene
- Runtime objects, their scene, parent, kind, active state, and local transform
- Camera, light, material, image, text, and interaction values
- The stable Unity Animator state, persistent bool/int/float parameters, and
  speed that should be visible after recovery

It does not contain custom-handler state, one-time sounds, particles, a hover
pulse, or progress through an attack animation.

Applying a large snapshot may take more than one frame. Masonry caps splittable
work with a **per-frame Masonry scheduling budget**, a provisional time limit
for tasks such as creating many objects.
The [Performance and logging](#performance-and-logging) section explains how benchmarks will replace that provisional limit. Snapshot application then proceeds as follows:

1. Stop accepting input and discard queued batches from the old state.
2. Keep the last complete world visible while downloading assets and loading
   any new additive scenes in an inactive state.
3. Check the snapshot without changing the visible world.
4. Hide Masonry's containers and every root GameObject in Masonry-loaded content
   scenes, apply object changes over as many frames as needed, switch the
   primary scene, and remove obsolete scenes.
5. Reveal the new complete world and resume input.

This is not full double buffering. During step 4 the game-owned loading screen
may be visible while Masonry-controlled content is hidden. A normal scene-change
batch can keep the old scene visible while a new additive scene loads, then cut
over after it is ready.

Once Masonry begins a batch that replaces the primary scene, pointer input on
the outgoing scene is disabled until cutover. Otherwise the player could click
an old object while its replacement is being constructed. Persistent
bootstrap objects may remain interactive only when their snapshot entries say
so.

One bootstrap scene persists for the life of the client. Content scenes must be
Addressable and load additively. Exactly one loaded content scene is primary.
V1 cannot load the same scene address twice at once.

Every runtime object belongs to one content scene or to the bootstrap scene.
Objects may only be parented within the same scene. Unloading a content scene
also removes its authored scene objects and every Masonry runtime object in that
scene. Those authored objects are considered part of the scene Masonry was
asked to load; Masonry still does not touch unrelated bootstrap objects.

Masonry loads and unloads authored objects inside a content scene, but v1
commands cannot target them individually. Targetable objects must be created by
Masonry or instantiated as prefabs.

## Runtime object types

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
runtime object also destroys its runtime-object descendants unless they were
reparented first.

Material support is intentionally small. Masonry may assign a prepared material
to all renderer slots or one slot on a supported root renderer. It does not edit
shader properties, keywords, or arbitrary material values. The built-in image
quad is a specific exception backed by a Masonry-owned URP material.

## Planned core commands

V1 needs built-in commands for the following parts of a 3D world. Exact names
and payloads depend on the schema prototype.

| Area | Commands needed for v1 |
|---|---|
| Assets | Replace the complete prepared asset set |
| Scenes | Load, unload, choose primary scene |
| Objects | Create, instantiate prefab, destroy, activate/deactivate |
| Hierarchy | Reparent within one scene |
| Transform | Set or tween position, rotation, and scale |
| Renderer | Assign prepared material by slot |
| Camera | Enable, transform, projection, field of view, clipping, clear settings |
| Light | Type, transform, color, intensity, range, spot angle, shadows, enabled state |
| Image quad | Texture, size, fitting, tint, opacity, face-camera option |
| World text | Text, font, size, color, alignment, wrapping, rich-text toggle, face-camera option |
| Animator state | Play or cross-fade to a stable state |
| Animator parameters | Set persistent bool, int, or float values |
| Animator trigger | Fire a trigger |
| Animator speed | Set playback speed |
| Particles | Play/stop root system; spawn temporary effect prefab |
| Audio | Play/stop prepared clip; spatial mode, volume, pitch, loop, fade |
| Timing | Wait, cancel identified operation, blocking/nonblocking groups |
| Input | Enable pointer events on objects and discrete global keys |
| Custom | Dispatch to an explicitly registered game handler |

Standard cameras are controlled directly. Cinemachine rigs may live in game
prefabs but require custom code. URP-specific volumes and renderer features are
also authored or custom rather than part of the core protocol.

## Assets and Addressables

Game content such as prefabs, textures, audio clips, and scenes cannot all be
loaded eagerly or referenced directly from Masonry's package. To instantiate
that content by the stable addresses supplied in JSON, Masonry relies on Unity
Addressables, introduced in the initial snapshot example. Masonry accesses
Addressables through an interface so tests can substitute an in-memory asset
store.

JSON refers directly to namespaced logical addresses. There is no separate
asset UUID manifest. Addresses are part of the content contract; they are not
CDN URLs, filesystem paths, or generated Unity GUIDs. Renaming one requires an
alias or a coordinated content update.

Each prepared entry includes its expected type:

```json
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

Scene preparation downloads and verifies dependencies. Unity still constructs
and activates the scene during the scene-load command. That unavoidable Unity
work must be measured on representative scenes.

Masonry keeps each Addressables load handle—the Unity object used to retain and
later release a loaded asset—for as long as its address remains prepared.
Prepared prefabs are instantiated from that loaded asset instead of asking
Addressables to load again.

Temporary effect pooling is opt-in. A component on the effect prefab root
declares the maximum inactive count and reset hook. Masonry reuses inactive
instances through Unity's
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

**Open question:** Which Addressables package revision and remote-catalog rules
should Masonry support?

**Recommendation:** Pin a revision that passes the Unity 6.5 platform tests.
Make the initial release work offline with the catalog built into the player;
add remote catalog updates only after their failure and rollback behavior has
separate tests. The `@latest` documentation link above does not pin a package.

## Pointer and keyboard input

To keep Unity from reporting input that the rules engine did not request, each
snapshot enables specific pointer events on specific objects and lists the
global keys enabled for the session. An object emits nothing unless its entry
enables that event. Pointer misses emit nothing in v1.

Pointer raycasts use one camera UUID named by the snapshot. A hit on a child
collider walks upward to the nearest `MasonryIdentity`. Built-in shapes receive
colliders only when they have pointer events. Prefabs provide their own
colliders. World text does not receive an automatically resizing collider.

Mouse and touch use the same action types. Each event includes `pointerId`, so
separate touches remain distinguishable. V1 reports enter, exit, down, up, and
click. Exact event ordering and multi-touch behavior are acceptance criteria for
the input prototype rather than assumptions in this document.

Actions are sent as soon as they occur, even while unrelated animations run.
For a hover scale effect, the rules engine may return a transform batch in the
action response. If that batch targets a property already being animated, it
must say whether to cancel or wait for the existing operation.

**Open question:** Should pointer input use Unity's EventSystem plus
PhysicsRaycaster, or a small raycast service built on the new Input System?

**Recommendation:** Prototype the built-in EventSystem and PhysicsRaycaster
first because they already model pointer enter, exit, down, up, and click. Keep
them only if they pass the recorded mouse, touch, child-collider, and event-order
tests without adding UI behavior to Masonry.

## Animation, Animator, particles, and audio

Commands describe animation in Masonry terms so that JSON producers do not need
to know which C# tween library Unity uses. The protocol therefore defines its
own easing names, durations, and repetition options instead of exposing a
library's enums or handles. V1 can tween object transforms, camera field of view
or orthographic size, light intensity/color, audio volume, world-text
color/size, and image-quad color/opacity.

A tween can provide duration, delay, a Masonry easing name, an optional cubic
Bezier curve, repeat count, and restart or back-and-forth repetition. Infinite
repetition must be nonblocking. V1 paths use sequential move commands rather
than splines.

[PrimeTween](https://github.com/KyryloKuzyk/PrimeTween) is the current candidate.
Masonry would install its published package from an npm registry that Unity
Package Manager can read; it would not copy PrimeTween source. The game manifest
would therefore include PrimeTween's registry and package dependency.

**Open question:** Should the v1 tween adapter use PrimeTween or
[LitMotion](https://github.com/annulusgames/LitMotion)?

**Recommendation:** Start with PrimeTween. Build the same small adapter with
LitMotion before committing, and switch only if it performs or integrates
better in Unity 6.5 and IL2CPP tests. Compare allocations, cancellation on
object destruction, group completion, infinite loops, and custom curves.

Animator commands target the root Unity Animator. They specify state name,
layer, cross-fade time, normalized start time, and speed. The rules engine supplies
the time that later groups should wait; Masonry does not guess from clips and
transitions. A looping `Idle` state is nonblocking.

Particle commands play or stop a root ParticleSystem, or spawn a prepared
effect prefab at an object or world position. The rules engine supplies both the
effect lifetime and any wait before the next group.

Audio commands play prepared clips globally, at a position, or attached to an
object. Snapshots do not restart or resume audio. Advanced mixer setup stays in
authored Unity content or custom code.

## Custom C# code

Masonry never receives source code or an arbitrary method name in JSON. A game
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
the call and converts exceptions into batch failures. Invoking a handler always
occupies Unity's main thread until the handler returns. If the handler starts
work that continues afterward, `blocking` says whether the next group waits for
that work.

Handlers are trusted and may call Unity APIs directly. They must respond to
cancellation if they start work that outlives the call. Snapshots describe only
core Masonry-controlled content, so game code is responsible for cleaning up or
reconstructing any additional Unity state created by a custom handler.

Game code may emit a typed custom action through Masonry. It uses the configured
transport and receives action-response batches like a pointer action. Custom code
does not call the native plugin directly.

Describing Unity content created by custom handlers in a snapshot is deferred.
V1 snapshots cover only the built-in Masonry content listed above.

## Schemas for built-in and game-specific messages

Masonry needs a language-neutral definition of every built-in message and
command. That definition is the **core schema**, a JSON Schema generated in the
separate `masonry-rust` repository from optional Rust types. Those types use
Serde for JSON encoding and Schemars for schema generation. `masonry-rust`
initially generates JSON Schema Draft 7 because Dreamtides—an existing
Unity/Rust game whose native bridge motivated Masonry—already generates that
schema format with Schemars. Rust is not required at runtime or for another
rules engine implementation.

This Unity repository commits three things together:

1. The generated core JSON Schema.
2. The generated core C# payload types.
3. The exact `masonry-rust` Git commit used to generate them.

CI regenerates the files and fails on a difference.

A game keeps a separate schema for its custom command and action payloads. For
example, `mygame.character.flash` belongs to the game schema, not the Masonry
schema. Its build generates `MyFlashPayload` into the game assembly and
registers that type with the handler. Masonry parses the fields shared by every
command, while the registered handler deserializes its own payload type. The
registration API
rejects duplicate command strings. Namespaced prefixes reduce accidental
collisions; the game project decides which package owns a prefix.

For a particular game build, the public JSON contract is the pinned core schema
plus that game's schema. Any language may produce JSON matching those artifacts.

### Compatibility without a protocol version

Masonry is independently installable, but producer and client are not promised
to be independently deployable after a breaking schema change. A game chooses
and tests one Masonry package revision together with its rules engine build.

Masonry's JSON parser ignores unknown properties on recognized command types.
Adding an optional property is backward compatible. Adding a required property, removing
an accepted alias, changing a field's meaning, or adding a required command
handler requires a coordinated producer/client release.

An unknown command type fails the batch. It is not skipped, because later
commands may depend on it. Package semantic versions describe package releases;
the client and rules engine do not exchange protocol versions during connection
to choose between schemas.

### C# generation prototype

**Open question:** Which tool should generate core C# payload types from the
Schemars-style JSON Schema?

**Recommendation:** Use Quicktype as the comparison baseline because Dreamtides
already uses it to generate C# from its Schemars schema.
The [End-to-end protocol example](#end-to-end-protocol-example) section supplies the messages that every candidate must generate. Add a fixture schema covering required and optional fields, UUIDs, and the `type` plus `payload` command shape. Compare these generators:

- [Quicktype](https://github.com/glideapps/quicktype)
- [NJsonSchema](https://github.com/RicoSuter/NJsonSchema)
- [Corvus.JsonSchema](https://github.com/corvus-dotnet/Corvus.JsonSchema)

A generator passes this prototype only if its output compiles in Unity 6.5 and
IL2CPP on every v1 platform, preserves required/optional fields and UUIDs,
handles `type` plus `payload`, produces readable errors, and meets measured
parsing and allocation targets.
The [Testing and release checks](#testing-and-release-checks) section defines the additional release checks it must pass. The same prototype compares Newtonsoft JSON with a UTF-8-oriented parser.

Full JSON Schema validation runs in CI, producer tests, recorded-message tests,
and an explicitly enabled diagnostic mode. The normal Unity path performs
generated deserialization and the basic envelope checks described above; it
does not run a general schema validator on every message.

## Transports

Masonry can reach the same rules engine through a native plugin, a development HTTP
server, or a recorded JSON file. A common transport interface hides those
delivery details from command processing. V1 includes all three
implementations.

### Native plugin

Calling a rules engine compiled as a native plugin requires a stable **C application
binary interface (C ABI)**: a small set of functions and memory-layout rules
that both C# and the plugin understand. Masonry's adapter defines functions for
connect/action, `has_pending_messages`, poll, and freeing a response.
`masonry-rust` provides the reference implementation, but another language may
implement the same interface.

Short computations run synchronously inside the action function, which may
return batches in the same call. Expensive work, such as AI search, runs on a
background thread owned by the rules engine and becomes available through poll.
Native plugins never call back into Unity.

Masonry calls `has_pending_messages` once per Unity frame. It calls poll only
when work exists and stops starting polled work when it reaches its per-frame
Masonry scheduling budget.

Each successful plugin call returns a UTF-8 response buffer containing pointer,
length, and capacity. The plugin owns that memory until Masonry calls
`free_response` in a `finally` block. This replaces Dreamtides' fixed 10 MB C#
response allocation and avoids repeating an action merely to discover response
size. If the chosen
JSON parser cannot read unmanaged UTF-8, Masonry copies the exact response into
a rented managed buffer.

A native call blocks Unity until it returns. It does not guarantee same-frame
application: parsing or the per-frame Masonry scheduling budget may defer
execution. Slow calls are logged against the provisional latency target
described below.

No C# exception or Rust panic crosses the C ABI. Functions return a status code
and optional error JSON. Lengths are checked before copying or allocating.

**Open question:** What exact C ABI should native rules engines implement?

**Recommendation:** Prototype the native-owned pointer/length/capacity response
described above on every v1 platform, then freeze the C struct layout, integer
widths, exported function names, library names, ownership of error memory, and
iOS `__Internal` behavior.

### Localhost HTTP

Development HTTP preserves the native plugin's request-response timing:

- `POST /connect` blocks Unity's main thread until it returns a session and
  snapshot or reaches a short, configurable timeout.
- `POST /actions` blocks Unity's main thread until it returns action-response
  batches or reaches a short, configurable timeout. Short rules computations
  therefore complete in the same call, as they do through the native plugin.
- `GET /poll` is the exception: it runs asynchronously off the main thread and
  waits for a delayed batch or a timeout. A long poll never blocks Unity's main
  thread.

The HTTP client reuses persistent localhost connections. Expensive rules work
must not extend a blocking action request; the rules engine moves that work to
its background processing and exposes the result through poll, matching the
native transport. After a poll finishes, Masonry opens the next one. A poll
response may be processed on a later Unity frame.

Blocking an action request avoids adding a frame solely because of asynchronous
HTTP scheduling, but it does not guarantee same-frame application. Response
parsing, command checks, or the per-frame Masonry scheduling budget may still
defer execution, just as they can for a native call. Slow and timed-out HTTP
calls are logged because they directly extend the Unity frame.

### Recorded-file transport

For deterministic tests and Unity content work, a recorded-file transport reads
snapshots and batches from JSON files and records emitted actions. This path
does not require Rust or a native build. Recorded sessions become regression
fixtures.

## Failure and recovery

Masonry moves through these runtime states:

```text
Startup or reconnect -> AwaitingSnapshot -> ApplyingSnapshot -> Running
Running -> Recovering -> AwaitingSnapshot -> ApplyingSnapshot -> Running
Running -> ApplyingSnapshot -> Running  (rules engine sends a replacement snapshot)
```

### AwaitingSnapshot

Input is disabled while Masonry connects or waits for a recovery snapshot. A
valid snapshot for the current session moves the client to ApplyingSnapshot.
Messages for another session are discarded.

### Running

Input and new batches are accepted. If a command fails, Masonry stops the
remaining commands in that batch and reports `masonry.batch.failed`. Earlier
commands are not rolled back. Masonry remains in Running unless the rules engine
sends a replacement snapshot or a connection-level failure requires recovery.

### Recovering

Masonry stops input, cancels operations it owns, discards queued batches, and
requests a snapshot. Actions are not emitted while Masonry's controlled world
may be incomplete.

After a disconnect, the last fully applied world may remain visible while the
snapshot arrives. Masonry hides its controlled content when it cannot maintain
the connection or while it applies a replacement snapshot, leaving the
game-owned loading or connection UI visible.

If snapshot retrieval fails, Masonry remains in AwaitingSnapshot. The common
transport layer shared by the native and HTTP adapters logs each failure and
owns the retry schedule.

**Open question:** How long should recovery retries continue, and how quickly
should their delay grow?

**Recommendation:** Retry until the client exits or reconnects. Start after 250
ms, double the delay after each failure, and cap it at 5 seconds. Keep these
values configurable for a game that needs different connection behavior.

Custom code may ignore cancellation, which is one reason recovery cannot
promise rollback of arbitrary handler side effects.

### ApplyingSnapshot

Masonry hides its controlled containers, applies the snapshot within its
per-frame Masonry scheduling budget, then reveals the new world and resumes
input. The new snapshot replaces Masonry's controlled Unity content.

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

Disconnect and mobile resume also enter recovery. Resume does not replay every
sound or animation queued while the app was suspended. The rules engine may send an
intentional recap after the fresh snapshot.

Configurable limits cover JSON bytes, commands per batch, objects per snapshot,
string length, prepared assets, animation duration, and queued batches.

## Performance and logging

Masonry must sustain 60 FPS during normal input, command dispatch, and
animation. Scene activation and a single complex prefab can call Unity code
that Masonry cannot split across frames, so representative content needs
separate performance tests.

Until those tests establish real limits, these values remain hypotheses rather
than release gates:

- At most 4 ms of work that Masonry can schedule in one frame.
- Native action-response hover processing below 2 ms at the 95th percentile
  (p95) on target desktop hardware and 5 ms p95 on target mobile hardware.
- Move large-message parsing off the main thread if a representative parse
  exceeds 4 ms.

Performance prototypes must replace these numbers after choosing target
devices, representative payloads, warm-up, sample count, and measurement
interval. They must also measure representative scene activation. If activation
drops a frame, the project needs content limits or an intentional loading cut.

Hover latency is measured from the start of Masonry's input callback until the
returned batch has been decoded, checked, and queued for Unity execution. It
includes action serialization, time spent in the native rules engine, response
parsing, and basic envelope checks. It does not include display scanout or the
duration of the tween itself.

Masonry spreads work it controls across frames. For example, it may instantiate
part of a large snapshot, yield when the current frame's budget is exhausted,
and continue next frame while the controlled world remains hidden.

Each of these stages is timed:

- Action serialization
- Native or HTTP request
- Response copy and deserialization
- Basic envelope checks
- Unity calls made by Masonry
- Custom handlers
- Poll count, queue depth, and Masonry work per frame

Unity Profiler markers cover the same stages. If one timed Unity call exceeds
its threshold, Masonry logs the profiler stage, Unity API call, related IDs, and
duration. If a frame exceeds 16.67 ms and Masonry did work, the log lists
Masonry's measured contribution without claiming it was the only cause.
Repeated warnings are rate-limited.

Large-message background parsing is conditional on the benchmark. If enabled,
the worker parses only plain C# data and places results into a first-in,
first-out queue consumed on Unity's main thread. No Unity API runs on the worker.

Logging uses one structured interface with Unity console output by default.
Games may add file, crash-reporting, or telemetry outputs. Records include
severity, stable event name, relevant session/action/batch/command/object IDs,
duration, payload bytes, and queue depth when applicable. Frequent pointer
events are trace-only. A small in-memory buffer retains recent warnings and
errors for crash reports. Raw JSON logging is opt-in and size-limited.

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
- Child collider input emits the runtime object root's UUID.
- Native response memory is freed on success, parse error, and exception.
- HTTP connect and action requests block the Unity main thread until response or
  timeout, while HTTP long polls never block the Unity main thread.
- HTTP long-poll ordering matches native poll ordering.
- A custom-handler exception reports a batch failure and stops the rest of that
  batch.

Recorded protocol traces drive end-to-end Unity tests. A test-only instant
animation mode applies final values immediately while preserving group order.

Platform checks compile and run IL2CPP smoke tests on macOS, Windows, iOS, and
Android. Performance fixtures cover hover actions, large snapshots, concurrent
tweens, pooled effect bursts, representative prefabs, scene activation, and a
sustained poll queue. Reports include p50, p95, and p99 rather than averages
alone.

Content checks are initially test helpers rather than a polished editor tool.
They verify Addressables addresses and types, required root components, custom
handler registration, and recorded JSON against the current project.

## Distribution

Masonry ships as a reusable package inside a Unity project that supplies
integration scenes and performance fixtures:

```text
Packages/com.masonry.client/   Reusable package
Assets/                        Integration scenes and performance fixtures
docs/                          Design and installation documentation
```

Public C# types use the `Masonry` namespace. Consumers initially install a
tagged Git revision. A game keeps its handlers and other C# code in its own
assembly or UPM package, so upgrading Masonry does not require merging a fork.

## Appendix: lessons from Dreamtides

Dreamtides is an existing Unity/Rust game whose native Unity bridge served as a
starting point for Masonry. Three parts of that implementation constrain this
design:

- Its command sequence already uses ordered groups whose members are launched
  without waiting for one another to finish.
- It has both JSON over a native C interface and a localhost development server.
- Its C# plugin wrapper allocates a fixed 10 MB response array for each call.
  Masonry instead returns an exact native-owned response buffer.
