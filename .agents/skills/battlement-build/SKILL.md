---
name: battlement-build
description: Build or run Battlement samples, author Unity projects, replace native plugins, or regenerate Addressables constants and Reactant assets.
---

# Build and generate

Run from the task worktree root. Use the checkout CLI so commands match source:
`cargo run --quiet -p battlement-cli -- <command>`. Consult its `--help` for
options; the parser is `crates/battlement-cli/src/main.rs`.

Read `rust-toolchain.toml`, the chosen sample's `sample.toml`, and its
`ProjectSettings/ProjectVersion.txt` for tool and player inputs. Do not copy
version pins into guidance. CLI tool resolution is in
`crates/battlement-cli/src/tools.rs`.

| Task | CLI command after `--` |
| --- | --- |
| Build native player | `sample build <sample>` |
| Build and run native player | `sample run <sample>` |
| Open Unity authoring | `author --project samples/<sample>` |
| Regenerate typed Addressables constants | `generate samples/<sample>` |
| Check those constants | `generate samples/<sample> --check` |
| Generate declared Reactant paint | `reactant assets generate --project samples/<sample>` |
| Check generated paint | `reactant assets check --project samples/<sample>` |
| Generate and open paint gallery | `reactant assets preview --project samples/<sample>` |

Use `--manifest-path` where supported for a nonstandard rules manifest.
Inspect `crates/battlement-cli/src/generate.rs` and
`crates/battlement-cli/src/reactant_assets.rs` for generation behavior.
Edit declarations and Unity authoring inputs, not generated PNGs, metadata,
Addressables exports, or staged plugin binaries. Check the resulting diff.

For replacing a plugin in an existing stopped macOS player, use
`plugin inspect <app>`, `plugin install <app> <library>`, and
`plugin verify <library>`. `plugin restore <app>` restores its saved original.
Read `crates/battlement-cli/src/plugin.rs` and command help for Cargo-build and signing options.

Prefer native Ditto for validation. Web review uses `battlement-web`.
Track any authoring/player process you launch and stop that owned process when
finished. Never substitute an old installed CLI or cached player without
verifying it matches the source being tested.
