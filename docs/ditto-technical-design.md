# Battlement Ditto technical design

Status: proposed

## Summary

Battlement Ditto is the screenshot testing, visual review, and rapid debugging
tool for Battlement games. It builds or reuses an immutable Unity player, runs a
short TOML scenario, and returns screenshots, ordered logs, timings, and a
stable machine-readable result. Saved screenshots can be compared in tests or
opened in a local review interface.

The **Unity player** is the long-lived presentation and input process. The
**Rust engine** is the native game-state engine that the player connects to;
Ditto destroys and recreates it for every scenario. The Unity player applies
snapshots and commands from that engine but does not own the game rules.

Ditto is designed for the inner development loop as much as for CI. It keeps a
player, browser or Simulator, control connection, loaded Unity assets, and image
comparator warm while it creates a fresh Rust engine for each scenario. Given a
matching cached build, the target is dozens of useful checks in seconds without
rebuilding or restarting between scenarios. Build time is reported separately.

The first version runs on Apple silicon and Intel macOS hosts and targets macOS
players, Unity WebGL, and iOS Simulator. Every retained image is a PNG.
Battlement's own baseline store uses Cloudflare R2, while other repositories
may use R2 or a filesystem. Git LFS is not part of the design.

The current visual-capture workflow remains operational until Ditto, its sample
scenarios, and replacement CI checks are all working. The final migration is a
single cutover described in [Adoption and cutover](#adoption-and-cutover).

## Related information

The following Battlement documents provide the runtime and logging foundations
that Ditto extends:

- [Battlement technical design](technical-design.md)
- [Battlement file logging design](file-logging-design.md)
- [Battlement implementation plan](implementation-plan.md), which identifies
  Tollgate as the repository's CI and promotion mechanism
- [Current visual-capture workflow](visual-capture.md), which remains the
  active workflow until the Ditto cutover

The implementation must use the primary sources below when pinning tools or
implementing platform adapters:

- [ODiff source and documentation][odiff]
- [ODiff v4.5.0 release][odiff-release]
- [NativeWebSocket source and documentation][native-websocket]
- [Unity ScreenCapture API][unity-screen-capture]
- [Unity asynchronous GPU readback API][unity-async-readback]
- [Unity Web networking restrictions][unity-web-networking]
- [Canvas `toBlob`][canvas-to-blob]
- [Canvas `captureStream`][canvas-capture-stream]
- [Cloudflare R2 S3 API compatibility][r2-s3]
- [Cloudflare R2 with the Rust AWS SDK][r2-rust]
- [Cloudflare R2 API token permissions][r2-auth]
- [Cloudflare R2 pricing][r2-pricing]
- [GitHub repository limits][github-limits]
- [Git Large File Storage behavior][github-lfs]
- [`img-comparison-slider`][comparison-slider]
- [`@panzoom/panzoom`][panzoom]
- [Jest snapshot update behavior][jest-snapshots]
- [FFmpeg documentation][ffmpeg]

## Goals and principles

Ditto has four primary goals:

1. Make it quick to verify a game change through the same rendered player that
   a user sees.
2. Make failures diagnosable through correlated screenshots, logs, timings,
   and stable structured output instead of hangs or isolated image files.
3. Make simple throwaway scenarios easy for agents to author and run at the end
   of a development session.
4. Make broad sample screenshot coverage cheap enough to run in Battlement CI.

The design follows these principles:

- A scenario uses player-facing input and visible Battlement state. It does not
  mutate Unity objects through a private testing API.
- **Battlement-owned behavior** is presentation or an effect created and timed
  by Battlement commands. Ditto makes that behavior deterministic, but does not
  claim to control arbitrary game scripts or shaders.
- A test run starts from a fresh Rust engine even when expensive host processes
  stay warm.
- Missing screenshots, runtime errors, panics, and control failures become
  bounded failures with retained diagnostics.
- Baseline data is easy to obtain after a normal clone without making every Git
  history revision carry every image revision.
- Human and machine interfaces describe the same run. Agents do not need to
  scrape terminal prose or operate the review page to understand a failure.

## Command-line experience

The workspace installs a `ditto` executable and a `cargo-battlement`
executable. These two forms invoke the same parser and implementation:

```text
ditto run
cargo battlement ditto run
```

Both forms search from the current directory upward for `ditto.toml` unless
`--config PATH` is supplied. Paths in the suite are relative to the directory
containing that file. Commands never depend on the current directory after the
suite has been loaded.

### Commands

`ditto run [FILTER ...]` executes selected scenarios and compares reached
screenshot checkpoints with the baseline manifest. A missing baseline is a
failure in an ordinary run.

`ditto capture [FILTER ...] [--fragment PATH|-]` executes scenarios without
fetching, comparing, accepting, or uploading baselines. `--fragment PATH` reads
a full suite or small scenario fragment; `--fragment -` reads a fragment from
standard input. Bare `ditto capture` uses the discovered suite. Positional
arguments are always scenario filters, never paths. This is the main throwaway
workflow for agents. Screenshots, logs, and `result.json` stay in the local run
directory. Runtime and assertion failures still fail the command.

`ditto review [RUN]` opens the local review application for a retained run. If
`RUN` is omitted, it selects the newest run containing a failed image or an
unreviewed capture.

`ditto fetch [FILTER ...]` downloads the baselines required by selected
scenarios. `ditto fetch --all` downloads every object named by `ditto.lock` and
is the supported way to prewarm a fresh clone for offline work.

`ditto list [FILTER ...]` resolves configuration and prints profiles,
scenarios, checkpoint names, and skip reasons without launching a player.

`ditto doctor [--profile NAME]` checks the host, Unity installation, selected
platform tools, ODiff, FFmpeg when requested by the suite, cache permissions,
baseline-store reachability, and write credentials. Read and write checks are
reported separately.

`ditto clean runs`, `ditto clean builds`, and `ditto clean baselines` prune the
corresponding local cache while respecting active leases. `ditto clean storage`
performs the remote tombstone cleanup described under
[Replacement retention](#replacement-retention). It requires write credentials
and never guesses which unreferenced objects are safe to delete.

`ditto storage publish` is a default-branch CI operation. It publishes the
merged `ditto.lock` as the canonical remote reachability index and creates
replacement tombstones. Feature branches and local acceptance never invoke it
automatically.

### Common options

The following options have the same meaning for both entry points:

- `-u` and `--update` accept every screenshot checkpoint reached by `run`.
- `-w` and `--watch` keep the selected player and review session warm and rerun
  affected scenarios after changes.
- Positional filters and `--scenario GLOB` select by scenario name. Repeating a
  selector takes the union; `--exclude GLOB` then removes matches.
- `--profile NAME` selects exactly one profile. A command never combines
  profiles into one run.
- `--bail[=N]` stops after the first failure or after `N` failures. Without it,
  recovery is attempted and remaining scenarios continue.
- `--no-build` requires an existing matching immutable build and fails if none
  is cached.
- `--review` opens the review application after the run. An ordinary failed run
  does not open a browser by itself.
- `--json` writes the stable result object to standard output. Human progress
  goes to standard error, so the JSON stream is not mixed with prose. In watch
  mode it writes one newline-delimited result object after each completed cycle.
- `--output PATH` additionally copies `result.json` to the requested path.

`--watch --update` is rejected. Automatically accepting every change in a
long-running process is too easy to overlook; a developer must leave watch mode
and run one explicit update. In watch mode, `--output` is an atomically replaced
view of the latest completed cycle. Every cycle still has its own immutable run
directory and `result.json`.

The process exits with `0` when every selected scenario passes or skips, `1`
when a scenario, assertion, or image comparison fails, `2` for configuration
or infrastructure failure, and `130` after an interrupt. A run with no selected
scenarios is a configuration failure unless `--allow-empty` is present.

### Fast verification loop

Ditto is also the standard verification record for routine development. A
scenario may contain no screenshot steps. Such a scenario can validate setup,
input, visible object assertions, logs, error handling, and timing for a
nonvisual change. `run` succeeds without consulting the baseline store when no
screenshot is reached; `capture` retains the same diagnostics without requiring
a committed suite entry.

```toml
[aliases]
status = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"
[[scenarios]]
name = "menu becomes ready"
[[scenarios.steps]]
assert = { object = "status", state = "text", text = "Ready" }
```

Assertions and unexpected error or fatal records gate the command. Ordinary log
messages, warnings, and phase timings are retained for human or agent inspection
but cannot be asserted declaratively in v1.

A normal agent or developer loop is:

1. Change the game or Battlement.
2. Run an existing scenario with `ditto run`, or author a temporary fragment
   and use `ditto capture`.
3. Read the terminal status and `result.json`. Inspect the correlated JSONL
   span before opening an image when the failure is nonvisual.
4. Open `ditto review` only when visual comparison helps.
5. Use watch mode for repeated edits so the player, comparator, logs, and one
   optional review tab remain warm.

For example:

```text
ditto capture --fragment /tmp/menu-check.toml --profile macos-local \
  --output /tmp/ditto-result.json
ditto review
```

The first command is sufficient for an agent. The review application is never
required to discover status, error IDs, log ranges, screenshot paths, or phase
timings. At the eventual repository cutover, a task must run Ditto when it
changes any player source dependency or claims a player-visible runtime result.
Its handoff names the run ID and result path. An existing suite scenario is
preferred; a throwaway fragment is appropriate when the change does not deserve
permanent coverage.

A documentation-only or repository-tooling task that cannot affect the player
records `Ditto: not applicable` and a short reason instead. This makes the rule
explicit without forcing a meaningless player launch for every worktree.

Standard-input fragments are one-shot and reject `--watch`, because there is no
file to reload. File-backed fragments support watch mode. Changes to staged,
unstaged, and untracked files that participate in the build are all visible to
[source fingerprinting](#builds-and-caches), so an agent can verify its current
worktree before it commits.

## Suite configuration

One `ditto.toml` describes a suite. It contains suite defaults, profiles,
readable aliases for Battlement UUIDs, and linear scenarios. Profiles express
how to launch; scenarios express behavior and do not contain platform branches.

```toml
schema = 1
name = "tictactoe"
default_profile = "macos-local"

[defaults]
step_timeout = "2s"
scenario_timeout = "10s"
seed = 117
motion = "instant"

[defaults.comparison]
threshold = 0.1
anti_alias = true
max_changed_percent = 0.01

[timeouts]
run = "5m"
build = "15m"
launch = "90s"
baseline_download = "2m"
simulator_boot = "5m"

[player]
unity_project = "."
scene = "Assets/Scenes/TicTacToe.unity"
rust_manifest = "rules/Cargo.toml"

[aliases]
top_left = "1f160ce4-dcdc-47ac-9613-31011f8afc96"
new_game = "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6"

[baseline]
kind = "r2"
namespace = "battlement/tictactoe"
public_base_url = "https://baseline.example.net"
account_id_env = "BATTLEMENT_R2_ACCOUNT_ID"
bucket_env = "BATTLEMENT_R2_BUCKET"
access_key_id_env = "BATTLEMENT_R2_ACCESS_KEY_ID"
secret_access_key_env = "BATTLEMENT_R2_SECRET_ACCESS_KEY"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[profiles.web-ci]
target = "webgl"
display = { width = 1280, height = 720, scale = 1.0 }
headless_command = ["chromium", "--headless", "{url}"]

[profiles.iphone-ci]
target = "ios-simulator"
device = "iPhone 16 Pro"
orientation = "portrait"

[[scenarios]]
name = "human wins top row"
fixture = "fresh_match"
seed = 42
motion = "instant"

[[scenarios.steps]]
click = { target = "top_left" }

[[scenarios.steps]]
wait = { object = "new_game", state = "visible" }

[[scenarios.steps]]
name = "opening"
screenshot = { name = "opening-move" }

[[scenarios.steps]]
assert = { object = "new_game", state = "enabled" }
```

`fresh_match` names a native pre-connect setup hook; it does not resolve through
the UUID aliases. See
[Setup and fresh engine isolation](#setup-and-fresh-engine-isolation) for hook
behavior.

The misspelling protection provided by schema validation is strict: unknown
keys, duplicate profile or scenario names, duplicate checkpoint names within a
scenario, invalid UUIDs, and ambiguous step tables are errors. Durations use an
integer followed by `ms`, `s`, or `m`. Configuration errors include the file,
line, key path, and a suggested valid key when one is close.

The account environment variable in the example is illustrative; Battlement's
committed suite uses consistently named `BATTLEMENT_R2_*` variables. Secret
values are never allowed directly in `ditto.toml` or `ditto.lock`.

### Complete suite fields

- **Top level:** `schema`, `name`, `default_profile`, `player`, `profiles`, and
  `scenarios` are required. Schema is integer `1`. Names are nonempty UTF-8;
  scenarios and checkpoints are unique in their containing scope.
- **Player:** `unity_project`, `scene`, and `rust_manifest` are the only fields.
  All are repository-relative. The scene belongs to the Unity project, and the
  manifest builds its native engine. Ditto supplies supported build methods;
  suites cannot name arbitrary editor methods or shell commands.
- **Timeouts:** `run`, `build`, `launch`, `baseline_download`, and
  `simulator_boot` are positive and at most one hour. The shown defaults apply
  member by member. Run time starts after hydration and handshake and includes
  setup, scenarios, comparison, failure capture, and recovery. Watch gives each
  cycle a new run deadline.
- **Defaults:** `step_timeout`, `scenario_timeout`, `seed`, `motion`, and
  `comparison` are optional. Seed is an unsigned 64-bit integer. Motion is
  `instant`, `controlled`, or `real-time`. A scenario may override all except
  comparison, which a screenshot overrides member by member.
- **Comparison:** `threshold` is from `0.0` through `1.0`. `anti_alias` is a
  Boolean. `max_changed_percent` is from `0.0` through `100.0` and is parsed as
  an exact decimal rather than binary floating point.
- **Aliases:** each key is a readable nonempty case-sensitive identifier that
  does not look like a UUID. Every value is a Battlement UUID string.
- **Baseline:** kind is `filesystem` or `r2`. Filesystem requires `root`. R2
  requires `namespace`, `public_base_url`, and four environment-variable names.
  A namespace uses slash-separated letters, digits, periods, underscores, and
  hyphens and has no empty, `.`, or `..` segment.
- **Profile:** macOS and WebGL require `display`; WebGL may add one
  `headless_command` array with exactly one `{url}` argument. iOS Simulator
  requires `device` and an orientation of `portrait`,
  `portrait-upside-down`, `landscape-left`, or `landscape-right`. Fields from a
  different target are errors.

### Profiles

A profile has one of three targets: `macos`, `webgl`, or `ios-simulator`.
macOS and WebGL profiles specify the Unity render size and scale. Ditto rejects
a player whose startup handshake reports different effective values.

An iOS Simulator profile specifies an installed device type and orientation.
Ditto asks Simulator for the resulting pixel dimensions, scale, and safe-area
insets; it does not synthesize an arbitrary resolution. An unavailable device
type is an infrastructure failure with the installed alternatives listed.

The default profile is intended for a developer workstation. CI always names a
profile explicitly. A baseline identity includes the profile, because rendering
engines, safe areas, and device dimensions can legitimately differ.

### Scenario fragments

`capture --fragment` accepts a full suite or a fragment containing `defaults`,
`aliases`, and one or more `scenarios`. A fragment inherits launch and
baseline-neutral settings from the nearest repository `ditto.toml`; `--config`
makes that choice explicit. Standard input uses the same TOML shape and may set
a synthetic name. Its relative fixture and save paths are resolved from the
repository root, not from a temporary file.

A file containing `schema`, `player`, and `profiles` is a full suite and does
not inherit. A fragment may contain only `name`, `defaults`, `aliases`, and
`scenarios`. It inherits the repository suite's player, timeouts, selected
profile, and launch settings. Fragment defaults override repository defaults
member by member. Fragment aliases are added; redefining an inherited alias to
a different UUID is an error. Only fragment scenarios run, so they do not merge
with repository scenarios. CLI values take precedence over the fragment, then
the repository suite, then built-in defaults. Baseline settings are ignored by
`capture` even when inherited.

An agent can therefore run:

```sh
ditto capture --fragment - --profile macos-local --json <<'TOML'
[aliases]
menu = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"
[[scenarios]]
name = "check menu"
[[scenarios.steps]]
assert = { object = "menu", state = "visible" }
[[scenarios.steps]]
screenshot = { name = "menu" }
TOML
```

The stable result identifies a standard-input scenario by its declared name and
content hash. Ditto never writes a fragment into the repository implicitly.

## Scenario model

A scenario is a name, optional setup, optional seed, motion mode, and an ordered
list of steps. Steps are executed serially. A step may have an optional `name`
for results and log correlation and an optional timeout smaller than the
scenario deadline.

Supported action steps are:

- `click`, which moves to the target and performs one press and release;
- `hover`, which moves without changing button state;
- `drag`, which presses at one target, follows deterministic intermediate
  points, and releases at another target;
- `type`, which enters text through virtual keyboard transitions;
- `key`, which presses, releases, or taps an Input System key;
- `wait`, which waits for a number of controlled frames or a black-box object
  condition;
- `assert`, which checks a black-box object condition immediately;
- `screenshot`, which settles, captures, and optionally compares an image; and
- `video`, an experimental bounded recording step.

Every `scenarios.steps` table contains at most `name`, `timeout`, and exactly
one step key from the list above. `name` is optional and unique within the
scenario. `timeout` is a positive duration no greater than the remaining
scenario deadline. The following is a complete example of the step shapes:

```toml
[aliases]
top_left = "1f160ce4-dcdc-47ac-9613-31011f8afc96"
new_game = "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6"
name_field = "8b1968f8-3eb9-4839-8fe3-a01fe9d43bbf"
status = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"

[[scenarios]]
name = "all step shapes"
motion = "controlled"

[[scenarios.steps]]
name = "click by id"
click = { target = "top_left" }

[[scenarios.steps]]
click = { target = [0.5, 0.75] }

[[scenarios.steps]]
hover = { target = "new_game" }

[[scenarios.steps]]
drag = { from = "top_left", to = [0.75, 0.75] }

[[scenarios.steps]]
click = { target = "name_field" }

[[scenarios.steps]]
type = { text = "Ada" }

[[scenarios.steps]]
key = { key = "Enter", action = "tap" }

[[scenarios.steps]]
wait = { frames = 3 }

[[scenarios.steps]]
timeout = "750ms"
wait = { object = "status", state = "text", text = "Ready" }

[[scenarios.steps]]
assert = { object = "new_game", state = "enabled" }

[[scenarios.steps]]
screenshot = { name = "ready" }

[[scenarios.steps]]
[scenarios.steps.screenshot]
name = "strict"
threshold = 0.05
anti_alias = false
max_changed_percent = 0.0

[[scenarios.steps]]
[scenarios.steps.video]
action = "start"
name = "move"
motion = "real-time"
max_duration = "5s"

[[scenarios.steps]]
click = { target = "new_game" }

[[scenarios.steps]]
video = { action = "stop" }
```

An input target is either a UUID or alias string, or a two-element array of
normalized `x` and `y` coordinates. Both coordinates are finite and from `0.0`
through `1.0`. `click` and `hover` require `target`. `drag` requires `from` and
`to`; each independently accepts either target form. There are no additional
input-target fields.

`type` requires one UTF-8 `text` value no larger than 64 KiB and enters it into
the current focus. A scenario clicks the intended field first. `key.key` is a
case-sensitive Unity Input System `Key` enum name. `key.action` is `down`, `up`,
or `tap` and defaults to `tap`. Held keys are allowed between explicit `down`
and `up` steps but must be released before the scenario ends.

`wait` has exactly one of `frames` or `object`. A frame count is a positive
32-bit integer and is allowed only in controlled mode. An object wait also
requires `state`. `assert` always requires `object` and `state`. State is one of
`exists`, `absent`, `visible`, `hidden`, `enabled`, `disabled`, or `text`.
`text` state requires one exact UTF-8 `text` value; every other state rejects a
`text` field.

`screenshot.name` is required. Its three optional comparison values have the
same ranges as `defaults.comparison` and override them member by member. A
screenshot step may not disable comparison in `run`; `capture` makes all
screenshot steps capture-only.

A video uses paired `video` steps, not nested actions. `action = "start"`
requires a unique `name` and may set `motion` and `max_duration`.
`max_duration` defaults to 30 seconds and may not exceed 30 seconds. Actions,
assertions, and screenshots between start and stop execute normally while the
clip records. `action = "stop"` has no other fields. Videos may not overlap, a
stop without a start is an error, and an unclosed video is a configuration
error. Runtime failure stops and finalizes the active clip when possible;
assertion and screenshot outcomes retain their normal status.

Scenario fields are `name`, `fixture`, `save`, `seed`, `motion`, `timeout`, and
`steps`. Fixture is a nonempty native hook name. Save is an opaque
repository-relative file path. They may be used together and are passed to the
hook in that order. Seed and motion follow the suite rules. Timeout is positive,
at most the run deadline, and defaults to `defaults.scenario_timeout`.

Object conditions are limited to facts Battlement already exposes to a player:
`exists`, `absent`, `visible`, `hidden`, `enabled`, `disabled`, and exact
visible text for Battlement-owned UI text. These checks use the latest
game-state snapshot published by the Rust engine and its resulting Unity
presentation. They do not read arbitrary C# fields, invoke game methods, or
treat an object existing off-screen as visible.

### Targeting and input

An input target is either a stable Battlement UUID, an alias resolving to such a
UUID, or normalized render coordinates. Coordinates use a top-left origin and
each value is in the closed range from `0.0` to `1.0`. Pixel coordinates,
hierarchy paths, CSS selectors, object names, and runtime query languages are
not supported.

For an object target, the player computes a visible point inside the rendered
bounds, then verifies it with the same Unity EventSystem, UI Toolkit panel pick,
and physics raycast path used by production input. It samples candidate points
deterministically until it finds one for which the requested object is the
frontmost eligible receiver. A click never bypasses an overlay or invokes an
object action directly. If no point is visible and unobscured, the step fails
with the target bounds, candidate points, and the UUID of the blocking object
when known.

Ditto injects complete virtual Input System state transitions. A click uses a
move frame, press frame, and release frame. A drag uses a press followed by a
fixed number of linearly spaced move frames based on normalized distance, then
a release. Key and text input likewise use balanced transitions. The player
fails reset if a pointer button or key remains held.

Input stays inside the Unity player. Ditto never moves the host pointer, sends
host keyboard events, or asks macOS for Accessibility access. Native framebuffer
capture does not require Screen Recording access, and the macOS player does not
need to be frontmost. WebGL and iOS likewise inject through their authenticated
player bridges rather than automating browser or Simulator chrome.

iOS maps click and drag steps to one-finger touch sequences. A scenario that
contains any hover step is skipped as a whole on iOS Simulator, with
`unsupported_input:hover` in its result. This is a platform skip, not a pass or
failure, and happens before setup is run.

### Setup and fresh engine isolation

Every scenario creates a new native Rust engine inside the long-lived player.
The preceding engine is disconnected and destroyed, Battlement-owned Unity
objects and input devices are reset, log correlation advances to a new session,
and the new engine connects from a clean state. A reset failure causes the
player to be relaunched before the next scenario.

`Connect` gains optional `ditto` data with the scenario identity, motion mode,
and seed. The default seed is stable and may be overridden by suite or scenario.
The presence of this object is the only runtime indication that Ditto is
driving the game; normal players omit it.

Native games may implement one pre-connect Ditto hook. The hook receives an
optional named fixture and optional opaque save bytes loaded from a
repository-relative path. It runs after the old engine is destroyed and before
the new engine connects. Ditto does not define the save format or copy the save
into long-term storage. Unknown fixture names and unreadable saves fail setup.
The hook is native-only in v1; games that require it cannot run that scenario
through an HTTP rules-engine transport.

Most scenarios need no hook. Recreating the engine and providing a stable seed
is the default setup mechanism. Fixture code should establish semantic game
state, not Unity presentation state.

### Settling and deadlines

After every input or setup action, Ditto performs implicit settling before the
next assertion or capture. A player is settled when:

1. Battlement has no queued command groups or pending Rust request.
2. Every finite Battlement-owned operation required by the current motion mode
   is complete.
3. Unity has committed two consecutive rendered frames without a Battlement
   state or layout change.

The player reports the counters used in this decision. Work created after the
second quiet frame belongs to a later step. An explicit frame wait advances an
exact number of controlled frames. An object wait polls a named condition after
each committed frame. These are escape hatches for game-owned asynchronous
behavior, not required ceremony before every screenshot.

Each step has a default two-second deadline and each scenario a default
ten-second deadline. The suite and an individual step or scenario may override
them. A scenario deadline includes setup, actions, settling, captures, and
assertions but excludes player launch, build, baseline download, and Simulator
boot. Image comparison time is recorded separately and is bounded by the run
deadline. Every control request carries its remaining timeout, so a lost
message cannot produce an indefinite wait.

### Motion modes

`instant` is the default. Battlement-owned tweens commit their final values,
Battlement-owned particles are suppressed, and Battlement-owned audio is muted.
This makes state transitions fast without asking scenarios to sleep through
animations.

`controlled` advances a deterministic frame clock only when Ditto requests a
frame. Battlement tweens, particles, and audio timing use that clock. A scenario
may advance exact frames before capturing intermediate states.

`real-time` lets Unity advance normally and is intended for diagnostic captures
and video. Settling still uses Battlement work and quiet-frame signals, but
elapsed time is wall-clock time.

These modes apply only to behavior owned by Battlement. Arbitrary `Update`
methods, coroutines, shaders, custom particle systems, physics, networking, and
random number generators remain the game's responsibility. A diagnostic
warning lists observed non-Battlement animators or particle systems, but Ditto
does not silently disable them.

## Crates and shared tooling

The implementation adds two workspace crates.

`battlement-tooling` is the supported library for discovering Unity and Apple
tools, fingerprinting build inputs, creating immutable player builds, retaining
build logs, and leasing entries in a shared build cache. Its public surface is
narrowly about Battlement tooling; it is not a generic Unity build framework.

`battlement-ditto` contains the suite model, runner, platform adapters, control
protocol, comparison client, baseline stores, run artifacts, and review server.
It provides a library and the `ditto` binary and depends on
`battlement-tooling`.

The existing `battlement-cli` depends on both crates and delegates
`cargo battlement ditto ...` to the same library entry point as the standalone
binary. It does not shell out to `ditto`. Argument parsing, defaults, output,
and exit codes are therefore identical.

`scripts/ci.py` consumes the same discovery, fingerprint, build, and lease
foundations through a small supported invocation surface. This does not turn
the Python script into a public CI framework or require rewriting unrelated CI
orchestration in Rust.

## Builds and caches

Build reuse depends on two distinct content hashes. Neither value uses Git
commit identity or filesystem timestamps.

- The **source fingerprint** identifies the repository inputs that can affect
  the player. It is the SHA-256 of a versioned, sorted list of normalized paths,
  file modes, and file bytes.
- The **build fingerprint** identifies one reusable player output. It hashes the
  source fingerprint together with target, selected profile build settings,
  Unity and Apple toolchain versions, Rust toolchain version, diagnostics and
  capture-adapter versions, native build settings, and build-tool version.
- The immutable build cache uses the build fingerprint. The control handshake
  reports both values. `ditto.lock` records the source fingerprint only as
  diagnostic context; neither fingerprint changes baseline identity.

`battlement-tooling` computes the source dependency closure from declared roots,
not from `git ls-files`:

- Unity roots are the configured scene and project. The closure includes
  `ProjectSettings`, `Packages/manifest.json`, `Packages/packages-lock.json`,
  local package contents, verified registry-package contents, every compiled
  C# or assembly-definition file, and recursive Asset Database dependencies of
  the scene and serialized runner modules.
- Rust roots are the configured Cargo manifest. Cargo metadata supplies all
  reachable local packages. Their manifests, lock file, source, build scripts,
  Cargo configuration, and declared generated inputs enter the closure.
- Native bindings, catalogs, or other generated inputs enter as the generator
  name, generator version, logical input name, and generated byte hash whenever
  a build consumes them. Generated run data, baselines, logs, caches, and editor
  temporaries are excluded.
- Repository-relative roots and symlinks may not escape the repository. This
  prevents an undeclared host file from changing a supposedly reusable build.

Relevant bytes participate whether committed, staged, unstaged, or untracked.
The fingerprint manifest is retained with the run, so `--no-build` can name the
paths whose hashes differ from the nearest cached build.

A successful build is immutable and addressed by the build fingerprint. A
temporary directory becomes visible to readers only after the player, metadata,
fingerprint manifest, and full build log are complete. Concurrent callers share
a per-fingerprint lease. A failed build retains its log but never creates a
reusable cache entry.

A build failure still produces `result.json` with infrastructure status, the
failed phase, compiler or Unity error IDs, and the full build-log path. The
terminal prints a short error summary and that path. Ditto does not launch an
older cached build after the current source fingerprint fails to compile.

All Battlement tools share a user-level build cache with a default 20 GB LRU
limit. The path and limit are configurable, and CI may point them at a restored
job cache. Active builds and running players hold leases and cannot be evicted.
An entry larger than the limit may complete; cleanup then evicts older inactive
entries and reports the remaining oversize entry.

Build time is outside scenario performance measurements and is reported as
`build.created`, `build.reused`, or `build.required_by_no_build`. `--no-build`
fails with the expected fingerprint and a short list of changed fingerprint
inputs when no exact build exists.

### Warm execution and watch mode

A run selects one profile and starts one player, page, or Simulator app. The
target, loaded Unity assets, control channel, and ODiff process stay alive while
scenarios execute serially. Rust engine instances never overlap.

CI obtains concurrency by running separate sample suites or profiles in
parallel, each with its own target and run directory. A single suite process
does not multiplex scenarios across one player because that would make logs,
input, and recovery harder to reason about.

Watch mode keeps the target, ODiff, and one review tab alive. It debounces file
events and reacts as follows:

- scenario or suite changes reload configuration, reset, and rerun selected
  scenarios;
- `ditto.lock` changes recompare retained current images when possible;
- a compiled input change computes a new fingerprint, builds if necessary, and
  relaunches once before rerunning; and
- review acceptance refreshes the active result without opening another tab.

Each watch cycle has its own run ID and records whether it reused images, reset
a player, launched a build, or performed comparison only. Watch mode keeps the
current player alive until a replacement build is complete.

After a replacement build fails:

- The failed cycle has infrastructure status, no scenarios, and the build log.
  The review tab switches to it while retaining navigation to older runs.
- The stale player remains alive but idle. It never runs scenarios for the new
  source fingerprint.
- Another build-input edit creates a new source fingerprint and retries.
- A scenario-only edit is queued but does not retry the same broken build.
  Pressing `r` in the watch terminal explicitly retries that fingerprint.
- A lock-only edit may refresh comparison of a retained older run, visibly
  labeled with its older source fingerprint. It cannot create a passing result
  for the current source.
- Review acceptance updates the selected retained run and does not retry a
  build.

The state sequence is therefore:

```text
code edit -> build F2 fails -> old player idle, F2 result shows build log
scenario edit -> scenario queued, no build
lock edit -> older retained images recompare, still no F2 scenario result
code edit -> build F3 passes -> old player replaced -> queued scenario runs
```

This preserves warm resources without presenting results from old code.

## Runner diagnostics and control

`BattlementRunner` gains a serialized `runner diagnostics` option that is
enabled by default. When enabled, development and release players include the
existing log viewer and the Ditto control bridge. When disabled, ordinary
Battlement file logging remains active, but the viewer and bridge are omitted
or stripped and Ditto reports that the build cannot be automated.

The bridge remains dormant until launch supplies a random, one-run credential.
The credential has at least 128 bits of entropy, is never written to suite or
lock files, is redacted from command lines and logs where the platform permits,
and expires with the bridge. Connections bind to loopback only. The local
review server uses a separate random token for state-changing requests.

### Control transport

The player initiates a loopback WebSocket connection to Ditto. The Unity client
pins NativeWebSocket `2.0.7` at commit
`d516a8b378e233514ba0096c4502679cfaf67e5d`. WebGL uses its JavaScript
implementation; native players use its managed implementation. Ditto rejects
non-loopback peers and requires the launch credential in the first message.

Requests and replies are serial. JSON text frames carry commands, status,
diagnostics, and metadata. A successful screenshot reply sends one small JSON
header followed by one binary frame containing the PNG bytes. The protocol
does not base64-encode images. Every request has an ID and timeout; replies
repeat the ID. Unknown fields are rejected for the initial schema version.

Both sides send heartbeats during idle and active requests. Heartbeats and
events may interleave with a request, but never between a binary header and its
binary frame. Missing heartbeats, malformed frames, duplicate replies,
unexpected binary data, and expired deadlines fail the current scenario and
close the connection. The runner then attempts target recovery.

The startup handshake validates:

- protocol version and one-run credential;
- target platform and capture adapter;
- build and source fingerprints;
- Unity version and diagnostics setting;
- effective render dimensions, scale, orientation, and safe area; and
- supported input, motion, log, capture, and video capabilities.

A mismatch is an infrastructure failure before any scenario runs. The full
handshake, with credentials redacted, is retained in `result.json`.

### Protocol version 1

The Rust protocol types, generated C# types, and a checked-in JSON Schema named
`ditto-control-v1.schema.json` define the wire format. CI validates every
example below against that schema and round-trips it through both languages.
All JSON is UTF-8. Text frames are limited to 1 MiB and binary frames to 64 MiB.
Integers are JSON integers. Fields ending in `_ms` are integer milliseconds;
fields ending in `_bytes` are integer byte counts.

The first player frame is a hello object. Its required shape is:

```json
{
  "type": "hello",
  "protocol": 1,
  "credential": "<one-run secret>",
  "platform": "macos",
  "capture_adapter": "unity-async-readback-png-v1",
  "build_fingerprint": "sha256:<hex>",
  "source_fingerprint": "sha256:<hex>",
  "unity_version": "6000.0.56f1",
  "diagnostics": true,
  "display": {
    "width": 1280,
    "height": 720,
    "scale": 1.0,
    "orientation": null,
    "safe_area": [0, 0, 1280, 720]
  },
  "capabilities": ["click", "drag", "hover", "png"]
}
```

Platform is `macos`, `webgl`, or `ios-simulator`. Orientation is null except on
iOS. Safe-area values are integer pixels in `x`, `y`, `width`, `height` order
with a bottom-left Unity origin. The schema fixes the capability enum and
capture-adapter names. Ditto replies once with `hello-accepted` or closes with a
redacted handshake error.

A request has this envelope:

```json
{
  "type": "request",
  "id": 7,
  "command": "capture",
  "timeout_ms": 2000,
  "payload": {
    "scenario_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
    "step_id": 3,
    "format": "png"
  }
}
```

IDs are monotonically increasing unsigned 64-bit integers and one request may
be active. Timeout is positive and cannot exceed the remaining scenario or run
deadline. Ditto and the player each start a local monotonic countdown when the
frame is sent or received; wall-clock timestamps never decide expiry. The
allowed commands are `reset`, `prepare`, `connect`, `input`, `wait`, `assert`,
`capture`, `video-start`, `video-stop`, `freeze`, `flush-logs`, and `shutdown`.
Every payload includes `scenario_id` and `step_id`, except reset and shutdown.
Their remaining fields are fixed as follows:

- `reset` has no payload fields and destroys the engine, Unity presentation,
  and virtual input state.
- `prepare` has nullable `fixture` and nullable `save`. Save contains decoded
  byte length, SHA-256, and base64 bytes. Decoded saves are limited to 512 KiB.
- `connect` has scenario name, seed, and motion and creates `Connect.ditto`.
- `input` has one validated click, hover, drag, type, or key object from the
  suite schema.
- `wait` and `assert` have one validated condition from the suite schema.
- `capture` has only `format = "png"`.
- `video-start` has name, motion, and maximum duration; `video-stop` has no
  additional fields.
- `freeze`, `flush-logs`, and `shutdown` have no additional fields.

The player verifies save length and hash before calling the native setup hook.
Any payload field not listed above is rejected.

A non-image reply is:

```json
{
  "type": "reply",
  "id": 7,
  "status": "ok",
  "payload": {}
}
```

Status is `ok` or `error`. An error payload requires `error_id`, `kind`, and
`message`; kind is `assertion`, `runtime`, `timeout`, `protocol`, or
`unsupported`. The player sends no later reply for an errored request.

A successful capture first sends this header and then exactly one binary frame:

```json
{
  "type": "binary-reply",
  "id": 7,
  "media_type": "image/png",
  "size_bytes": 42817,
  "sha256": "<hex>",
  "width": 1280,
  "height": 720
}
```

The receiver verifies frame length, hash, PNG signature, and decoded dimensions
before completing the request. No other frame may occur between the header and
binary frame.

Unsolicited player events use `type = "event"`, an event name, a monotonically
increasing event sequence, and a payload. Version 1 events are `log`,
`runtime-failure`, `settle-state`, and `page-exit`. A log payload is one
complete record from the file-logging design. A runtime failure payload
contains the stable error ID and source; it preempts an active successful reply.

Each side sends a heartbeat with its own sequence after one second without any
frame. Three seconds without receiving any frame fails the connection. A frame
received before the limit resets the receive timer. Heartbeats never carry an
image and never extend a request beyond its explicit timeout.

### WebGL launcher

For WebGL, Ditto serves the immutable build and a minimal same-origin launcher
on loopback. Local runs open the URL with the operating system. CI profiles may
provide a headless command containing a required `{url}` placeholder. Ditto
starts that command directly and does not require Playwright, Selenium, a
browser extension, or a hosted page.

Unity WebGL does not provide normal .NET sockets. A small `.jslib` adapter and
NativeWebSocket's browser implementation connect the page to the same control
protocol. The launcher forwards browser console errors, unhandled JavaScript
exceptions, and unhandled promise rejections as ordered diagnostic records.
Page exit and browser exit are explicit failures instead of connection hangs.

## Capture adapters

All capture adapters return the Unity render surface only. Window borders,
browser chrome, Simulator chrome, cursors, and host notifications are excluded.
The handshake names the selected adapter and its effective dimensions.

### macOS and iOS Simulator

Native targets capture Unity's framebuffer through `ScreenCapture` into a
render texture and use asynchronous GPU readback to avoid a blocking read on
the render thread. Encoding happens after the captured frame is committed.
The resulting PNG must match the reported Unity surface dimensions exactly.

The implementation probes support at startup with a known two-color frame and
validates dimensions, alpha handling, row direction, and channel order. A
failed probe prevents scenarios from running. Synchronous readback may be used
only during the probe or as an explicit diagnostic override; it is not an
automatic performance fallback.

Simulator launch uses `xcrun simctl`, an already installed runtime, and an
isolated application install. Simulator boot time is measured separately. A
profile may reuse a booted matching Simulator, but Ditto must reinstall or
verify the exact app build before running.

### WebGL

WebGL calls `canvas.toBlob("image/png")` on Unity's render canvas after the
requested frame is presented. The blob is transferred as the binary screenshot
reply. The adapter does not take a browser screenshot and does not encode WebP.

Startup draws and captures a conformance frame that checks dimensions, alpha,
orientation, and representative color values. A tainted canvas, null blob,
wrong canvas, browser encoding failure, or dimension mismatch fails startup
with a specific adapter diagnostic. Ditto does not silently switch to a browser
screen capture, because that would change what is under comparison.

## Image comparison

Every baseline, actual image, failure frame, and generated image mask retained
by Ditto is PNG. This avoids a lossy codec and keeps capture and comparison free
of an extra transcode. Storage is handled by content addressing and retention,
not by slowing the main loop with WebP encoding.

ODiff v4.5.0 is the comparison engine. Ditto downloads and verifies the
official macOS binary for the host architecture, caches it with other tool
dependencies, and keeps one ODiff server process alive for the run. It never
starts one process per screenshot. `DITTO_ODIFF_PATH` is an explicit binary
override for development and air-gapped hosts; `doctor` reports its version and
does not claim it is the pinned binary.

The verified official binary digests are:

- `odiff-macos-arm64`:
  `3c681171c158f95e7e62d636ddd00c33e8f971c23c85239c6192b72d76ad665b`
- `odiff-macos-x64`:
  `73e565e2a777b653fa0ceb90c138dec1c396c990913fdc1221fe8b01fa70c171`

The default comparison requires exact dimensions and uses:

- ODiff threshold `0.1`;
- anti-alias detection enabled; and
- no more than `0.01%` materially changed pixels.

Each screenshot checkpoint may override the threshold, anti-alias behavior, or
changed-pixel percentage. The effective values are recorded with the result.
Ignored regions are not supported. A changing region should instead be made
deterministic, separated into a different view, or excluded from the captured
scene by the game.

Ditto decides the percentage limit from ODiff's integer `diffCount`, not its
rounded `diffPercentage`. For the default, the comparison passes this part only
when:

```text
diffCount * 10_000 <= width * height
```

For an override, Ditto converts the configured decimal percentage to an exact
integer numerator and scale and applies the equivalent integer inequality. It
records changed and total pixel counts and computes an unrounded display
percentage itself. This prevents a value rounded to `0.01` by ODiff from
accepting more pixels than the configured boundary.

ODiff writes a red difference mask whenever any nonzero difference exists,
including a difference within the permitted percentage. Passing comparisons
may omit the mask from long-term run retention after the run expires, but the
score and effective settings remain in `result.json`.

Wrong dimensions fail before pixel tolerance is considered. ODiff failure,
server exit, unreadable PNG data, or comparison timeout is infrastructure
failure, not an image mismatch.

## Baseline manifest and update behavior

`ditto.lock` is the generated, deterministic, tracked index of accepted
screenshots. `ditto.toml` is authored; `ditto.lock` must be changed through
Ditto. The lock is TOML with this schema:

```toml
schema = 1
suite = "tictactoe"
namespace = "battlement/tictactoe"

[[baselines]]
profile = "macos-local"
scenario = "human wins top row"
checkpoint = "opening-move"
sha256 = "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
width = 1280
height = 720
size_bytes = 42817
source = "4be9d2e4c4e44e3b9b11c840754a1c744be9d2e4c4e44e3b9b11c840754a1c74"

[baselines.comparison]
threshold = 0.1
anti_alias = true
max_changed_percent = 0.01
```

The illustrative hashes have the required shape but are not fixture values.
Entries are sorted by profile, scenario, and checkpoint and contain:

- a schema version and suite namespace;
- profile, scenario, and checkpoint identity;
- the SHA-256 of the exact PNG bytes;
- width, height, and byte size;
- comparison overrides, if any; and
- the 64-hex-digit source fingerprint in `source` for diagnostics.

Rewriting an unchanged manifest produces identical bytes. Acceptance time stays
in the local run result and remote replacement metadata rather than the lock.
Absolute paths, credentials, run IDs, and host names never appear in the file.

An ordinary `run` fails a reached checkpoint that has no manifest entry. It
automatically downloads a hash-pinned object when the entry exists but the
local baseline cache does not. A hash mismatch is an infrastructure failure and
the invalid object is not cached.

### Full update

`run --update` follows Jest's reached-snapshot behavior. Every reached
screenshot becomes the proposed baseline even when a later assertion, panic,
timeout, or other non-image step fails. Unreached checkpoints remain unchanged.
The run still exits nonzero for its non-image failure.

Ditto captures all reachable proposals first. At the end of the run it uploads
every proposed object, verifies successful storage, and then rewrites
`ditto.lock` once with an atomic local rename. If any upload fails, the lock
file remains byte-for-byte unchanged and the result lists uploaded objects that
are safe but not yet referenced. A retry reuses those content-addressed
objects.

An unfiltered full-suite update removes entries for checkpoints that no longer
exist in configuration. A filtered update preserves entries belonging to
unselected scenarios and profiles. Skipped and unreached checkpoints never
delete an entry.

At run start, Ditto records the SHA-256 of `ditto.lock` and each selected entry.
Before any update or review acceptance, it acquires a suite-local file lease and
rereads the lock. If the whole starting digest differs, acceptance is stale and
the lock is not changed, even when the selected entries happen to match. The
already uploaded content-addressed PNGs are harmless and reusable. This avoids
silently overwriting an edit, Git operation, or acceptance from another Ditto
process. Atomic rename prevents a torn write; the digest check prevents a lost
write.

`capture` is deliberately different: screenshot steps only retain actual
images. It performs no baseline lookup, comparison, upload, acceptance, or
manifest edit.

## Baseline stores

Ditto provides filesystem and Cloudflare R2 stores behind the same
content-addressed interface. An object key is derived from the suite namespace
and PNG SHA-256, not from a mutable scenario path. Different checkpoints that
produce identical bytes share one object.

The filesystem store is useful for private or small repositories. It may point
inside or outside the repository. `doctor` warns when a tracked filesystem
store is large or rapidly changing but does not forbid it.

Battlement uses an R2 Standard bucket. Public, credential-free HTTP reads make
normal clones and CI runs simple. Writes use R2's S3-compatible API through the
Rust `aws-sdk-s3` crate. The account, bucket, public base URL, and environment
variable names are configured in the suite. Bucket-scoped access key ID and
secret access key values come only from the environment. A read-only run never
requires Cloudflare credentials.

Git LFS is not used. R2 is an S3-compatible object store rather than a Git LFS
server, and placing an LFS gateway between Git and R2 would add authentication,
filter installation, pointer, and clone failure modes. Ditto's manifest already
provides the useful pointer behavior and can hydrate on first use or with one
explicit command. GitHub also recommends object storage for generated files,
and avoiding image history protects clone and repository size.

R2 Standard storage is appropriate because baseline objects are small and may
be read frequently. The implementation should revisit costs from Cloudflare's
published pricing before enabling the shared bucket, but the free allocation,
low storage price, and free direct egress make expected sample-suite use small.

### Hydration and offline use

Before comparison, Ditto checks the local baseline cache by hash. A miss causes
one download from the configured public URL, verification, and atomic cache
insert. Concurrent requests for the same hash share a lease. `ditto fetch
--all` performs this for every manifest entry with bounded parallelism.

Verified cached baselines work offline. If a required object is neither cached
nor reachable, the checkpoint is not treated as a visual mismatch; the command
ends with an infrastructure failure naming the hash and public URL. There is no
fallback to a mutable latest image.

Only accepted baseline PNGs use the shared R2 bucket. Actual images, diff masks,
logs, videos, and `result.json` remain local run data or normal CI artifacts and
are never published by default.

### Replacement retention

Acceptance on any branch uploads immutable PNG objects immediately and updates
only that worktree's `ditto.lock`. It does not tombstone a replaced hash. This
prevents a feature branch from scheduling deletion of a baseline still used by
the default branch.

After a baseline change merges, default-branch CI runs
`ditto storage publish`. The command verifies that every object in the merged
lock is readable, then publishes
`<namespace>/metadata/canonical-v1.json`. That object contains schema `1`, the
lock SHA-256, a monotonically increasing generation, publication time, and the
sorted set of live PNG hashes. It compares the previous live set with the new
one and adds removed hashes and their publication time to
`tombstones-v1.json`. A hash restored to the canonical set is removed from the
tombstones.

Publish and cleanup share a remote suite mutation lease. R2 acquires the lease
with a conditional object write, records a random owner and short expiry, and
refreshes it while work continues. Metadata writes use the ETag read under that
lease as an `If-Match` condition. Losing the lease or seeing another ETag aborts
without deleting data. If publication stops between the tombstone and
canonical writes, cleanup still sees the old canonical hash as live and cannot
delete it; the next publish safely retries.

A later write-capable Ditto run may opportunistically perform cleanup after its
test result is complete. Cleanup rereads canonical and tombstone metadata under
the lease and deletes only hashes tombstoned for at least seven days and absent
from the canonical live set. It then conditionally rewrites the tombstones.
Missing objects are treated as already deleted. Cleanup is best effort and
never changes a test result.

`ditto clean storage` performs the same guarded cleanup explicitly and prints a
dry-run plan unless `--apply` is supplied. It never infers reachability from the
current feature branch or a filtered run. Only `storage publish` changes the
canonical live set.

Seven-day replacement retention means an old Git branch may eventually name an
object no longer available remotely. A developer who still has it cached can
work offline; otherwise the branch needs a current accepted baseline. This is
an intentional tradeoff because retaining old screenshot versions is not a
project requirement.

## Local run data

Each non-watch invocation creates one active run directory before discovery or
build. Each watch cycle creates its own active run directory. A directory is
append-only while work is active and is marked incomplete until Ditto atomically
writes the terminal `result.json`. It becomes immutable after finalization. An
interrupted directory is finalized on the next Ditto startup.

Every watch cycle has its own run ID, active directory, terminal result, and
retention lifetime. The review tab follows the active cycle but does not own or
mutate its files.

Each finalized run contains:

- the resolved suite and redacted profile;
- actual screenshots, downloaded baseline references, and diff masks;
- any automatic failure frame and experimental video;
- one ordered `logs/events.jsonl` stream for every scenario and session;
- `logs/build.log` when build output exists;
- player, browser, Simulator, build, and ODiff diagnostics; and
- one stable `result.json`.

`result.json` includes schema version, run and scenario IDs, target handshake,
build reuse and launch timing, setup and step timing, skip reasons, warnings,
assertions, screenshot hashes and scores, effective comparison settings, error
IDs, log offsets, recovery actions, and final status. Arrays preserve execution
order. Maps with user-defined names are serialized in lexical key order. Local
paths are repository-relative where possible and secrets are redacted.

The checked-in `ditto-result-v1.schema.json` is normative. Rust types generate
results against it, and stable fixtures validate deserialization. This complete
example shows the common envelope and an image mismatch:

```json
{
  "schema": 1,
  "run_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "command": "run",
  "cycle": 1,
  "suite": "tictactoe",
  "profile": "macos-local",
  "started_at": "2026-08-27T19:10:00Z",
  "duration_ms": 1842,
  "status": "failed",
  "exit_code": 1,
  "build": {
    "source_fingerprint": "sha256:<hex>",
    "fingerprint": "sha256:<hex>",
    "disposition": "reused",
    "duration_ms": 0,
    "log_path": null
  },
  "phases": [
    {
      "name": "discovery",
      "status": "passed",
      "duration_ms": 8,
      "log_path": null,
      "error_ids": []
    },
    {
      "name": "build",
      "status": "passed",
      "duration_ms": 0,
      "log_path": null,
      "error_ids": []
    },
    {
      "name": "hydrate",
      "status": "passed",
      "duration_ms": 2,
      "log_path": null,
      "error_ids": []
    },
    {
      "name": "launch",
      "status": "passed",
      "duration_ms": 1480,
      "log_path": "logs/events.jsonl",
      "error_ids": []
    },
    {
      "name": "handshake",
      "status": "passed",
      "duration_ms": 19,
      "log_path": "logs/events.jsonl",
      "error_ids": []
    },
    {
      "name": "scenarios",
      "status": "failed",
      "duration_ms": 311,
      "log_path": "logs/events.jsonl",
      "error_ids": ["D_IMAGE_MISMATCH_1"]
    },
    {
      "name": "cleanup",
      "status": "passed",
      "duration_ms": 22,
      "log_path": "logs/events.jsonl",
      "error_ids": []
    }
  ],
  "handshake": {
    "platform": "macos",
    "capture_adapter": "unity-async-readback-png-v1",
    "width": 1280,
    "height": 720,
    "scale": 1.0
  },
  "scenarios": [
    {
      "id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
      "name": "human wins top row",
      "status": "failed",
      "skip_reason": null,
      "seed": 42,
      "motion": "instant",
      "duration_ms": 311,
      "steps": [
        {
          "index": 0,
          "name": null,
          "kind": "click",
          "status": "passed",
          "duration_ms": 8,
          "error_ids": [],
          "assertion": null,
          "screenshot": null,
          "video": null
        },
        {
          "index": 1,
          "name": null,
          "kind": "wait",
          "status": "passed",
          "duration_ms": 6,
          "error_ids": [],
          "assertion": null,
          "screenshot": null,
          "video": null
        },
        {
          "index": 2,
          "name": "opening",
          "kind": "screenshot",
          "status": "failed",
          "duration_ms": 22,
          "error_ids": ["D_IMAGE_MISMATCH_1"],
          "assertion": null,
          "screenshot": {
            "checkpoint": "opening-move",
            "baseline_sha256": "<hex>",
            "actual_sha256": "<hex>",
            "diff_sha256": "<hex>",
            "width": 1280,
            "height": 720,
            "changed_pixels": 117,
            "total_pixels": 921600,
            "comparison": {
              "threshold": 0.1,
              "anti_alias": true,
              "max_changed_percent": 0.01
            },
            "passed": false
          },
          "video": null
        },
        {
          "index": 3,
          "name": null,
          "kind": "assert",
          "status": "not-run",
          "duration_ms": 0,
          "error_ids": [],
          "assertion": null,
          "screenshot": null,
          "video": null
        }
      ],
      "logs": {
        "first_sequence": 81,
        "last_sequence": 114,
        "path": "logs/events.jsonl"
      },
      "recovery": "reset"
    }
  ],
  "failure_frame": null,
  "warnings": [],
  "errors": [
    {
      "id": "D_IMAGE_MISMATCH_1",
      "kind": "image-mismatch",
      "source": "odiff",
      "message": "opening-move differs from its baseline",
      "scenario_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
      "step_id": 2,
      "log_sequence": null
    }
  ],
  "artifacts": ["actual/human-wins-top-row/opening-move.png"]
}
```

The common envelope requires every top-level field shown above. Conditional
members follow these rules:

- `build` is null only when discovery fails before a fingerprint exists.
  `log_path` is null when no build ran.
- `handshake` is null until a player completes the handshake.
- `phases` contains ordered `discovery`, `build`, `hydrate`, `launch`,
  `handshake`, `scenarios`, and `cleanup` entries through the last reached
  phase. Phase status is `passed`, `failed`, or `interrupted`.
- Every step has nullable `assertion`, `screenshot`, and `video` members.
  For a reached assertion, screenshot, or video, exactly the matching member is
  non-null. A `not-run` step has all three null.
- A screenshot's `comparison` is null in `capture`. In `run`, it contains the
  effective `threshold`, `anti_alias`, and `max_changed_percent` values. On a
  missing baseline, `baseline_sha256`, `diff_sha256`, `changed_pixels`, and
  `total_pixels` are null, the actual hash remains present, and `passed` is
  false.
- An assertion contains `object`, `state`, `expected`, `observed`, and `passed`.
  Expected and observed are JSON booleans or strings as required by the state.
- `failure_frame` is null when no runtime failure occurred. Otherwise it has
  status `captured` with path, hash, width, and height, or status `unavailable`
  with a reason and null media fields.
- Every `error_ids` value resolves to one top-level error. Error kind is
  `configuration`, `build`, `infrastructure`, `assertion`, `image-mismatch`,
  `runtime`, `panic`, `timeout`, or `protocol`.
- Artifact paths are slash-separated paths relative to the run directory.
  Image hashes are null only when the corresponding image does not exist.

### Result variants

The following fragments show the fields that distinguish other terminal
outcomes. They use the same required common envelope as the full example.

A build failure has no handshake or scenarios and points directly to the full
build log:

```json
{
  "status": "infrastructure-error",
  "exit_code": 2,
  "build": {
    "source_fingerprint": "sha256:<hex>",
    "fingerprint": "sha256:<hex>",
    "disposition": "failed",
    "duration_ms": 412,
    "log_path": "logs/build.log"
  },
  "handshake": null,
  "phases": [
    {
      "name": "build",
      "status": "failed",
      "duration_ms": 412,
      "log_path": "logs/build.log",
      "error_ids": ["D_BUILD_1"]
    }
  ],
  "scenarios": [],
  "failure_frame": null,
  "errors": [
    {
      "id": "D_BUILD_1",
      "kind": "build",
      "source": "unity",
      "message": "player build failed",
      "scenario_id": null,
      "step_id": null,
      "log_sequence": null
    }
  ],
  "artifacts": ["logs/build.log"]
}
```

A zero-screenshot assertion failure has no image artifact. The assertion and
unexpected error records are machine-gated; ordinary log contents and phase
timings are retained for inspection and cannot be asserted in v1.

```json
{
  "status": "failed",
  "exit_code": 1,
  "scenarios": [
    {
      "id": "<uuid>",
      "name": "check menu",
      "status": "failed",
      "skip_reason": null,
      "seed": 117,
      "motion": "instant",
      "duration_ms": 4,
      "steps": [
        {
          "index": 0,
          "name": null,
          "kind": "assert",
          "status": "failed",
          "duration_ms": 4,
          "error_ids": ["D_ASSERT_1"],
          "assertion": {
            "object": "menu",
            "state": "visible",
            "expected": true,
            "observed": false,
            "passed": false
          },
          "screenshot": null,
          "video": null
        }
      ],
      "logs": {
        "first_sequence": 12,
        "last_sequence": 18,
        "path": "logs/events.jsonl"
      },
      "recovery": "reset"
    }
  ],
  "errors": [
    {
      "id": "D_ASSERT_1",
      "kind": "assertion",
      "source": "ditto-player",
      "message": "menu was not visible",
      "scenario_id": "<uuid>",
      "step_id": 0,
      "log_sequence": 18
    }
  ],
  "artifacts": ["logs/events.jsonl"]
}
```

A runtime crash has a structured error and an explicit unavailable frame when
the player exits before capture:

```json
{
  "status": "failed",
  "exit_code": 1,
  "scenarios": [
    {
      "id": "<uuid>",
      "name": "drag card",
      "status": "failed",
      "skip_reason": null,
      "seed": 117,
      "motion": "instant",
      "duration_ms": 91,
      "steps": [
        {
          "index": 2,
          "name": null,
          "kind": "drag",
          "status": "failed",
          "duration_ms": 7,
          "error_ids": ["D_RUNTIME_1"],
          "assertion": null,
          "screenshot": null,
          "video": null
        }
      ],
      "logs": {
        "first_sequence": 61,
        "last_sequence": 77,
        "path": "logs/events.jsonl"
      },
      "recovery": "relaunch"
    }
  ],
  "failure_frame": {
    "status": "unavailable",
    "reason": "player-exited",
    "path": null,
    "sha256": null,
    "width": null,
    "height": null
  },
  "errors": [
    {
      "id": "D_RUNTIME_1",
      "kind": "runtime",
      "source": "unity",
      "message": "player exited during drag",
      "scenario_id": "<uuid>",
      "step_id": 2,
      "log_sequence": 77
    }
  ],
  "artifacts": ["logs/events.jsonl"]
}
```

An interrupted run records the active scenario and step as interrupted, then
finalizes the partial directory:

```json
{
  "status": "interrupted",
  "exit_code": 130,
  "scenarios": [
    {
      "id": "<uuid>",
      "name": "check menu",
      "status": "interrupted",
      "skip_reason": null,
      "seed": 117,
      "motion": "real-time",
      "duration_ms": 203,
      "steps": [
        {
          "index": 1,
          "name": null,
          "kind": "wait",
          "status": "interrupted",
          "duration_ms": 91,
          "error_ids": [],
          "assertion": null,
          "screenshot": null,
          "video": null
        }
      ],
      "logs": {
        "first_sequence": 20,
        "last_sequence": 32,
        "path": "logs/events.jsonl"
      },
      "recovery": "none"
    }
  ],
  "failure_frame": null,
  "errors": [],
  "artifacts": ["logs/events.jsonl"]
}
```

Run status is `passed`, `failed`, `infrastructure-error`, or `interrupted`.
Scenario status additionally allows `skipped` and `interrupted`; step status
additionally allows `not-run` and `interrupted`. An interrupt takes precedence,
then any infrastructure error, then any scenario or image failure, then passed
or skipped scenarios. These map to exit codes `130`, `2`, `1`, and `0`. A
runtime-failure event preempts a racing successful step reply. The first
terminal cause is primary; later log-flush or recovery errors are attached but
do not replace it.

Durations use local monotonic clocks and integer milliseconds. `started_at` is
informational UTC. JSON object order is not semantic. `result.json` uses lexical
keys, two-space indentation, and a final newline; `--json` emits the same object
on one line. Paths, messages, and command arrays are redacted by replacing any
launch credential, review token, R2 secret, or configured secret environment
value with the literal `<redacted>` before persistence.

Every ordered JSONL record streamed during the run is written immediately, so
`--json` is not the only machine interface. A terminal JSON result is still
written after a failed scenario or recoverable target relaunch. If Ditto itself
is killed before that point, the partial directory is marked interrupted on the
next startup.

Completed and interrupted runs are retained for seven days. An independent
configurable 1 GB LRU limit applies to run data. The active run is protected
until its process releases the lease. A run larger than the limit completes and
is kept while older inactive runs are evicted; cleanup then reports that the
newest run alone exceeds the limit. This cache is separate from the 20 GB build
cache and the baseline cache.

## Logging, errors, and recovery

Ditto tails Battlement's existing ordered JSONL records through the
authenticated bridge instead of creating a second logging format. The bridge
streams complete records as they are appended and identifies the active
scenario, session, and step. The retained span starts before setup and ends
after failure capture and reset, so a developer can see events immediately
surrounding the problem.

Ditto allocates the run directory and creates `logs/events.jsonl` before build
or launch. Its first standard-error line has stable framing:

```text
DITTO_RUN_DIR=/absolute/path/to/.ditto/runs/<run-id>
```

An agent may follow `<run-dir>/logs/events.jsonl` immediately and later use the
scenario sequence range in `result.json`. Build output uses the equally stable
`<run-dir>/logs/build.log` path and is created only when a build runs. Human
terminal output shows step transitions, warnings, failures, stable error IDs,
and retained paths rather than repeating every record. `--json` keeps standard
output reserved for the terminal result object.

For example, after reading the first progress line:

```text
tail -f /absolute/path/to/.ditto/runs/<run-id>/logs/events.jsonl
```

Any unexpected error or fatal Battlement record, Unity exception or assert,
Rust panic, bridge protocol failure, heartbeat loss, or deadline failure fails
the current scenario. Warnings are displayed and retained but do not fail by
default. Ditto does not support declarative expected-log assertions in v1;
scenario assertions should describe visible game behavior.

On a responsive runtime failure, Ditto asks the player to freeze its controlled
clock, commit no further Battlement work, capture the last responsive frame,
and flush current logs. It then retains:

- the failed step and deadline state;
- stable error and panic IDs;
- the full correlated log span;
- step, settle, capture, comparison, and recovery timings;
- reached screenshots and diff masks; and
- the automatic failure frame.

If the target process or page has already crashed, the result says why an
automatic frame was unavailable and retains the last successful screenshot or
no image at all. A retained older screenshot is labeled as historical. Ditto
never substitutes it while labeling it as the failure frame.

After failure, the runner requests a clean reset and handshake. If reset fails,
it relaunches the same immutable build and resumes with the next scenario. If
relaunch fails, remaining scenarios receive an infrastructure status without
waiting for their individual deadlines. `--bail` stops after the configured
failure count but still completes log flush, failure capture, and run metadata.

Unity player logs, browser console diagnostics, and Simulator application logs
are attached as secondary sources. Their timestamps are correlated to the
handshake clock, while Battlement JSONL sequence remains the authoritative
application order.

## Local review application

`ditto review` serves a loopback-only static application from the Ditto binary.
It vendors `img-comparison-slider` `8.0.7` and `@panzoom/panzoom` `4.6.2`; it
requires no CDN, package installation, or network access at review time. The
source pins and license notices are checked into the crate.

For each checkpoint, the application provides:

- baseline and actual images side by side;
- a draggable swipe comparison;
- adjustable alpha overlay;
- the ODiff red mask;
- synchronized zoom and pan across views;
- scenario and step navigation;
- correlated logs and step timing; and
- selective acceptance of one or more reached screenshots.

The UI reads `result.json` rather than reconstructing status from filenames.
It preserves pixel rendering at integer zoom levels and reports cursor
coordinates, image dimensions, diff score, and effective thresholds.

During watch mode, the review server also exposes a read-only live event stream
from the active run. The existing tab appends correlated logs, step status,
timings, and newly completed screenshots without polling or reloading. The
stored JSONL and terminal `result.json` remain authoritative if the page closes.

Acceptance is a state-changing loopback request protected by a random review
token. The server verifies that the actual hash still matches the run result,
uploads accepted objects immediately, then performs one atomic manifest rewrite
for the selected set. Unaccepted checkpoints remain unchanged. If write
credentials are unavailable, comparison remains fully usable and acceptance is
disabled with a clear credential message.

The page opens only for `ditto review`, `--review`, or the first watch cycle.
Watch mode refreshes one live tab and does not create a new tab per failure.

## Experimental video

A `video` step records an MP4 for debugging and demonstration. Video is
experimental and non-gating: the file itself is never compared with a baseline
and does not change screenshot pass or failure status. Runtime errors during the
recorded actions still fail the scenario.

FFmpeg is required only when a selected scenario reaches a video step. `doctor`
reports it as optional otherwise. If absent when needed, Ditto fails that step
with installation guidance rather than disabling it silently.

WebGL records the Unity canvas with `captureStream` and `MediaRecorder`, then
uses FFmpeg when normalization or MP4 conversion is required. Native targets
collect timestamped Unity framebuffer frames and pass them to FFmpeg. A clip may
use controlled or real-time motion, includes no review UI or host chrome, and is
capped at 30 seconds. The runner stops and finalizes the clip on failure when
possible. Partial encoding diagnostics are retained if finalization fails.

## Performance requirements

The primary benchmark uses a previously compiled and locally cached 1280 by 720
player on a CI-class Apple Silicon macOS host. It runs 20 representative
scenarios containing 40 total screenshots, fresh Rust engine creation, normal
settling, PNG capture, local baseline reads, and ODiff comparison.

The requirements are:

- no more than 20 seconds including a cold player launch; and
- no more than 5 seconds when the player, connection, and ODiff server are warm.

Compilation, baseline downloads, and Simulator boot are measured and reported
separately. The iOS form of the benchmark assumes a matching Simulator is
already booted. CI records phase percentiles so a regression cannot be hidden by
subtracting time without reporting it.

The design avoids the likely critical-path costs: no restart per scenario, no
ODiff process per image, no browser screen capture, no PNG-to-WebP transcode,
no fixed animation sleeps, no baseline upload on ordinary runs, and no public
review application startup unless requested.

## CI integration

Battlement CI adds representative macOS screenshot suites for the Basic,
Tic-Tac-Toe, Chess, and UI samples. Their already scheduled sample builds feed
the shared immutable build cache. The four suites run in parallel jobs or
Tollgate slots, while scenarios within a suite remain serial on one player.

WebGL and iOS Simulator receive focused adapter smoke suites. They validate
startup handshake, input, settling, PNG conformance, a passing comparison,
runtime failure reporting, and log forwarding. CI does not build every sample
for every platform in v1.

The old player capture smoke remains until the new sample suites and adapter
smokes pass reliably. At cutover, the new checks replace it rather than running
two permanent systems. Failed CI uploads the local run directory as a normal CI
artifact. Approved R2 baselines remain separate from those private diagnostics.

## Adoption and cutover

Ditto is introduced alongside the current capture workflow. Before any removal,
the repository must have the new crates and CLI entry points, macOS sample
scenarios and accepted baselines, WebGL and iOS adapter smokes, replacement CI
calls, stable JSON output, and the documented agent workflow.

A **Tollgate-managed worktree task** is an implementation performed in an
isolated Git worktree and submitted through the repository's required CI and
promotion mechanism. The
[Battlement implementation plan](implementation-plan.md) provides the related
repository context.

Once those pieces pass together, one atomic change performs all of the
following:

1. Move the generic Unity editor lease from the visual-capture Python module to
   neutral shared tooling and update every remaining consumer.
2. Replace the visual-capture player smoke and CI invocations with Ditto runs.
3. Update `AGENTS.md`, the implementation plan, UI documentation, and other
   active guidance to use `ditto capture` or `ditto run` for screenshots, logs,
   and end-of-session verification. The agent guidance requires Ditto for any
   Tollgate-managed worktree task that changes a player source dependency or
   claims a player-visible runtime result.
4. Remove `docs/visual-capture.md`, the capture scripts and tests, editor build
   methods, reusable capture assets, assembly references, and every C# capture
   fixture.

The cutover change must prove that no active documentation, CI command, or
assembly definition references the removed workflow. It must also demonstrate
all four macOS sample suites from a clean checkout. Until then, Ditto code must
not repurpose names or assets on which the current workflow depends.

After cutover, every agent-driven screenshot or short video of a Battlement
player goes through Ditto. Agents do not invoke removed capture scripts, take
host-window screenshots as a verification substitute, or bypass Ditto's input,
log correlation, failure handling, and run retention. Human exploratory use of
the Unity Editor remains ordinary development and is not a retained Ditto run.

The worktree handoff uses this machine-copyable shape for either a retained
suite scenario or a throwaway fragment:

```text
Ditto: passed
run_id: 0197b35f-6c59-7b98-b1f0-a39f5ee54db8
profile: macos-local
result: /absolute/path/to/result.json
scenarios: menu becomes ready
screenshots: none
logs: logs/events.jsonl sequences 12-18
```

When Ditto does not apply, the handoff instead uses:

```text
Ditto: not applicable - documentation-only change
```

## Implementation validation

Unit tests should focus on deterministic manifest serialization, strict TOML
diagnostics, filtering, update transactions, content hashing, result schemas,
deadline state, and retention decisions. Storage tests use a filesystem fake at
the S3 operation boundary and include concurrent manifest changes. Protocol
tests use a fake player to exercise malformed frames, heartbeat loss, binary
ordering, failure capture, reset, and relaunch.

Black-box adapter tests build a small player with known colored regions and a
known clickable Battlement object. They validate the same released player
surface, input path, and diagnostics that a game uses. Performance tests retain
phase timings and compare them against the two explicit budgets.

Changes to `ditto.toml`, `ditto.lock`, or `result.json` schemas require fixtures
that a previous supported binary can reject clearly. The project does not need
a compatibility or migration scheme before v1; while the design is being
implemented, schema changes update fixtures and all callers together.

[canvas-capture-stream]:
  https://developer.mozilla.org/docs/Web/API/HTMLCanvasElement/captureStream
[canvas-to-blob]:
  https://developer.mozilla.org/docs/Web/API/HTMLCanvasElement/toBlob
[comparison-slider]:
  https://github.com/sneas/img-comparison-slider
[ffmpeg]:
  https://ffmpeg.org/documentation.html
[github-lfs]:
  https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-git-large-file-storage
[github-limits]:
  https://docs.github.com/en/repositories/creating-and-managing-repositories/repository-limits
[jest-snapshots]:
  https://jestjs.io/docs/snapshot-testing
[native-websocket]:
  https://github.com/endel/NativeWebSocket
[odiff]:
  https://github.com/dmtrKovalenko/odiff
[odiff-release]:
  https://github.com/dmtrKovalenko/odiff/releases/tag/v4.5.0
[panzoom]:
  https://github.com/timmywil/panzoom
[r2-auth]:
  https://developers.cloudflare.com/r2/api/tokens/
[r2-pricing]:
  https://developers.cloudflare.com/r2/pricing/
[r2-rust]:
  https://developers.cloudflare.com/r2/examples/aws/aws-sdk-rust/
[r2-s3]:
  https://developers.cloudflare.com/r2/api/s3/api/
[unity-async-readback]:
  https://docs.unity3d.com/ScriptReference/Rendering.AsyncGPUReadback.html
[unity-screen-capture]:
  https://docs.unity3d.com/ScriptReference/ScreenCapture.html
[unity-web-networking]:
  https://docs.unity3d.com/Manual/webgl-networking.html

## Manual QA

Perform this checklist from a clean clone on each supported host architecture
before the initial cutover and before changing a capture adapter.

### CLI, fragments, and machine results

- **Setup:** configure macOS, WebGL, and iOS Simulator profiles in a clean
  clone.
- **Action:** run `doctor`, `list`, bare `capture`, filtered `capture`, and
  `capture --fragment` with a file and standard input through both CLI entry
  points.
- **Expected:** doctor separates required, optional, read-only, and write
  dependencies without printing secrets. Positional values are filters. Capture
  never fetches or changes a baseline. Standard input rejects watch, while a
  file fragment supports it.
- **Action:** exercise every step and target shape, timeout, fixture, save,
  seed, motion, and comparison override. Include a zero-screenshot assertion
  failure and an interrupted run.
- **Expected:** fragment precedence and alias conflicts follow the schema. The
  zero-screenshot result gates on its assertion and unexpected errors, retains
  log and timing data, and has no image artifact. All result variants validate
  against the schema with correct status precedence, units, paths, and
  redaction.
- **Action:** read the first stderr line while a run is active and follow
  `logs/events.jsonl` through its terminal sequence.
- **Expected:** framing and paths are stable, records are complete and ordered,
  and the final scenario range selects the same records.

### macOS suites, isolation, and input

- **Setup:** use the Basic, Tic-Tac-Toe, Chess, and UI macOS suites.
- **Action:** run click, hover, drag, type, and key steps through UUID aliases
  and normalized coordinates. Obscure one UUID target with another object.
- **Expected:** each suite reuses one player, every scenario has a fresh Rust
  engine and seed, and the obscured click fails with the blocking UUID. No
  object action is called directly.
- **Expected:** the player need not be frontmost. The host pointer and keyboard
  do not move, and macOS requests neither Accessibility nor Screen Recording
  access. Captures have the exact configured Unity surface dimensions.
- **Action:** use a named fixture, opaque save, default seed, and overridden
  seed in consecutive scenarios.
- **Expected:** setup runs before connection and no engine, Unity object, input
  state, or log correlation leaks between scenarios.

### Settling, motion, and deadlines

- **Action:** wait for drained Battlement work and two quiet frames, then test
  exact-frame and object waits with delayed game-owned work.
- **Expected:** implicit settling does not require sleeps. Step, scenario, and
  run deadlines fail promptly, while build, hydration, launch, and Simulator
  boot use their separate deadlines.
- **Action:** run the same tween, particle, and audio behavior in instant,
  controlled, and real-time modes.
- **Expected:** Battlement-owned behavior follows the selected clock. Custom
  scripts and shaders are neither disabled nor reported as controlled.

### Runtime failures and recovery

- **Action:** cause an unexpected error, fatal record, Unity assert, managed
  exception, Rust panic, malformed bridge frame, heartbeat loss, and timeout.
- **Expected:** each failure terminates within its deadline with stable IDs and
  the complete log span. Protocol examples pass schema validation, frame-size
  limits hold, and a runtime failure beats a racing successful reply.
- **Action:** fail while responsive, then crash the player or page before frame
  capture.
- **Expected:** a responsive failure freezes and captures a separately labeled
  final frame. A crash reports the frame unavailable; any older screenshot is
  labeled historical and is never substituted.
- **Action:** run several failures without bail, then with `--bail` and
  `--bail=2`.
- **Expected:** reset or relaunch lets later scenarios run without bail. Bail
  stops at the selected count only after diagnostics are flushed.

### WebGL adapter

- **Action:** run the WebGL conformance and failure smokes through the minimal
  local launcher and a configured headless command.
- **Expected:** `canvas.toBlob` produces exact PNG dimensions. UUID and
  normalized input use the in-page bridge, browser errors and page exit reach
  ordered logs, and images exclude browser chrome. No browser automation
  package is required.

### iOS Simulator adapter

- **Action:** run portrait and landscape profiles on two installed device
  types, including click, drag, and a scenario containing hover.
- **Expected:** dimensions and safe areas come from Simulator. Click and drag
  become touch sequences. The hover scenario skips before setup with its
  documented reason.

### R2 hydration, update, and retention

- **Action:** delete the baseline cache and run without credentials, run
  `fetch --all`, then disconnect the network.
- **Expected:** public objects hydrate and verify by hash. The prewarmed cache
  works offline; a genuine miss is an infrastructure error.
- **Action:** update with write credentials and a later assertion failure.
  Repeat with an upload failure, filtered selection, full selection, and a
  concurrent local lock edit.
- **Expected:** every reached image uploads before one manifest rewrite, even
  though the assertion still fails. Upload failure leaves the lock unchanged.
  Full update prunes removed checkpoints, filtered update preserves unselected
  entries, and stale acceptance never overwrites the concurrent edit.
- **Action:** accept a feature-branch replacement, merge it, and publish the
  default branch. Exercise remote lease loss, ETag conflict, interrupted
  publish, dry-run cleanup, and applied cleanup after seven days.
- **Expected:** the feature branch creates no tombstone. Canonical publication
  creates retention metadata. No live canonical hash is deleted, and cleanup
  failure never changes a test result.

### Filesystem storage, comparison, and review

- **Action:** repeat hydration, update, and offline runs with a filesystem store
  and a large tracked store.
- **Expected:** behavior matches R2 where applicable; the large-store warning
  does not block the run.
- **Action:** compare images at, below, and one pixel above the default `0.01%`
  boundary. Change dimensions and exercise per-checkpoint overrides.
- **Expected:** integer `diffCount` controls the boundary, rounded percentages
  cannot admit an extra pixel, and wrong dimensions fail before tolerance.
- **Action:** inspect side-by-side, swipe, overlay, red mask, synchronized zoom
  and pan, logs, and timing. Accept selected images, then remove write
  credentials.
- **Expected:** only selected entries change. Review remains available without
  credentials while acceptance is clearly disabled.

### Watch, builds, and live review

- **Action:** change a scenario, lock, and build input in watch mode.
- **Expected:** one review tab remains live. Cheap changes reuse the target; a
  new build fingerprint causes one build and relaunch. JSON output is one object
  per cycle, `--output` atomically shows the latest cycle, and watch rejects
  update mode.
- **Action:** make replacement build `F2` fail, then edit only the scenario,
  only the lock, explicitly retry, and finally create build fingerprint `F3`.
- **Expected:** the F2 cycle has no scenarios and links `logs/build.log`. The
  old player remains idle. Scenario changes queue, lock changes can only
  recompare a labeled older run, explicit retry retries F2, and successful F3
  replaces the player and runs queued scenarios. Live review shows the current
  cycle's logs and completed images while retaining older-run navigation.
- **Action:** change committed, staged, unstaged, and untracked files in each
  dependency-closure category.
- **Expected:** source fingerprints follow file bytes, build fingerprints also
  follow toolchain and profile inputs, and `--no-build` reports changed paths.

### Diagnostics and experimental video

- **Action:** build with runner diagnostics disabled.
- **Expected:** ordinary file logging still works, the viewer and bridge are
  unavailable, and Ditto reports a diagnostics-disabled handshake failure.
- **Action:** run paired video steps with actions, assertions, and a screenshot,
  both with and without FFmpeg, and exceed 30 seconds.
- **Expected:** FFmpeg matters only when video starts. MP4 contains only the
  Unity surface, failure finalizes when possible, duration is capped, and video
  never gates an otherwise passing screenshot comparison.

### Cache and performance budgets

- **Action:** fill run and build caches past their limits while holding active
  leases and create an oversize newest entry.
- **Expected:** active data is protected, oversize entries are reported,
  seven-day run retention applies, and the 1 GB and 20 GB limits are separate.
- **Action:** run 20 scenarios and 40 screenshots at 1280 by 720 from a cold
  player, then with the player and ODiff warm.
- **Expected:** cold time is at most 20 seconds and warm time at most 5 seconds.
  Build, download, and Simulator boot time are shown separately.

### CI and atomic cutover

- **Setup:** use a clean checkout with replacement macOS suites and focused
  WebGL and iOS smokes passing.
- **Action:** perform the atomic documentation, CI, lease, assembly, fixture,
  asset, script, test, and old-document removal.
- **Expected:** Basic, Tic-Tac-Toe, Chess, and UI pass in parallel. No active
  reference names the removed workflow. A player-affecting Tollgate worktree
  task uses only Ditto for player media and reports the required handoff fields;
  a task outside Ditto's trigger records a specific not-applicable reason.
