# 07. Unify Reactant project tooling under `rt`

Reactant users have one general `rt` command for project workflows. Battlement
keeps reusable lower-level build support, while this repository's `just`
recipes own sample selection and defaults.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Sample migration](../migration.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 06: Extract shared Reactant core, UI layer, and
facade](06-crate-boundaries.md) and all its required follow-ups must be
integrated.

**Starting code:** Battlement CLI parser; standalone Ditto executable; Reactant
asset pipeline; plugin and Addressables commands; sample build/run and author
preparation; Unity editor generated assets; repository `justfile`.

## Example

The public CLI operates on an explicit project and has no sample-name lookup:

```text
rt build --project ../games/card-table
rt run --project ../games/card-table --web
rt author --project ../games/card-table
rt ditto --config ../games/card-table/ditto.toml gallery
rt addressables check --project ../games/card-table
rt plugin inspect ../games/CardTable.app
```

Repository recipes provide shorter sample commands by supplying those project
inputs themselves:

```text
just sample reactant --web
# delegates to rt run --project samples/reactant --web
```

## Implementation

1. Rename and move the existing `battlement-cli` package to Reactant ownership
   as the `rt` package, declaring one binary named `rt`. Move the existing
   general build, run, author, Ditto, plugin, Addressables, and Reactant asset
   entry points beneath that command. Remove the `cargo-battlement` binary.

2. Make `rt build`, `rt run`, and `rt author` resolve an explicit Reactant
   project root, defaulting to the current directory. Read `application`,
   `scene`, and optional `manifest-path` from the root's `reactant.toml`, using
   `rules/Cargo.toml` as the manifest default. `rt assets` uses the same project
   resolution. Resolve file paths from the project root and let explicit flags
   override file values. Keep target, profile, output, and Web-server settings
   as invocation flags. Preserve `--web`, `--release`, and the Web-only `--port`
   option with its 8000 default. `rt addressables` accepts an explicit Unity
   project without requiring Reactant metadata; `rt plugin` uses explicit
   application/library paths. `rt ditto` accepts an explicit Ditto
   configuration whose player marks whether it uses Reactant. Accept paths
   containing spaces without shell interpolation. Remove repository-root
   discovery, `samples/<name>` lookup, chess defaults, and the `sample`
   subcommand.

3. Share one build operation across the commands. `rt build` performs Reactant
   asset preparation, Rust application-plugin construction, and Unity-player
   construction. `rt run` always invokes that operation before launching or
   serving; normal Cargo and Unity incremental behavior may reuse unchanged
   work. Authoring shares the project resolver, while Ditto receives its
   configuration directly rather than using sample-specific paths. Treat UI-only
   Reactant applications as ordinary application plugins without requiring a
   game session or game-specific metadata.

4. Preserve typed Addressables generation and checking under
   `rt addressables`, native-plugin inspect/install/restore/verify under
   `rt plugin`, and Reactant asset discovery/generation/check/preview under
   `rt assets`. Preserve the plugin commands' current macOS application and
   signing scope. Keep their generic mechanics in reusable Battlement libraries.
   Remove Reactant dependencies from those Battlement layers; the
   Reactant-owned `rt` executable depends on both sides and composes them.

5. Keep Ditto's parser, player build, and execution reusable as Battlement
   library code called by `rt ditto`, but make `battlement-ditto` library-only
   by removing its standalone binary and Reactant asset dependency. For a
   player marked `reactant = true`, resolve its Unity project from the Ditto
   configuration, load `reactant.toml`, and run `rt`'s shared asset preparation
   before calling Ditto. Skip that preparation for direct Battlement players.
   Preserve argument forwarding, exit codes, interruption, and remaining
   configuration semantics through `rt`.

6. Move Battlement repository policy into [justfile](../../../justfile).
   Only its recipes may select sample names, configuration files, default
   scenes, and review modes. They pass explicit inputs to `rt` or to
   parameterized lower-level scripts. Those scripts must accept project paths
   and options; they must not maintain another sample registry. Direct
   Battlement fixtures such as basic and ui do not create another public CLI.

7. Convert the existing Reactant and chess-ui projects from `sample.toml` to
   `reactant.toml`, mark their Ditto players as Reactant, and route their `just`
   recipes through `rt`. Mark direct Battlement Ditto players as non-Reactant.
   At this point in the task order, basic, ui, tic-tac-toe, and chess remain
   direct or legacy Battlement projects; keep their `just` recipes on
   parameterized repository build/run mechanics. Tasks 30 and 32 move
   tic-tac-toe and chess to `rt` when those applications migrate to Reactant.

8. Split Unity editor code so generic import helpers stay in Battlement and
   Reactant-specific preparation stays in its own assembly/package layer.
   Update CI entry points, the source map, and build guidance to use `rt` and
   repository recipes rather than preserving contradictory commands. Preserve
   authoring's open-and-enter-Play-mode behavior.

## Acceptance

- `rt --help` exposes build, run, author, Ditto, plugin, Addressables, and
  Reactant asset workflows with no nested Battlement/Reactant namespace or
  sample-name command.

- Cargo metadata shows one `rt` package and binary for these workflows, no
  `battlement-cli`/`cargo-battlement` target, and a library-only
  `battlement-ditto` package.

- An external Reactant project builds, runs, opens for authoring, and executes a
  Ditto scenario through explicit `rt` inputs without living under this
  repository's `samples/` directory.

- `reactant.toml` supplies the required application and scene plus the default
  application-plugin manifest. Explicit overrides win, relative paths use the
  project root, and target/profile/output/Web options do not become persistent
  project policy.

- `rt run` invokes the same preparation/build operation as `rt build`. A path
  containing spaces works, and a failed preparation or build stops before
  launch with a clear error. A UI-only Reactant project uses the same flow
  without game metadata.

- Reactant generated-asset discovery, generation, check, and preview retain
  their behavior under `rt assets`.

- Typed Addressables generation and plugin inspect/install/restore/verify retain
  their behavior through `rt`, while the standalone Ditto executable is absent
  and `rt ditto` preserves its exit and interruption behavior.

- A Reactant Ditto player runs shared asset preparation exactly once before the
  generic Ditto path. A direct Battlement player performs no Reactant work, and
  the `battlement-ditto` dependency graph contains no Reactant crate.

- The repository's documented `just` recipes run all samples through explicit
  paths. Reactant and chess-ui use `rt`; basic, ui, tic-tac-toe, and chess use
  parameterized repository mechanics until their assigned migrations. No
  script outside `justfile` selects a sample name or default.

- Cargo and Unity dependency inspection shows Reactant/`rt` depending on
  Battlement's reusable tooling, with no reverse Reactant dependency in
  Battlement crates or assemblies.

Run the public scenarios available at this task, affected regressions, native
checks for rendered claims, and staged aggregate CI described in
[validation](../validation.md).

## Scope of this task

New Hearts assets belong to task 35. This task changes tooling ownership and
invocation, not artwork.

## Manual QA

From outside the Battlement checkout, run build, native/Web run, authoring, and
a Ditto scenario against a small Reactant project whose path contains spaces.
Exercise Reactant asset generation/check/preview and typed Addressables
generation/check, then force preparation to fail and verify that no player
launches. Inspect, install, verify, and restore a plugin in a disposable macOS
application. Interrupt a long-running command and verify its exit behavior. In
the Battlement checkout, use documented `just` recipes to run Reactant, chess,
and one direct Battlement fixture. Confirm `rt --help` contains no
repository-specific sample names and Cargo metadata contains no superseded
public binary.
