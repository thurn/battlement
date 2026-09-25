---
name: battlement-build
description: Build or run Battlement samples, author Unity projects, replace native plugins, or regenerate Addressables constants and Reactant assets.
---

# Build and generate

Run from the task worktree root. Use the checkout CLI so commands match source:
`cargo run --quiet -p rt -- <command>`. Consult its `--help` for
options; the parser is `crates/rt/src/command.rs`.

Before building, run `python3 scripts/prepare_validation.py check`. When it
identifies stale generated inputs, run `python3 scripts/prepare_validation.py
generate --sample <sample>`, inspect the returned manifest and patch, then
stage only the intended files. For staged C# additions, Unity transactions retain
generated metadata; inspect `generated-metadata.json`, then use
`scripts/unity_metadata.py --help` for explicit adoption before staging it.
Command help owns selection and output details.

After editing wire schemas, run `python3 scripts/generate_flatbuffers.py` to
refresh bindings and contract fingerprints together; `--check` verifies both
without writing. Do not copy digest literals by hand.

Read `rust-toolchain.toml`, the chosen project's `reactant.toml`, and its
`ProjectSettings/ProjectVersion.txt` for tool and player inputs. Do not copy
version pins into guidance. CLI tool resolution is in
`crates/battlement-tooling/src/developer_tools.rs`.

| Task | CLI command after `--` |
| --- | --- |
| Build native player | `build --project samples/<sample>` |
| Build and run native player | `run --project samples/<sample>` |
| Open Unity authoring | `author --project samples/<sample>` |
| Regenerate typed Addressables constants | `addressables generate --project samples/<sample>` |
| Check those constants | `addressables check --project samples/<sample>` |
| Generate declared Reactant paint | `assets generate --project samples/<sample>` |
| Check generated paint | `assets check --project samples/<sample>` |
| Generate and open paint gallery | `assets preview --project samples/<sample>` |

Use `--manifest-path` where supported for a nonstandard rules manifest.
Inspect `crates/rt/src/addressables.rs` and
`crates/rt/src/assets.rs` for generation behavior.
Edit declarations and Unity authoring inputs, not generated PNGs, metadata,
Addressables exports, or staged plugin binaries. Check the resulting diff.

For replacing a plugin in an existing stopped macOS player, use
`plugin inspect <app>`, `plugin install <app> <library>`, and
`plugin verify <library>`. `plugin restore <app>` restores its saved original.
Read `crates/rt/src/plugin.rs` and command help for Cargo-build and signing options.

Prefer native Ditto for validation. Web review uses `battlement-web`.
Track any authoring/player process you launch and stop that owned process when
finished. Never substitute an old installed CLI or cached player without
verifying it matches the source being tested.
