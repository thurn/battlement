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
  --scenario masonry-demo-pointer \
  --scene Assets/MasonryDemo/Capture.unity \
  --cargo-package masonry-rules \
  --transport native \
  --smoke

./scripts/capture-visual-evidence.py \
  --task 37A \
  --scenario masonry-demo-pointer \
  --scene Assets/MasonryDemo/Capture.unity \
  --cargo-package masonry-rules \
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
matching `MasonryCaptureScenario` and one instance of the reusable capture
shell. The generated scenario demonstrates the minimum ready → pointer click →
passed sequence. Replace its sample assertions and event handling with the task
behavior.

For hand-authored scenarios, derive one `MonoBehaviour` from
`MasonryCaptureScenario` and place exactly one instance for its stable
`ScenarioName` in the selected scene. After the intended initial frame is
visibly rendered, call `RequestPointerInput` with assertions already observed,
a `CapturePointerAction`, and normalized pointer coordinates. Coordinates have
a top-left origin. Use `RequestKeyInput` with a `CaptureKeyAction` and Input
System `Key` for keyboard transitions. Do not request input from `Awake` or
before asynchronous setup and rendering finish.

The host supports pointer movement, primary-button down/up, and keyboard
down/up. It maintains complete virtual device state and queues exactly the
requested transition through the Input System, so events continue through the
normal EventSystem, raycast/collider, Masonry, Rust, and rendering path. A click
or drag uses explicit down, move, and up requests. The player rejects duplicate
requests, invalid state transitions, non-consecutive dispatches, and successful
completion with a held button or key. Capture fixtures must use the Input
System (`Mouse.current` and `Keyboard.current`), not legacy `Input` APIs.

After the final behavior has rendered, call `SignalPassed` with all observed
assertions, or `SignalFailed` with a diagnostic. Scenario and assertion names
must remain stable and machine-readable.

## Reusable capture shell and build-safe colors

`Assets/VisualCapture/MasonryCaptureShell.prefab` provides:

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
  -executeMethod Masonry.Editor.VisualCaptureAssets.Rebuild
```

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

## Build reuse, isolation, and cleanliness

The driver fingerprints relevant files under `Assets`, `Packages`,
`ProjectSettings`, `scripts`, and `crates`, plus Cargo manifests, the selected
scene/scenario/transport, and a prebuilt plugin digest when supplied. It builds
inside a disposable project copy and stores the resulting app in a
content-addressed cache under
`~/Library/Caches/Masonry/visual-capture/` by default. This user-level cache is
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
are never overwritten.

## Options and prerequisites

Use `--plugin PATH` instead of `--cargo-package NAME` to stage a prebuilt
host-architecture `libmasonry_rules.dylib`. Omit both for a scenario without a
native plugin and choose `--transport http` or `--transport none`. Additional
options include `--artifact-root PATH`, `--build-cache PATH`, `--run-id ID`,
`--video-seconds N`, `--interaction-timeout N`, and `--show-overlay`. Choose a
video duration long enough for the initial hold and complete interaction; the
run fails if recording finishes before the scenario passes.

The default drivers are `--input-driver in-player` and `--media-driver
in-player`. The old native path remains available with `--input-driver
macos-hid --media-driver screen-capture-kit`; selecting either legacy driver
also takes a single cross-process legacy slot. Use that combination only for a
serialized release smoke covering actual OS input and native window capture.
It takes focus and requires the corresponding Accessibility and Screen
Recording permissions.

Capture requires macOS, the project-pinned Unity editor, Xcode command-line
tools, an external FFmpeg/FFprobe installation for in-player video, and a
logged-in GUI session with Metal. Rust is required with `--cargo-package`.
Routine in-player runs require neither Accessibility nor Screen Recording
permission. Headless and remote-login capture are not supported.
