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

Each task states its own visual-evidence requirement. Visual evidence is only
required when a screenshot or short video can directly show player-visible or
Editor-visible behavior. Low-level protocol, transport, validation, logging,
and packaging work explicitly says that no visual evidence is required; tests
and machine-readable checks remain the proof for those tasks. Screenshots of
terminals, test reports, logs, or other text-only output never count as visual
evidence.

When required, visual evidence supplements rather than replaces automated
acceptance. A screenshot records meaningful rendered state. A 5–20 second
video records interaction, animation, ordering, or another temporal
result from initial state through expected behavior. Use the least expensive
environment that truthfully shows the result, normally the Unity Game view.
Only Tasks 12A and 37A require capture from a packaged macOS Release app;
other tasks name a different environment only when it matters. A clip should
show one representative behavior, not attempt to visualize every automated
acceptance case. Capture procedure and retention follow Task 12A; large media
does not enter Git. Tasks already marked `DONE` when Task 12A is added are
exempt, with no retrospective backfill.

Follow the [visual capture guide](visual-capture.md) whenever a task requires
visual evidence. Author an `Assets/*.unity` scene containing exactly one
`MasonryCaptureScenario` with the requested stable name. After deterministic
state is visibly rendered, the scenario calls `RequestInput` with its observed
assertions, one `CaptureInput`, and a normalized top-left-origin pointer target.
Each real Unity pointer handler may request the next move, button-down, or
button-up event after the task-defined state and timing are ready. The driver
preserves a two-second behavior-free initial video frame before the first
requested input by default. The scenario calls `SignalPassed` or `SignalFailed`
when the sequence completes. Run the Release-player capture from
the repository root, selecting the task's actual transport and native plugin
source when applicable:

```sh
./scripts/capture-visual-evidence.sh \
  --task TASK_ID \
  --scenario SCENARIO_NAME \
  --scene Assets/Path/To/Capture.unity \
  --transport native \
  --cargo-package CARGO_PACKAGE \
  --capture both \
  --dimensions 1280x720
```

Use `--plugin PATH` instead of `--cargo-package` for a prebuilt native library,
or omit both and select `--transport http` or `--transport none` for scenarios
without a native plugin. Run `--smoke` first, then retain the final before/after
PNGs, video, and run log under the ignored evidence root. Reuse the verified
content-addressed packaged build while iterating on media. Prefer the reusable
capture shell and scaffold; task-local one-off scenario source and scenes may
remain uncommitted and be removed after the evidence is reviewed.

## Dependency overview

Implementation proceeds in the following dependency waves. Work inside a wave
may be parallelized when its listed prerequisites are complete.

| Wave | Tasks | Result |
|---|---|---|
| 1 | 01 | Public host and test boundary |
| 2 | 02–10 | Rust adapter, transports, macOS player smoke, lifecycle, and failures |
| 3 | 11, 12A, 12–17 | Visual capture, owned scenes, assets, identities, and all snapshot object kinds |
| 4 | 18–20 | Direct replacement snapshots |
| 5 | 21–24 | Ordered batches, operations, conflicts, and tweens |
| 6 | 25–31 | Complete core command execution |
| 7 | 32–34 | Pointer/keyboard input and custom code |
| 8 | 35–37, 37A, 38 | Contract coverage, permanent demo, evidence, and performance |

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
| 12 | 200–300 | 32 | 350–500† |
| 13 | 250–350 | 33 | 250–350 |
| 14 | 300–400 | 34 | 300–450† |
| 15 | 200–300 | 35 | 300–450† |
| 16 | 300–400 | 36 | 350–500† |
| 17 | 250–350 | 37 | 250–400 |
| 18 | 200–300 | 38 | 100–200 |
| 19 | 150–250 | | |
| 20 | 200–300 | | |

Lettered task IDs preserve the identifiers of the existing implementation
plan. Task 12A is expected to require 250–400 handwritten production and test
lines; Task 37A is expected to require 300–450.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

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

**Visual evidence:** not required; this task was completed before Task 12A.

### **[DONE]** Task 12A — Build release-player screenshot and video infrastructure

**Prerequisites:** Tasks 06 and 11.

Add one repeatable capture command that builds a non-Development macOS player,
stages a host-architecture Rust fixture engine through the production Unity
native-plugin import path, launches the resulting `.app` without Editor-only
library search paths, selects a named deterministic scenario, and captures a
PNG screenshot or short video. The same scenario API may run in the Editor for
fast authoring, but release-player capture is the required path.

Give capture scenarios stable names, deterministic initial state and timing,
explicit scenario-driven input requests, visible step/state labels where the
rendered result would otherwise be ambiguous, and machine-readable pass/fail
assertions. The harness must frame the game viewport consistently, dispatch
only the requested low-level input, support arbitrary interaction and
time-based sequences, and hide capture-only overlays unless requested. It must
fail on build or launch failure,
missing native plugin, timeout, player crash, assertion failure, or capture
failure, and it must clean generated plugins, builds, logs, and player
processes without deleting the requested evidence artifacts.

The command accepts a task ID, scenario name, artifact root, capture kind,
dimensions, video duration, and interaction timeout. Defaults are a 1280×720
PNG or a 1280×720, 30 fps H.264 MP4; a single invocation may request both.
Capture only the player content window at native pixel dimensions. Refuse to
overwrite a run ID. Drive actual pointer input against the focused minimal
fixture to prove window focus and interaction capture. This task does not
depend on the later Masonry pointer/action pipeline; Task 37A proves that
complete path. Document supported macOS prerequisites,
including a logged-in GUI session and any Screen Recording or Accessibility
permission, and fail preflight when they are absent. Headless capture is not
required.

Retain the media and concise run log. Clean transient staged
plugins, builds, raw Unity logs, and player processes. Do not backfill visual
evidence for tasks completed before this infrastructure exists.

**Black-box acceptance:** from a clean checkout, one command builds and launches
the packaged release `.app`, proves it loaded the bundled Rust dylib and
completed a recognizable C# → Rust → C# exchange, captures a screenshot and a
short video at the requested dimensions, and leaves
no generated plugin, build, or player process behind. A failure injected before
the ready signal produces no misleading success artifact.

**Visual evidence:** a short video captured by the new command from its minimal
deterministic Release-player fixture. Automated assertions, rather than the
video itself, prove the bundled native round trip.

### **[DONE]** Task 12 — Wrap Addressables and manage the prepared set

**Prerequisites:** Tasks 01 and 11.

Implement a thin asset-storage adapter over Addressables for the seven fixed
kinds. Use typed Addressables operations, their completion/error state, and
their existing handle reference counting rather than reproducing resource
lifetime management. Scene preparation resolves and type-checks the scene and
downloads its dependencies without constructing it; Task 13 performs the
Addressables scene load.

Add the Masonry prepared-set manager around that adapter. Validate the fixed
prepared-asset/count/string limits, load and type-check every addition before
changing the active set, retain exactly one owned handle per prepared address,
atomically commit the new set, reuse matching handles across snapshots, and
release removed handles when their Masonry usage count reaches zero. A command
replacement rejects an in-use removal; an authoritative snapshot may retire
the address from lookup while retaining its handle until teardown releases the
last usage lease. Failed or superseded preparation releases its handles;
"cancellation" means Masonry abandons the result and releases the handle when
safe, not that Addressables must stop underlying work. Do not update catalogs,
clear Addressables' download cache, add retries, or retain a second cache of
unprepared assets.

Provide prepared-only typed lookup and a separate Masonry usage-lease/count
mechanism for enforcing `asset_in_use`. This is protocol accounting, not a
second Addressables resource reference count. Consumer tasks acquire and
release leases when they introduce scenes, objects, assignments, effects, and
audio; command-side lookup must never start an implicit load.

**Black-box acceptance:** fake handles expose load/release counts while runner
MessagePack drives initial preparation, matching-handle reuse, replacement,
load/type failure, and abandonment. Cover every kind and each fixed limit, and
assert only public store calls, submitted errors, prepared lookup results, and
handle balance. Consumer-specific `asset_in_use` behavior is covered by the
tasks that create those uses.

**Visual evidence:** not required; prepared-set accounting has no meaningful
rendered behavior of its own.

### **[DONE]** Task 13 — Implement additive content-scene ownership

**Prerequisites:** Tasks 11, 12, and 12A.

Load prepared scenes additively under unique scene UUID/address pairs, create
scene containers, enforce 32 scenes and one instance per address, track one
primary content scene, and unload only Masonry-owned content. Implement primary
cutover without conflating it with the input camera. Destroy registered scene
objects on unload and preserve authored scene objects until their scene unloads.
Hold a prepared-asset usage lease for each loaded scene and release it only
after the Addressables scene unload completes.

Support reuse only when scene UUID and address both match. A changed UUID at the
same address forces the mutation-phase unload/reload path.

**Black-box acceptance:** an in-memory scene-store fixture and small authored
test scenes verify additive load/unload, primary selection, duplicate-address
rejection, persistent-container survival, authored-object lifetime, and no
bootstrap collateral damage.

**Visual evidence:** a short video of visibly distinct additive content loading
and unloading while bootstrap content remains visible. Automated tests prove
primary-scene ownership and lifecycle details.

### **[DONE]** Task 14 — Construct empty, primitive, and prefab objects

**Prerequisites:** Tasks 11–13.

Create empty GameObjects, Unity primitive shapes, and instances of prepared
prefabs. Apply parent-scene selection, topological parent, `activeSelf`, local
transform/defaults, identity, and initial pointer-event collider policy. Check
prefab root supported-component counts and never target authored children.
Primitive colliders exist only when pointer events require them. A prefab
instance retains a usage lease on its prepared prefab until destruction.

**Black-box acceptance:** snapshot/object MessagePack creates every base kind under
primary, named, persistent, and parented placements. Tests inspect public Unity
state and cover defaults, hierarchy, inactive parents, duplicate IDs, wrong
asset kinds, missing prefabs, and unsupported component counts.

**Visual evidence:** a screenshot of representative primitive and prefab
objects in their final rendered arrangement. Empty-object and hierarchy rules
remain automated-test assertions because they are not directly visible.

### Task 15 — Construct image objects and their owned material

**Prerequisites:** Tasks 12 and 14.

Create the centered image quad with its Masonry-owned URP material, prepared
texture, positive world size, stretch/contain/cover UV behavior, RGB tint,
separate opacity, optional face-camera behavior, and a 0.01-depth centered
BoxCollider when pointer events are enabled. Resize the collider with the image
and preserve texture filtering/wrapping. Retain the prepared texture's usage
lease for as long as it is assigned.

**Black-box acceptance:** MessagePack fixtures create and mutate representative aspect
ratios; tests inspect mesh bounds/UVs, material-visible values, collider size,
linear tint/opacity, and billboard output relative to rolled and coincident
cameras.

**Visual evidence:** a short video showing one representative image resize/fit
change followed by face-camera behavior.

### Task 16 — Construct text, camera, and light objects

**Prerequisites:** Tasks 12 and 14.

Create world-space TMP text with prepared font, size/color/alignment/wrapping/
rich-text defaults and billboard behavior. Create standard Camera and Light
components with the complete snapshot state and defaults, including projection,
clipping, clear behavior, light type/range/spot/shadows, and separate component
enabled state. Do not create automatic colliders for these kinds. Retain the
prepared font's usage lease for as long as it is assigned.

**Black-box acceptance:** literal records for every projection, clear mode,
light type, alignment, and wrapping mode produce observable component state;
invalid ranges, missing fonts, and disabled input-camera candidates fail with
the specified error class.

**Visual evidence:** a screenshot showing representative text as rendered by
the configured camera and lighting.

### Task 17 — Apply materials and stable Animator snapshot state

**Prerequisites:** Tasks 12, 14, and 16.

Assign prepared materials to unique zero-based root-renderer slots on primitives
and prefabs through `sharedMaterials`; support assign-all and individual slots,
excluding image/text renderers. Apply prefab-root Animator stable state, layer,
normalized start time, persistent bool/int/float parameters, and speed without
restoring triggers or playback progress. Each distinct prepared material
assigned to a live object retains a usage lease until replacement or object
destruction.

**Black-box acceptance:** prefab and primitive fixtures expose renderer and
Animator state through Unity components. Cover multiple slots, duplicate/out-of-
range slots, missing/multiple root components, wrong asset kinds, and exact
snapshot defaults.

**Visual evidence:** a screenshot showing a representative material assignment
and the Animator's configured stable pose.

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

**Visual evidence:** not required; snapshot rejection and unchanged-world
invariants are not directly demonstrated by a single rendered artifact.

### Task 19 — Apply snapshot assets and scenes directly

**Prerequisites:** Task 18.

On a valid current-session snapshot, stop accepting input, cancel operations,
replace the prepared set, and reconcile additive scenes in snapshot order.
Reuse matching handles and scene UUID/address pairs, and release resources on
failure. Removed addresses used only by the world being replaced become
unavailable to lookup immediately but retain their handles until Task 20
destroys that world and releases its usage leases. Do not maintain a staged
copy of the old and new worlds. A later snapshot waits for the current
replacement like any other later message.

**Black-box acceptance:** delayed fake loads prove input gating, handle and
scene reuse, failure cleanup, ordered snapshot processing, and balanced handles.

**Visual evidence:** a short video showing the visible scene cutover during
snapshot replacement. Automated tests prove input gating and handle reuse.

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

**Visual evidence:** a short video showing the old objects disappear and the
replacement snapshot's visibly different object arrangement appear. Automated
tests prove hierarchy and identity replacement.

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

**Visual evidence:** not required; admission and duplicate suppression cannot
be established from rendered pixels without an evidence-only event display.

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

**Visual evidence:** not required; no production player-visible command family
exists yet at this dependency point. Task 25 provides the first visual batch
sequence; automated tests prove scheduling and failure propagation here.

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

**Visual evidence:** not required; the production tween adapter needed to show
continuous conflict cancellation is introduced by Task 24. Automated tests
prove operation tracking, queued waits, and conflict bookkeeping here.

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

**Visual evidence:** a short video showing one representative ping-pong tween
and one cancellation that continues from the displayed value.

## Wave 6: core command families

### Task 25 — Execute asset, scene, object, renderer, and input-control commands

**Prerequisites:** Tasks 12–14 and 21–24.

Implement `assets.replaceSet`; scene load/unload/set-primary; object create,
destroy, active, and reparent; renderer material assignment; input enable;
input-camera selection; pointer-event selection; and global-key selection.
Apply every command at execution time with the ownership, preparation, hierarchy,
scene, collider, and conflict/cancellation rules already established. Reject a
replace-set removal with a live usage lease, and exchange material leases only
after a renderer assignment succeeds.

**Black-box acceptance:** batches combine these commands so later commands
observe earlier immediate effects. Cover scene unload restrictions, input cutover,
new-ID history, descendant destruction, `worldPositionStays`, null-parent scene
container behavior, live-asset removal, and renderer slot errors.

**Visual evidence:** a short video showing one representative batch of scene,
object, and renderer commands with a visible group boundary. Automated tests
cover the remaining command families and scheduling cases.

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

**Visual evidence:** a short video of one representative parented transform
tween and its continuous cancellation from the displayed value.

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

**Visual evidence:** a short video showing camera projection or field-of-view
changes alongside light color and intensity changes.

### Task 28 — Execute image and text commands

**Prerequisites:** Tasks 15–16 and 23–24.

Implement image texture/size/fit/tint/opacity/face-camera commands and text
content/font/size/color/alignment/wrapping/rich-text/face-camera commands,
including tween variants and exact conflict keys. Update mesh/UV/collider state
atomically for size/fit changes. Billboard `LateUpdate` behavior runs after
tweens and uses input-camera position/up, retaining prior rotation when
coincident. Texture and font changes atomically exchange their prepared-asset
usage leases.

**Black-box acceptance:** batches mutate visible meshes, materials, TMP state,
colliders, and billboards. Cover prepared-type errors, all fit/alignment modes,
wrapping width conditions, opacity vs RGB tint, conflict independence, camera
roll, coincident positions, and rotation rejection while enabled.

**Visual evidence:** a short video showing one representative image mutation
and one text mutation.

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

**Visual evidence:** a short video showing Animator play/cross-fade and a
visible command after the explicit wait boundary.

### Task 30 — Execute particle commands and pooled effects

**Prerequisites:** Tasks 12, 14, and 22–24.

Implement recursive root particle play/stop and prepared-effect spawn at object
or world location. Blocking spawn completes at positive `lifetimeMs`; root play
has no inferred end and must be nonblocking. Add opt-in `MasonryEffectPool`,
`IMasonryPoolReset`, root component-order callbacks, transform reset, recursive
particle stop/clear, max-inactive enforcement, low-memory clearing, and safe
fallback destruction. Active and pooled instances retain prepared-effect usage
leases; destroying an instance or clearing an inactive pool releases them.

**Black-box acceptance:** particle fixtures verify recursion, restart/clear,
locations, blocking timing, UUID cleanup, pooled reuse/reset order, cap behavior,
reset exceptions, cancellation, handle lifetime, and non-pooled destruction
through visible components and public callbacks.

**Visual evidence:** a short video showing representative particle play and
effect spawn behavior. Automated tests prove pooling and reset semantics.

### Task 31 — Execute two-dimensional audio commands

**Prerequisites:** Tasks 12 and 22–24.

Implement Masonry-owned AudioSource pooling at the current input camera for
play, stop/fade, set/tween volume, pitch, loop, and fade-in. Use the audio play
command UUID as the target/key; finite blocking play completes when the source
stops, loops must be nonblocking, and changing the input camera re-associates
live sources without restarting. Snapshot/session cancellation stops and
releases sources. A source retains its prepared-clip usage lease while the clip
is assigned and releases it when the source is reset. Clear inactive sources on
Unity's low-memory notification so Unity can unload now-unused resources.

**Black-box acceptance:** use short generated AudioClip fixtures and the
deterministic clock to verify source placement, volume/pitch/range validation,
fade timing/conflicts, blocking completion, loop restrictions, stop behavior,
camera reassociation, pooling, and snapshot cancellation without audio resume.

**Visual evidence:** not required; audio behavior is not visual, and the task's
timing, pooling, and lifecycle requirements are covered by automated tests.

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

**Visual evidence:** a short video showing one ordinary pointer hover and click
produce visible responses. Automated tests cover multi-pointer ordering and
press-cancellation edge cases.

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

**Visual evidence:** a short video showing one enabled key produce a visible
response. Automated tests prove repeat suppression and input gating.

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

**Visual evidence:** a short video showing the visible scene change produced by
one representative custom action/command flow. Automated assertions prove the
Rust boundary and custom payload path.

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

**Visual evidence:** not required; protocol limits and fixture compatibility
have no meaningful rendered behavior of their own.

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

**Visual evidence:** not required; transport equivalence is not visually
distinguishable and is proved by the shared automated scenario assertions.

### Task 37 — Add content checks and representative integration fixtures

**Prerequisites:** Tasks 12–17 and 30–34.

Replace the placeholder sample with a small scene named **Masonry Integration
Fixture** plus Addressable scenes, prefabs, materials,
textures, fonts, audio, effects, Animator controllers, colliders, and one custom
handler fixture. Add test helpers that validate address existence/type, required
root component counts, handler registration, and protocol fixture compatibility.
These remain test/build helpers, not an editor product.

Keep game-owned loading/bootstrap objects visibly separate from Masonry
ownership.

**Acceptance:** a clean Addressables build passes all content checks and the
reference scene can run the end-to-end snapshot/click/command flow through the
fixture engine without manual asset repair.

**Visual evidence:** a Unity Game-view screenshot of the rendered **Masonry
Integration Fixture** scene with its representative content visible.

### Task 37A — Ship the permanent Masonry Demo scene and native engine

**Prerequisites:** Tasks 12A and 25–37.

Create a permanent, game-facing scene named **Masonry Demo** in the reference
project. It is a maintained example and release fixture, not a generated smoke
scene. Its default configuration builds a real Rust demo engine as the
`masonry_rules` native plugin, connects through the production native
transport, receives its initial snapshot from Rust, and sends pointer actions
back to Rust. The Rust engine returns ordinary Masonry commands; the Unity scene
must not move or recolor demo objects through a parallel local gameplay script.

Keep the behavior intentionally small and legible: several cubes identify
themselves visually, pointer enter/exit changes the hovered cube's color, and a
click moves a cube with a short tween. Include at least one change delivered by
poll so connect, action submission, immediate response, and poll are all
visible. A small unobtrusive status surface must show connection state,
transport, last action, last command, and whether the last response was
immediate or polled so reviewers can distinguish the end-to-end protocol from
local animation.

The deterministic walkthrough uses three gray cubes. Hover changes only the
target cube to yellow and exit restores gray. Clicking moves that cube between
two marked positions two world units apart over 500 ms. The next successful
poll makes a different cube blue and labels the response as polled. These
fixed values belong to the demo fixture, not the Masonry protocol.

Provide one command that builds a non-Development native macOS `.app`, launches
it without `DYLD_LIBRARY_PATH` or Editor fallback paths, drives the deterministic
demo walkthrough, and invokes Task 12A to record its evidence. Preserve normal
mouse interaction when the app is launched manually. The demo content and
engine stay permanently buildable from the repository and use the same package,
ABI, codec, Addressables, and player configuration shipped to consumers.

**Black-box acceptance:** the packaged app visibly reaches Running from a Rust
snapshot; hovering a cube sends pointer actions to Rust and applies returned
color commands; clicking sends an action and applies a returned movement tween;
a polled Rust response causes a separate visible change. The capture completes
from the release player with native transport, the bundled dylib, and successful
assertions. No demo gameplay behavior depends on an Editor-only component or a
Unity-side rules shortcut.

**Visual evidence:** a short video from the packaged Release app showing hover
color changes, click-driven cube movement, and the distinct polled change.

### Task 38 — Run a representative performance smoke check

**Prerequisites:** Tasks 09, 12A, 36–37A.

Add one repeatable development-player scenario covering a pointer action, an
immediate response, and a tween while collecting Unity profiler markers and
allocations. Keep it diagnostic and non-gating; expand performance work only
when measurements identify a concrete problem.

**Acceptance:** one local command runs the scenario and prints a compact report
that is useful for spotting regressions without depending on named hardware.

**Visual evidence:** not required; the profiler markers, allocations, and
compact performance report are the meaningful evidence for this diagnostic
task.

## Completion criteria

V1 is complete only when all 40 numbered and lettered tasks are integrated and
the following are true:

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
- The permanent Masonry Demo completes the native C# → Rust → C# path in a
  release macOS app, and every task completed after Task 12A whose acceptance
  calls for visual evidence has its reproducible screenshot or short video.
- No generated contract artifacts exist.
