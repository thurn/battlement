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
the player executor destroys and recreates it for every scenario. The Unity
player applies snapshots and commands from that engine but does not own the
game rules.

Ditto is designed for the inner development loop as much as for CI. It keeps a
player, browser or Simulator, loopback HTTP session, loaded Unity assets, and
image comparator warm while it creates a fresh Rust engine for each scenario.
Given a matching cached build, the target is dozens of useful checks in seconds
without rebuilding or restarting between scenarios. Build time is reported
separately.

Ditto runs on Apple silicon and Intel macOS hosts and targets macOS
players, Unity WebGL, and iOS Simulator. Every retained image is a PNG.
Battlement's own baseline store uses Cloudflare R2, while other repositories
may use R2 or a filesystem. Git LFS is not part of the design.

The current visual-capture workflow remains operational until Ditto, its sample
scenarios, and replacement CI checks are all working. The final migration is a
single cutover described in [Adoption and cutover](#adoption-and-cutover).

Ditto uses these project-specific identities throughout the design:

- A **run** is one `run` or `capture` invocation, one watch execution cycle, or
  one comparison-only acceptance or recomparison cycle. It owns a run ID, one
  immutable run directory, and one terminal `result.json`.
- A **job** is one resolved JSON batch supplied to a player for a run. A run may
  use another job after a player relaunch, but a job belongs to exactly one run.
- A **player session** is one launched player or browser page and its HTTP route
  token. Watch mode may use one player session for many runs.
- An **engine session** is the fresh Rust engine created for exactly one
  scenario. It never spans scenarios, jobs, or runs.
- A **reached step** started execution. A **durable scenario** has completed
  reset, log and artifact flush, reached-only hydration, comparison and media
  processing, and atomic decision persistence.

## Related information

The following Battlement documents provide the runtime and logging foundations
that Ditto extends:

- [Battlement technical design](technical-design.md)
- [Battlement logging design](logging-design.md)
- [Battlement implementation plan](implementation-plan.md), which identifies
  Tollgate as the repository's CI and promotion mechanism
- [Current visual-capture workflow](visual-capture.md), which remains the
  active workflow until the Ditto cutover

The implementation must use the primary sources below when pinning tools or
implementing platform adapters:

- [ODiff source and documentation][odiff]
- [ODiff v4.5.0 release][odiff-release]
- [UnityWebRequest API][unity-web-request]
- [Unity ScreenCapture API][unity-screen-capture]
- [Unity asynchronous GPU readback API][unity-async-readback]
- [Unity Web networking restrictions][unity-web-networking]
- [Canvas `toBlob`][canvas-to-blob]
- [Cloudflare R2 S3 API compatibility][r2-s3]
- [Cloudflare R2 with the Rust AWS SDK][r2-rust]
- [Cloudflare R2 API token permissions][r2-auth]
- [Cloudflare R2 pricing][r2-pricing]
- [GitHub repository limits][github-limits]
- [Git Large File Storage behavior][github-lfs]
- [`img-comparison-slider`][comparison-slider]
- [`@panzoom/panzoom`][panzoom]
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
suite has been loaded. The **repository root** is the Git worktree root that
contains the selected full suite. Resolved paths may not escape it.
The only exception is an explicitly external filesystem baseline root, as
described under [Baseline stores](#baseline-stores).

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
`RUN` is omitted, it selects the newest retained run with an image mismatch or
missing baseline, otherwise the newest capture containing an image. Ditto does
not persist mutable "reviewed" state.

`ditto fetch [FILTER ...]` downloads the baselines required by selected
scenarios. `ditto fetch --all` downloads every object named by `ditto.lock` and
is the supported way to prewarm a fresh clone for offline work.

`ditto list [FILTER ...]` resolves configuration and prints profiles,
scenarios, checkpoint names, and skip reasons without launching a player.

`ditto doctor [--profile NAME]` checks the host, Unity installation, selected
platform tools, ODiff, FFmpeg when requested by the suite, cache permissions,
baseline-store reachability, and write credentials. Read and write checks are
reported separately.

`ditto clean runs`, `ditto clean builds`, and `ditto clean baselines` print a
plan and byte count, then prune only the discovered suite by default while
respecting active leases. `--global` broadens inactive run and build cleanup;
baseline cleanup always names a namespace. The baseline hydration cache has no
automatic eviction, preserving the `fetch --all` offline guarantee.
`ditto clean storage` performs store tombstone cleanup described under
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
- `--bail[=N]` stops before the next scenario after the first failed scenario or
  after `N` failed scenarios. Without it, recovery is attempted and remaining
  scenarios continue.
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
scenario may contain no screenshot steps. Such a scenario can validate input,
visible object assertions, logs, error handling, and timing for a
nonvisual change. `run` succeeds without consulting the baseline store when no
screenshot is reached; `capture` retains the same diagnostics without requiring
a committed suite entry.

```toml
[aliases]
status = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"
[[scenarios]]
name = "menu becomes ready"
[[scenarios.steps]]
assert = { object = "status", state = "visible" }
```

Assertions and unexpected error or fatal records gate the command. Ordinary log
messages, warnings, and phase timings are retained for human or agent inspection
but cannot be asserted declaratively.

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
required to discover status, error IDs, log spans, screenshot paths, or phase
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
name = "tictactoe"
default_profile = "macos-local"

[defaults]
step_timeout = "2s"
scenario_timeout = "10s"
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

The misspelling protection provided by strict TOML validation rejects unknown
keys, duplicate profile or scenario names, duplicate checkpoint names within a
scenario, invalid UUIDs, and ambiguous step tables are errors. Durations use an
integer followed by `ms`, `s`, or `m`. Configuration errors include the file,
line, key path, and a suggested valid key when one is close.

The account environment variable in the example is illustrative; Battlement's
committed suite uses consistently named `BATTLEMENT_R2_*` variables. Secret
values are never allowed directly in `ditto.toml` or `ditto.lock`.

### Complete suite fields

- **Top level:** `name`, `default_profile`, `player`, `profiles`, and
  `scenarios` are required. Names are nonempty UTF-8; scenarios and checkpoints
  are unique in their containing scope. A suite has at most 128 scenarios.
- **Player:** `unity_project`, `scene`, and `rust_manifest` are the only fields.
  All are relative to the full suite file and must resolve inside the repository
  root. The scene belongs to the Unity project, and the manifest builds its
  native engine. Ditto supplies supported build methods; suites cannot name
  arbitrary editor methods or shell commands.
- **Timeouts:** `run`, `build`, `launch`, `baseline_download`, and
  `simulator_boot` are positive and at most one hour. The shown defaults apply
  member by member. Run time starts when Ditto accepts the job's first
  `started` request and includes startup, execution, reset, reached-only baseline
  download, comparison, failure capture, recovery, and final durability. Watch
  gives each cycle a new run deadline.
- **Defaults:** `step_timeout`, `scenario_timeout`, `motion`, and `comparison`
  are optional. Motion is `instant`, `controlled`, or `real-time`. A scenario
  may override all except comparison, which a screenshot overrides member by
  member.
- **Comparison:** `threshold` is from `0.0` through `1.0`. `anti_alias` is a
  Boolean. `max_changed_percent` is from `0.0` through `100.0` and is parsed as
  an exact decimal rather than binary floating point.
- **Aliases:** each key is a readable nonempty case-sensitive identifier that
  does not look like a UUID. Every value is a Battlement UUID string.
- **Baseline:** the entire table is optional and is required only when ordinary
  execution reaches baseline behavior. Kind is `filesystem` or `r2`; both
  require `namespace`. Filesystem also requires `root`. R2 also requires
  `public_base_url` and four environment-variable names.
  A namespace uses slash-separated letters, digits, periods, underscores, and
  hyphens and has no empty, `.`, or `..` segment. A relative filesystem `root`
  resolves from the full suite file and is the only suite path allowed outside
  the repository. Ditto resolves symlinks through the nearest existing parent,
  rejects a nonexistent parent, and retains the resulting absolute root only in
  redacted runtime configuration, never in `ditto.lock` or `result.json`.

  `capture` and scenarios that reach no screenshot never require this table.
  Ordinary `run`, `run --update`, `fetch`, publication, cleanup, and acceptance
  validate it only when their selected work actually reaches baseline storage.
- **Profile:** macOS and WebGL require `display`; WebGL may add one
  `headless_command` array with exactly one `{url}` argument. iOS Simulator
  requires `device` and an orientation of `portrait`,
  `portrait-upside-down`, `landscape-left`, or `landscape-right`. Fields from a
  different target are errors.

### Profiles

A profile has one of three targets: `macos`, `webgl`, or `ios-simulator`.
macOS and WebGL profiles specify the Unity render size and scale. Ditto rejects
a player whose startup report contains different effective values.

An iOS Simulator profile specifies an installed device type and orientation.
Ditto asks Simulator for the resulting pixel dimensions, scale, and safe-area
insets; it does not synthesize an arbitrary resolution. An unavailable device
type or runtime is a configuration failure with the installed alternatives
listed.

The default profile is intended for a developer workstation. CI always names a
profile explicitly. A baseline identity includes the profile, because rendering
engines, safe areas, and device dimensions can legitimately differ.

Platform capability decisions happen before engine creation:

| Capability | macOS | WebGL | iOS Simulator |
| --- | --- | --- | --- |
| Click and drag | Supported | Supported | One-finger touch |
| Hover | Supported | Supported | Scenario skipped |
| Key | Supported | Supported | Supported |
| PNG capture | Native framebuffer | Unity canvas | Native framebuffer |
| Video | Native temporary file | Scenario skipped | Native temporary file |

A scenario containing an unsupported capability is skipped as a whole with a
specific reason such as `unsupported-input:hover` or
`unsupported-step:video`. Skipping occurs before engine creation.
The host resolves these static decisions from the selected profile before it
constructs a job. Skipped scenarios are omitted from `Job.scenarios` and
materialized directly as skipped result entries. The startup capability report
verifies the host's assumption; a mismatch is a startup infrastructure error,
not a late platform skip.

### Scenario fragments

`capture --fragment` accepts a full suite or a fragment containing `defaults`,
`aliases`, and one or more `scenarios`. A fragment inherits launch and
baseline-neutral settings from the full `ditto.toml` discovered upward from the
fragment file; `--config` makes that choice explicit. Standard input discovers
from the command's starting directory and may set a synthetic name.

A file containing `player` and `profiles` is a full suite and does not inherit.
A fragment may contain only `name`, `defaults`, `aliases`, and `scenarios`. It
inherits the repository suite's player, timeouts, selected profile, and launch
settings. Fragment defaults override repository defaults member by member.
Fragment aliases are added; redefining an inherited alias to a different UUID
is an error. Only fragment scenarios run, so they do not merge with repository
scenarios. CLI values take precedence over the fragment, then the repository
suite, then built-in defaults. Baseline settings are ignored by `capture` even
when inherited.

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

A scenario is a name, a motion mode, and an ordered list of steps. The Ditto
CLI validates TOML and sends a resolved JSON job to the player. The player's
Ditto executor owns the scenario lifecycle and serial step execution. A step
may have an optional `name` for results and log correlation and an optional
timeout smaller than the scenario deadline.

Supported steps are:

- `click`, which moves to the target and performs one press and release;
- `hover`, which moves without changing button state;
- `drag`, which presses at one target, follows deterministic intermediate
  points, and releases at another target;
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
key = { key = "Enter", action = "tap" }

[[scenarios.steps]]
wait = { frames = 3 }

[[scenarios.steps]]
timeout = "750ms"
wait = { object = "status", state = "visible" }

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

`key.key` is a case-sensitive Unity Input System `Key` enum name.
`key.action` is `down`, `up`, or `tap` and defaults to `tap`. Held keys are
allowed between explicit `down` and `up` steps but must be released before the
scenario ends.

`wait` has exactly one of `frames` or `object`. A frame count is a positive
32-bit integer and is allowed only in controlled mode. An object wait also
requires `state`. `assert` always requires `object` and `state`. State is one of
`exists`, `absent`, `visible`, `hidden`, `enabled`, or `disabled`. No condition
reads arbitrary component fields.

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

An omitted video motion resolves to `real-time`; a supplied value is
`controlled` or `real-time`, never `instant`. At video start the executor saves
the scenario motion and uses the resolved clip motion through stop or automatic
truncation. It then restores the scenario motion before settling subsequent
work. The required `VideoStep::Start.motion` is this resolved value.

Scenario fields are `name`, `motion`, `timeout`, and `steps`. Motion follows the
suite rules. Timeout is positive, at most the run deadline, and defaults to
`defaults.scenario_timeout`.

Object conditions are limited to facts Battlement already exposes to a player:
`exists`, `absent`, `visible`, `hidden`, `enabled`, and `disabled`. These checks
use the latest game-state snapshot published by the Rust engine and its
resulting Unity presentation. They do not read arbitrary C# fields or invoke
game methods.

### Targeting and input

An input target is either a stable Battlement UUID, an alias resolving to such a
UUID, or normalized render coordinates. Coordinates use a top-left origin and
each value is in the closed range from `0.0` to `1.0`. Pixel coordinates,
hierarchy paths, CSS selectors, object names, and runtime query languages are
not supported.

Presentation visibility and input reachability are different contracts.
`visible` requires that the object exists, is attached and active, is not
hidden by its own or an ancestor's display, visibility, or zero opacity, and
has nonempty projected bounds intersecting the render surface after clipping.
It does not test whether another object occludes it. `hidden` requires the
object to exist and fail that visibility test. An absent object is neither
visible nor hidden.

`enabled` and `disabled` apply only to UI elements and use
`enabledInHierarchy`. They ignore Ditto's transient global input suppression.
Using either state on a world object is an unsupported condition and fails the
step.

For object input, the player evaluates a fixed 5 by 5 lattice over the clipped
projected bounds. It tries the center first, then increasing squared distance
from center, with ties ordered top-to-bottom and left-to-right. A point is
usable only when the production EventSystem, UI Toolkit panel pick, or physics
raycast reports the requested object as the frontmost eligible receiver. A
click never bypasses an overlay or invokes an object action directly. Failure
diagnostics retain the bounds, every candidate, and the blocking object UUID
when known.

Ditto injects complete virtual Input System state transitions. A click uses a
move frame, press frame, and release frame. A drag uses
`ceil(normalized_distance / 0.05)` linear move segments, with at least one
segment. Each segment consumes one frame, the final segment reaches the exact
destination, and release consumes the following frame. Key input uses balanced
transitions. The player fails reset if a pointer button or key remains held.

Input stays inside the Unity player. Ditto never moves the host pointer, sends
host keyboard events, or asks macOS for Accessibility access. Native framebuffer
capture does not require Screen Recording access, and the macOS player does not
need to be frontmost. WebGL and iOS likewise inject inside the player rather
than automating browser or Simulator chrome.

iOS maps click and drag steps to one-finger touch sequences. A scenario that
contains any hover step is skipped as a whole on iOS Simulator, with
`unsupported-input:hover` in its result. This is a platform skip, not a pass or
failure, and happens before an engine is created.

### Fresh engine isolation

Ordinary `BattlementRunner.Reconnect()` keeps its existing contract: it
reconnects the retained native engine and preserves authoritative engine state.
Ditto does not overload that method with scenario semantics.

Ditto uses an explicit scenario boundary inside the native transport. Before
the first reached scenario, no engine exists. Before each later reached
scenario, the prior scenario's post-execution boundary has already destroyed
its engine and reset Unity state. To start a reached scenario, the player
performs this sequence:

1. Confirm that no native engine or held virtual input remains.
2. Allocate and activate a new engine-session ID.
3. Create a new native engine.
4. Connect the new engine and begin scenario execution.

Engine destruction and Unity reset belong exclusively to the scenario's
post-execution lifecycle. Starting a scenario never repeats them. A failed
post-execution destroy or reset marks the player non-reusable and forces a
relaunch before a later scenario.

The native `battlement_engine_destroy` ABI returns a status and writes a
bounded diagnostic buffer. A caught destructor panic marks the player
non-reusable; Ditto relaunches it before any later scenario. A reset failure is
a run-level infrastructure error. The reached scenario retains its functional
status, and Ditto relaunches to continue later scenarios within the remaining
run budget. If relaunch fails, later selected scenarios are `not-run` with
reason `run-infrastructure-error`.

The fixed ABI changes globally; there is no Ditto-only export or compatibility
shim:

```c
int32_t battlement_engine_destroy(
    BattlementEngine *engine, BattlementBuffer *out_error);
```

Status `0` confirms destruction and returns an empty buffer. A nonzero status
returns a caller-owned UTF-8 diagnostic using the existing buffer-freeing
contract. The ABI catches unwinding panics and never lets them cross into C#.

Ditto uses the ordinary `Connect` message unchanged and adds no method to the
`Engine` trait. Scenario identity and motion remain player-executor concerns;
they do not enter the rules-engine protocol. Every scenario begins from the
game's normal freshly constructed engine state and reaches any later state
through its player-facing steps.

### Settling and deadlines

A **command group** is one ordered batch of Battlement presentation commands
created from engine state. After every input action, Ditto performs
implicit settling before the next assertion or capture. A player is settled
when:

1. Battlement has no queued command groups or pending Rust request.
2. Every finite Battlement-owned operation required by the current motion mode
   is complete.
3. Unity has committed two consecutive rendered frames without a Battlement
   state or layout change.

The player executor observes the counters used in this decision. Work created
after the second quiet frame belongs to a later step. An explicit frame wait
advances an exact number of controlled frames. An object wait polls a named
condition after each committed frame. These are escape hatches for game-owned
asynchronous behavior, not required ceremony before every screenshot.

After an action, Ditto fully settles before the next step unless that
step is an explicit wait. An exact-frame or object wait preserves the exact
controlled state it reaches. A following assertion or screenshot observes that
state without completing the remaining timed operations. A direct
action-to-assertion or action-to-screenshot transition performs the full
settle. Instant and real-time motion always use the full settle rule.

Each step has a default two-second deadline and each scenario a default
ten-second deadline. The suite and an individual step or scenario may override
them. A declared step timeout must not exceed the configured scenario timeout.
At runtime, the effective step deadline is the earliest of its configured
timeout, the remaining scenario execution deadline, and the remaining run
deadline. Results record which deadline expired.

The scenario deadline starts before engine creation and ends after the last
executed step. It includes engine startup, actions, settling, captures, and
assertions. It excludes reset, hydration, image comparison, media processing,
final durability, player launch, build, and Simulator boot. `scenario.duration_ms`
measures this execution interval only.

The deadline model is normative. Every cap after startup is shortened by the
remaining run deadline:

| Deadline | Start to stop | Owner | Cap | Class |
| --- | --- | --- | ---: | --- |
| Build | Begin to immutable build | Host | `build` | T |
| Launch | Launch/dispatch to startup request | Host | `launch` | T |
| Startup | Request to acceptance | Both | Remaining launch | T |
| Simulator | Boot request to booted | Host | `simulator_boot` | T |
| Run | First accepted startup to result | Host | `run` | T |
| Scenario | Engine creation to final step | Player | Configured | F |
| Step | Step start through settle | Player | Configured | F |
| Reset | Freeze through destroy/reset | Player | 10 seconds | R |
| Baseline | Hydration to durable cache | Host | Configured | T |
| ODiff | Compare start to durable result | Host | 30 s/image | T |
| FFmpeg | Encode start to durable result | Host | 2 min/clip | T |
| Durability | Final flush to result commit | Both | 10 seconds | T |

`F` is scenario functional failure. `R` is recoverable run infrastructure:
Ditto may relaunch for later scenarios. `T` is terminal run infrastructure:
later scenarios do not start. The baseline cap is `baseline_download`.

The player enforces step and scenario deadlines locally. Ditto enforces phase,
HTTP, launch, platform, and run deadlines. No phase can extend the run deadline.
Every expired reached phase sets `PhaseResult.expired_deadline`; an expired step
also sets `StepResult.expired_deadline`. A run-deadline expiry records `run`
even when it shortened another cap.

### Motion modes

`instant` is the default. Battlement-owned tweens commit their final values,
Battlement-owned particles are suppressed, and Battlement-owned audio is muted.
This makes state transitions fast without asking scenarios to sleep through
animations.

`controlled` advances a deterministic frame clock only when the player executor
advances a scenario frame. Battlement tweens, particles, and audio timing use
that clock. A scenario may advance exact frames before capturing intermediate
states.

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

`battlement-ditto` contains the suite model, runner, platform adapters, HTTP
server, comparison client, baseline stores, run artifacts, and review server.
It provides a library and the `ditto` binary and depends on
`battlement-tooling`.

The existing `battlement-cli` depends on both crates and delegates
`cargo battlement ditto ...` to the same library entry point as the standalone
binary. It does not shell out to `ditto`. Argument parsing, defaults, output,
and exit codes are therefore identical.

Ditto owns its immutable build, fingerprint, and cache implementation.
`scripts/ci.py` shares only the machine-wide Unity editor capacity lease
protocol and lease paths. The Python and Rust clients implement that small
contract independently; unrelated CI orchestration remains unchanged.

## Builds and caches

Build reuse depends on two distinct content hashes. Neither value uses Git
commit identity or filesystem timestamps.

- The **source fingerprint** identifies the repository inputs that can affect
  the player. It is the SHA-256 of a sorted list of normalized paths,
  file modes, and file bytes.
- The **build fingerprint** identifies one reusable player output. It hashes the
  source fingerprint with the target, Unity, Rust, and applicable Apple
  toolchains, byte-affecting build options, diagnostics setting, capture
  adapter, and native plugin inputs. Profile name, display size, device type,
  orientation, and headless browser command are runtime inputs and are
  excluded. The iOS SDK and Apple toolchain are included; the Simulator runtime
  and device are excluded.
- The immutable build cache uses the build fingerprint. The player startup
  report contains both values. `ditto.lock` records the source fingerprint only
  as diagnostic context; neither fingerprint changes baseline identity.

`battlement-tooling` deliberately hashes conservative source roots instead of
attempting an exact dependency closure:

- Unity roots include the project's `Assets`, `Packages`, and
  `ProjectSettings`, plus the complete contents of referenced local Unity
  packages. Registry caches are excluded because the resolved package lock and
  installed toolchain identify them.
- Rust roots include every reachable local Cargo package's manifest, sources,
  build scripts, Cargo configuration, and applicable lock file.
- Native bindings, catalogs, or other generated inputs enter as the generator
  name, generator version, logical input name, and generated byte hash whenever
  a build consumes them. Generated run data, baselines, logs, caches, and editor
  temporaries are excluded.
- Repository-relative roots and symlinks may not escape the repository. This
  prevents an undeclared host file from changing a supposedly reusable build.

The reference performance budget for hashing these broad roots is 250
milliseconds on the repository's CI-class Apple Silicon host. The manifest is
sorted and streamed; the design does not add dependency-graph caching merely to
meet that budget.

Suite files, fragments, `ditto.lock`, aliases, scenario steps, and motion modes
do not enter the build fingerprint. They are
resolved into jobs or comparison inputs after build selection. Changing them
may rerun or recompare work, but does not rebuild the player unless the same
file also belongs to a declared Unity or Rust build dependency.

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
target, loaded Unity assets, HTTP session, and ODiff process stay alive while
the player executes scenarios serially. Rust engine instances never overlap.

CI obtains concurrency by running separate sample suites or profiles in
parallel, each with its own target and run directory. A single suite process
does not multiplex scenarios across one player because that would make logs,
input, and recovery harder to reason about.

Watch mode keeps the target, ODiff, and one review tab alive. It debounces file
events and reacts as follows:

- scenario or suite changes reload configuration, reset, and rerun selected
  scenarios;
- `ditto.lock` changes create a comparison-only cycle from retained current
  images when possible;
- a compiled input change computes a new fingerprint, builds if necessary, and
  relaunches once before rerunning; and
- review acceptance creates a comparison-only cycle without opening another
  tab.

Each watch cycle has its own run ID and records whether it reused images, reset
a player, launched a build, or performed comparison only. Watch mode keeps the
current player alive until a replacement build is complete. A comparison-only
cycle has `source_run_id` set to the immutable run that supplied its actual
images and records the current lock digest. It never edits that source run.

After a replacement build fails:

- The failed cycle has infrastructure status, no scenarios, and the build log.
  The review tab switches to it while retaining navigation to older runs.
- The stale player remains alive but idle. It never runs scenarios for the new
  source fingerprint.
- Another build-input edit creates a new source fingerprint and retries.
- A scenario-only edit is queued but does not retry the same broken build.
  Pressing `r` in the watch terminal explicitly retries that fingerprint.
- A lock-only edit may create a comparison-only cycle from an older run,
  visibly labeled with its older source fingerprint. It cannot create a passing
  result for the current source.
- Review acceptance creates a comparison-only cycle and does not retry a build.

For example, a code edit can produce a failed F2 build while the old player
remains idle and the F2 result retains the build log. A subsequent scenario
edit queues the scenario without building. A lock edit can recompare older
retained images, but it cannot create an F2 scenario result. When another code
edit produces a successful F3 build, Ditto replaces the old player and runs
the queued scenario.

This preserves warm resources without presenting results from old code.

## Runner diagnostics and HTTP session

`BattlementRunner` gains a serialized `runner diagnostics` option that is
enabled by default. When enabled, development and release players include the
existing log viewer and the Ditto scenario executor. When disabled, ordinary
Battlement Unity logging remains active, but the viewer and executor are omitted
or stripped and Ditto reports that the build cannot be automated.

Ditto serves one HTTP/1.1 endpoint on an available loopback port before it
launches a player. macOS players and Simulator apps receive its base URL as a
launch argument. WebGL receives the same value through its launcher page. All
targets use UnityWebRequest or the equivalent browser `fetch` implementation.

### Execution ownership

The CLI owns configuration, builds, baselines, comparison, retention, and the
terminal result. The player owns execution of each resolved scenario. This
keeps platform-sensitive input, settling, capture, and failure handling inside
Unity without making Unity parse the authoring format.

The boundary has these rules:

- Rust parses and validates TOML, applies defaults and filters, resolves aliases
  and paths, and serializes one platform-neutral JSON job.
- The player validates the normative job model, creates and connects a fresh
  engine, runs steps serially, and enforces step and scenario deadlines.
- The player uploads ordered batches copied from Battlement's unified managed
  log store and uploads PNG artifacts while it runs. It never compares
  baselines or writes the authoritative `result.json`.
- Ditto compares reached screenshots and decides whether later scenarios may
  run under `--bail`.
- The player handles a responsive runtime failure locally by freezing its
  controlled clock and capturing diagnostics. It then destroys and resets,
  flushes logs and artifacts, and waits for the host decision.

A job contains only selected runnable scenarios and already resolved runtime
data. This abbreviated, non-normative excerpt is not a fixture and omits
required fields for readability:

```json
{
  "job_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "run_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "remaining_run_timeout_ms": 10000,
  "scenarios": [{ "name": "human wins top row", "timeout_ms": 2000 }]
}
```

Resolved scenarios and steps each carry their own timeout duration. The player
starts these local monotonic timers when execution begins. Ditto starts a run's
authoritative monotonic timer when
it accepts that run's first `POST jobs/{job_id}/started`. Every recovery job
carries `remaining_run_timeout_ms`, computed from the original deadline;
relaunch never resets the budget. A later watch cycle is a new run with a new
deadline. The player treats the supplied remaining duration as a defensive
upper bound.

### Session isolation

Each launched player receives an unguessable **route token**, which is an
ephemeral URL path component that isolates one active player session. It is not
a credential against a malicious local user and requires no authentication
handshake or secret-redaction system.

An example base URL is:

```text
http://127.0.0.1:49152/ditto/7fe2d870b58242eeac46fcad5d89c8b1
```

The server applies these restrictions:

- It binds only to explicit IPv4 loopback and never accepts a remote peer.
- It returns `404` for an unknown or expired route token.
- It accepts WebGL requests only from the origin serving that player's launcher
  and sends no permissive CORS headers. Native players send no `Origin` header.
- It accepts only the documented methods, content types, and size limits.
- It invalidates the route when the player exits or the warm session ends.

The route token prevents accidental cross-run writes and unrelated web pages
from guessing an active endpoint. A local process with access to launch
arguments can discover it; defending against that process is outside the local
test runner's threat model.

### HTTP API

The Rust/Serde types below are the normative wire contract. The Unity package
implements matching C# models by hand. Shared positive and negative fixtures
prove that Rust and C# accept and reject the same payloads. Every struct rejects
unknown fields. The CLI and player are built and validated together; wire
compatibility between different builds is not required.

All Rust blocks labeled as normative in this document compose one module;
later blocks may use types introduced earlier, and the review-event types may
refer to the result types defined later.

#### Normative wire models

```rust
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub job_id: String,
    pub run_id: String,
    pub remaining_run_timeout_ms: u64,
    pub log_redactions: Vec<String>,
    pub command: Command,
    pub profile: ResolvedProfile,
    pub scenarios: Vec<ResolvedScenario>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Command { Run, Capture }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedProfile {
    pub name: String,
    pub platform: Platform,
    pub display: Display,
    pub build_fingerprint: String,
    pub source_fingerprint: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform { Macos, Webgl, IosSimulator }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Display {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub orientation: Option<Orientation>,
    pub safe_area: [u32; 4],
}

pub type DecimalString = String;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation { Portrait, PortraitUpsideDown, LandscapeLeft,
    LandscapeRight }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability { Click, Hover, Drag, Key, Png, Video }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedScenario {
    pub id: String,
    pub run_index: u32,
    pub name: String,
    pub motion: Motion,
    pub timeout_ms: u64,
    pub steps: Vec<ResolvedStep>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Motion { Instant, Controlled, RealTime }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedStep {
    pub index: u32,
    pub name: Option<String>,
    pub timeout_ms: u64,
    pub action: StepKind,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum StepKind {
    Click { target: InputTarget },
    Hover { target: InputTarget },
    Drag { from: InputTarget, to: InputTarget },
    Key { key: String, action: KeyAction },
    Wait(WaitStep),
    Assert(ObjectCondition),
    Screenshot(ScreenshotStep),
    Video(VideoStep),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InputTarget { Object(String), Coordinates([f64; 2]) }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyAction { Down, Up, Tap }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WaitStep {
    Frames(FrameWait),
    Object(ObjectCondition),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameWait { pub frames: u32 }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectCondition {
    pub object: String,
    pub state: ObjectState,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectState { Exists, Absent, Visible, Hidden, Enabled, Disabled }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenshotStep {
    pub name: String,
    pub comparison: Comparison,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub threshold: DecimalString,
    pub anti_alias: bool,
    pub max_changed_percent: DecimalString,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VideoStep {
    Start { name: String, motion: Motion, max_duration_ms: u64 },
    Stop,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Started {
    pub job_id: String,
    pub run_id: String,
    pub player_session_id: String,
    pub first_log_sequence: Option<u64>,
    pub startup_failure: Option<PlayerInfrastructureFailure>,
    pub startup_log_failure: Option<PlayerInfrastructureFailure>,
    pub identity: StartupIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StartupIdentity {
    Report(StartupReportIdentity),
    Accepted(AcceptedPlayerSessionIdentity),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupReportIdentity {
    pub startup_report: StartupReport,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedPlayerSessionIdentity {
    pub accepted_player_session_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupReport {
    pub platform: Platform,
    pub capture_adapter: String,
    pub build_fingerprint: String,
    pub source_fingerprint: String,
    pub unity_version: String,
    pub diagnostics: bool,
    pub display: Display,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogBatchAck {
    pub player_session_id: String,
    pub next_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInfrastructureFailure {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAck {
    pub artifact_id: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioComplete {
    pub scenario_id: String,
    pub execution_status: ExecutionStatus,
    pub steps: Vec<PlayerStepResult>,
    pub artifacts: Vec<ReachedArtifact>,
    pub failure_frame: Option<PlayerFailureFrame>,
    pub video_inputs: Vec<NativeVideoInput>,
    pub last_log_sequence: u64,
    pub execution_duration_ms: u64,
    pub startup_duration_ms: u64,
    pub boundary: ScenarioBoundaryOutcome,
    pub primary_error_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStepResult {
    pub index: u32,
    pub name: Option<String>,
    pub kind: StepName,
    pub status: StepStatus,
    pub duration_ms: u64,
    pub expired_deadline: Option<DeadlineKind>,
    pub error_refs: Vec<String>,
    pub assertion: Option<AssertionResult>,
    pub screenshot_artifact_id: Option<String>,
    pub video_input_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReachedArtifact {
    pub artifact_id: String,
    pub step_index: Option<u32>,
    pub kind: ArtifactKind,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ArtifactKind {
    Screenshot { checkpoint: String },
    FailureFrame,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PlayerFailureFrame {
    Captured { artifact_id: String },
    Unavailable { reason: String, error_ref: Option<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeVideoInput {
    pub input_id: String,
    pub start_step_index: u32,
    pub path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub frame_count: u64,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStatus { Passed, Failed, Interrupted }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScenarioBoundaryOutcome {
    Passed { duration_ms: u64 },
    Failed { duration_ms: u64, stage: BoundaryStage, error_ref: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoundaryStage { Destroy, Reset }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDecision {
    pub action: NextAction,
    pub completed_failures: u32,
    pub error_id: Option<String>,
    pub error_code: Option<ErrorCode>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NextAction { Continue, Stop, Relaunch }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobComplete {
    pub job_id: String,
    pub last_log_sequence: u64,
    pub executed_scenario_ids: Vec<String>,
    pub unstarted_scenarios: Vec<UnstartedScenario>,
    pub reason: TerminalReason,
    pub execution_duration_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobCompleteAck { pub job_id: String }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobFailed {
    pub job_id: String,
    pub failure: PlayerInfrastructureFailure,
    pub last_log_sequence: Option<u64>,
    pub executed_scenario_ids: Vec<String>,
    pub unstarted_scenarios: Vec<UnstartedScenario>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobFailedAck { pub job_id: String, pub error_id: String }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnstartedScenario { pub scenario_id: String, pub reason: String }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalReason { Completed, Bail, InfrastructureError, Interrupted }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DittoContextRecord {
    pub schema: u32,
    pub job_id: String,
    pub player_session_id: String,
    pub sequence: u64,
    pub timestamp_unix_us: i64,
    pub source: DittoLogSource,
    pub severity: DittoLogSeverity,
    pub event_name: String,
    pub message: String,
    pub body: DittoContext,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DittoLogRecord {
    pub schema: u32,
    pub job_id: String,
    pub player_session_id: String,
    pub sequence: u64,
    pub timestamp_unix_us: i64,
    pub source: DittoLogSource,
    pub severity: DittoLogSeverity,
    pub event_name: String,
    pub message: String,
    pub fields: BTreeMap<String, String>,
    pub exception: Option<String>,
    pub stack_trace: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DittoEventRecord {
    Context(DittoContextRecord),
    Log(DittoLogRecord),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DittoLogSource { Battlement, Rust, Unity, DittoPlayer }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DittoLogSeverity { Trace, Debug, Information, Warning, Error }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "context", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DittoContext {
    JobStarted { run_id: String },
    JobEnded { reason: TerminalReason },
    EngineStarted { engine_session_id: String, scenario_id: String },
    EngineEnded { engine_session_id: String, status: ExecutionStatus },
    ScenarioStarted { scenario_id: String },
    ScenarioEnded { scenario_id: String, execution_status: ExecutionStatus,
        failure_frame: Option<PlayerFailureFrame>,
        video_inputs: Vec<NativeVideoInput>, execution_duration_ms: u64,
        startup_duration_ms: u64, boundary: ScenarioBoundaryOutcome,
        primary_error_ref: Option<String> },
    StepStarted { scenario_id: String, step_index: u32 },
    StepEnded { scenario_id: String, result: PlayerStepResult },
    ArtifactAccepted { scenario_id: String, step_index: Option<u32>,
        artifact_id: String, artifact_kind: ArtifactKind },
    ErrorObserved { scenario_id: String, step_index: Option<u32>,
        error_ref: String, code: ErrorCode, source: ErrorSource,
        record_sequence: Option<u64>, battlement_error_id: Option<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpError {
    pub error_id: String,
    pub code: ErrorCode,
    pub message: String,
    pub expected_sequence: Option<u64>,
    pub related_run_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvent {
    pub id: u64,
    pub body: ReviewEventBody,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReviewEventBody {
    Snapshot { result: RunResult },
    LogBatch { player_session_id: String, first_sequence: u64,
        last_sequence: u64 },
    ScenarioCompleted { scenario_id: String },
    RunCompleted { run_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewAcceptance {
    pub request_id: String,
    pub run_id: String,
    pub lock_sha256: Option<String>,
    pub selections: Vec<ReviewSelection>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSelection {
    pub profile: String,
    pub scenario: String,
    pub checkpoint: String,
    pub width: u32,
    pub height: u32,
    pub actual_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewAcceptanceResult {
    pub comparison_run_id: String,
    pub lock_sha256: String,
}
```

Validation beyond Serde shape checks enforces decimal ranges, UUID and SHA-256
syntax, unique IDs and names, timeout relationships,
paired videos, platform capabilities, and the mutually exclusive startup
identity variants.

All identifiers and authored names are at most 128 UTF-8 bytes. A suite has at
most 128 scenarios. A scenario has at most 128 steps, 128 reached artifact
entries, and 64 native video inputs.
Each step has at most 16 player error references. A native path is at most 1024
UTF-8 bytes, and any diagnostic reason is at most 4096 UTF-8 bytes. Manifest
validation rejects inputs that exceed these fixed limits. With the fixed-size
fields above, every valid `ScenarioComplete` serializes below the 1 MiB JSON
body limit; a player treats a locally oversize completion as a structured
protocol failure rather than retrying a request that can only receive `413`.

Player error references are scenario-local strings `P0001`, `P0002`, and so
on. Each reference names one already-flushed `error-observed` context record in
that scenario; the surrounding ordinary error record provides its message and
diagnostics. When accepting completion, the host allocates the next run-local
`E####` occurrence for each previously unseen player reference and rewrites all
result references to those host IDs. Host-originated errors use the same
allocator directly. A retried completion reuses the stored mapping. Player and
host counters therefore cannot collide.

For every `DittoContextRecord`, `schema` is `1`, `player_session_id` is the
route-assigned pending or accepted player UUID, `source` is `ditto-player`, and
`event_name` is `ditto.context`. Lifecycle records use severity `information`;
`error-observed` uses
severity `error`. `message` is a bounded human rendering and is not parsed.
Start and end records are properly nested in job, engine, scenario, and step
order. An accepted artifact record is added to the unified store and uploaded
immediately after its artifact acknowledgement, before the player begins more
work. Player context ends with that player; relaunch recovery is represented by
`ScenarioResult.recovery` and the ordered old and new `JobResult` entries, not
by a context pair spanning two processes.

`DittoLogRecord` is the transport envelope for one immutable
ordinary `BattlementLogEntry` payload. The serializer adds `schema`, `job_id`,
and `player_session_id`; all other fields come from the store entry. A context
payload uses the same common presentation record but also retains one typed
`DittoContext`, which serializes in `body` instead of the ordinary string field
map. `DittoEventRecord` is the complete per-line union. The host rejects a line
whose job, player session, or sequence disagrees with its route and batch
position.

`DecimalString` is an unsigned base-10 value with at least one digit, optional
fractional digits, no exponent or sign, and no redundant leading zero. The
parser normalizes trailing fractional zeros before comparison. Input coordinates
and display scale remain JSON numbers and must be finite; coordinates are in
the closed range from zero through one and scale is positive.

#### Routes and idempotency

The session exposes these routes beneath its base URL:

- `GET job` returns the current resolved job. Repeating the request returns the
  same bytes until Ditto installs a later watch job.
- `POST jobs/{job_id}/started` validates the player and starts or resumes the
  run deadline, then returns `continue` or `stop`.
- `PUT jobs/{job_id}/logs/{player_session_id}?first_sequence={sequence}` accepts
  one contiguous NDJSON batch copied from the unified managed log store.
- `PUT jobs/{job_id}/artifacts/{artifact_id}` accepts one PNG body.
- `POST jobs/{job_id}/scenarios/{scenario_id}/complete` compares reached
  screenshots and returns `continue`, `stop`, or `relaunch`.
- `POST jobs/{job_id}/complete` accepts the player's terminal execution summary.
- `POST jobs/{job_id}/failed` accepts a small structured infrastructure failure
  when log delivery or another player-side transport facility cannot represent
  the failure in the log stream.
- `GET next-job?after={job_id}` waits for another watch job or session shutdown.

Every mutating route is idempotent:

- `started` is keyed by job ID. Its body names the player session and contains
  the nullable first log sequence plus the full startup report for a session's
  first job or its accepted session ID for a later warm job. A non-null value
  initializes that job's expected sequence. An identical retry returns the
  stored startup decision; a different body returns `409`.
- A log batch is keyed by job ID, player session ID, and first sequence. The
  host accepts it only at the next expected sequence. An identical batch retry
  returns the stored acknowledgement; different bytes for an accepted sequence
  or a gap return `409` with the expected sequence.
- An artifact is keyed by job ID and artifact ID. Identical bytes are
  acknowledged with the same `ArtifactAck`; different bytes return `409`.
- Scenario completion is keyed by job ID and scenario ID. Ditto stores its
  comparison and bail decision before replying. A retry returns that decision
  without comparing or counting the failure again.
- Terminal completion is keyed by job ID. A retry returns the stored
  `JobCompleteAck` and never finalizes the run twice.
- Infrastructure failure is keyed by job ID. A retry with the identical
  `JobFailed` returns the stored `JobFailedAck`; a different body returns
  `409`. `complete` and `failed` are mutually exclusive terminal operations.

All JSON is UTF-8. JSON request bodies and log-batch bodies are limited to
1 MiB. PNG bodies are limited to 64 MiB. The `GET job` response is limited to
128 MiB; the fixed suite, scenario, step, and string limits guarantee that every
valid resolved job fits. Integers are JSON integers. Fields ending
in `_ms` are integer milliseconds, and fields ending in `_bytes` are integer
byte counts.

Successful JSON routes return `200`; a new artifact returns `201`, with an
`ArtifactAck` body in both cases. A malformed request returns `400`, an
expired route returns `404`, a replay conflict or log gap returns `409`, an
ended warm session returns `410`, an oversize body returns `413`, and an
unsupported media type returns `415`. Error responses use the normative JSON
shape below. Internal storage or comparison failure returns `500` and makes the
run an infrastructure failure.

```json
{"error_id":"E0003","code":"transport.log-gap","message":"log sequence gap","expected_sequence":8192,"related_run_id":null}
```

A connection failure or response timeout has an uncertain outcome. The player
waits 100 milliseconds and retries the exact idempotent request once, bounded
by the remaining run deadline. A received `500`, model validation error,
conflict, or
second uncertain failure is terminal and is not retried. Long polling follows
the watch rules and is not part of this retry policy.

The normative Rust types contain the complete wire shapes. Their invariants
include:

- A job has `job_id`, `run_id`, `remaining_run_timeout_ms`, command, profile,
  ordered resolved scenarios, player-side log redactions, and every resolved
  motion, timeout, alias, input, assertion, capture, and video value needed to
  execute. Redactions contain only secrets already
  available to that player through its URL, arguments, or environment; host-only
  storage credentials never enter the job.
- `ResolvedScenario.run_index` is its zero-based position in
  `RunResult.scenarios`, including host-materialized skips. `ResolvedStep.index`
  is its zero-based authored position and is copied unchanged to every step,
  artifact, context, error, and result reference. A startup failure has no step
  index. Job first and last scenario indices use the same run coordinate
  system.
- A started body has `job_id`, `run_id`, `player_session_id`, nullable
  `first_log_sequence`, nullable `startup_failure`, nullable
  `startup_log_failure`, and either `startup_report` or
  `accepted_player_session_id`, but never both. `startup_failure` and
  `startup_log_failure` are mutually exclusive. `first_log_sequence` is null
  only when no entry can be uploaded.
- `startup_failure` reports a player-side probe, adapter, or other startup
  failure that ordinary log batching can still represent. It selects `stop`,
  followed by log flush and terminal `complete` with `infrastructure-error`.
- `startup_log_failure` reports a logging bridge failure, delivery overflow, or
  oversize record detected before acceptance. It selects `stop` and the small
  `failed` terminal route because the ordinary stream is incomplete.
- On a first job, `player_session_id` equals the route's pending player session.
  On a warm job it equals both the route's accepted session and
  `accepted_player_session_id`. A mismatch returns `409` without starting the
  run.
- A log batch has one player session and one nonempty, contiguous sequence
  range. The HTTP body contains complete NDJSON records in sequence order.
  The path carries job and player session, `first_sequence` is the sole query
  parameter, `Content-Length` supplies byte length, and `X-Ditto-SHA256`
  supplies the body hash. Its response is `LogBatchAck`. The request is raw
  NDJSON plus HTTP metadata, not a JSON object, so there is intentionally no
  aggregate `LogBatch` Serde type or C# body model.
- Scenario completion has scenario ID and status, reached artifact IDs, last
  log sequence, timings, boundary outcome, and nullable primary error
  reference. The host accepts it only after every log through that sequence is
  durable.
  Its response has action, completed-failure count, and nullable error ID,
  stable code, and message. `relaunch` is returned only for a failed destroy or
  reset boundary that the current player reported durably.
- Terminal completion has job ID, last log sequence, executed and unstarted
  scenario IDs, terminal reason, and execution timing. Its response is
  `JobCompleteAck`. The host accepts it only after every log through that
  sequence is durable.
- Infrastructure failure has a stable error code, bounded message, nullable
  last uploadable log sequence, and scenario accounting. The host durably
  accepts every available log through that sequence before acknowledging the
  failure. If the failure route is also unreachable, player supervision and
  secondary platform diagnostics synthesize the infrastructure occurrence.
- A successful next-job response is exactly a job object. `204` has no body,
  and `410` uses the common error object. All other successful acknowledgements
  have the fields described here and no additional fields.

#### Startup handshake

The startup report contains the facts that Ditto must verify before a scenario
runs:

```json
{
  "platform": "macos",
  "capture_adapter": "unity-async-readback-png",
  "build_fingerprint": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "source_fingerprint": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
  "unity_version": "6000.0.56f1",
  "diagnostics": true,
  "display": {
    "width": 1280,
    "height": 720,
    "scale": 1.0,
    "orientation": "landscape-left",
    "safe_area": [0, 0, 1280, 720]
  },
  "capabilities": ["click", "drag", "hover", "png"]
}
```

Display width and height are physical Unity framebuffer pixels. `safe_area`
uses the same pixels in `x`, `y`, `width`, `height` order with Unity's
bottom-left origin. `scale` is the diagnostic OS logical-to-physical ratio;
normalized input always maps over the full framebuffer. Ditto rejects a target,
build, source, Unity version, diagnostics setting, display, adapter, or
capability mismatch with `stop`. The accepted startup report is retained in
`result.json`.

Successful and rejected startup responses use the same small shape:

```json
{"action":"stop","completed_failures":0,"error_id":"E0001","error_code":"startup.mismatch","message":"wrong width"}
```

`error_id`, `error_code`, and `message` are null when `action` is `continue`.
Repeating an identical startup report returns the same response. A conflicting
repeated report is a transport failure.

A startup `stop` response does not immediately expire the route. The player
first uploads every representable queued record and then posts terminal
`complete` with `infrastructure-error`, or posts `failed` when log transport
itself is the reason startup cannot continue. The route expires after that
terminal acknowledgement or the run deadline. This preserves startup logs
without allowing engine creation or a scenario step to run after rejection.

When `Started.startup_log_failure` is non-null, the stop response allocates its
run-local occurrence immediately. The later identical `JobFailed.failure`
finalizes that occurrence, and `JobFailedAck.error_id` returns the same ID. It
does not count the startup failure twice.

When `Started.startup_failure` is non-null, the stop response likewise allocates
one occurrence. The player flushes through `JobComplete.last_log_sequence` and
posts `complete`; that terminal request finalizes the existing occurrence
without allocating another one. A host-detected startup mismatch follows the
same complete path using the occurrence returned in the stop response.

#### Events and artifacts

Battlement's unified managed log store is the source of application records on
every target. The store retains the newest 2,048 immutable entries. Under one
ordering lock, it assigns each entry a process-lifetime `u64` sequence, UTC
timestamp, source, severity, event name, message, string fields, optional
exception, and optional stack trace. Sources are `battlement`, `rust`, `unity`,
and `ditto-player`; severities are the `DittoLogSeverity` values above.

The immutable store payload is discriminated. An ordinary payload contains one
`BattlementLogRecord`. A context payload contains both a normal presentation
record and the lossless typed `DittoContext`. The presentation record lets the
Unity console, in-game viewer, and recent-error history handle the marker like
any other log; the typed body survives snapshots, observer delivery, and viewer
eviction without being reconstructed from strings.

```csharp
abstract record BattlementStoredPayload;
sealed record Ordinary(BattlementLogRecord Record) : BattlementStoredPayload;
sealed record Context(BattlementLogRecord Record, DittoContext Body)
    : BattlementStoredPayload;
```

Normal logging adds `Ordinary`. The Ditto executor adds `Context` through an
internal store operation that assigns its sequence and timestamp under the same
ordering lock. The observer maps these variants directly to
`DittoEventRecord::Log` and `DittoEventRecord::Context`.

Rust `tracing` first crosses the existing native queue, which is independently
bounded to 2,048 records and 4 MiB. `BattlementLoggingHost` drains that queue
into the managed store and Unity's logging APIs. Managed Battlement records and
ordinary Unity messages enter the same store. Ditto does not introduce a
native file logger or a log rotation ABI.

When runner diagnostics are included, the store exposes an internal observer
registration. Registration, the retained snapshot, and later callbacks share
the store's ordering lock, so an observer sees each sequence exactly once and
in order. The callback copies the immutable entry into a separate locked queue
before viewer retention may evict it. It performs no JSON serialization, file
IO, HTTP, or Unity API calls.

A **log capture window** is the interval whose entries belong to one Ditto job.
The first window opens during early bootstrap, before game startup code runs;
its entries remain temporarily unbound until the first job is fetched. Fetching
that job binds those entries to its ID, drains native tracing, and adds
`job-started`. A warm window closes after `job-ended` and terminal
acknowledgement. Entries emitted while the player waits for `next-job` remain in
the viewer and platform log but are not copied into a run. The next window
opens as soon as long polling returns a job and before any work for that job.

The delivery queue is transport state, not another application log. The queue
and any active retry batch together retain at most 2,048 records. Crossing that
limit records one `transport.log-buffer-overflow` failure without recursively
logging from the observer callback, stops admitting later entries, and leaves
the retained prefix contiguous. Before acceptance this becomes
`Started.startup_log_failure`; during a job it is terminal run infrastructure
failure. The ordinary Unity player log and already accepted Ditto records
remain secondary diagnostics.

Ditto adds ordered context records through the same managed logging path for
job, engine, scenario, step, artifact, and error boundaries. The serializer
adds the active job and player session IDs to each captured entry. It does not
duplicate scenario or step IDs in ordinary records and does not infer
asynchronous attribution. Each reached scenario result stores the exact
player-session sequence range selected by its context records.

Before serialization, the player replaces route tokens and configured
player-side `Job.log_redactions` values with `<redacted>` in every free-form
string. This includes messages, ordinary field values, exceptions, stack
traces, authored names, diagnostic reasons, and paths nested anywhere in a
context body. Generated UUIDs, hashes, stable codes, enum tags, field names,
sources, and event names are identifiers and may not contain secrets. The host
redacts its own output before persistence but never mutates an accepted log
body.

Each NDJSON line is one `DittoEventRecord` with `schema` equal to `1`. Bodies
are UTF-8, contain no blank lines, use one LF after every JSON object, and end
with LF. The host validates each object and appends the exact uploaded bytes;
it does not parse and reserialize accepted records. This abbreviated mixed batch
shows an application record followed by a context record:

```json
{"schema":1,"job_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8","player_session_id":"0197b35f-6d12-71ac-b370-0bb2cbced1b2","sequence":81,"timestamp_unix_us":1787953800000000,"source":"rust","severity":"information","event_name":"chess.move","message":"move applied","fields":{"piece":"knight"},"exception":null,"stack_trace":null}
{"schema":1,"job_id":"0197b35f-6c59-7b98-b1f0-a39f5ee54db8","player_session_id":"0197b35f-6d12-71ac-b370-0bb2cbced1b2","sequence":82,"timestamp_unix_us":1787953800000100,"source":"ditto-player","severity":"information","event_name":"ditto.context","message":"scenario started","body":{"context":"scenario-started","scenario_id":"0197b35f-6e24-75d8-9482-aa6c22a15133"}}
```

The executor forces the existing native tracing drain before every context
boundary, then serializes the next complete batch on the main thread. It
uploads during activity at most once every 100 milliseconds and flushes at
step, scenario, and terminal boundaries. One request is at most 1 MiB; a larger
backlog is split between records. Exact serialized bytes remain in the retry
buffer until the host acknowledges the next sequence.

One serialized record, including LF, must fit in one 1 MiB request. An entry
that cannot fit produces `transport.log-record-oversize` and terminal
infrastructure failure; Ditto does not silently omit or truncate it. The native
queue may emit `battlement.logging.records_dropped` when Unity did not drain
quickly enough. Ditto treats that warning, and `battlement.logging.failed`, as
terminal infrastructure because application history is incomplete.

Store sequences increase for one process and restart at process launch, so
`player_session_id` namespaces each sequence. A warm player keeps the same
session and increasing store sequence across jobs, although idle entries may
create a gap between job ranges. `Started.first_log_sequence` establishes each
job's inclusive first captured sequence. A relaunched player receives a new
player session and may restart at sequence one. Player session plus sequence
identifies the source store entry; job ID additionally identifies its transport
and run ownership.

For example, a batch request carries its first sequence in the URL and the
exact NDJSON bytes in the body:

```http
PUT jobs/0197.../logs/0197...?first_sequence=8192 HTTP/1.1
Content-Type: application/x-ndjson
Content-Length: 16384
X-Ditto-SHA256: <hex>
```

The host validates the body hash, player session, complete-record boundaries,
and contiguous sequences. It appends the records to the run's
`logs/events.jsonl`, syncs them, and only then acknowledges the next sequence.
Identical retries are harmless. Gaps and conflicting duplicates are terminal
transport failures and report the expected sequence. A run deadline, delivery
overflow, or host disk failure terminates the run; Ditto never discards queued
records to let execution continue.

##### Log lifecycle and correlation

Context markers follow one emission order. This order defines scenario spans
and decides whether an error is functional or infrastructure:

1. After fetching a job, bind the open capture window to that job, drain native
   tracing, add `job-started`, and post `started`. No engine is created before
   the host returns `continue`.
2. Before engine creation, drain tracing and add `scenario-started`. The
   functional error gate opens at this sequence, so creation and connection
   failures belong to the reached scenario.
3. Allocate the engine-session ID. After native creation succeeds, add
   `engine-started` and connect. A creation failure has no engine start marker;
   a connection failure does.
4. Around each reached step, drain tracing and add `step-started`, then freeze
   execution before adding `step-ended` with the complete `PlayerStepResult`.
   Actions, settling, assertions, and capture occur between those markers.
5. When a functional failure freezes execution, add `error-observed` directly
   after the source record when one exists. `record_sequence` names that record;
   `battlement_error_id` carries its local Battlement error ID when present.
   Capture the failure frame only after this correlation marker is queued.
6. Close the functional gate and attempt engine destruction. Add
   `engine-ended` only when `engine-started` exists, then reset Unity. Destroy
   or reset errors are infrastructure even though they remain inside the
   scenario's diagnostic span. Add `scenario-ended` with the complete scenario
   boundary payload after the boundary attempt, then flush logs and artifacts
   before scenario completion.
7. After the last selected scenario or a host stop decision, drain tracing, add
   `job-ended`, flush through that sequence, and post terminal completion. Close
   the capture window only after acknowledgement.

A responsive failure follows all reachable markers. A process or page crash
may omit `step-ended`, `engine-ended`, `scenario-ended`, and `job-ended`. The
host then synthesizes a scenario whose `LogSpan.complete` is false, beginning
at `scenario-started` and ending at the last durable sequence for that player
session. A complete span has both scenario markers.

If `scenario-ended` is durable but the completion request is not, its step-end,
artifact-accepted, error-observed, and scenario-end records contain the full
player payload needed to reconstruct `ScenarioComplete`. The host performs the
ordinary comparison once, retains the scenario's functional status, and adds a
run-level `runtime.process-exit` occurrence. Its `LogSpan.complete` is true
because that field describes log bracketing, not receipt of the HTTP commit.
No synthesized end marker is inserted into `events.jsonl` in either crash case.

A PNG artifact upload uses `Content-Length`, SHA-256, decoded width, and decoded
height headers. Repeating identical bytes succeeds; reusing an artifact ID for
different bytes is a transport failure. Native video input is never uploaded
through HTTP.

A PNG upload has this shape:

```http
PUT jobs/0197.../artifacts/0197... HTTP/1.1
Content-Type: image/png
Content-Length: 42817
X-Ditto-SHA256: <hex>
X-Ditto-Width: 1280
X-Ditto-Height: 720
```

The player receives acknowledgement for every PNG before writing the context
record that references it. Before scenario completion, it waits for every
referenced artifact and log record to be durable on the host.

That context record is `artifact-accepted` and contains the scenario ID, step
index, artifact ID, and tagged artifact kind. Screenshot artifacts require the
capturing step's index. A failure frame uses the active step index, or null when
the failure occurs during startup or between steps. It is written immediately
after the upload acknowledgement, then immediately batch-uploaded and
acknowledged before execution continues. If the player crashes before
completion, these records let the host retain reached
screenshots, identify a captured failure frame, and synthesize the active
scenario without interpreting artifact IDs.

#### Scenario completion and bail

A player stops executing between scenarios while Ditto finalizes the completed
scenario. This is the only required host decision during an ordinary job. The
following state machine is normative:

1. Create and connect a fresh engine.
2. Execute steps until completion or a terminal step failure.
3. Freeze controlled work and finalize any active video as truncated.
4. Destroy the engine and reset Battlement-owned Unity and input state.
5. Flush all referenced JSONL bytes and PNG artifacts durably to the host.
6. Download only baselines for checkpoints actually reached.
7. Compare reached images and process native video inputs as one host batch.
8. Atomically update `partial-result.json`, then store the scenario decision.
9. Return `continue` or `stop`; only then may another scenario start.

Step 4 always produces `ScenarioBoundaryOutcome`. If destroy or reset fails but
the executor remains responsive, it records the boundary error, performs the
remaining safe cleanup, flushes, and posts `ScenarioComplete`. Ditto durably
records the scenario, replies `relaunch`, and starts a recovery job at the next
scenario after the player exits. If the failure also kills transport, the host
synthesizes an incomplete-boundary infrastructure failure from the last durable
context and owned-process exit, commits the active scenario, and relaunches.
The precise destroy-versus-reset stage is retained only when its structured
record was durable. Ditto never parses free-form log text to determine this
transition.

The scenario deadline ends after step execution. Reset, hydration, comparison,
media, and durability use their separate phase caps. A completion retry with an
identical body returns the stored decision without repeating comparison,
publication, or bail accounting.

`continue` requires a passed boundary and no bail or terminal infrastructure
decision. `relaunch` requires a durably reported failed boundary and never
starts another scenario in that player. `stop` covers bail, interrupt, and
terminal infrastructure. In all three cases, the host stores the decision
before responding.

`--bail` and `--bail=N` stop before the next scenario once the failed-scenario
count reaches the selected value. Each failed scenario increments the count
once, regardless of its number of failed steps or screenshots. A screenshot
mismatch does not interrupt the remaining steps of its current scenario. The
player flushes diagnostics and posts `complete` before it exits after `stop`.

Capability skips occur before engine creation and are `skipped` with explicit
reasons.
Scenarios suppressed by bail are `not-run` with reason `bail`. Scenarios never
started after terminal infrastructure failure are `not-run` with reason
`run-infrastructure-error`.

The ordinary response is deliberately small:

```json
{"action":"continue","completed_failures":0,"error_id":null,"error_code":null,"message":null}
```

This acknowledgement is request and response, not a general remote-control
protocol. Input, waits, assertions, capture, freeze, reset, and shutdown are
local player operations.

A `--bail=2` run in which both scenarios fail has this coarse exchange:

1. The player gets job A, which contains S1 and S2, and posts `started`.
2. It uploads S1 artifacts and completes S1. Ditto replies `continue` with one
   completed failure.
3. It uploads S2 artifacts and completes S2. Ditto replies `stop` with two
   completed failures.
4. It posts job completion, receives acknowledgement, and exits.

#### Watch mode

Watch mode keeps the same player and HTTP session warm without server-initiated
messages. After posting a cycle's `complete`, the player long-polls
`next-job?after={job_id}`.

- A scenario or suite edit installs a new resolved job and completes the poll.
- A poll returns after at most 30 seconds with `204`, after which the player
  immediately repeats it.
- Session shutdown returns `410`, which makes the player exit cleanly.
- A compiled input change builds a replacement first. Ditto closes the old
  session only after that build succeeds, then launches the replacement.
- A lock-only edit reuses retained images when possible and does not wake the
  player merely to repeat execution.

An edit that arrives during an active cycle never changes its immutable job.
Ditto coalesces pending edits and snapshots their latest resolved state into
the next cycle after the active cycle finalizes. A successful replacement build
waits idle until that boundary. A player loss while no run is active does not
create a failed run; Ditto marks the warm session stale and launches a new one
when the next execution job is ready.

The long poll is outside every scenario and run deadline. Each returned watch
job belongs to a new run. That run's timer starts when Ditto accepts its
job-scoped `started` request.

#### Transport failures

HTTP requests use remaining run time as their upper bound. An uncertain request
is retried exactly once after 100 milliseconds because every mutating operation
is idempotent. The player does not begin another step while a required log or
artifact upload is unresolved.

If transport cannot recover before the run deadline:

- the player stops scenario execution and records the failure in its ordinary
  Unity log when possible;
- the player posts `failed` when that small route remains reachable; otherwise
  Ditto synthesizes the occurrence from supervision and secondary diagnostics;
- Ditto finalizes uploaded log batches and artifacts as partial diagnostics;
- Ditto reports an infrastructure failure rather than a scenario failure; and
- remaining scenarios become `not-run` with reason
  `run-infrastructure-error`.

No heartbeat is required. Ditto uses HTTP request deadlines, the overall run
deadline, and process or page supervision to detect a stalled target.

A crash after Ditto durably accepts scenario completion but before the next
scenario begins does not synthesize another failure. Ditto relaunches at the
next unstarted scenario with the original run's remaining budget. A crash while
the player is waiting for another watch job has no active run to fail. A
graceful unload or owned-process exit marks that player session stale; an
unobservable local WebGL tab loss is discovered when the next job fails its
launch deadline, and that new run receives infrastructure status.

Dispatching a watch job to a presumed-warm player starts that run's launch
phase immediately, even though no new process is launched. The phase ends when
the job-scoped startup request arrives. If a lost WebGL tab never requests
startup, the configured launch deadline expires, the phase records
`expired_deadline = launch`, and the run finalizes as infrastructure failure.
The run timer still starts only after accepted startup, so this pre-start
watchdog is exclusively the launch deadline.

#### Platform launchers

macOS starts the immutable player with the session base URL as a launch
argument. The player fetches its job before creating an engine and exits after
Ditto acknowledges completion unless watch mode supplies another job.

iOS Simulator uses the same HTTP API. Every player session creates one
ephemeral Ditto-owned Simulator device with the profile's exact installed
runtime, device type, and orientation. Watch jobs reuse that device until the
session ends. Normal and stale-session cleanup delete it. An unavailable
runtime or device type is a configuration error that lists installed
alternatives. Ditto passes the base URL through `simctl launch`, uses explicit
IPv4 loopback, and builds the app with the narrow local-network transport
setting required by supported Simulator runtimes. A physical iOS device is not
a target.

For WebGL, Ditto serves the immutable build, HTTP API, and minimal launcher from
one loopback origin. Local runs open the URL with the operating system. CI
profiles may provide a headless command containing a required `{url}`
placeholder. Ditto starts that command directly and does not require
Playwright, Selenium, a browser extension, or a hosted page.

The WebGL launcher gives the player its session base URL and uses a small
`.jslib` adapter where UnityWebRequest cannot efficiently pass a browser Blob.
The adapter uploads `canvas.toBlob` results as raw HTTP bodies. Browser console
errors, unhandled JavaScript exceptions, and unhandled promise rejections call
a bounded JS-to-managed bridge while the page is responsive. The bridge emits
ordinary Unity entries into the managed log store. Ditto supervises the process
of a configured headless command and synthesizes an occurrence from a nonzero
exit. An abrupt exit from an operating-system-opened browser is not directly
observable and therefore becomes a launch or run deadline failure; it cannot
create a final managed record after the page is gone.

## Capture adapters

All capture adapters return the Unity render surface only. Window borders,
browser chrome, Simulator chrome, cursors, and host notifications are excluded.
The startup report names the selected adapter and its effective dimensions.

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

Simulator launch uses `xcrun simctl` and an ephemeral device owned by the player
session. Simulator boot time is measured separately. Ditto installs and
verifies the exact app build before running, reuses the device only for warm
jobs in that session, and deletes it during normal or stale-session cleanup.

### WebGL

WebGL calls `canvas.toBlob("image/png")` on Unity's render canvas after the
requested frame is presented. The blob is uploaded as the raw body of an
artifact `PUT`. The adapter does not take a browser screenshot and does not
encode WebP.

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
Ditto. The lock uses this TOML shape:

```toml
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
```

The illustrative hashes have the required shape but are not fixture values.
Entries are sorted by profile, scenario, and checkpoint and contain:

- the suite namespace;
- profile, scenario, and checkpoint identity;
- the SHA-256 of the exact PNG bytes;
- width, height, and byte size;
- the 64-hex-digit source fingerprint in `source` for diagnostics.

Comparison policy exists only in `ditto.toml`. `result.json` records the
effective settings used for each reached screenshot. Changing tolerance never
rewrites `ditto.lock`.

Rewriting an unchanged manifest produces identical bytes. Acceptance time stays
in the local run result and remote replacement metadata rather than the lock.
Absolute paths, credentials, run IDs, and host names never appear in the file.

An ordinary `run` fails a reached checkpoint that has no manifest entry. If a
reached entry is absent from the local cache, Ditto downloads it only after the
scenario has executed and reset. Download time counts against the run and is
bounded by the earlier of `baseline_download` and the remaining run deadline.
A hash mismatch is infrastructure failure and the invalid object is not cached.

### Full update

`run --update` proposes screenshots only from scenarios that complete execution
and reset without assertion, timeout, runtime, panic, or reset failure. Captures
from ineligible scenarios remain local run artifacts and are never published.
Passing scenarios remain eligible when a different scenario fails. Unreached
checkpoints remain unchanged, and the command still exits nonzero for failures.

A missing or mismatching baseline is not a scenario failure in update mode and
does not increment the bail counter. Its screenshot result records
`matched_before_update = false` and `updated = true` after the atomic manifest
rewrite succeeds. The screenshot step then passes. A matching checkpoint records
`matched_before_update = true` and `updated = false`. A storage or manifest
transaction failure is infrastructure failure, leaves `updated = false`, and
prevents the command from reporting success.

At the end of the run, Ditto uploads every eligible proposed object, verifies
successful storage, and rewrites `ditto.lock` once with an atomic local rename.
The store and manifest transaction is all-or-nothing even though eligibility is
per scenario. If any upload fails, the lock remains byte-for-byte unchanged and
the result lists uploaded objects that are safe but not yet referenced.

An unfiltered update removes, within the selected profile, every manifest entry
whose scenario or checkpoint no longer exists in the full suite. Entries in
other profiles are untouched. A scenario-filtered update never prunes an entry
for an unselected scenario and cannot target a name absent from the current
suite. Within each selected existing scenario it prunes checkpoints removed
from that scenario. Runtime skips and unreached checkpoints do not affect this
authoring comparison and never delete an entry that still exists in the suite.

At run start, Ditto records the SHA-256 of `ditto.lock` and each selected entry.
Before any update or review acceptance, it acquires a suite-local file lease and
rereads the lock. If the whole starting digest differs, acceptance is stale and
the lock is not changed, even when the selected entries happen to match. The
already uploaded content-addressed PNGs are harmless and reusable. This avoids
silently overwriting an edit, Git operation, or acceptance from another Ditto
process. Atomic rename prevents a torn write; the digest check prevents a lost
write.

A missing `ditto.lock` is the explicit absent state, not a configuration error
or an implicit empty file. `RunResult.lock_sha256` and
`ReviewAcceptance.lock_sha256` are null for that state. An update or acceptance
may create the initial lock only if it is still absent under the mutation lease.
Any newly appearing file makes the request stale. The successful atomic write
records the new file's canonical 64-hex digest.

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

After a scenario executes and resets, Ditto checks the local cache for each
reached baseline hash. A miss causes one download from the configured public
URL, verification, and atomic cache insert. Concurrent requests for the same
hash share a lease. Earlier functional failures therefore are not masked by a
network request for a checkpoint execution never reached. `ditto fetch --all`
prewarms every manifest entry with bounded parallelism.

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
lock is readable, then replaces one namespace state document. It contains the
generation, lock digest, publication time, sorted live PNG hashes, tombstones
with removal times, and last applied cleanup time. A restored hash is removed
from the tombstones.

The first published state has generation `1`. Every successful publication and
every applied cleanup increments generation exactly once; dry runs and failed
mutations do not. When publication moves a live hash to a tombstone,
`removed_at` is that publication's timestamp. Republishing a tombstoned hash
removes its tombstone. Removing it again later creates a new tombstone with the
new publication time.

R2 stores it at `<namespace>/metadata/state.json`. A filesystem store uses
`<root>/<namespace>/metadata/state.json`. PNG objects use
`<namespace>/objects/<first-two-hash-characters>/<sha256>.png` in either store.

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineStoreState {
    pub generation: u64,
    pub lock_sha256: String,
    pub published_at: String,
    pub live_sha256: Vec<String>,
    pub tombstones: Vec<BaselineTombstone>,
    pub cleanup_applied_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineTombstone {
    pub sha256: String,
    pub removed_at: String,
}
```

Arrays are sorted by SHA-256. UTC timestamps use RFC 3339 with whole-second
precision and a trailing `Z`. State serialization uses lexical keys, two-space
indentation, and one final newline so filesystem and R2 bytes are identical.

Publish and cleanup share a suite mutation lease. R2 acquires the lease with a
conditional object write, records a random owner and short expiry, and refreshes
it while work continues. State replacement uses the ETag read under that lease
as an `If-Match` condition. Losing the lease or ETag race aborts without
deleting data.

The lease object is `<namespace>/metadata/write-lease.json`. It contains a
random owner and UTC expiry 60 seconds ahead. Acquisition uses `If-None-Match`
when absent or `If-Match` against an expired lease's ETag. The owner refreshes
every 20 seconds with `If-Match`. State creation uses `If-None-Match`; later
replacement uses the state ETag's `If-Match`. Lease loss aborts before another
delete or state write.

On every success or failure path, the owner conditionally deletes the lease
using its last ETag after its final state operation. An ETag mismatch means
ownership was already lost and no delete is attempted. Failure to release after
a successful mutation emits a warning but does not roll back that mutation;
the lease then expires naturally within 60 seconds. A later writer never
deletes a live predecessor's lease merely to avoid waiting.

`ditto clean storage` is the only operation that deletes baseline objects. It
rereads state under the lease, deletes hashes tombstoned for at least seven days
and absent from the live set, treats missing objects as already deleted, and
conditionally replaces state with the applied cleanup time. It prints a dry-run
plan unless `--apply` is supplied. Only `storage publish` changes the live set.

Filesystem stores use the same namespace state, live set, tombstones, seven-day
retention, publication command, and explicit cleanup behavior. They replace
state with atomic rename while holding an advisory store lock. `doctor` verifies
locking and rename behavior before any write-capable command. R2 uses
conditional replacement under its remote lease.

The filesystem lock is `<root>/<namespace>/metadata/write.lock`. Writers hold
an exclusive advisory lock from state read through object verification and
state rename. They write and sync a sibling temporary file, atomically rename
it over `state.json`, then sync the metadata directory before releasing the
lock.

Write-capable update, acceptance, `doctor`, and `storage publish` warn when
eligible tombstones exist and no cleanup was applied in the last 14 days.
Read-only runs do not warn. Ordinary runs never perform cleanup.

Seven-day replacement retention means an old Git branch may eventually name an
object no longer available remotely. A developer who still has it cached can
work offline; otherwise the branch needs a current accepted baseline. This is
an intentional tradeoff because retaining old screenshot versions is not a
project requirement.

## Local run data

`run`, `capture`, each watch execution cycle, and each comparison-only
acceptance or recomparison cycle create a run. `doctor`, `list`, `fetch`, clean
commands, `storage publish`, and merely opening review do not. Ditto allocates a
user-cache run directory before discovery or build, then indexes its repository
and suite identity after discovery.

An active run atomically replaces `partial-result.json` after phase start,
accepted player startup, every durable scenario, every relaunch, and comparison
work. Terminal `result.json` removes the partial file and makes the run
immutable. On the next startup, Ditto acquires an abandoned run's expired lease,
converts its checkpoint into an interrupted terminal result, and retains later
complete logs or artifacts as unreferenced diagnostics rather than guessing
that they belonged to a completed step.

If final rename or directory sync fails, Ditto leaves the last synced
`partial-result.json` and writes its terminal candidate there when storage still
permits. It does not claim the run finalized, and exits `2` with the run path on
stderr; `--json` also emits the candidate as best-effort diagnostics. The next
Ditto startup acquires the expired run lease and retries terminal commit with a
`durability.result-commit-failed` occurrence. If no further write is possible,
the directory remains abandoned until storage is repaired. Only a successfully
committed `result.json` is an authoritative terminal result.

A lock edit, review acceptance, or other comparison-only refresh creates a new
run directory. It records `source_run_id`, materializes the reused actual images
with hard links or copies, and writes a new `result.json` using the current lock
digest. It materializes the source `logs/events.jsonl` and every referenced
secondary diagnostic the same way, then keeps each `LogSpan.path` and
`diagnostic_paths` relative to the derived run. The source run remains immutable
and independently retainable. Ditto holds a lease on it until the derived run
has materialized every referenced artifact.

Every watch cycle has its own run ID, active directory, terminal result, and
retention lifetime. The review tab follows the active cycle but does not own or
mutate its files.

Each successfully finalized run contains:

- the resolved suite and redacted profile;
- actual screenshots, downloaded baseline references, and diff masks;
- any automatic failure frame and experimental video;
- one ordered `logs/events.jsonl` stream containing job-qualified events from
  every scenario and player session;
- `logs/build.log` when build output exists;
- player, browser, Simulator, build, and ODiff diagnostics; and
- one stable `result.json`.

Artifact directories use readable slugs. A slug has a bounded ASCII prefix,
percent-encodes unsafe UTF-8 bytes, and ends in a deterministic hash suffix.
Prefix truncation never splits an escape. The suffix prevents case-folding,
sanitization, and truncation collisions; the original scenario and checkpoint
names in `result.json` remain authoritative.

`result.json` includes run, job, player-session, and scenario IDs; every
accepted player startup report; build reuse and launch timing; startup and step
timing;
skip reasons; warnings; assertions; screenshot hashes, paths, and scores;
effective comparison settings; error IDs; player-qualified log sequences;
recovery actions; and final status. Arrays preserve execution order. Maps with
user-defined names are serialized in lexical key order. Local paths are
repository-relative where possible and secrets are redacted.

The Rust/Serde types below are the complete normative result contract. They use
the shared wire enums defined above and reject unknown fields.

### Normative result model

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub run_id: String,
    pub source_run_id: Option<String>,
    pub lock_sha256: Option<String>,
    pub command: ResultCommand,
    pub source_command: Option<ResultCommand>,
    pub cycle: u32,
    pub suite: Option<String>,
    pub profile: Option<String>,
    pub started_at: String,
    pub duration_ms: u64,
    pub status: RunStatus,
    pub exit_code: u8,
    pub build: Option<BuildResult>,
    pub phases: Vec<PhaseResult>,
    pub player_sessions: Vec<PlayerSessionResult>,
    pub jobs: Vec<JobResult>,
    pub scenarios: Vec<ScenarioResult>,
    pub warnings: Vec<String>,
    pub errors: Vec<ErrorOccurrence>,
    pub baseline_writes: Vec<BaselineWriteResult>,
    pub artifacts: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultCommand { Run, Capture, ComparisonOnly }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus { Passed, Failed, InfrastructureError, Interrupted }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildResult {
    pub source_fingerprint: String,
    pub fingerprint: String,
    pub disposition: BuildDisposition,
    pub duration_ms: u64,
    pub log_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildDisposition { Created, Reused, RequiredByNoBuild, Failed }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseResult {
    pub name: PhaseName,
    pub status: PhaseStatus,
    pub duration_ms: u64,
    pub expired_deadline: Option<DeadlineKind>,
    pub log_path: Option<String>,
    pub error_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseName { Discovery, Build, Launch, Startup, Scenarios, Cleanup,
    SimulatorBoot, Reset, BaselineDownload, Comparison, Media, Durability }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseStatus { Passed, Failed, Interrupted }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSessionResult {
    pub player_session_id: String,
    pub accepted: bool,
    pub startup_report: StartupReport,
    pub diagnostic_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobResult {
    pub job_id: String,
    pub player_session_id: String,
    pub status: JobStatus,
    pub first_scenario_index: Option<u32>,
    pub last_scenario_index: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus { Passed, Failed, InfrastructureError, Interrupted }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioResult {
    pub id: String,
    pub name: String,
    pub status: ScenarioStatus,
    pub status_reason: Option<String>,
    pub motion: Motion,
    pub duration_ms: u64,
    pub expired_deadline: Option<DeadlineKind>,
    pub timings: ScenarioTimings,
    pub steps: Vec<StepResult>,
    pub logs: Option<LogSpan>,
    pub failure_frame: Option<MediaCapture>,
    pub recovery: Recovery,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioStatus { Passed, Failed, Skipped, NotRun,
    InfrastructureError, Interrupted }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioTimings {
    pub startup_ms: Option<u64>,
    pub reset_ms: Option<u64>,
    pub baseline_download_ms: Option<u64>,
    pub comparison_ms: Option<u64>,
    pub media_ms: Option<u64>,
    pub durability_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogSpan {
    pub job_id: String,
    pub player_session_id: String,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub complete: bool,
    pub path: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Recovery { None, Reset, Relaunch, RelaunchFailed }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StepResult {
    pub index: u32,
    pub name: Option<String>,
    pub kind: StepName,
    pub status: StepStatus,
    pub status_reason: Option<String>,
    pub duration_ms: u64,
    pub expired_deadline: Option<DeadlineKind>,
    pub error_ids: Vec<String>,
    pub assertion: Option<AssertionResult>,
    pub screenshot: Option<ScreenshotResult>,
    pub video: Option<VideoResult>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepName { Click, Hover, Drag, Key, Wait, Assert, Screenshot, Video }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus { Passed, Failed, NotRun, InfrastructureError, Interrupted }

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeadlineKind { Step, Scenario, Run, Reset, BaselineDownload,
    Build, Launch, Startup, SimulatorBoot, Comparison, Media, Durability }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineWriteResult {
    pub sha256: String,
    pub profile: String,
    pub scenario: String,
    pub checkpoint: String,
    pub status: BaselineWriteStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BaselineWriteStatus { Proposed, UploadedUnreferenced, Published }

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssertionResult {
    pub object: String,
    pub state: ObjectState,
    pub expected: bool,
    pub observed: bool,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScreenshotResult {
    Captured {
        checkpoint: String,
        actual: ImageFile,
        baseline: BaselineOutcome,
        comparison: Option<ComparisonOutcome>,
        matched_before_update: Option<bool>,
        updated: Option<bool>,
    },
    Unavailable { reason: String, error_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BaselineOutcome {
    NotLoaded,
    Missing,
    Loaded { image: ImageFile },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ComparisonOutcome {
    Passed { changed_pixels: u64, total_pixels: u64,
        settings: Comparison },
    Mismatch { changed_pixels: u64, total_pixels: u64,
        settings: Comparison, diff: ImageFile },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageFile {
    pub path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VideoResult {
    Encoded { path: String, sha256: String, width: u32, height: u32,
        frame_rate: u32, duration_ms: u64, truncated: bool },
    Failed { error_id: String, diagnostic_paths: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MediaCapture {
    Captured { image: ImageFile },
    Unavailable { reason: String, error_id: Option<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorOccurrence {
    pub id: String,
    pub code: ErrorCode,
    pub source: ErrorSource,
    pub message: String,
    pub job_id: Option<String>,
    pub player_session_id: Option<String>,
    pub scenario_id: Option<String>,
    pub step_index: Option<u32>,
    pub log_sequence: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ErrorCode {
    #[serde(rename = "configuration.invalid")]
    ConfigurationInvalid,
    #[serde(rename = "build.failed")]
    BuildFailed,
    #[serde(rename = "launch.failed")]
    LaunchFailed,
    #[serde(rename = "simulator.boot-failed")]
    SimulatorBootFailed,
    #[serde(rename = "startup.mismatch")]
    StartupMismatch,
    #[serde(rename = "startup.probe-failed")]
    StartupProbeFailed,
    #[serde(rename = "assertion.failed")]
    AssertionFailed,
    #[serde(rename = "input.unreachable")]
    InputUnreachable,
    #[serde(rename = "condition.unsupported")]
    ConditionUnsupported,
    #[serde(rename = "image.mismatch")]
    ImageMismatch,
    #[serde(rename = "image.missing-baseline")]
    ImageMissingBaseline,
    #[serde(rename = "image.capture-failed")]
    ImageCaptureFailed,
    #[serde(rename = "image.comparison-failed")]
    ImageComparisonFailed,
    #[serde(rename = "baseline.download-failed")]
    BaselineDownloadFailed,
    #[serde(rename = "baseline.hash-mismatch")]
    BaselineHashMismatch,
    #[serde(rename = "baseline.store-conflict")]
    BaselineStoreConflict,
    #[serde(rename = "runtime.unity-error")]
    RuntimeUnityError,
    #[serde(rename = "runtime.unity-assert")]
    RuntimeUnityAssert,
    #[serde(rename = "runtime.unity-exception")]
    RuntimeUnityException,
    #[serde(rename = "runtime.fatal")]
    RuntimeFatal,
    #[serde(rename = "runtime.panic")]
    RuntimePanic,
    #[serde(rename = "runtime.process-exit")]
    RuntimeProcessExit,
    #[serde(rename = "runtime.reset-failed")]
    RuntimeResetFailed,
    #[serde(rename = "runtime.destroy-failed")]
    RuntimeDestroyFailed,
    #[serde(rename = "deadline.expired")]
    DeadlineExpired,
    #[serde(rename = "transport.request-failed")]
    TransportRequestFailed,
    #[serde(rename = "transport.log-buffer-overflow")]
    TransportLogBufferOverflow,
    #[serde(rename = "transport.log-record-oversize")]
    TransportLogRecordOversize,
    #[serde(rename = "transport.log-gap")]
    TransportLogGap,
    #[serde(rename = "transport.log-conflict")]
    TransportLogConflict,
    #[serde(rename = "transport.artifact-conflict")]
    TransportArtifactConflict,
    #[serde(rename = "media.insufficient-space")]
    MediaInsufficientSpace,
    #[serde(rename = "media.recording-failed")]
    MediaRecordingFailed,
    #[serde(rename = "media.ffmpeg-failed")]
    MediaFfmpegFailed,
    #[serde(rename = "durability.failed")]
    DurabilityFailed,
    #[serde(rename = "durability.result-commit-failed")]
    DurabilityResultCommitFailed,
    #[serde(rename = "baseline.lock-stale")]
    BaselineLockStale,
    #[serde(rename = "baseline.manifest-write-failed")]
    BaselineManifestWriteFailed,
    #[serde(rename = "baseline.publish-failed")]
    BaselinePublishFailed,
    #[serde(rename = "baseline.lease-lost")]
    BaselineLeaseLost,
    #[serde(rename = "baseline.cleanup-failed")]
    BaselineCleanupFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorSource {
    Ditto,
    DittoPlayer,
    Unity,
    Rust,
    #[serde(rename = "odiff")]
    ODiff,
    #[serde(rename = "ffmpeg")]
    FFmpeg,
    Filesystem,
    #[serde(rename = "r2")]
    R2,
}
```

`error_id` values are run-local occurrences allocated monotonically as `E0001`,
`E0002`, and so on. `ErrorCode` is the stable automation identity. Every
reference resolves to one top-level occurrence; duplicate representations of
the same underlying failure share that occurrence.

Run status is `passed`, `failed`, `infrastructure-error`, or `interrupted`.
Interrupt takes precedence, then infrastructure failure, then functional or
image failure. Exit codes are `0`, `1`, `2`, and `130`. The first terminal
player event remains primary; later durability or recovery errors are attached
without replacing it.

### Representative result example

This deliberately incomplete, non-normative excerpt illustrates an image
mismatch. It is not a fixture and cannot be deserialized as a full `RunResult`.
The Rust types above define the complete envelope:

```json
{
  "run_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
  "source_run_id": null,
  "lock_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "command": "run",
  "cycle": 1,
  "suite": "tictactoe",
  "profile": "macos-local",
  "started_at": "2026-08-27T19:10:00Z",
  "duration_ms": 1842,
  "status": "failed",
  "exit_code": 1,
  "build": {
    "source_fingerprint": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
    "fingerprint": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
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
      "name": "launch",
      "status": "passed",
      "duration_ms": 1480,
      "log_path": "logs/events.jsonl",
      "error_ids": []
    },
    {
      "name": "startup",
      "status": "passed",
      "duration_ms": 19,
      "log_path": "logs/events.jsonl",
      "error_ids": []
    },
    {
      "name": "scenarios",
      "status": "failed",
      "duration_ms": 61,
      "log_path": "logs/events.jsonl",
      "error_ids": ["E0001"]
    },
    {
      "name": "cleanup",
      "status": "passed",
      "duration_ms": 22,
      "log_path": "logs/events.jsonl",
      "error_ids": []
    }
  ],
  "player_sessions": [
    {
      "player_session_id": "0197b35f-6d12-71ac-b370-0bb2cbced1b2",
      "accepted": true,
      "startup_report": {
        "platform": "macos",
        "capture_adapter": "unity-async-readback-png",
        "build_fingerprint": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "source_fingerprint": "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210",
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
      },
      "diagnostic_paths": [
        "logs/player-0197b35f-6d12-71ac-b370-0bb2cbced1b2.log"
      ]
    }
  ],
  "jobs": [
    {
      "job_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
      "player_session_id": "0197b35f-6d12-71ac-b370-0bb2cbced1b2",
      "status": "failed",
      "first_scenario_index": 0,
      "last_scenario_index": 0
    }
  ],
  "scenarios": [
    {
      "id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
      "name": "human wins top row",
      "status": "failed",
      "status_reason": null,
      "motion": "instant",
      "duration_ms": 39,
      "timings": {
        "reset_ms": 5,
        "baseline_download_ms": 0,
        "comparison_ms": 14,
        "media_ms": null,
        "durability_ms": 3
      },
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
          "error_ids": ["E0001"],
          "assertion": null,
          "screenshot": {
            "status": "captured",
            "checkpoint": "opening-move",
            "actual": {
              "path": "actual/human-wins-top-row/opening-move.png",
              "sha256": "<hex>", "width": 1280, "height": 720
            },
            "baseline": {
              "status": "loaded",
              "image": {
                "path": "baseline/human-wins-top-row/opening-move.png",
                "sha256": "<hex>", "width": 1280, "height": 720
              }
            },
            "comparison": {
              "status": "mismatch",
              "changed_pixels": 117,
              "total_pixels": 921600,
              "settings": {
                "threshold": "0.1", "anti_alias": true,
                "max_changed_percent": "0.01"
              },
              "diff": {
                "path": "diff/human-wins-top-row/opening-move.png",
                "sha256": "<hex>", "width": 1280, "height": 720
              }
            },
            "matched_before_update": null,
            "updated": null
          },
          "video": null
        },
        {
          "index": 3,
          "name": null,
          "kind": "assert",
          "status": "passed",
          "duration_ms": 3,
          "error_ids": [],
          "assertion": {
            "object": "new_game",
            "state": "enabled",
            "expected": true,
            "observed": true,
            "passed": true
          },
          "screenshot": null,
          "video": null
        }
      ],
      "logs": {
        "job_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
        "player_session_id": "0197b35f-6d12-71ac-b370-0bb2cbced1b2",
        "first_sequence": 82,
        "last_sequence": 114,
        "complete": true,
        "path": "logs/events.jsonl"
      },
      "recovery": "reset"
    }
  ],
  "warnings": [],
  "errors": [
    {
      "id": "E0001",
      "code": "image.mismatch",
      "source": "odiff",
      "message": "opening-move differs from its baseline",
      "job_id": "0197b35f-6c59-7b98-b1f0-a39f5ee54db8",
      "player_session_id": null,
      "scenario_id": "0197b35f-6e24-75d8-9482-aa6c22a15133",
      "step_index": 2,
      "log_sequence": null
    }
  ],
  "artifacts": [
    "actual/human-wins-top-row/opening-move.png",
    "baseline/human-wins-top-row/opening-move.png",
    "diff/human-wins-top-row/opening-move.png"
  ]
}
```

### Result invariants

The important conditional rules are:

- `build` is null only when discovery fails before a fingerprint exists.
  `log_path` is null when no build ran.
- `source_run_id` is null for executed runs and names the immutable source run
  for comparison-only results. `lock_sha256` is the lock digest used for that
  result. It is null when the command fails before loading the lock and for
  every `capture`, which deliberately does not load the lock. It is also null
  for the explicit absent-lock state used by an initial update or acceptance.
  `source_command` is null for executed runs and is the source run's `run` or
  `capture` command for comparison-only results. A comparison-only result
  preserves that source command, source fingerprint, execution
  diagnostics, and non-image failures; only comparison-derived fields and final
  status are recomputed.
- `suite` and `profile` are null only when discovery or configuration fails
  before that identity resolves. Otherwise both contain the selected values.
  Ordinary and standalone runs use cycle `1`. Watch increments cycle for every
  execution or comparison-only cycle. Review acceptance outside watch uses
  cycle `1` in its derived run.
- `player_sessions` and `jobs` are ordered arrays. Every posted `started`
  request creates one session entry containing its complete startup report,
  acceptance flag, and retained secondary diagnostic paths. A rejected pending
  session therefore remains addressable by its uploaded records. Every job
  names exactly one run and player session. Recovery jobs append entries, so
  log sequences are interpreted only with both their job and player session.
  Both arrays are empty only if no player posts `started`.
- A `LogSpan` covers inclusive sequences from one accepted player session. Its
  job and player session identify the stream; sequence establishes application
  order. `complete` is true only when both scenario context markers are durable.
  A crash span starts at `scenario-started`, ends at the last durable sequence,
  and has `complete = false`. The path locates the host-created run artifact and
  is not a player-side log file.
- `ScenarioResult.expired_deadline` is null unless the scenario or run deadline
  ends startup or execution. A step-level expiry is also recorded on its step.
  This field identifies startup and between-step expiry without attributing it
  to a nonexistent step.
- `ScenarioTimings.startup_ms` is engine creation and connection time.
  `reset_ms` is the complete destroy/reset boundary duration from
  `ScenarioComplete.boundary`, including a failed boundary. Their sum need not
  equal `duration_ms`, because execution also contains steps and settling.
- `phases` contains reached work in execution order. Hydration is part of each
  reached scenario's post-reset timing, not a run-start phase. A
  comparison-only result uses `comparison` without player phases.
- Every step has nullable `assertion`, `screenshot`, and `video` members.
  For a reached assertion or screenshot, exactly the matching member is
  non-null. A completed video's `VideoResult` belongs only to its start step;
  its stop step has a null `video` and ordinary passed status.
  `NativeVideoInput` therefore carries the start-step index. A `not-run` step
  has all three null.
  Every configured step of a
  skipped or not-run scenario is retained as `not-run` with the scenario's
  reason.
- `status_reason` is required for `skipped` and `not-run` scenarios and steps,
  and null for every reached status. Capability reasons use the documented
  `unsupported-input:*` or `unsupported-step:*` form; bail and terminal
  infrastructure use `bail` and `run-infrastructure-error`.
- A skipped or not-run scenario has zero execution duration, all-null phase
  timings, no log span, no failure frame, and recovery `none`.
- `ScreenshotResult` is a tagged `captured` or `unavailable` variant. A captured
  screenshot always contains its actual image and closed tagged baseline and
  comparison outcomes. Update fields are non-null only for `run --update`.
- A successful video has MP4 path and hash, dimensions, fixed frame rate 30,
  duration, and `truncated`. A failed video contains its primary occurrence ID
  and any retained diagnostic paths.
- An assertion contains `object`, `state`, `expected`, `observed`, and `passed`.
  Expected is always true and observed is the Boolean result of the condition.
- `failure_frame` belongs to its scenario. It is absent when no automatic frame
  was attempted and otherwise is a closed `captured` or `unavailable` variant.
- Every `error_ids` value resolves to one top-level occurrence. Its `code` is
  stable for automation; its `E####` ID is stable only within that run. A
  non-null `log_sequence` requires non-null matching `job_id` and
  `player_session_id`; all three are null for a host-only occurrence.
- `baseline_writes` records every update or acceptance proposal. A proposal
  advances from `proposed` to `published`; an object uploaded before a failed
  atomic manifest rewrite is `uploaded-unreferenced`. Read-only and capture
  runs use an empty array.
- `artifacts` is the exhaustive, lexically sorted list of retained regular
  files beneath the run directory except `result.json`,
  `partial-result.json`, and internal lease or index files. It includes images,
  logs, build and platform diagnostics, raw video retained after failure, and
  encoded video. Artifact paths are slash-separated and run-directory-relative.
  Every `ImageFile` has a canonical hash. An unavailable tagged variant has no
  image field and therefore no hash.

Every SHA-256, source fingerprint, and build fingerprint is exactly 64
lowercase hexadecimal digits with no prefix. UUID fields use canonical
lowercase hyphenated text. These rules apply equally to configuration, HTTP
headers, store state, locks, fixtures, and results.

Durations use local monotonic clocks and integer milliseconds. `started_at` is
informational UTC. JSON object order is not semantic. `result.json` uses lexical
keys, two-space indentation, and a final newline; `--json` emits the same object
on one line. Host-originated paths, messages, and command arrays are redacted by
replacing any review token, R2 secret, or configured secret environment value
with the literal `<redacted>` before persistence. Player log messages, field
values, exceptions, and stacks follow the pre-serialization rule in
[Events and artifacts](#events-and-artifacts), so accepted body hashes and
persisted bytes remain identical.

Every ordered JSONL record streamed during the run is written immediately, so
`--json` is not the only machine interface. A terminal JSON result is still
written after a failed scenario or recoverable target relaunch. If Ditto itself
is killed before that point, the partial directory is marked interrupted on the
next startup.

All terminal runs are retained for at most seven days. An independent
configurable 1 GB LRU limit may evict younger inactive run artifacts first.
Active leases are protected. A run larger than the limit may finish and remain
until later cleanup; review retains lightweight indexed history and marks its
artifacts evicted. This cache is separate from the 20 GB build cache and the
non-evicting baseline hydration cache.

## Logging, errors, and recovery

The player observes Battlement's unified managed log store and serializes those
entries for Ditto. It does not create a second runtime logger, a native file
sink, or scrape Unity's formatted player log. Ordered context markers identify
job, engine, scenario, step, artifact, and error boundaries; ordinary records
do not duplicate all context fields. Each reached scenario result names one
player session and its exact sequence range.

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

Failure classification determines whether the player may keep executing. These
rules are authoritative when one failure causes secondary errors:

- **Screenshot mismatch or missing baseline in ordinary `run`:** Continue the
  remaining steps. Fail the scenario and count it once for bail at completion.
- **Assertion failure or step timeout:** Stop the remaining steps. Fail the
  scenario, capture diagnostics, and reset.
- **Unity Error, Assert, or Exception; Rust panic; or structured Battlement
  fatal:** Stop the remaining steps. Fail the scenario, capture a failure
  frame, and reset. Duplicate records for one underlying failure share an
  occurrence ID.
- **Native process or page crash during a scenario:** Stop execution. Synthesize
  one failed scenario and relaunch at the next scenario.
- **Destroy or reset failure:** Mark the run as infrastructure failure, discard
  the player, and relaunch for the next scenario when the remaining run budget
  permits. This is the only recoverable run-infrastructure class.
- **Invalid HTTP data, log-batch conflict, delivery overflow, upload failure,
  or result commit
  failure:** Stop execution. Fail the run as terminal infrastructure and do not
  run later scenarios.
- **ODiff, baseline, capture-adapter, or media-processing failure:** Stop
  execution. Fail the run as terminal infrastructure and do not run later
  scenarios.
- **User interrupt:** Freeze work and request cancellation. Allow two seconds
  to flush logs and atomically finalize the interrupted result, without normal
  scenario reset. Then terminate the owned player if needed and exit `130`.

Warnings, informational records, and debug records never gate merely because of
their severity. The specific `battlement.logging.records_dropped` and
`battlement.logging.failed` events are transport-integrity failures under the
rules above. Ditto does not support declarative expected-log assertions;
scenario assertions should describe visible game behavior.

Battlement may report an otherwise-caught native or managed failure through
`Debug.LogException` so Unity Diagnostics can create an exception occurrence.
That `BattlementCaughtFailureException` envelope remains in the ordered log
stream, but it is diagnostic evidence for the already recorded
`BattlementError`, not a second Ditto failure. Assert and exception gating uses
the existing `BattlementUnityErrors` subscription, which suppresses this bridge
envelope before Ditto allocates an error reference. The original structured
error is correlated by its local `error_id`. An ordinary Unity exception is
already visible to Unity and does not receive this extra envelope.

The optional `BattlementDiagnosticsModule`, Unity's project-level collection
setting, and its exception-capture and recent-log settings never change Ditto's
local collection or failure classification. They only determine whether Unity
later uploads eligible Unity logs and caught-failure envelopes.

Structured correlation, not free-form text, creates a player error reference:

- `unity.log` at error severity maps to `runtime.unity-error`;
  `unity.assert` maps to `runtime.unity-assert`; and `unity.exception` maps to
  `runtime.unity-exception`, except for the caught-failure envelope above.
- A native ABI `PANIC` outcome maps to `runtime.panic`. Another fatal
  Battlement boundary outcome maps to `runtime.fatal`. A Rust `tracing` error
  record alone is diagnostic and does not claim that boundary outcome.
- A structured `BattlementError` record carries `error_id`, `type`, and
  `source` string fields, plus optional `exception_type`. The executor records
  that ID in `battlement_error_id`; ordinary Unity errors leave it null.
- The executor allocates the next scenario-local `P####`, adds one
  `error-observed` context with the stable code, source, source-record sequence,
  and optional Battlement error ID, then attaches that reference to the step or
  boundary result. The context record is authoritative for classification.
- The host maps a previously unseen `P####` to one run-local `E####`. Replayed
  completion reuses the mapping. Multiple representations of the same source
  record reuse its `P####`; the caught-failure envelope never allocates one.

Shared positive and negative fixtures include each mapping, an error without a
source record, a replayed completion, and an original Battlement error followed
by its caught-failure envelope.

The functional log gate starts with `scenario-started` and ends when execution
freezes. An error-class record during reset or later host processing is instead
the infrastructure failure of that phase. Records emitted while no run is
active remain diagnostics for the warm player session and do not retroactively
change a result.

`--bail` counts failed scenarios, not failed steps or screenshots. Multiple
failures in one scenario increment the count once. A process crash during a
scenario also counts once after Ditto synthesizes its durable failed result.
Skipped scenarios do not count. Infrastructure failure terminates the run and
does not participate in the bail counter.

On a responsive runtime failure, the player executor freezes its controlled
clock, commits no further Battlement work, and captures the last responsive
frame. It then follows the normative lifecycle: destroy and reset first, flush
logs and artifacts second, and let the host hydrate, compare, process media,
commit the partial result, and decide bail. Ditto then retains:

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

After every reached scenario, the player executor performs a clean reset before
host comparison or bail. A responsive boundary failure is carried by
`ScenarioComplete.boundary`; a transport-ending failure is synthesized from
the structured boundary record and process exit. The scenario keeps its
functional status, while the run becomes infrastructure-error. Ditto relaunches
the same immutable build with a new job beginning at the next configured
scenario when the remaining run budget permits. A crashed player follows the
same rule after Ditto synthesizes the active scenario from partial logs and
platform diagnostics. No reached scenario is retried automatically.

If relaunch fails, remaining scenarios become `not-run` with reason
`run-infrastructure-error`. Reaching the bail count suppresses only the start of
later scenarios. It never suppresses reset, log flush, failure capture,
comparison, or durable completion for the active scenario.

Secondary diagnostics are retained per accepted player session:

- macOS snapshots Unity's player log to
  `logs/player-<player-session-id>.log` at job finalization or after exit;
- a configured WebGL command writes combined launcher output to
  `logs/browser-<player-session-id>.log`, which Ditto snapshots at job
  finalization, while responsive console and unhandled-error callbacks also
  enter the managed stream; and
- iOS Simulator captures the launched application's scoped unified-log output
  through the job boundary in `logs/simulator-<player-session-id>.log`.

`PlayerSessionResult.diagnostic_paths` lists exactly these retained files. A
host-observed process or page exit becomes a structured occurrence whose player
session is explicit; it is not inserted retroactively into the managed stream.
Secondary timestamps support correlation, while the
player-session-qualified managed-store sequence remains authoritative for
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

During watch mode, the review server exposes Server-Sent Events with monotonic
review event IDs. `Last-Event-ID` replays retained events; if replay is no
longer available, the server sends one full immutable snapshot before live
updates. The existing tab appends correlated logs, step status, timings, and
newly completed screenshots without polling or reloading.

Acceptance is a state-changing loopback request protected by a random review
token. Before mutation, the server allocates an immutable derived attempt run
whose `source_run_id` names the reviewed run. It verifies that the actual hash
still matches, uploads accepted objects, then performs one atomic manifest
rewrite for the selected set. On success it completes comparison in that run
and switches the UI to it. On failure the attempt run ends as infrastructure
failure and retains proposal or `uploaded-unreferenced` states. The reviewed run
is never changed. Unaccepted checkpoints remain unchanged. If write credentials
are unavailable, comparison remains fully usable and acceptance is disabled
with a clear credential message.

One `ReviewAcceptance` contains a nonempty, duplicate-free `selections` vector.
The server validates every selection and the starting lock digest before any
manifest mutation, then applies all selections in one lock transaction. If any
validation, upload, lease, or rewrite fails, the lock is byte-for-byte
unchanged, the attempt run is finalized without comparison, and the response is
`HttpError`. Sequential single-checkpoint rewrites are not the protocol for a
multi-selection action.

`request_id` is a caller-generated UUID and is the acceptance idempotency key.
The server durably stores the request hash and terminal response under
`(run_id, request_id)` before replying. An identical retry returns the stored
`ReviewAcceptanceResult` or `HttpError`, including after a successful lock
rewrite. Reusing the key with different bytes returns `409`.
An acceptance `HttpError.related_run_id` names its immutable attempt run; other
HTTP errors use null unless a diagnostic run was already allocated.

Acceptance also verifies the current lock digest, suite, checkpoint, profile
dimensions, and actual hash. It intentionally does not require the current
worktree source fingerprint to match the reviewed run; accepting an older-source
image is allowed and the old source remains diagnostic metadata. A fragment-only
checkpoint cannot be accepted unless the current full suite contains the same
profile, scenario, checkpoint, and dimensions.

The page opens only for `ditto review`, `--review`, or the first watch cycle.
Watch mode refreshes one live tab and does not create a new tab per failure.

## Experimental video

A `video` step records an MP4 for debugging and demonstration. Video is
experimental and is not baseline-gated: its pixels are never compared and do
not change screenshot comparison status. Failure to start, capture, upload, or
encode a requested video is media-processing infrastructure failure. Runtime
errors during the recorded actions retain their normal scenario behavior.

FFmpeg is required only for a selected native scenario containing video.
`doctor` reports it as optional otherwise. Before execution, Ditto verifies that
the target filesystem has capacity for the declared maximum recording. Failure
is infrastructure error with the required and available byte counts.

macOS and iOS Simulator players record a fixed 30 frames per second to a
temporary file in their filesystem. The job result reports the path and media
metadata; no video bytes travel over HTTP. The macOS host reads the path
directly. For iOS Simulator it resolves the Ditto-owned app container and copies
the file. Host FFmpeg produces the retained MP4 after scenario reset.

The temporary input is tightly packed RGBA8 raw video: exactly
`width * height * 4` bytes per frame, rows in top-to-bottom order, with no file
header or per-frame prefix. Metadata supplies width, height, frame count, and
the fixed 30 fps rate. The player writes a frame only after complete GPU
readback, syncs and closes the file before reporting it, and rejects a size that
does not equal `frame_count * width * height * 4`. FFmpeg receives equivalent
`rawvideo`, `rgba`, size, and rate arguments.

The retained MP4 uses H.264 through `libx264`, `yuv420p`, constant-rate-factor
18, no audio track, and `+faststart`. Frames keep their input order and receive
timestamps at exact 1/30-second intervals. The result duration is
`frame_count * 1000 / 30`, rounded down to integer milliseconds.

Disk preflight requires the complete declared raw size plus 64 MiB for the MP4,
temporary copy, and filesystem overhead. The calculation uses checked 64-bit
arithmetic and `ceil(max_duration_ms * 30 / 1000)` frames. An overflow is a
configuration error; insufficient space is media infrastructure failure.

WebGL scenarios containing video are skipped before engine creation with
`unsupported-step:video`. A clip may use controlled or real-time motion and
includes no host chrome. Reaching `max_duration` automatically finalizes a
truncated clip. Its later authored stop is a passing no-op that consumes the
paired state. Runtime failure also finalizes the active clip as truncated when
possible. An FFmpeg or media I/O failure stops later scenarios, preserves raw
inputs and diagnostics, and never reports an incomplete MP4 as successful.

After a successful MP4 and terminal result commit, Ditto deletes the raw input
and any Simulator-side copy. A recording or FFmpeg failure moves every complete
raw input into the run's diagnostics before session cleanup. Partial final
frames are discarded and never counted in `frame_count`.

## Performance requirements

The primary benchmark uses a previously compiled and locally cached 1280 by 720
player on a CI-class Apple Silicon macOS host. It runs 20 representative
scenarios containing 40 total screenshots, fresh Rust engine creation, normal
settling, PNG capture, local baseline reads, and ODiff comparison.

The requirements are:

- no more than 20 seconds including a cold player launch; and
- no more than 5 seconds when the player, HTTP session, and ODiff server are
  warm.

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
HTTP startup, input, settling, PNG conformance, a passing comparison, runtime
failure reporting, and log-batch upload. CI does not build every sample for
every platform.

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

Unit tests focus on deterministic manifest serialization, strict TOML
diagnostics, filtering, update transactions, content hashing, normative model
fixtures, deadline state, and retention decisions. Rust and C# deserialize the
same positive fixtures and reject the same unknown fields, invalid variants,
conflicting retries, and malformed ranges.

Automated Rust fake-player and fake-store tests cover lifecycle success,
functional failure, reset failure and relaunch, bail, interrupt, abandoned-run
recovery, terminal infrastructure failure, reached-only hydration, update
eligibility, stale acceptance, fragment rejection, explicit cleanup, store
lease races, and publication conflicts.

Log transport tests cover observer registration, viewer-store eviction,
delivery overflow, batch retry, conflicting duplicates, lost acknowledgements,
startup rejection and drain, oversize records, exact NDJSON framing and
redaction, partial crash spans, warm idle gaps, comparison-only materialization,
and player-session sequence namespaces. Shared Rust and C# fixtures contain
mixed application and context batches plus malformed unions and sequence
conflicts. A test-only Unity fixture covers failure-frame capture, fresh engine
isolation, input targeting, controlled settling, and native video paths. Media
tests cover disk preflight, truncation, FFmpeg failure, retained raw
diagnostics, and WebGL pre-execution skipping. No production fault-injection
command exists.

Black-box adapter tests build a small player with known colored regions and a
known clickable Battlement object. They validate the same released player
surface, input path, and diagnostics that a game uses. Performance tests retain
phase timings and compare them against the two explicit budgets.

Changes to `ditto.toml`, `ditto.lock`, HTTP types, or result types update shared
fixtures and every caller together. Compatibility or versioning is not required.

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
[unity-web-request]:
  https://docs.unity3d.com/ScriptReference/Networking.UnityWebRequest.html
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
- **Action:** run one representative passing scenario, one assertion failure,
  one image mismatch, and one Ctrl-C interruption from the terminal.
- **Expected:** terminal status, retained paths, review selection, and exit
  codes match the visible behavior. The interrupted command returns within two
  seconds when the player is responsive.
- **Action:** read the first stderr line while a run is active and follow
  `logs/events.jsonl` through its terminal sequence.
- **Expected:** framing and paths are stable, records are complete and ordered,
  and the final scenario range selects the same records.

### macOS suites, isolation, and input

- **Setup:** use the Basic, Tic-Tac-Toe, Chess, and UI macOS suites.
- **Action:** run click, hover, drag, and key steps through UUID aliases
  and normalized coordinates. Obscure one UUID target with another object.
- **Expected:** each suite reuses one player, every scenario has a fresh Rust
  engine, and the obscured click fails with the blocking UUID. No object action
  is called directly.
- **Expected:** the player need not be frontmost. The host pointer and keyboard
  do not move, and macOS requests neither Accessibility nor Screen Recording
  access. Captures have the exact configured Unity surface dimensions.
- **Action:** run consecutive scenarios that reach distinct states through
  player-facing input.
- **Expected:** no engine, Unity object, input state, or log correlation leaks
  between scenarios.

### Settling, motion, and deadlines

- **Action:** wait for drained Battlement work and two quiet frames, then test
  exact-frame and object waits with delayed game-owned work.
- **Expected:** implicit settling does not require sleeps. Step, scenario, and
  run deadlines fail promptly and identify `expired_deadline`, while build,
  hydration, launch, and Simulator boot use their separate phase deadlines.
- **Action:** run the same tween, particle, and audio behavior in instant,
  controlled, and real-time modes.
- **Expected:** Battlement-owned behavior follows the selected clock. Custom
  scripts and shaders are neither disabled nor reported as controlled.

### Runtime failures and recovery

- **Action:** trigger one Unity error in a responsive player, then terminate an
  owned player during a later scenario.
- **Expected:** the first scenario stops with a correlated per-scenario failure
  frame. The crash reports its frame unavailable, preserves the durably
  uploaded log span plus the secondary player log, relaunches, and begins only
  the next scenario.
- **Action:** run several failures without bail, then with `--bail` and
  `--bail=2`.
- **Expected:** reset or relaunch lets later scenarios run without bail. Bail
  stops before the next scenario at the selected count only after diagnostics
  and artifacts are acknowledged.

### WebGL adapter

- **Action:** run the WebGL conformance and failure smokes through the minimal
  local launcher and a configured headless command.
- **Expected:** `canvas.toBlob` produces exact PNG dimensions. UUID and
  normalized input stay inside the player, raw HTTP uploads retain exact PNG
  bytes, and responsive browser errors reach ordered logs. A supervised
  headless exit becomes a host occurrence with browser output; an abrupt
  operating-system-opened exit becomes a deadline failure. Images exclude
  browser chrome. No browser automation package is required.

### iOS Simulator adapter

- **Action:** run portrait and landscape profiles on two installed device
  types, including click, drag, and a scenario containing hover.
- **Expected:** dimensions and safe areas come from Simulator. Click and drag
  become touch sequences. The hover scenario skips before engine creation with
  its documented reason.
- **Action:** launch through `simctl` with an explicit IPv4 loopback session URL
  and upload logs, screenshots, and completion through the common HTTP API.
- **Expected:** Simulator requires no interactive local-network permission,
  exact artifacts reach the host, and an unavailable endpoint fails within the
  run deadline with retained Simulator diagnostics.

### R2 hydration, update, and retention

- **Action:** delete the baseline cache and run without credentials, run
  `fetch --all`, then disconnect the network.
- **Expected:** public objects hydrate and verify by hash. The prewarmed cache
  works offline; a genuine miss is an infrastructure error.
- **Action:** update two passing scenarios and one scenario with a later
  assertion failure, then repeat with a scenario filter within the selected
  profile.
- **Expected:** passing scenarios publish in one manifest transaction. The
  failed scenario's capture remains local and is not published. Full update
  prunes only the selected profile; filtering preserves unselected entries.
- **Action:** accept a feature-branch replacement, merge it, publish the default
  branch, inspect cleanup's dry-run, and apply eligible cleanup after seven
  days.
- **Expected:** the feature branch creates no tombstone. Canonical publication
  creates retention metadata. No live canonical hash is deleted, and cleanup
  failure never changes a test result.

### Filesystem storage, comparison, and review

- **Action:** repeat hydration, update, and offline runs with a filesystem store
  and a large tracked store.
- **Expected:** behavior matches R2 where applicable; the large-store warning
  does not block the run.
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
  update mode. A lock edit or review acceptance creates a new comparison-only
  run with `source_run_id`; the source run remains byte-for-byte unchanged.

### Diagnostics and experimental video

- **Action:** build with runner diagnostics disabled.
- **Expected:** ordinary Unity logging still works, the viewer and scenario
  executor are unavailable, and Ditto rejects the build as non-automatable
  before launch.
- **Action:** with runner diagnostics enabled, emit interleaved Rust `tracing`,
  managed Battlement, and ordinary Unity records while enough later records
  evict the earliest entries from the in-game viewer store.
- **Expected:** `logs/events.jsonl` retains one contiguous sequence for the
  accepted player session, the scenario span selects the same ordered records,
  and viewer eviction creates no transport gap.
- **Action:** trigger one caught Rust panic and one caught managed exception,
  first without `BattlementDiagnosticsModule` and then with it selected.
- **Expected:** each original structured error produces one Ditto occurrence.
  Its `BattlementCaughtFailureException` remains visible in the ordered logs but
  does not produce another failure. Module selection does not change local
  result status or retained records.
- **Action:** use the test-only logging fixture to overflow the native tracing
  queue, then separately stall log upload until the Ditto delivery queue fills.
- **Expected:** native loss produces `battlement.logging.records_dropped`; the
  stalled delivery produces `transport.log-buffer-overflow`. Both terminate the
  run as infrastructure failure without silently passing or recursively
  logging from the observer callback.
- **Action:** reject startup after emitting startup logs, then repeat with a
  pre-start delivery overflow and with one record larger than 1 MiB.
- **Expected:** the rejected route remains valid until available records and a
  terminal completion are acknowledged. Overflow and oversize use the small
  `failed` route, preserve the contiguous accepted prefix, and create no engine.
- **Action:** run two warm jobs with idle Unity logs between them, then crash
  the second job before its scenario end marker and relaunch a third player
  whose store sequence restarts.
- **Expected:** idle records belong only to secondary diagnostics. The crashed
  scenario has `complete = false`; the relaunched player's occurrences include
  its new player session, so equal numeric sequences are unambiguous.
- **Action:** place a route token in a message, field value, exception, stack
  trace, failure-frame reason, and native-video path, then create a
  comparison-only run from the result.
- **Expected:** uploaded and persisted NDJSON bytes are identical and contain
  `<redacted>` in the ordinary record and nested context body. The derived run
  independently materializes the event stream and secondary diagnostics, and
  every relative result path resolves inside it.
- **Action:** record paired native video steps with actions and a screenshot,
  then exceed the declared maximum. Run the same scenario on WebGL.
- **Expected:** the native MP4 contains only the Unity surface at 30 fps. The
  maximum produces `truncated = true`, and its later stop passes as a no-op.
  Video pixels never enter screenshot comparison. WebGL skips before engine
  creation with `unsupported-step:video`.

### Cache and performance budgets

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
