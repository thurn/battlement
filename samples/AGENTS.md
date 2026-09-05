# Samples

Each sample is a Unity project. Its `rules/` directory owns the Rust game or UI
implementation; `sample.toml` describes player builds and `ditto.toml` describes
scenario execution. Inspect those files for the selected sample.

Read [Rust conventions](../crates/AGENTS.md) before changing rules code. Sample
Cargo workspaces are not all root workspace members: use the sample's manifest
for focused Rust checks.

Use `battlement-build` for authoring, player builds, and generated assets;
`battlement-ditto` for native behavior checks. Do not hand-edit generated Unity
assets or plugin outputs. Use `battlement-web` only for web-specific work.

For chess UI port work, start with the
[retained plan](../docs/reactant/chess-ui-implementation-plan.md) and read only
its applicable shared contracts and selected page.
