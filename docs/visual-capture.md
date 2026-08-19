# Visual evidence capture

`scripts/capture-visual-evidence.sh` is the reusable macOS Release-player
capture driver. Task-specific scenes and scenario implementations are inputs to
the driver, not part of the infrastructure. A one-off scenario may remain
uncommitted and be deleted after its retained evidence has been reviewed.

## Authoring a scenario

Create a `MonoBehaviour` derived from `MasonryCaptureScenario` and place exactly
one instance for its stable `ScenarioName` in an authored scene under `Assets`.
The build preflight rejects a scene with zero or multiple matching scenarios.

Implement `BeginCapture` to establish deterministic state and timing. Once the
intended starting frame is visibly rendered, call `RequestInput` with the
assertions already observed, a `CaptureInput`, and its normalized pointer
position. Coordinates use a top-left origin, so `(0.5, 0.5)` selects the center
of the player content window. Do not request input from `Awake` or before
asynchronous setup and rendering have completed.

The available host inputs are pointer movement, primary-button down, and
primary-button up. The capture driver sends exactly the requested event, with
no implicit movement, click, or delay. A scenario may request any number of
inputs: handle each real Unity event, wait for whatever frames or asynchronous
work the behavior requires, then publish the next request. For example, a
hover walkthrough can move outside the target, move onto it, wait for the color
change to render, move away, and then pass without ever clicking. A click or
drag is expressed explicitly with separate button-down and button-up requests.

After the sequence, call `SignalPassed` with the complete set of observed
assertion names. Call `SignalFailed` when asynchronous work or an assertion
fails. Exceptions thrown synchronously by `BeginCapture` are reported
automatically. `ShowCaptureOverlay` tells a scenario whether the caller
requested capture-only labels; such overlays remain hidden otherwise.

Scenario names and assertion names are stable machine-readable identifiers.
Keep initial data, random seeds, clock progression, focus target, and completion
conditions deterministic. Player-visible labels should explain states that
would otherwise be ambiguous in media.

## Running a capture

For a scenario that builds its native engine from a Cargo package:

```sh
./scripts/capture-visual-evidence.sh \
  --task 37A \
  --scenario masonry-demo-pointer \
  --scene Assets/MasonryDemo/Capture.unity \
  --cargo-package masonry-rules \
  --transport native \
  --capture both \
  --dimensions 1280x720
```

Use `--plugin PATH` instead of `--cargo-package NAME` to stage an already-built
host-architecture `libmasonry_rules.dylib`. Omit both for a scenario that does
not use a native plugin, and select `--transport http` or `--transport none`.

The command also accepts `--artifact-root PATH`, `--run-id ID`,
`--video-seconds N`, `--interaction-timeout N`, and `--show-overlay`. Choose a
video duration long enough to include the complete requested sequence; capture
fails if recording ends first. Existing run directories are never overwritten.
The ignored default root is
`artifacts/visual-evidence/<revision>/<task-id>/<run-id>/`.

## Host prerequisites and outputs

Capture requires macOS, the project-pinned Unity editor, Rust when using
`--cargo-package`, Xcode command-line tools, `jq`, a logged-in GUI session, and
permission for the invoking app under **Privacy & Security → Screen & System
Audio Recording** and **Accessibility**. Preflight fails before building when
either permission is missing. Headless and remote-login capture are not
supported.

Each successful run retains requested 1280×720 PNG and/or 30 fps H.264 MP4
media by default and a concise log of the run and passed assertions.

The driver removes only infrastructure-owned transient state: its staged
plugin, `.app`, raw Unity/player logs, helper binary, power assertion, and
player process. It does not delete the caller's authored scenario source or
scene. A task author decides whether those inputs are durable fixtures or
one-off local artifacts. A failed run is retained only as diagnostic output,
never as successful visual evidence.
