# Visual evidence capture

`scripts/capture-visual-evidence.py` drives deterministic scenarios in a
non-Development macOS player. Scenario code owns state and assertions; the
driver owns building, media, timing, input dispatch, identity, and cleanup.
By default, input and framebuffer capture stay inside each player. The player
never takes focus, moves the physical pointer, consumes the physical keyboard,
or asks for Accessibility or Screen Recording permission.

## Recommended workflow

Use the same command inputs for both steps:

1. Run smoke validation. This builds or reuses the packaged player and drives
   the complete ready → requested input → assertions-passed protocol without
   recording media.
2. Correct the scenario, framing, labels, or assertions until smoke passes.
3. Run final media capture. The unchanged packaged build is reused after its
   content identity has been verified.

```sh
./scripts/capture-visual-evidence.py \
  --task 37A \
  --scenario battlement-demo-pointer \
  --scene Assets/BattlementDemo/Capture.unity \
  --cargo-package battlement-rules \
  --transport native \
  --smoke

./scripts/capture-visual-evidence.py \
  --task 37A \
  --scenario battlement-demo-pointer \
  --scene Assets/BattlementDemo/Capture.unity \
  --cargo-package battlement-rules \
  --transport native \
  --capture both \
  --dimensions 1280x720
```

Smoke mode produces only a run log. It still launches the exact Release player,
waits for `ready`, dispatches every requested virtual input, and requires the
terminal assertions to pass. It does not initialize framebuffer capture or
FFmpeg.

## Authoring and scaffolding a scenario

Create the repetitive starting point with:

```sh
./scripts/scaffold-visual-capture.py \
  --scenario task-21-card-move \
  --type Task21CardMoveCapture \
  --output Assets/Task21/VisualCapture
```

The command refuses to overwrite files. It creates a formatted scenario
component and `.meta`, then asks Unity to author a scene containing exactly one
matching `BattlementCaptureScenario` and one instance of the reusable capture
shell. The generated scenario demonstrates the minimum ready → pointer click →
passed sequence. Replace its sample assertions and event handling with the task
behavior. Inspect `git status` after scaffolding. Unity may create incidental
project files such as `ProjectSettings/SceneTemplateSettings.json`; keep only
files that are intentional parts of the scenario. The capture cleanliness check
detects changes made during a run, but cannot decide whether a pre-existing
untracked file belongs in the final change.

For hand-authored scenarios, derive one `MonoBehaviour` from
`BattlementCaptureScenario` and place exactly one instance for its stable
`ScenarioName` in the selected scene. After the intended initial frame is
visibly rendered, call `RequestPointerInput` with assertions already observed,
a `CapturePointerAction`, and normalized pointer coordinates. Coordinates have
a top-left origin. Use `RequestKeyInput` with a `CaptureKeyAction` and Input
System `Key` for keyboard transitions. Do not request input from `Awake` or
before asynchronous setup and rendering finish.

The host supports pointer movement, primary-button down/up, and keyboard
down/up. It maintains complete virtual device state and queues exactly the
requested transition through the Input System, so events continue through the
normal EventSystem, raycast/collider, Battlement, Rust, and rendering path. A click
or drag uses explicit down, move, and up requests. The player rejects duplicate
requests, invalid state transitions, non-consecutive dispatches, and successful
completion with a held button or key. Capture fixtures must use the Input
System (`Mouse.current` and `Keyboard.current`), not legacy `Input` APIs.

Do not advance a scenario from a transient `wasPressedThisFrame` or
`wasReleasedThisFrame` observation when the behavior under test may run later
in Unity's update order. Record a durable downstream observation—such as the
Battlement action received or rendered state reached—and request the next input
only after that observation. Requesting another transition before the driver
dispatches the current request fails with `A capture scenario cannot replace an
undispatched input request`.

After the final behavior has rendered, call `SignalPassed` with all observed
assertions, or `SignalFailed` with a diagnostic. Scenario and assertion names
must remain stable and machine-readable.

`SignalPassed` is valid only after the scenario has published `ready` through
an input request. A passive scenario that only waits for rendered state must
still request and observe one harmless interaction—normally a pointer move to
an inert corner—before it signals success. This gives the runner a deterministic
initial frame and proves that its Ready-state handshake completed.

## Reusable capture shell and build-safe colors

`Assets/VisualCapture/BattlementCaptureShell.prefab` provides:

- a deterministic camera and directional key light;
- primary, accent, and success presentation materials;
- title, phase, and legend labels;
- a bootstrap root that can opt into `DontDestroyOnLoad` when ownership
  survival across scene changes is part of the evidence.

Call `SetTitle`, `SetPhase`, and `SetLegend` to describe states that media alone
would make ambiguous. The three authored materials reference
`VisualCaptureUnlit.shader`; runtime code never discovers their shader with
`Shader.Find`. Because the scene references the prefab, materials, and shader,
Unity retains them in a non-Development Release player.

The default material assigned by `GameObject.CreatePrimitive` is not a
build-safe substitute: a primitive that looks correct in the Editor can render
magenta in the packaged Release player. Give runtime-created geometry an
authored material through a serialized scene or prefab reference. Do not depend
on the default primitive material or runtime shader discovery.

Every capture build validates the reusable shader, all three materials, the
shell prefab, the matching scenario count, and—when a shell is present—the
scene dependency graph. A missing or unsupported asset is reported by exact
path before player launch. The durable
`Assets/VisualCapture/Fixtures/ReleaseShellScenario.unity` fixture exercises the
authored blue, cyan, and green materials in a packaged Release player; magenta
indicates a failed build-safe material contract.

Regenerate the shared prefab and materials only after intentionally editing the
shell definition or palette:

```sh
unity -batchmode -nographics -quit -projectPath "$PWD" \
  -executeMethod Battlement.Editor.VisualCaptureAssets.Rebuild
```

## Capturing sample games

Use `scripts/capture-sample-visual-evidence.py` for a standalone Unity sample,
especially when the sample enforces a zero-C# contract. The wrapper:

1. copies the sample into a temporary project;
2. overlays `Assets/VisualCapture`, the sample capture build method, and the
   current `Packages/com.battlement.client` contents;
3. builds the sample's Rust plugin from the supplied Cargo manifest;
4. builds a non-Development macOS player with the plugin and Addressables
   catalog embedded; and
5. delegates input, assertions, framebuffer capture, identity, caching, and
   cleanup to `capture-visual-evidence.py`.

The checked-out sample is never modified and does not acquire capture-only C#.
The copied Battlement package participates in the content fingerprint, so changing
runtime or shader code invalidates the cached player rather than silently
reusing an older build.

### Add a sample scenario

Capture behavior belongs in the repository harness, not in the sample:

- Add a `BattlementCaptureScenario` subclass under
  `Assets/VisualCapture/Fixtures/`. Give `ScenarioName` a stable,
  command-line-safe value.
- Register that scenario in `Assets/Editor/SampleVisualCaptureBuild.cs`.
  `AddScenario` opens the sample's ordinary scene inside the disposable copy,
  adds the capture component, and saves a generated capture scene. Do not add a
  capture scene or C# file to the sample itself.
- Wait for durable rendered state before the first request. For a Battlement game,
  this normally means the Rust snapshot has created its camera, board, or other
  recognizable renderers.
- Request a click as separate move, left-button-down, and left-button-up
  transitions. Observe each dispatched transition before requesting the next.
- Pass only after the client-visible result is durable. A polling scenario must
  wait for the polled result itself, not merely for elapsed time.
- Publish assertion names that describe the production boundary, such as
  `rust-snapshot-rendered`, `human-x-rendered`, and `delayed-ai-o-rendered`.

The default `in-player` input driver creates virtual Input System devices in
the packaged player. It exercises normal raycasting and Battlement action handling
without focusing the window, moving the physical pointer, consuming keyboard
input, or requiring Accessibility permission. Do not select `macos-hid` for
routine sample capture.

### Smoke before recording

Run smoke validation with the same project, scene, scenario, transport, and
dimensions intended for final evidence. Smoke mode builds or reuses the exact
packaged player and drives the complete interaction, but retains only a log:

```sh
./scripts/capture-sample-visual-evidence.py \
  --sample-project samples/tictactoe \
  --cargo-manifest samples/tictactoe/rules/Cargo.toml \
  --task tictactoe \
  --scenario tictactoe-sample \
  --scene Assets/Scenes/TicTacToe.unity \
  --dimensions 1280x720 \
  --smoke
```

Do not proceed because the player merely launched. Smoke succeeds only after
the scenario reaches `SignalPassed`, every input transition is balanced, the
packaged plugin remains loaded, and repository cleanliness is unchanged.

### Retain screenshots or video

For a stable rendered result, capture PNG evidence:

```sh
./scripts/capture-sample-visual-evidence.py \
  --sample-project samples/tictactoe \
  --cargo-manifest samples/tictactoe/rules/Cargo.toml \
  --task tictactoe \
  --scenario tictactoe-sample \
  --scene Assets/Scenes/TicTacToe.unity \
  --dimensions 1280x720 \
  --capture png
```

This retains a cursor-free `before.png` after the initial ready signal and an
`after.png` after the terminal assertions. Use `--capture video` when timing,
animation, ordering, or polling delay is the evidence. Use `--capture both`
when reviewers need the interaction and clean endpoint frames; video requires
the FFmpeg setup described below.

The Tic-Tac-Toe scenario clicks the board through virtual input, waits for the
human X, then waits for the AI O returned by the 500 ms poll. Its after-frame is
not accepted until both marks have produced new renderers.

### Locate and inspect the evidence

Successful runs print absolute artifact paths under:

```text
artifacts/visual-evidence/<revision-and-fingerprint>/<task>/<run-id>/
```

The directory contains the retained media and run log. Open the final PNG or a
representative video frame and compare the actual player render against the
source assets or acceptance criteria. Scenario assertions prove that the
intended state was reached; visual inspection proves that the frame itself is
legible and undistorted. Report both forms of evidence.

If capture fails, read the retained run log first. The runner prints its path
at startup and retains the Unity player log beside it on failure. Ready and
capture timeouts also print the relevant player-log error or exception block,
so diagnose that runtime failure before treating the timeout as a scenario-only
problem. A ready timeout can still mean the scenario never observed its initial
durable state or never requested input; an assertion timeout can mean input was
dispatched but the scenario did not observe the required downstream render. A
build failure retains the isolated Unity build log. Never substitute a
screenshot from a manually launched or stale `.app` for a failed deterministic
capture.

## Initial video hold and paired screenshots

Video recording starts only after the scenario publishes its first `ready`
request. The driver then keeps the rendered starting state behavior-free for at
least two seconds before dispatching the first input. The before screenshot is
taken during that interval. Most videos should retain this default so a viewer
can parse the initial state before motion begins.

Use a different nonnegative duration only when the evidence needs it:

```sh
--initial-hold-seconds 3.5
```

Use `--initial-hold-seconds 0` only when immediate behavior is itself the
subject of the evidence. The override never injects task behavior; it merely
allows the first scenario-requested input to dispatch immediately after
recording is ready.

For `--capture png` or `--capture both`, the driver retains two verified files:

- `<run-id>-before.png`, after the first ready signal and before input;
- `<run-id>-after.png`, after every scenario assertion passes.

Both images must match `--dimensions`; a mismatch fails the run. The log names
both absolute paths. `--capture video` records only the MP4, while `both`
retains the video and both PNGs.

The player captures the completed framebuffer, including overlay UI and
post-processing, with one asynchronous GPU readback outstanding at a time.
PNGs encode the next completed cursor-free frame and are published atomically.
For video, the player composites a small capture-only cursor after readback and
streams raw BGRA frames to an external FFmpeg process at a monotonic 30 Hz.
Slow rendering repeats the newest completed frame, so a five-second recording
still contains exactly 150 frames and represents five seconds of wall time.

FFmpeg must expose `h264_videotoolbox`; the driver selects `ffmpeg` from `PATH`
or accepts an absolute executable through `--ffmpeg PATH`. It records the
executable's version and SHA-256, encodes H.264/yuv420p MP4 without audio, and
publishes the final file only after FFmpeg succeeds. FFprobe then verifies the
codec, dimensions, 30 fps, exact frame count, and duration. FFmpeg is a local
developer prerequisite and is neither copied into the player nor included in
the Unity build-cache key.

Preflight a video capture before spending time on a build:

```sh
command -v ffmpeg
command -v ffprobe
ffmpeg -hide_banner -encoders 2>/dev/null | rg h264_videotoolbox
```

On a Homebrew-managed macOS workstation, `brew install ffmpeg` supplies both
executables. Otherwise install them through the workstation's package manager
or pass the FFmpeg executable explicitly with `--ffmpeg`.

## Build reuse, isolation, and cleanliness

The driver fingerprints relevant files under `Assets`, `Packages`,
`ProjectSettings`, `scripts`, and `crates`, plus Cargo manifests, the selected
scene/scenario/transport, and a prebuilt plugin digest when supplied. It builds
inside a disposable project copy and stores the resulting app in a
content-addressed cache under
`~/Library/Caches/Battlement/visual-capture/` by default. This user-level cache is
shared by separate worktrees. Use `--build-cache PATH` to relocate it.

Reuse occurs only when the content fingerprint, scene, scenario, and Unity
version exactly match the cache manifest. Editing a relevant tracked or
untracked input produces a different key. Each cache key has a cross-process
file lock and is revalidated after the lock is acquired. Completed builds are
published by atomic rename. An incomplete or invalid entry is discarded and
rebuilt, so a stale player is never silently reused. Framing,
recording-duration, hold, and interaction retries do not affect player content
and can reuse the build.

Five capture slots permit five independent packaged players and encoders at
once; two build slots limit expensive Unity builds. Every run has its own
status, sequenced commands, acknowledgements, logs, temporary files, exact
player PID, and encoder PID. Commands are atomically published with consecutive
IDs, so one run cannot consume another run's input. Failure cleanup terminates
only those two owned processes. Five consumers of an uncached identical build
wait on one cache-key publisher and then reuse its result.

Unity imports, plugin staging, project serialization, and generated project
files occur only in the disposable copy. On every exit—including failures—the
driver compares the caller worktree's complete Git status with its starting
state. Any new, removed, or modified repository file fails the run and is
listed in the log. Pre-existing user changes are preserved byte-for-byte by the
workflow rather than backed up and restored.

## Artifact identity and output layout

Evidence may include uncommitted task work, so a clean-looking `HEAD` is not a
sufficient identity. Each run records both the source commit and a SHA-256
content fingerprint. Its artifact directory uses both:

```text
artifacts/visual-evidence/
  <commit>-<fingerprint-prefix>/
    <task-id>/
      <run-id>/
        <run-id>.log
        <run-id>-before.png
        <run-id>-after.png
        <run-id>.mp4
```

The log also records whether a verified packaged build was reused, the first
input's elapsed time from recording start, every retained media path, passed
assertions, and the repository cleanliness result. Existing run directories
are never overwritten. When the console reports only a high-level capture
failure, inspect `<run-id>.log` in the corresponding artifact directory for the
player, compiler, encoder, and protocol diagnostics. Failed runs retain this
log even when they publish no media.

## Options and prerequisites

Use `--plugin PATH` instead of `--cargo-package NAME` to stage a prebuilt
host-architecture `libbattlement_rules.dylib`. Omit both for a scenario without a
native plugin and choose `--transport http` or `--transport none`. Additional
options include `--artifact-root PATH`, `--build-cache PATH`, `--run-id ID`,
`--video-seconds N`, `--interaction-timeout N`, and `--show-overlay`. Choose a
video duration long enough for the initial hold, every driver round trip,
intentional delay or tween, and a safety margin. Six seconds can be too short
for a multi-step pointer scenario even when smoke passes; use 10–12 seconds when
uncertain. The run fails if recording finishes before the scenario passes.

The default drivers are `--input-driver in-player` and `--media-driver
in-player`. Input and media drivers are independent and may be mixed. If the
player reports that its framebuffer format does not support `ReadPixels`, or
that asynchronous GPU readback failed, retain deterministic virtual input and
switch only media capture:

```sh
./scripts/capture-visual-evidence.py \
  --task 32 \
  --scenario task-32-pointer-actions \
  --scene Assets/Task32/VisualCapture.unity \
  --cargo-package battlement-rules \
  --transport native \
  --input-driver in-player \
  --media-driver screen-capture-kit \
  --capture both \
  --dimensions 1280x720 \
  --video-seconds 12
```

This hybrid preserves deterministic, cursor-independent input but requires
Screen Recording permission and one cross-process legacy slot. Use
`--input-driver macos-hid` only when the evidence specifically needs actual OS
input. The physical pointer already has a position before the first requested
transition, so an object under it can receive ambient hover and advance a
poorly guarded scenario. Keep the initial target away from the physical cursor
and gate progress on acknowledged, requested behavior. macOS HID input also
takes focus, requires Accessibility permission, and shares the legacy slot.

Capture requires macOS, the project-pinned Unity editor, Xcode command-line
tools, an external FFmpeg/FFprobe installation for in-player video, and a
logged-in GUI session with Metal. Rust is required with `--cargo-package`.
Routine in-player runs require neither Accessibility nor Screen Recording
permission. Headless and remote-login capture are not supported.
