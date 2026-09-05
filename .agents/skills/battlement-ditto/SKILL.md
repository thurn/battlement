---
name: battlement-ditto
description: Validate player-visible behavior with native Ditto scenarios or temporary capture fragments, inspect retained results, and update intentional screenshot baselines.
---

# Native scenario checks

Run from the worktree root using
`cargo run --quiet -p battlement-ditto -- --config samples/<sample>/ditto.toml <command>`.
Below, commands follow that prefix. Read the suite for profile names, aliases,
and existing steps; the parser lives in `crates/battlement-ditto/src/cli.rs`.

1. Use `list` and `doctor --profile macos` before running a new target.
2. Use `run '<scenario>' --profile macos --json` for existing comparisons.
3. For exploration, write a scenario-only fragment outside tracked source:

```toml
[[scenarios]]
name = "probe"
[[scenarios.steps]]
screenshot = { name = "current" }
```

Run `capture --profile macos --fragment /tmp/<task>-probe.toml --json`.
Fragments inherit the selected suite. Capture does not change baselines.
Use existing suite examples and `crates/battlement-ditto/src/config/scenario.rs` to add
semantic actions and assertions. Prefer waiting for state over arbitrary delays.

Inspect the terminal result, screenshots, and retained logs; keep their paths
and run identity. `review` opens the retained run; stop its owned server after use.
A passing screenshot comparison is not evidence of fidelity to a supplied
reference: compare the intended result directly before accepting new images.

For intentional baseline changes, run the selected scenarios with `--update`,
inspect the `ditto.lock` diff for unrelated changes, then rerun without updating.
R2 mutation credentials are at `~/.config/battlement/r2.env`; never print them.

For CI reproduction, build the runner with `cargo build -p battlement-ditto`
and prepare the selected native player with its `build --profile macos` command
using `DITTO_CACHE_ROOT="${DITTO_CI_CACHE_ROOT:-$HOME/Library/Caches/Battlement/ditto-ci}"`.
Then use `python3 scripts/ditto_ci.py sample <sample> '<scenario>'`.
The wrapper requires the exact cached player and `target/debug/ditto`; set
`DITTO_CI_BINARY` explicitly if the current runner was built elsewhere.
For an exact retained run use `python3 scripts/ditto_ci.py replay <replay.json>`.
The replay record pins player/tool inputs; missing retained inputs are not
permission to silently rebuild and call it a replay. See `battlement-ci` for
failure diagnosis. Web interaction is reserved for web-specific validation.
