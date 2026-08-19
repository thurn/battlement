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
executors, registries, validators, or scheduler internals directly.

The shared Edit Mode harness provides:

- A host builder that creates an isolated bootstrap scene, runner, fake
  transport, fake Addressables store, and captured logger through public APIs.
- A deterministic way to drive one Masonry scheduling step and advance the
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
| 2 | 02–09 | Rust native adapter, Unity transports, lifecycle, FIFO, and failures |
| 3 | 10–16 | Owned scenes, assets, identities, and all snapshot object kinds |
| 4 | 17–19 | Transactional replacement snapshots |
| 5 | 20–23 | Ordered batches, operations, conflicts, and tweens |
| 6 | 24–30 | Complete core command execution |
| 7 | 31–33 | Pointer/keyboard input and custom code |
| 8 | 34–38 | Cross-cutting contract coverage, performance, content checks, and release |

Expected handwritten production-plus-test size is shown below. The upper end
of a daggered task is test-heavy; its production implementation should still be
only a few hundred lines.

| Task | Expected lines | Task | Expected lines |
|---:|---:|---:|---:|
| 01 | 250–350 | 20 | 250–350 |
| 02 | 250–350 | 21 | 250–350 |
| 03 | 200–300 | 22 | 300–450† |
| 04 | 250–350 | 23 | 300–450† |
| 05 | 250–350 | 24 | 350–450† |
| 06 | 250–350 | 25 | 200–300 |
| 07 | 300–400 | 26 | 300–400 |
| 08 | 250–350 | 27 | 300–450† |
| 09 | 200–300 | 28 | 250–350 |
| 10 | 250–350 | 29 | 300–450† |
| 11 | 300–450† | 30 | 300–400 |
| 12 | 250–350 | 31 | 350–500† |
| 13 | 300–400 | 32 | 250–350 |
| 14 | 200–300 | 33 | 300–450† |
| 15 | 300–400 | 34 | 300–450† |
| 16 | 250–350 | 35 | 350–500† |
| 17 | 350–500† | 36 | 250–400 |
| 18 | 300–400 | 37 | 300–450† |
| 19 | 300–450 | 38 | 200–350 |

## Wave 1: public host and test boundary

### [DONE] Task 01 — Build the public Edit Mode host harness boundary

**Prerequisites:** none.

Define the minimum public seams needed by a host and black-box tests:
`IMasonryTransport`, `IMasonryAssetStorage`, structured logging, clock/scheduling
input, immutable runner options, and native/HTTP serialized configuration.
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

### Task 04 — Implement the Unity native transport

**Prerequisites:** Tasks 01 and 03.

Implement the platform library-name mapping and P/Invoke declarations for the
fixed ABI. Copy binary inputs only for the synchronous call, validate output
pointers and lengths before managed allocation, and free every nonempty native
buffer in `finally` on success, parse error, managed exception, and cancellation.
Map fixed/unknown status values to transport results without applying responses
inside the transport.

The transport owns one engine handle, reuses it across reconnects, serializes
main-thread calls, and destroys the handle at runner shutdown. Poll is immediate
and exposes `NO_MESSAGE` distinctly from an empty response.

**Black-box acceptance:** use a compiled ABI fixture library through the public
runner, then assert connect/submit/poll behavior and native allocation counts.
Include success, 16 MiB boundary, malformed MessagePack, engine error, panic,
unknown status, and managed-exception cases.

### Task 05 — Implement the synchronous localhost HTTP transport

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

### Task 06 — Implement runner lifecycle and explicit reconnect

**Prerequisites:** Tasks 01, 04, and 05.

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

### Task 07 — Parse responses immediately and defer application

**Prerequisites:** Task 06.

Parse every successful connect, submit, and nonempty poll response immediately
on Unity's main thread. Reject responses larger than 16 MiB before parsing.
Append each parsed response to one inbound FIFO capped at 256 entries and apply
it only at the next safe scheduler step, never recursively inside input,
command, custom-handler, or response execution. The transports are blocking
and serialized on the main thread, so FIFO insertion preserves transport-call
order without sequence reconciliation, background parsing, or rented response
buffers.

**Black-box acceptance:** return varied response sizes from connect, submit,
and poll; verify each response is parsed synchronously, transport-call order
equals visible application order, the 16 MiB and queue limits are enforced,
and a custom submission cannot cause recursive application.

### Task 08 — Add the per-frame budget, polling loop, and structured logging

**Prerequisites:** Tasks 06 and 07.

Implement the fixed 4 ms Masonry scheduling budget around splittable work. Poll
once per frame and continue while budget remains; do not misclassify a single
unsplittable Unity call as permission for more work. Add one structured logging
interface with Unity console output, stable event names, relevant IDs, duration,
payload bytes, queue depth, trace-only frequent pointer events, rate-limited
warnings, and a bounded in-memory warning/error buffer.

Add profiler markers for serialization, transport, response parsing, format
checks, Unity calls, custom handlers, polling, and per-frame work.

**Black-box acceptance:** use a fake clock and logger to verify yield/resume,
poll counts, queue depth, rate limiting, stage timing, and long-frame records
without asserting internal method calls.

### Task 09 — Serialize and route failure submissions

**Prerequisites:** Tasks 06 and 07.

Create the common failure path for `masonry.batch.failed` and
`masonry.operation.failed`. Preserve session, batch, and command/operation IDs;
map core error codes exactly; bound diagnostic messages; submit failures
immediately through the active transport; parse any returned response
immediately and append it to the inbound FIFO. Separate recoverable
batch/operation failure from session-fatal transport, top-level MessagePack,
unknown-message, and snapshot failure.

Apply Editor behavior only at the outer host boundary: report first, then throw
on the main thread when the design requires it. Production reports without
throwing.

**Black-box acceptance:** fixture failures produce exact client MessagePack and stop
only the intended batch/session. Returned corrections are queued rather than
applied recursively, and logging contains the stable IDs and error code.

## Wave 3: world ownership and object construction

### Task 10 — Create containers, identity registration, and lookup behavior

**Prerequisites:** Tasks 01 and 06.

Create the persistent bootstrap container and per-content-scene containers.
Implement `MasonryIdentity` with a `System.Guid`, registration/unregistration,
Unity-aware destroyed-object handling, nearest-ancestor lookup for child hits,
session-lifetime no-reuse histories, and object/scene lookup errors. Never scan,
rename, reparent, or delete unrelated bootstrap objects.

**Black-box acceptance:** construct and destroy identified objects through the
public runner, inspect only Unity hierarchy/components, and verify UUID
uniqueness, descendant identity resolution, destroyed-reference cleanup, and
unrelated-object survival.

### Task 11 — Implement prepared assets and reference accounting

**Prerequisites:** Tasks 01 and 10.

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

### Task 12 — Implement additive content-scene ownership

**Prerequisites:** Tasks 10 and 11.

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

### Task 13 — Construct empty, primitive, and prefab objects

**Prerequisites:** Tasks 10–12.

Create empty GameObjects, Unity primitive shapes, and instances of prepared
prefabs. Apply parent-scene selection, topological parent, `activeSelf`, local
transform/defaults, identity, and initial pointer-event collider policy. Check
prefab root supported-component counts and never target authored children.
Primitive colliders exist only when pointer events require them.

**Black-box acceptance:** snapshot/object MessagePack creates every base kind under
primary, named, persistent, and parented placements. Tests inspect public Unity
state and cover defaults, hierarchy, inactive parents, duplicate IDs, wrong
asset kinds, missing prefabs, and unsupported component counts.

### Task 14 — Construct image objects and their owned material

**Prerequisites:** Tasks 11 and 13.

Create the centered image quad with its Masonry-owned URP material, prepared
texture, positive world size, stretch/contain/cover UV behavior, RGB tint,
separate opacity, optional face-camera behavior, and a 0.01-depth centered
BoxCollider when pointer events are enabled. Resize the collider with the image
and preserve texture filtering/wrapping.

**Black-box acceptance:** MessagePack fixtures create and mutate representative aspect
ratios; tests inspect mesh bounds/UVs, material-visible values, collider size,
linear tint/opacity, and billboard output relative to rolled and coincident
cameras.

### Task 15 — Construct text, camera, and light objects

**Prerequisites:** Tasks 11 and 13.

Create world-space TMP text with prepared font, size/color/alignment/wrapping/
rich-text defaults and billboard behavior. Create standard Camera and Light
components with the complete snapshot state and defaults, including projection,
clipping, clear behavior, light type/range/spot/shadows, and separate component
enabled state. Do not create automatic colliders for these kinds.

**Black-box acceptance:** literal records for every projection, clear mode,
light type, alignment, and wrapping mode produce observable component state;
invalid ranges, missing fonts, and disabled input-camera candidates fail with
the specified error class.

### Task 16 — Apply materials and stable Animator snapshot state

**Prerequisites:** Tasks 11, 13, and 15.

Assign prepared materials to unique zero-based root-renderer slots on primitives
and prefabs through `sharedMaterials`; support assign-all and individual slots,
excluding image/text renderers. Apply prefab-root Animator stable state, layer,
normalized start time, persistent bool/int/float parameters, and speed without
restoring triggers or playback progress.

**Black-box acceptance:** prefab and primitive fixtures expose renderer and
Animator state through Unity components. Cover multiple slots, duplicate/out-of-
range slots, missing/multiple root components, wrong asset kinds, and exact
snapshot defaults.

## Wave 4: transactional snapshots

### Task 17 — Validate and plan a complete snapshot before mutation

**Prerequisites:** Tasks 10–16.

Build a side-effect-free snapshot preparation phase over decoded DTOs. Validate
all hard limits, canonical/unique IDs, prepared addresses and types, primary
scene rules, input camera requirements, parent-scene resolution, acyclic
same-scene hierarchy, object-kind requirements, root components, material slots,
and scene-transition constraints. Produce an immutable private application plan
in topological order without changing the visible world.

Keep this implementation private and test only through submitted snapshots.

**Black-box acceptance:** a table of malformed snapshots submitted to a visible
old world leaves that world unchanged, disables input/stops the session as
required, and reports diagnostics. Valid 100k-object/count-boundary fixtures
can be generated in tests without committing enormous MessagePack.

### Task 18 — Stage snapshot assets and scenes under cancellation

**Prerequisites:** Task 17.

On a current-session snapshot, stop accepting input, cancel operations, discard
older queued messages, and stage prepared handles plus new additive scenes while
the last complete world remains visible. Reuse matching handles and scene
UUID/address pairs. Keep staged scene-controlled objects hidden and do not call
`SetActiveScene` before commit. Honor the 4 ms budget around splittable work and
release staging resources safely on cancellation/failure.

A newer snapshot cancels the in-progress preparation, discards messages between
boundaries, and begins staging from its own complete state.

**Black-box acceptance:** delayed fake loads prove old-world visibility,
input gating, reuse, safe cancellation, newer-snapshot precedence, queued
message ordering, and balanced handles without inspecting the staging plan.

### Task 19 — Commit, reveal, and fail snapshot replacement

**Prerequisites:** Task 18.

Implement the hidden mutation phase: hide every Masonry container and root in
owned content scenes, destroy/recreate every game object over budgeted frames,
switch primary scene, unload obsolete scenes, reveal only the complete new
world, and resume configured input. Same-address scene replacement unloads then
loads while controlled content is hidden. No GameObject instance survives a
snapshot boundary.

Once mutation begins, a failure leaves incomplete Masonry content hidden,
permanently stops the session, discards queues, and never rolls back or retries.
Post-snapshot messages wait until reveal.

**Black-box acceptance:** replacement fixtures verify recreation by Unity
instance identity, scene/handle reuse rules, hidden incomplete worlds, hierarchy
and visible final values, no resumed operation/audio/particle progress, fatal
mid-mutation failure, and post-reveal batch ordering.

## Wave 5: batch scheduling and operations

### Task 20 — Admit batches and enforce session/duplicate/start rules

**Prerequisites:** Tasks 09 and 19.

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

### Task 21 — Execute ordered groups and propagate batch failure

**Prerequisites:** Task 20.

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

### Task 22 — Track operations, conflicts, waits, and cancellation

**Prerequisites:** Task 21.

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

### Task 23 — Adapt PrimeTween and implement Masonry tween semantics

**Prerequisites:** Task 22.

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

### Task 24 — Execute asset, scene, object, renderer, and input-control commands

**Prerequisites:** Tasks 11–13 and 20–23.

Implement `assets.replaceSet`; scene load/unload/set-primary; object create,
destroy, active, and reparent; renderer material assignment; input enable;
input-camera selection; pointer-event selection; and global-key selection.
Apply every command at execution time with the ownership, preparation, hierarchy,
scene, collider, and conflict/cancellation rules already established.

**Black-box acceptance:** batches combine these commands so later commands
observe earlier immediate effects. Cover scene unload restrictions, input cutover,
new-ID history, descendant destruction, `worldPositionStays`, null-parent scene
container behavior, live-asset removal, and renderer slot errors.

### Task 25 — Execute transform commands

**Prerequisites:** Tasks 22–24.

Implement local/world position, local/world rotation, and local scale set/tween
variants. Share local/world position and rotation conflict keys, capture current
displayed values after cancellation, normalize quaternion inputs, reject
billboard-controlled rotation, and cancel all root transform operations on
reparent.

**Black-box acceptance:** parented-object fixtures assert local/world conversion,
set/tween conflict parity, cancellation continuity, scale independence,
quaternion normalization/shortest arc, reparent effects, and billboard failure
through public transforms and failure submissions.

### Task 26 — Execute camera and light commands

**Prerequisites:** Tasks 15 and 22–23.

Implement camera enable, projection switches, FOV/orthographic-size sets and
tweens, clipping, and clear state; implement light enable, type, color/intensity
sets and tweens, range, spot angles, and shadows. Projection switches cancel
both projection-value keys. Tweens require the matching active projection and
all cross-field/range checks run when the command executes.

**Black-box acceptance:** command sequences inspect public Camera/Light state
and cover projection cancellation, wrong-projection tweens, input-camera disable
effects, every clear/light/shadow enum, linear colors, range/angle boundaries,
and independent concurrent light keys.

### Task 27 — Execute image and text commands

**Prerequisites:** Tasks 14–15 and 22–23.

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

### Task 28 — Execute Animator and time commands

**Prerequisites:** Tasks 16 and 21–23.

Implement Animator play, cross-fade, persistent parameters, trigger, and speed,
plus `time.wait` and `operation.cancel`. Target only one root Animator, use exact
state/parameter names, enforce layer/start/cross-fade/speed ranges, and make
play/cross-fade completion depend only on explicit `waitMs`, never inferred clip
duration. Root looping play with no wait must be nonblocking.

**Black-box acceptance:** a controller fixture exposes current state and
parameters; deterministic scheduler tests verify explicit waits, cross-fade
duration, nonblocking loop use, missing state/component failures, group timing,
and explicit operation cancellation.

### Task 29 — Execute particle commands and pooled effects

**Prerequisites:** Tasks 11, 13, and 21–23.

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

### Task 30 — Execute two-dimensional audio commands

**Prerequisites:** Tasks 11 and 21–23.

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

### Task 31 — Emit deterministic pointer actions

**Prerequisites:** Tasks 10, 14, 24, and 30.

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

### Task 32 — Emit keyboard actions and apply input gates

**Prerequisites:** Tasks 24 and 31.

Map the exact Rust W3C physical-code enum to Input System controls. Emit one
down/up action per physical transition, suppress repeat, and honor the global
key set plus the common input-enabled gate. Changing the enabled set does not
create synthetic transitions. Session/snapshot/focus loss clears held tracking
without synthetic up actions.

**Black-box acceptance:** parameterized Input System fixtures cover every
supported code mapping, left/right modifiers, numpad distinctions, layout
independence, repeat suppression, dynamic key-set changes, gating, focus loss,
and exact serialized action IDs/session IDs.

### Task 33 — Register and run custom commands/actions

**Prerequisites:** Tasks 09 and 20–23.

Implement explicit generic command registration under namespaced strings,
duplicate rejection, connect-time reporting of every registered command type,
and payload deserialization through the formatter supplied by the game.
Handlers receive cancellation, logger, public object/prepared-asset lookup, and
tween helpers on Unity's main thread. They return completed or tracked work;
blocking and late nonblocking failures follow the design's distinct paths.

Implement typed custom-action emission through the active transport and inbound
FIFO. Do not provide a reusable custom Rust-to-C# generation pipeline, assembly
scanning, arbitrary method invocation, runtime compilation, or snapshot state
for handlers.

**Black-box acceptance:** a separate fixture assembly registers handlers and
uses only public APIs. Cover connect advertisement, duplicate/unregistered types,
payload failure, immediate exception, blocking failure, late operation failure,
cancellation, synchronous nested submission without recursive application, and
game-namespaced error codes.

## Wave 8: integration, hardening, and release

### Task 34 — Complete protocol limits and independent contract fixtures

**Prerequisites:** Tasks 19–33.

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

### Task 35 — Run cross-transport release scenarios

**Prerequisites:** Task 34.

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

### Task 36 — Add content checks and representative integration fixtures

**Prerequisites:** Tasks 11–16 and 29–33.

Replace the placeholder sample with small Addressable scenes, prefabs, materials,
textures, fonts, audio, effects, Animator controllers, colliders, and one custom
handler fixture. Add test helpers that validate address existence/type, required
root component counts, handler registration, and protocol fixture compatibility.
These remain test/build helpers, not an editor product.

Include representative complex-prefab and scene-activation fixtures used by
performance reporting. Keep game-owned loading/bootstrap objects visibly
separate from Masonry ownership.

**Acceptance:** a clean Addressables build passes all content checks and the
reference scene can run the end-to-end snapshot/click/command flow through the
fixture engine without manual asset repair.

### Task 37 — Implement performance fixtures and release measurements

**Prerequisites:** Tasks 08 and 35–36.

Build non-development IL2CPP performance players and fixtures for the exact
hover measurement: 300-frame warmup, 10,000 consecutive exchanges, native
single 80 ms scale tween response, and reports containing p50/p95/p99/max,
payload bytes, and allocations. Add large snapshot, concurrent tween, pooled
effect burst, complex prefab, scene activation, and sustained poll queue runs.

Report the prescribed stages and profiler markers. Enforce desktop/mobile hover
gates on the named reference hardware; always publish scene/prefab measurements
without converting them into unsupported hard gates. Store summarized artifacts,
not raw unbounded MessagePack logs.

**Acceptance:** repeatable local/Tollgate commands produce machine-readable and
human-readable reports, enforce 4 ms splittable-work and hover p95 gates, and
identify the exact stage/Unity call for threshold violations.

### Task 38 — Finish platform smoke builds, distribution, and release docs

**Prerequisites:** Tasks 35–37.

Add Tollgate-invoked build commands for macOS universal arm64/x86_64, Windows
x86_64, iOS device arm64, and Android arm64-v8a. Each smoke player links the
correct native library name, exercises connect/snapshot/action/batch/poll,
verifies native output freeing, and compiles the handwritten codec and custom handlers
under IL2CPP. Hardware-dependent jobs use the existing Tollgate environment;
do not introduce GitHub Actions.

Complete installation, native-library placement, Addressables catalog, custom
handler, HTTP-development, explicit reconnect, codec review, content
check, performance, and coordinated-release documentation. Verify the final
repository layout matches the design, no format-generated artifacts exist, dependency
versions are exact, and the tagged Git revision contains matching Rust crates,
UPM package, C# codecs, handlers/fixtures, and catalog.

**Acceptance:** all Tollgate steps pass from a clean checkout; package consumers
can follow the installation document without repository-local state; all four
platform smoke artifacts complete the common fixture; the final release report
contains contract, platform, content, and performance results.

## Completion criteria

V1 is complete only when all 38 remaining tasks are integrated and the following are true:

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
  black-box Edit Mode tests, content checks, required IL2CPP platform smoke
  builds, and performance gates from a clean checkout.
- No generated contract artifacts exist.
