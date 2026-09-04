# Battlement Ditto

Ditto is Battlement's screenshot test runner and local visual review tool. It
runs deterministic scenarios against packaged Unity players, compares exact
framebuffer PNGs, retains logs and timings, and gives CI and agents one stable
machine result.

Ditto runs on Apple silicon macOS. Its targets are native macOS, Unity WebGL,
and iOS Simulator. The [technical design](ditto-technical-design.md) is the
normative reference for wire formats, storage transactions, and failure rules.

Semantic assertions can include `checked = true` or `checked = false` for a
checkbox or switch. Omitting `checked` leaves that state unconstrained; mixed
and absent checked states do not satisfy either Boolean assertion.

## Install and inspect

Install either entry point from this checkout:

```sh
cargo install --path crates/battlement-ditto --locked
cargo install --path crates/battlement-cli --locked
```

`ditto` and `cargo battlement ditto` share the same parser and implementation.
From a sample directory, Ditto finds `ditto.toml` by searching upward. From
elsewhere, pass it explicitly:

```sh
ditto --config samples/chess/ditto.toml doctor --profile macos
cargo battlement ditto --config samples/chess/ditto.toml list
```

Run `doctor` before a new target. It reports required host tools, build and
baseline caches, and platform availability. Run `list` to see resolved
profiles, scenario names, and screenshot checkpoints without building a player.

## Author a suite

A suite names its player inputs, defaults, aliases, profiles, baseline store,
and scenarios. Paths are resolved from `ditto.toml`.

```toml
name = "game"
default_profile = "macos"

[defaults]
step_timeout = "5s"
scenario_timeout = "30s"
motion = "instant"

[defaults.comparison]
threshold = 0.1
anti_alias = true
max_changed_percent = 0.01

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[aliases]
board = "43000000-0000-4000-8200-000000000002"

[baseline]
kind = "filesystem"
namespace = "battlement/game"
root = "../../baselines"

[profiles.macos]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "opening"

[[scenarios.steps]]
wait = { object = "board", state = "visible" }

[[scenarios.steps]]
screenshot = { name = "initial" }
```

Profiles may use `target = "webgl"` with a display and optional
`headless_command`, or `target = "ios-simulator"` with a device and orientation.

Scenario and checkpoint names are stable public identities. Aliases map names
to production object UUIDs. Coverage enforces one owner per registered state.

## Motion, inputs, and assertions

Choose motion per scenario:

- `instant` completes Battlement animation and tween work immediately.
- `controlled` advances supported motion deterministically.
- `real-time` preserves player timing for behavior that depends on it.

Instant and controlled scenarios suppress particle playback and settle at the
scenario boundary. They also bypass sample start animations and deterministic
AI delays. Real-time scenarios keep ordinary effects and timing.

Steps execute serially. Supported inputs are `click`, `hover`, `drag`, and
`key`. A target is an alias/object UUID or normalized `[x, y]` coordinates.
Keyboard actions are `tap`, `down`, and `up`.

Use `wait = { frames = 2 }` only when frames themselves are the contract.
Prefer an object wait such as `{ object = "board", state = "visible" }`.
Assertions support `exists`, `absent`, `visible`, `hidden`, `enabled`, and
`disabled`. Screenshot steps compare the exact Unity surface.

## Run, capture, and fragments

`ditto run` executes the default profile and all scenarios.

Selectors are globs. Includes are combined, then excludes are removed:

```sh
ditto run 'opening*' --scenario promotion --exclude '*slow*' --bail=1
```

Use `--no-build` to require an exact cached player. Use `--allow-empty` only
when an empty selection is intentional. `--json` makes stdout one terminal
result object; progress and `DITTO_*` handoff fields stay on stderr.
`--output result.json` atomically copies the same terminal result.

`capture` executes without reading, comparing, or changing baselines. It is
useful for exploration and agent-authored fragments:

```sh
ditto capture --fragment /tmp/check.toml --review
printf '%s\n' '[[scenarios]]' 'name = "probe"' \
  '[[scenarios.steps]]' 'screenshot = { name = "current" }' \
  | ditto capture --fragment=- --json
```

A scenario-only fragment inherits the discovered suite member by member. A
full suite is self-contained. Standard-input fragments cannot use watch mode;
file fragments can.

### Reproduce a CI scenario

Use the CI wrapper for a focused run against current sources and their exact
cached player:

```sh
python3 scripts/ditto_ci.py sample chess-ui 'gallery reset'
```

It supplies the CI cache and ODiff environment. Setting `DITTO_CACHE_ROOT`
alone changes the default tool directory too; a direct invocation with only
that override does not reproduce the CI environment. Screenshot comparisons
check ODiff availability before building or launching a player.

Each CI result records a `replay.json` beside `result.json` and inside the
retained run archive. Replay the original selection, or name one of its
scenarios:

```sh
python3 scripts/ditto_ci.py replay artifacts/ditto-ci/chess-ui/replay.json
python3 scripts/ditto_ci.py replay artifacts/ditto-ci/chess-ui/replay.json 'gallery reset'
```

Replay uses the retained immutable macOS player, runner executable, ODiff and
FFmpeg executables, profile, tool environment, suite configuration, and baseline
lock. It never compiles a replacement player or regenerates assets. Configuration
and dependency hashes are checked before execution. A missing player or changed
configuration is an explicit failure; use a checkout with the recorded
configuration. Runs without a replay record cannot establish the original
runner and dependency identities from `result.json` alone.

Replay results have a separate directory under `artifacts/ditto-ci/replays` and
retain the original run ID and outcome in `source-replay.json`. A later pass does
not change the original failure. Replaced CI artifacts move into
`artifacts/ditto-ci/history`; failed player runs remain in the run cache.

The `DITTO_REPLAY_BUILD_FINGERPRINT` environment variable is reserved for the
replay wrapper. Ordinary CI clears it and always validates current source
fingerprints. Build and tool retention are local; an evicted dependency cannot
be reconstructed as an exact replay by silently substituting another build.

### Verify a public demo

Treat a tunnel failure as review-environment evidence. Preserve the URL, failed
resource URL and status, browser console, and tunnel log with the review record.
Do not infer native test outcomes from public-tunnel behavior.

Open the exact review URL in a fresh browser context. Check failed network
requests and console errors through application startup, including requests for
asset bundles after the Unity loader finishes. A canvas or loading shell only
proves the page exists. Verify the intended application screen, operate its
first control, and assert the resulting application state. For chess-ui, open
the gallery shell, activate **Change demonstration**, and verify **Changes: 1**;
then select **1. Gallery shell** and verify **Changes: 0**. Do not hand off a demo
with asset-preparation or dependent-batch failures. If the public check fails,
record it separately and perform the same checks against the local server.
External tunnel availability is not a deterministic CI requirement.

Use `ditto gallery` to open the complete `ditto.toml` source with the canonical
baseline inserted after every screenshot step. The default profile is used
unless `--profile` selects another one. Gallery reads are public and hydrate
missing local baseline objects automatically.

## Baselines and review

The coverage ledger requires every registered checkpoint in its canonical
profile. Additional profiles may retain baselines for those same checkpoints;
they must name a declared profile and cannot replace canonical coverage.


`ditto.lock` is the checked-in manifest mapping profile, scenario, and
checkpoint identities to immutable PNG objects. Hydrate selected objects with
`ditto fetch <glob>` or the entire lock with `ditto fetch --all`.

Before acceptance, use a focused `capture` or fragment to inspect the changed
state without changing baselines. For a port with a pinned visual reference,
compare against that reference and its acceptance tolerances first. A green
comparison against a newly accepted baseline proves stability, not fidelity
to the reference.

Run `ditto run --update` with selectors for the intentionally affected scenarios
to accept their reached checkpoints after a successful comparison run. Inspect
the `ditto.lock` diff for unrelated checkpoint changes, then run the same
selection without `--update` to verify ordinary comparison. Complete any
broader smoke/reset coverage required by the project. Do not accept unrelated
drift to make a suite green.

Open the newest retained execution with `ditto review`, or pass a run ID.
Retain that ID, the exact inputs, and paths to the result and screenshots;
reuse them while those inputs remain unchanged.

The review app works offline from retained data. It offers side-by-side, swipe,
alpha, and red-mask views; synchronized zoom and pan; integer pixels and
coordinates; logs; timings; and explicit missing-image states. Acceptance is a
credentialed, atomic request. Retrying the same request is safe, and changing
actual bytes or a fragment-only checkpoint invalidates acceptance.

For an R2 store, credentials are needed only for mutation. Read-only review and
public baseline hydration continue without them. Default-branch CI publishes
the accepted lock with `ditto storage publish`. Inspect remote cleanup with
`ditto clean storage`; add `--apply` only after reviewing its deletion plan.

## Watch, storage, and cleanup

`ditto run --watch` keeps one player and one review tab warm. Scenario or lock
changes produce comparison-only cycles when possible. A build-input change
creates a replacement before retiring the old player. Each NDJSON line is one
immutable cycle result, and `--output` always names the latest completed cycle.

Ditto stores content-addressed players, hydrated baselines, and immutable run
directories in its cache. Every run contains `result.json`, `orchestration.json`,
ordered `logs/events.jsonl`, player logs, diagnostics, and reached media.
Machine results reference these relative paths; terminal prose is not an API.

Remove inactive data with `ditto clean runs`, `ditto clean builds`, or
`ditto clean baselines`. Add `--global` only to run/build cleanup when every
inactive repository and suite is intended. Active leases are never removed.

## Diagnose failures

Start with the terminal `DITTO_RUN_DIR` and `DITTO_RESULT` paths. In
`result.json`, inspect `status`, `errors`, `phases`, the failing scenario and
step, `failure_frame`, log ranges, and image comparison counts. A scenario
failure is distinct from an infrastructure error such as build, launch,
capture, logging, storage, or cleanup failure.

Use `ditto doctor --profile <name>` for missing target support. Use the retained
build log for compiler or Unity failures, the player/platform log for startup
or application loss, and `diagnostics/odiff.log` for image-engine failures.
Ditto redacts configured secrets and never requires scraping console prose.

Exit codes are `0` for success, `1` for a completed failing run, `2` for usage
or setup errors, and `130` for interruption. Responsive interruption finalizes
partial evidence before exiting.

## Performance and CI

The release benchmark measures 20 scenarios and 40 screenshots. On the pinned
Apple silicon lane, source hashing must remain at or below 250 ms, a cold player
launch at or below 20 seconds, and a warm watch cycle at or below 5 seconds.
Build time is reported separately and is never counted as scenario latency.
The on-demand gate runs a curated 34 of the 68 authored scenarios across all
five samples, retaining 89 screenshot checkpoints within 120 seconds of both
wall-clock and added work. The fixed 20-scenario, 40-screenshot performance
benchmark remains fully represented in that selection. The remaining authored
scenarios stay available for targeted runs.

Tollgate runs repository CI, cold and warm preparation, all five macOS suites,
WebGL and iOS adapter smokes, the performance budget, and final baseline
publication. Failed-run archives retain the machine result and diagnostics;
successful sample archives retain their complete reviewable run.

Agent handoffs for player-visible work must contain:

```text
Ditto: passed
Run ID: <uuid>
Profile: <name>
Result: <absolute-path>/result.json
Review: ditto review <uuid>
```

A change that cannot affect a player records `Ditto: not applicable` followed
by a concrete reason. Agents use Ditto's inputs, screenshots, logs, timings,
and result object as the evidence; host-window captures and terminal-text
scraping are not substitutes.

## Sample coverage

A sample's `ditto-coverage.toml` selects a canonical macOS profile at device
scale 1. Its declared width and height define the capture size; every canonical
baseline must have those dimensions. The coverage ledger checks that each
registered visual state has an owned scenario checkpoint and accepted baseline.
