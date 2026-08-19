# Masonry v1 implementation plan

Status: implementation companion to `docs/technical-design.md`

This plan implements the approved v1 contract without expanding or revising its
scope. The technical design remains normative. If this plan and the technical
design disagree, the technical design wins.

## Decisions and starting point

The repository contains the canonical format-neutral Rust DTOs in
`crates/masonry`, the corresponding handwritten C# domain model, and complete
MessagePack codecs and interoperability fixtures for both languages. The v1 UPM
package and Unity 6000.5.8f1 reference-project baseline are also established.
Implementation now starts at the public host boundary and builds runtime behavior
on the completed MessagePack protocol foundation.

The following decisions were resolved while preparing this plan:

- Unity 6000.5.8f1 is the reference editor version. The technical design and
  `ProjectSettings/ProjectVersion.txt` are aligned with the existing CI lookup.
- Tollgate remains the CI and promotion mechanism. This project does not add a
  second CI system.
- Protocol DTOs are handwritten and are not projected through a code generator.
- The localhost implementation consists of the Unity HTTP client and a
  test-only server. Masonry does not ship a reusable Rust HTTP server crate.
- `masonry-native` exposes ordinary adapter functions and a small export macro.
  The macro only binds a game engine's constructor to the fixed C symbols; ABI,
  buffer, status, and panic logic stays in normal testable functions.
- The public host surface is a scene-authored `MasonryRunner` with explicit
  `Connect`, `Reconnect`, and `Stop` entry points, serialized native/HTTP
  transport configuration, and injectable public transport and asset-storage
  interfaces.
- Tests are black-box Unity Edit Mode tests wherever practical. The test
  assembly references only the package's public runtime assembly and does not
  receive `InternalsVisibleTo`. A small shared harness may absorb public host API
  changes, while individual tests assert MessagePack at the boundary and visible Unity
  behavior. Play Mode tests are avoided. Unity lifecycle behavior that cannot
  be made deterministic in Edit Mode belongs in focused player smoke fixtures,
  not a broad Play Mode suite.
- A task normally changes roughly 150–350 handwritten production and test
  lines. Test-heavy contract tasks may be larger. Vendored binaries, Unity
  `.meta` files, scene serialization, and lockfile churn do not count toward the
  estimate.

## Testing conventions used by every task

Each feature task includes its own black-box acceptance tests rather than
leaving testing to a final phase. Tests construct a runner through public host
APIs, supply MessagePack through a fake public transport, and inspect outcomes that a
game can observe: scene contents, GameObject/component state, emitted client
MessagePack, transport calls, and structured log records. Tests do not call command
executors, registries, validators, or batch-scheduling internals directly.

The shared Edit Mode harness provides:

- A host builder that creates an isolated bootstrap scene, runner, fake
  transport, fake Addressables store, and captured logger through public APIs.
- A deterministic way to drive one Masonry frame and advance the
  injected clock without relying on Editor wall time or MonoBehaviour `Update`.
- Typed Rust fixture builders for protocol values and MessagePack fixtures.
- Helpers that query Unity's public scene hierarchy and components rather than
  Masonry registries.
- Teardown that stops the session, destroys created Unity objects, unloads test
  scenes, releases fake handles, and reports leaked work.

The design's test-only instant-animation mode is exercised through public host
configuration. Timing-specific tests use the deterministic clock and normal
animation adapter. This keeps the suite in Edit Mode while still verifying
group ordering, delays, repeats, cancellation, and blocking completion.

Every task is complete only when its focused tests pass through
`./scripts/ci.sh`, formatting and analyzers pass, and the package has no new
warnings in Unity 6000.5.8f1.

## Dependency overview

Implementation proceeds in the following dependency waves. Work inside a wave
may be parallelized when its listed prerequisites are complete.

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01 | Public host and test boundary |
| 2 | 02–10 | Rust adapter, transports, macOS player smoke, lifecycle, and failures |
| 3 | 11–17 | Owned scenes, assets, identities, and all snapshot object kinds |
| 4 | 18–20 | Direct replacement snapshots |
| 5 | 21–24 | Ordered batches, operations, conflicts, and tweens |
| 6 | 25–31 | Complete core command execution |
| 7 | 32–34 | Pointer/keyboard input and custom code |
| 8 | 35–39 | Cross-cutting contract coverage, content checks, and release |

Expected handwritten production-plus-test size is shown below. The upper end
of a daggered task is test-heavy; its production implementation should still be
only a few hundred lines.

| Task | Expected lines | Task | Expected lines |
|---:|---:|---:|---:|
| 01 | 250–350 | 21 | 250–350 |
| 02 | 250–350 | 22 | 250–350 |
| 03 | 200–300 | 23 | 300–450† |
| 04 | 250–350 | 24 | 300–450† |
| 05 | 250–350 | 25 | 350–450† |
| 06 | 150–250 | 26 | 200–300 |
| 07 | 250–350 | 27 | 300–400 |
| 08 | 150–250 | 28 | 300–450† |
| 09 | 150–250 | 29 | 250–350 |
| 10 | 200–300 | 30 | 300–450† |
| 11 | 250–350 | 31 | 300–400 |
| 12 | 300–450† | 32 | 350–500† |
| 13 | 250–350 | 33 | 250–350 |
| 14 | 300–400 | 34 | 300–450† |
| 15 | 200–300 | 35 | 300–450† |
| 16 | 300–400 | 36 | 350–500† |
| 17 | 250–350 | 37 | 250–400 |
| 18 | 200–300 | 38 | 100–200 |
| 19 | 150–250 | 39 | 150–250 |
| 20 | 200–300 | | |

## Wave 1: public host and test boundary

### [DONE] Task 01 — Build the public Edit Mode host harness boundary

**Prerequisites:** none.

Define the minimum public seams needed by a host and black-box tests:
`IMasonryTransport`, `IMasonryAssetStorage`, structured logging, clock and
frame-driving input, immutable runner options, and native/HTTP serialized
configuration.
Create the scene-authored `MasonryRunner` shell with public `Connect`,
`Reconnect`, `Stop`, and deterministic per-frame behavior. Production uses its
MonoBehaviour callbacks; Edit Mode tests drive the same public `RunFrame` entry
point explicitly.

Create the shared test harness under `Assets/Tests/Editor` or a package test
assembly. The test assembly references only the public runtime assembly. Do not
add friend-assembly access, reflection into private state, or direct internal
executor construction.

**Black-box acceptance:** a test creates and tears down an isolated runner,
records transport calls and logs, advances a fake clock, and proves that no
Masonry object or fake handle leaks. No protocol behavior is implemented in
this task beyond enough wiring to exercise the host shell.

## Wave 2: native adapter, transports, and session plumbing

### [DONE] Task 02 — Add the reusable Rust engine adapter

**Prerequisites:** none.

Create `crates/masonry-native` with the engine trait, C-layout opaque handle and
buffer types, exact status constants, and ordinary functions implementing
create, destroy, connect, submit, poll, and buffer free against a concrete
engine factory. Keep unsafe code localized to the ABI boundary. Normal adapter
logic owns serial calls, UTF-8 conversion, response serialization, error text,
output initialization, and the one-live-instance lifecycle.

The engine trait exposes the three protocol operations and permits internal
workers to enqueue poll responses. A repeated connect reuses the engine,
clears pending old-session responses, and lets the implementation preserve its
authoritative game state.

**Black-box acceptance:** Rust integration tests call the adapter through its C
function signatures using a fake engine and verify every status, output
initialization, null no-op, input borrowing rule, serialized response, and
repeated-connect behavior.

### **[DONE]** Task 03 — Finish ABI exports, panic containment, and buffer ownership

**Prerequisites:** Task 02.

Add the smallest practical macro that takes a concrete engine constructor and
emits the seven required unmangled symbols. Each generated wrapper immediately
delegates to ordinary functions from Task 02. Catch panics at every exported
entry point, prevent Rust or C# exceptions from crossing the boundary, map
unknown/internal failures to the fixed status values, and retain the allocation
metadata required to free every nonempty output exactly once without exposing
capacity in the C ABI.

**Black-box acceptance:** compile a test `cdylib` using only a trait
implementation plus the macro; load/call all symbols; force panics and invalid
MessagePack; verify `PANIC`, diagnostic text where available, `{NULL,0}` rules, double
operation avoidance, and allocation balance. Tests exercise exported symbols,
not macro expansion details.

### **[DONE]** Task 04 — Implement the Unity native transport

**Prerequisites:** Tasks 01 and 03.

Implement the platform library-name mapping and P/Invoke declarations for the
fixed ABI. Copy binary inputs only for the synchronous call, validate output
pointers and lengths before managed allocation, and free every nonempty native
buffer in `finally` after successful copies and after validation or managed-copy
failures.
Map fixed/unknown status values to transport results without applying responses
inside the transport.

The transport owns one engine handle, reuses it across reconnects, serializes
main-thread calls, and destroys the handle at runner shutdown. Poll is immediate
and exposes `NO_MESSAGE` distinctly from an empty response.

**Black-box acceptance:** use a compiled ABI fixture library through the public
runner, then assert connect/submit/poll behavior and native allocation counts.
Include success, 16 MiB boundary, malformed MessagePack, engine error, panic,
unknown status, and managed-exception cases.

### **[DONE]** Task 05 — Implement the synchronous localhost HTTP transport

**Prerequisites:** Task 01.

Implement main-thread blocking `POST /connect`, `POST /messages`, and
non-long-polling `GET /poll` over one persistent localhost connection. Enforce
MessagePack content type, the 2-second connect timeout, 100 ms submit/poll timeout,
HTTP 204 poll semantics, 400/500 diagnostics, maximum body size, and fatal
handling for other statuses, refusal, timeout, or connection failure. Do not
add retries or background Unity web requests.

Add a test-only local server fixture in the reference project; do not create a
published Rust HTTP-server crate.

**Black-box acceptance:** runner tests hit the real loopback fixture and verify
methods, routes, headers, response ordering, persistent connection reuse,
timeouts, status mapping, 204 parity with native `NO_MESSAGE`, and no automatic
retry.

### **[DONE]** Task 06 — Prove native plugin round-trip in a macOS player

**Prerequisites:** Tasks 04 and 05.

Add a repeatable host-architecture smoke build that compiles the Rust fixture
engine as `libmasonry_rules.dylib`, stages it through Unity's macOS native-plugin
import path, and builds a Development macOS `.app`. The player must use the
production C# native transport rather than a test replacement or Editor library
search-path injection.

Launch the built player from the smoke command and have a small bootstrap
component perform connect, submit, and poll calls. Validate recognizable
MessagePack response data after each Rust round-trip, destroy the engine, and
verify that the fixture reports no outstanding native output buffers. Emit a
machine-readable success marker and fail on a bounded timeout, player crash,
missing dylib, unresolved symbol, invalid response, or allocation leak. Keep
the generated plugin and player build untracked and clean them up after the
check. This early smoke targets the current Mac architecture; universal binary
assembly remains part of release packaging.

**Black-box acceptance:** one command from a clean checkout builds the Rust
`cdylib`, packages it inside a real macOS Unity player, launches that player,
and proves C# → Rust → C# connect/submit/poll behavior plus buffer and engine
cleanup. The check resolves the library from the built `.app`, not from
`DYLD_LIBRARY_PATH`, the repository root, or an Editor-only process.

### **[DONE]** Task 07 — Implement runner lifecycle and explicit reconnect

**Prerequisites:** Tasks 01, 04, 05, and 06.

Implement `Stopped`, `AwaitingSnapshot`, `ApplyingSnapshot`, and `Running`
transitions behind the public runner. Connect disables input and requires the
first current-session response message to be a snapshot. Reconnect stops the
old session's work, retains the native handle, calls connect again, and requires
a new session. Build the connect payload from platform, Unity version, screen,
and every registered custom command type; include absolute persistent-data and
streaming-assets paths only for native connect. Stop, mobile resume, and player
shutdown cancel owned work and discard queued responses as specified.
Application focus loss cancels pointer presses but does not by itself stop the
session.

Do not automatically retry any failure. Keep the state representation private;
public behavior is observed through host methods, emitted submissions, logs,
and input availability.

**Black-box acceptance:** scripted transports drive every legal transition and
fatal edge, including wrong-session messages, missing initial snapshot,
explicit reconnect on the same native handle, and mobile-resume stop. Tests do
not inspect the internal state enum.

### **[DONE]** Task 08 — Process responses on the main thread

**Prerequisites:** Task 07.

Parse every successful connect, submit, and nonempty poll response on Unity's
main thread and reject responses larger than 16 MiB before parsing. When no
response is running, apply the parsed messages immediately. If a transport call
returns while a response or batch step is already running, append the parsed
return to a main-thread reentrancy deque. The outermost response-processing
call finishes the current work, then drains that deque in call order before
returning. The deque exists only to prevent recursive application; it is not a cross-frame
scheduler queue, background parsing pipeline, or response-resequencing layer.

**Black-box acceptance:** return varied response sizes from connect, submit,
and poll; verify synchronous parsing, call-order application, the 16 MiB limit,
and nonrecursive deque draining when a custom submission returns more work.

### **[DONE]** Task 09 — Add polling and performance instrumentation

**Prerequisites:** Tasks 07 and 08.

Poll the active transport exactly once per Unity frame. Process a returned
response through the existing main-thread response processor; do not add a loop
that polls repeatedly in the same frame or a scheduler that slices ordinary
response work across frames.

Use the structured logger already exposed by the host, with Unity console output
by default. Record failures and slow Masonry frames with a stable event name,
relevant IDs, duration, and payload bytes when applicable. Do not emit routine
success logs for empty polls or high-frequency pointer events.

Add coarse Unity Profiler markers for frame/poll work, serialization and
transport, response parsing, response application, and custom handlers. These
markers establish where time is spent before introducing more scheduling
machinery.

**Black-box acceptance:** use a fake transport, clock, logger, and Unity's public
profiler recorder APIs to verify one poll per frame, ordered response
application, the coarse markers, and slow-frame records without asserting
internal method calls.

### **[DONE]** Task 10 — Serialize and route failure submissions

**Prerequisites:** Tasks 07 and 08.

Create the common failure path for `masonry.batch.failed` and
`masonry.operation.failed`. Preserve session, batch, and command/operation IDs;
map core error codes exactly; bound diagnostic messages; submit failures
immediately through the active transport; and hand any returned response to the
same response processor and reentrancy deque. Separate recoverable
batch/operation failure from session-fatal transport, top-level MessagePack,
unknown-message, and snapshot failure.

Apply Editor behavior only at the outer host boundary: report first, then throw
on the main thread when the design requires it. Production reports without
throwing.

**Black-box acceptance:** fixture failures produce exact client MessagePack and
stop only the intended batch/session. Returned corrections run after the current
work rather than recursively, and logging contains the stable IDs and error
code.

## Wave 3: world ownership and object construction

### **[DONE]** Task 11 — Create containers, identity registration, and lookup behavior

**Prerequisites:** Tasks 01 and 07.

Create the persistent bootstrap container and per-content-scene containers.
Implement `MasonryIdentity` with a `System.Guid`, registration/unregistration,
Unity-aware destroyed-object handling, nearest-ancestor lookup for child hits,
session-lifetime no-reuse histories, and object/scene lookup errors. Never scan,
rename, reparent, or delete unrelated bootstrap objects.

**Black-box acceptance:** construct and destroy identified objects through the
public runner, inspect only Unity hierarchy/components, and verify UUID
uniqueness, descendant identity resolution, destroyed-reference cleanup, and
unrelated-object survival.

### Task 12 — Implement prepared assets and reference accounting

**Prerequisites:** Tasks 01 and 11.

Implement the Addressables-backed asset-storage adapter and prepared-set manager
for the seven fixed kinds. Load/type-check additions before changing the active
set, retain one handle per prepared address, atomically commit replacement,
release removals only when unused, reuse matching handles across snapshots,
and prohibit command-side implicit loads. Track live object, scene, audio, and
effect references needed to report `asset_in_use` accurately.

Handle load failure, cancellation, low-memory cleanup of inactive pools and
unprepared caches, and fixed prepared-asset/count/string limits. Do not update
catalogs or add retries.

**Black-box acceptance:** fake handles expose load/release counts while runner
MessagePack drives prepare, use, replacement, failure, cancellation, and low-memory
flows. Assert only public store calls, visible objects, submitted errors, and
handle balance.

### Task 13 — Implement additive content-scene ownership

**Prerequisites:** Tasks 11 and 12.

Load prepared scenes additively under unique scene UUID/address pairs, create
scene containers, enforce 32 scenes and one instance per address, track one
primary content scene, and unload only Masonry-owned content. Implement primary
cutover without conflating it with the input camera. Destroy registered scene
objects on unload and preserve authored scene objects until their scene unloads.

Support reuse only when scene UUID and address both match. A changed UUID at the
same address forces the mutation-phase unload/reload path.

**Black-box acceptance:** an in-memory scene-store fixture and small authored
test scenes verify additive load/unload, primary selection, duplicate-address
rejection, persistent-container survival, authored-object lifetime, and no
bootstrap collateral damage.

### Task 14 — Construct empty, primitive, and prefab objects

**Prerequisites:** Tasks 11–13.

Create empty GameObjects, Unity primitive shapes, and instances of prepared
prefabs. Apply parent-scene selection, topological parent, `activeSelf`, local
transform/defaults, identity, and initial pointer-event collider policy. Check
prefab root supported-component counts and never target authored children.
Primitive colliders exist only when pointer events require them.

**Black-box acceptance:** snapshot/object MessagePack creates every base kind under
primary, named, persistent, and parented placements. Tests inspect public Unity
state and cover defaults, hierarchy, inactive parents, duplicate IDs, wrong
asset kinds, missing prefabs, and unsupported component counts.

### Task 15 — Construct image objects and their owned material

**Prerequisites:** Tasks 12 and 14.

Create the centered image quad with its Masonry-owned URP material, prepared
texture, positive world size, stretch/contain/cover UV behavior, RGB tint,
separate opacity, optional face-camera behavior, and a 0.01-depth centered
BoxCollider when pointer events are enabled. Resize the collider with the image
and preserve texture filtering/wrapping.

**Black-box acceptance:** MessagePack fixtures create and mutate representative aspect
ratios; tests inspect mesh bounds/UVs, material-visible values, collider size,
linear tint/opacity, and billboard output relative to rolled and coincident
cameras.

### Task 16 — Construct text, camera, and light objects

**Prerequisites:** Tasks 12 and 14.

Create world-space TMP text with prepared font, size/color/alignment/wrapping/
rich-text defaults and billboard behavior. Create standard Camera and Light
components with the complete snapshot state and defaults, including projection,
clipping, clear behavior, light type/range/spot/shadows, and separate component
enabled state. Do not create automatic colliders for these kinds.

**Black-box acceptance:** literal records for every projection, clear mode,
light type, alignment, and wrapping mode produce observable component state;
invalid ranges, missing fonts, and disabled input-camera candidates fail with
the specified error class.

### Task 17 — Apply materials and stable Animator snapshot state

**Prerequisites:** Tasks 12, 14, and 16.

Assign prepared materials to unique zero-based root-renderer slots on primitives
and prefabs through `sharedMaterials`; support assign-all and individual slots,
excluding image/text renderers. Apply prefab-root Animator stable state, layer,
normalized start time, persistent bool/int/float parameters, and speed without
restoring triggers or playback progress.

**Black-box acceptance:** prefab and primitive fixtures expose renderer and
Animator state through Unity components. Cover multiple slots, duplicate/out-of-
range slots, missing/multiple root components, wrong asset kinds, and exact
snapshot defaults.

## Wave 4: direct replacement snapshots

### Task 18 — Validate a complete snapshot before replacement

**Prerequisites:** Tasks 11–17.

Validate decoded snapshot DTOs before replacing the current world: hard limits,
canonical/unique IDs, prepared addresses and types, primary-scene and input-
camera requirements, parent-scene resolution, acyclic same-scene hierarchy,
object-kind requirements, root components, material slots, and scene-transition
constraints. Compute only the topological object order needed by application;
do not build a second immutable representation of the snapshot.

Keep this implementation private and test only through submitted snapshots.

**Black-box acceptance:** malformed snapshots fail before object replacement,
stop the session as required, and report diagnostics. Count-boundary fixtures
are generated in tests without committing enormous MessagePack.

### Task 19 — Apply snapshot assets and scenes directly

**Prerequisites:** Task 18.

On a valid current-session snapshot, stop accepting input, cancel operations,
replace the prepared set, and reconcile additive scenes in snapshot order.
Reuse matching handles and scene UUID/address pairs, and release resources on
failure. Do not maintain a staged copy of the old and new worlds. A later
snapshot waits for the current replacement like any other later message.

**Black-box acceptance:** delayed fake loads prove input gating, handle and
scene reuse, failure cleanup, ordered snapshot processing, and balanced handles.

### Task 20 — Replace snapshot objects directly

**Prerequisites:** Task 19.

Destroy existing Masonry-created objects, recreate the snapshot objects in
topological order, select the primary scene, and resume configured input when
finished. Once required asynchronous loads are ready, run this replacement work
directly to completion on the main thread rather than slicing it across frames.
Do not hide containers, preserve the old world, stage a second world, or provide
atomic reveal or rollback. A failure stops the session and may leave a partially
replaced world visible. Later messages wait until replacement finishes.

**Black-box acceptance:** replacement fixtures verify recreation by Unity
instance identity, scene/handle reuse rules, final hierarchy and values, no
resumed operation/audio/particle progress, fatal replacement failure, and
subsequent batch ordering.

## Wave 5: batch scheduling and operations

### Task 21 — Admit batches and enforce session/duplicate/start rules

**Prerequisites:** Tasks 10 and 20.

Validate response/session agreement, batch/group/command limits, nonempty lists,
IDs, common command fields, and duplicate batch UUIDs before scheduling. Retain
every executed batch UUID for the session; log and ignore exact duplicates.
Implement `start: now` and the admission dependency for
`afterEarlierBlockingWork` without waiting for nonblocking work.

Malformed records with enough identity/order information report a batch
failure; unorderable responses stop the session.

**Black-box acceptance:** serialized responses cover both start modes, wrong
sessions, duplicates after long intervals, each hard limit, and malformed common
fields. Observable command effects and captured failure MessagePack establish behavior.

### Task 22 — Execute ordered groups and propagate batch failure

**Prerequisites:** Task 21.

Execute groups in list order, launch commands within a group in command-list
order, and advance only after that group's blocking commands complete. A batch
completes when all group-blocking work completes even if nonblocking work lives.
Resolve targets when each command runs rather than simulating future commands.
Stop remaining commands on the first failure without rollback.

Propagate `earlier_batch_failed` through consecutive dependent batches while
allowing the next `start: now` batch to execute.

**Black-box acceptance:** use visible object creation and deterministic waits to
verify the design's 0/300/800 ms timeline, create-then-target behavior,
partial effects after failure, parallel launch ordering, and dependent failure
chains.

### Task 23 — Track operations, conflicts, waits, and cancellation

**Prerequisites:** Task 22.

Track running and pending operations by the starting command UUID, retain every
executed command UUID for the session, and index the exact canonical property
conflict keys. Implement default cancel-from-current-value, `onConflict: wait`,
blocking/nonblocking wait semantics, infinite-wait rejection, explicit cancel
no-op for known completed commands, `unknown_command` for never-executed IDs,
and cancellation on object destruction, reparent, snapshot, or session stop.

Late failure of a nonblocking operation emits `masonry.operation.failed`; a
blocking failure fails its waiting batch.

**Black-box acceptance:** overlapping MessagePack commands exercise every shared and
independent key, queued waits, snapshot/destruction cancellation, infinite
operations, known-completed cancel, late failure, and current displayed start
values through visible component state and emitted MessagePack.

### Task 24 — Adapt PrimeTween and implement Masonry tween semantics

**Prerequisites:** Task 23.

Create the sole PrimeTween adapter. Map all fixed easing names, unscaled time,
delay-once, zero duration, bounded/forever repeats, restart jumps, ping-pong
traversals, completion/cancellation, and target destruction. Normalize
quaternions and use shortest-arc spherical interpolation. Provide the specified
test-only instant mode while preserving scheduler completion order.

**Black-box acceptance:** deterministic-clock Edit Mode tests send transform
tweens and assert intermediate/final Unity values, easing samples, repeat counts,
delay behavior, time-scale independence, cancellation without completion,
shortest rotation, forever/blocking rejection, and instant-mode group order.

## Wave 6: core command families

### Task 25 — Execute asset, scene, object, renderer, and input-control commands

**Prerequisites:** Tasks 12–14 and 21–24.

Implement `assets.replaceSet`; scene load/unload/set-primary; object create,
destroy, active, and reparent; renderer material assignment; input enable;
input-camera selection; pointer-event selection; and global-key selection.
Apply every command at execution time with the ownership, preparation, hierarchy,
scene, collider, and conflict/cancellation rules already established.

**Black-box acceptance:** batches combine these commands so later commands
observe earlier immediate effects. Cover scene unload restrictions, input cutover,
new-ID history, descendant destruction, `worldPositionStays`, null-parent scene
container behavior, live-asset removal, and renderer slot errors.

### Task 26 — Execute transform commands

**Prerequisites:** Tasks 23–25.

Implement local/world position, local/world rotation, and local scale set/tween
variants. Share local/world position and rotation conflict keys, capture current
displayed values after cancellation, normalize quaternion inputs, reject
billboard-controlled rotation, and cancel all root transform operations on
reparent.

**Black-box acceptance:** parented-object fixtures assert local/world conversion,
set/tween conflict parity, cancellation continuity, scale independence,
quaternion normalization/shortest arc, reparent effects, and billboard failure
through public transforms and failure submissions.

### Task 27 — Execute camera and light commands

**Prerequisites:** Tasks 16 and 23–24.

Implement camera enable, projection switches, FOV/orthographic-size sets and
tweens, clipping, and clear state; implement light enable, type, color/intensity
sets and tweens, range, spot angles, and shadows. Projection switches cancel
both projection-value keys. Tweens require the matching active projection and
all cross-field/range checks run when the command executes.

**Black-box acceptance:** command sequences inspect public Camera/Light state
and cover projection cancellation, wrong-projection tweens, input-camera disable
effects, every clear/light/shadow enum, linear colors, range/angle boundaries,
and independent concurrent light keys.

### Task 28 — Execute image and text commands

**Prerequisites:** Tasks 15–16 and 23–24.

Implement image texture/size/fit/tint/opacity/face-camera commands and text
content/font/size/color/alignment/wrapping/rich-text/face-camera commands,
including tween variants and exact conflict keys. Update mesh/UV/collider state
atomically for size/fit changes. Billboard `LateUpdate` behavior runs after
tweens and uses input-camera position/up, retaining prior rotation when
coincident.

**Black-box acceptance:** batches mutate visible meshes, materials, TMP state,
colliders, and billboards. Cover prepared-type errors, all fit/alignment modes,
wrapping width conditions, opacity vs RGB tint, conflict independence, camera
roll, coincident positions, and rotation rejection while enabled.

### Task 29 — Execute Animator and time commands

**Prerequisites:** Tasks 17 and 22–24.

Implement Animator play, cross-fade, persistent parameters, trigger, and speed,
plus `time.wait` and `operation.cancel`. Target only one root Animator, use exact
state/parameter names, enforce layer/start/cross-fade/speed ranges, and make
play/cross-fade completion depend only on explicit `waitMs`, never inferred clip
duration. Root looping play with no wait must be nonblocking.

**Black-box acceptance:** a controller fixture exposes current state and
parameters; deterministic scheduler tests verify explicit waits, cross-fade
duration, nonblocking loop use, missing state/component failures, group timing,
and explicit operation cancellation.

### Task 30 — Execute particle commands and pooled effects

**Prerequisites:** Tasks 12, 14, and 22–24.

Implement recursive root particle play/stop and prepared-effect spawn at object
or world location. Blocking spawn completes at positive `lifetimeMs`; root play
has no inferred end and must be nonblocking. Add opt-in `MasonryEffectPool`,
`IMasonryPoolReset`, root component-order callbacks, transform reset, recursive
particle stop/clear, max-inactive enforcement, low-memory clearing, and safe
fallback destruction.

**Black-box acceptance:** particle fixtures verify recursion, restart/clear,
locations, blocking timing, UUID cleanup, pooled reuse/reset order, cap behavior,
reset exceptions, cancellation, handle lifetime, and non-pooled destruction
through visible components and public callbacks.

### Task 31 — Execute two-dimensional audio commands

**Prerequisites:** Tasks 12 and 22–24.

Implement Masonry-owned AudioSource pooling at the current input camera for
play, stop/fade, set/tween volume, pitch, loop, and fade-in. Use the audio play
command UUID as the target/key; finite blocking play completes when the source
stops, loops must be nonblocking, and changing the input camera re-associates
live sources without restarting. Snapshot/session cancellation stops and
releases sources.

**Black-box acceptance:** use short generated AudioClip fixtures and the
deterministic clock to verify source placement, volume/pitch/range validation,
fade timing/conflicts, blocking completion, loop restrictions, stop behavior,
camera reassociation, pooling, and snapshot cancellation without audio resume.

## Wave 7: input and custom code

### Task 32 — Emit deterministic pointer actions

**Prerequisites:** Tasks 11, 15, 25, and 31.

Configure EventSystem, Input System UI module, and PhysicsRaycaster around the
enabled input camera. Track mouse/touch pointers in ascending pointer-ID order,
use the closest blocking physics hit, walk to the nearest identity without
searching behind an ineligible collider, and emit enabled enter/exit/down/up/
click actions with exact screen/world/button payloads. Preserve last exit hit,
matching runtime-object press/release semantics, and all press-cancellation
conditions.

Use Input System test devices and public runner configuration in Edit Mode;
avoid a Play Mode test suite.

**Black-box acceptance:** synthetic mouse/touch events against real colliders
verify ordering, bottom-left pixels, pointer IDs, buttons, child lookup, blocking
unidentified colliders, move-away-and-back clicks, mismatch/no-click, misses,
multi-pointer order, disabled events, and cancellation on disable/snapshot/focus/
destroy/deactivate. Captured transport MessagePack is the primary assertion.

### Task 33 — Emit keyboard actions and apply input gates

**Prerequisites:** Tasks 25 and 32.

Map the exact Rust W3C physical-code enum to Input System controls. Emit one
down/up action per physical transition, suppress repeat, and honor the global
key set plus the common input-enabled gate. Changing the enabled set does not
create synthetic transitions. Session/snapshot/focus loss clears held tracking
without synthetic up actions.

**Black-box acceptance:** parameterized Input System fixtures cover every
supported code mapping, left/right modifiers, numpad distinctions, layout
independence, repeat suppression, dynamic key-set changes, gating, focus loss,
and exact serialized action IDs/session IDs.

### Task 34 — Register and run custom commands/actions

**Prerequisites:** Tasks 10 and 21–24.

Implement explicit generic command registration under namespaced strings,
duplicate rejection, connect-time reporting of every registered command type,
and payload deserialization through the formatter supplied by the game.
Handlers receive cancellation, logger, public object/prepared-asset lookup, and
tween helpers on Unity's main thread. They return completed or tracked work;
blocking and late nonblocking failures follow the design's distinct paths.

Implement typed custom-action emission through the active transport and the
main-thread response processor. Nested returns use its reentrancy deque. Do not
provide a reusable custom Rust-to-C#
generation pipeline, assembly scanning, arbitrary method invocation, runtime
compilation, or snapshot state for handlers.

**Black-box acceptance:** a separate fixture assembly registers handlers and
uses only public APIs. Cover connect advertisement, duplicate/unregistered types,
payload failure, immediate exception, blocking failure, late operation failure,
cancellation, synchronous nested submission without recursive application, and
game-namespaced error codes.

## Wave 8: integration, hardening, and release

### Task 35 — Complete protocol limits and independent contract fixtures

**Prerequisites:** Tasks 20–34.

Audit every fixed v1 limit and validation rule against both Rust domain model and fixtures
and the Unity boundary. Generate valid/invalid MessagePack fixtures from Rust in
temporary build output and consume them at the public Unity protocol boundary.
Keep literal independent fixtures for the documented connect/snapshot/action/
batch/failure examples so the producer and consumer cannot agree on the same
accidental mistake.

Reconcile defaults, discriminators, error-code values, UUID rules, numeric
ranges, uniqueness, collection sizes, string/response byte limits, and every
cross-field condition. Any required protocol correction changes the design,
Rust type, handwritten C# codec, and fixtures together.

**Black-box acceptance:** Rust-generated and independent literal fixtures agree
on valid behavior and expected failures through the public codec/runner. No
test assembly sees package internals, and no format-generated artifact remains after the
test run.

### Task 36 — Run cross-transport release scenarios

**Prerequisites:** Task 35.

Drive the same scenario corpus end-to-end through native and HTTP runner
configurations. Keep transport-neutral expected observations so the tests prove
equivalence rather than duplicating each implementation's assumptions.

Add the release-check scenarios from the design that span multiple subsystems:
partial batch failure, nonblocking group timing, duplicate lifetime, snapshot
correction, destroyed lookup, asset preparation/use, child input, transport
memory ownership, custom failure, pointer ordering, snapshot recreation, and
fatal explicit reconnect.

**Black-box acceptance:** all tests stay outside package internals and produce
the same visible results and client MessagePack through both transports. This is an
intentionally test-heavy task and may exceed the normal line target.

### Task 37 — Add content checks and representative integration fixtures

**Prerequisites:** Tasks 12–17 and 30–34.

Replace the placeholder sample with small Addressable scenes, prefabs, materials,
textures, fonts, audio, effects, Animator controllers, colliders, and one custom
handler fixture. Add test helpers that validate address existence/type, required
root component counts, handler registration, and protocol fixture compatibility.
These remain test/build helpers, not an editor product.

Keep game-owned loading/bootstrap objects visibly separate from Masonry
ownership.

**Acceptance:** a clean Addressables build passes all content checks and the
reference scene can run the end-to-end snapshot/click/command flow through the
fixture engine without manual asset repair.

### Task 38 — Run a representative performance smoke check

**Prerequisites:** Tasks 09 and 36–37.

Add one repeatable development-player scenario covering a pointer action, an
immediate response, and a tween while collecting Unity profiler markers and
allocations. Keep it diagnostic and non-gating; expand performance work only
when measurements identify a concrete problem.

**Acceptance:** one local command runs the scenario and prints a compact report
that is useful for spotting regressions without depending on named hardware.

### Task 39 — Finish distribution and release docs

**Prerequisites:** Tasks 36–38.

Extend the Task 06 host-platform smoke player to exercise connect, snapshot,
action, batch, poll, and native output freeing through the complete runtime.
Supported target packaging remains documented, but v1 does not require a
hardware or IL2CPP smoke matrix for every target.

Complete installation, native-library placement, Addressables catalog, custom
handler, HTTP-development, explicit reconnect, codec review, content check, and
coordinated-release documentation. Verify the final
repository layout matches the design, no format-generated artifacts exist, dependency
versions are exact, and the tagged Git revision contains matching Rust crates,
UPM package, C# codecs, handlers/fixtures, and catalog.

**Acceptance:** checks pass from a clean checkout, the host-platform smoke player
completes the common fixture, and package consumers can follow the installation
document without repository-local state.

## Completion criteria

V1 is complete only when all 39 tasks are integrated and the following are true:

- The canonical Rust types remain independent of the selected binary codec and
  Unity transport implementation.
- Both transports expose identical ordered protocol behavior, with exact native
  memory ownership and HTTP timeout/status semantics.
- Every snapshot, object kind, core command, input action, custom handler path,
  failure rule, hard limit, conflict key, and reconnect transition in the
  technical design has black-box coverage or a narrowly documented player smoke
  check where Edit Mode cannot exercise the Unity lifecycle faithfully.
- The package never treats its Unity view as authoritative game state, never
  touches unrelated bootstrap objects, never loads an unprepared asset as a
  command side effect, and never retries or recursively applies a response.
- Tollgate passes Rust checks, Unity compilation,
  black-box Edit Mode tests, content checks, and the host-platform smoke build
  from a clean checkout.
- No generated contract artifacts exist.
