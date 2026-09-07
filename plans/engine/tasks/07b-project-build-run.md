# 07b. Add standalone rt project resolution and shared build/run/author

[Task group 07](07-asset-tooling-boundary.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Sample migration](../migration.md)
- [Validation](../validation.md)

**Prerequisite:** [07a: Extract reusable tooling and Unity editor
ownership](07a-tooling-libraries.md) is integrated.

**Starting code:** Battlement CLI parser; standalone Ditto executable; Reactant asset
pipeline; plugin and Addressables commands; sample build/run and author preparation;
Unity editor generated assets; repository `justfile`.

## Implementation

1. Introduce the Reactant-owned rt package/binary with build, run, and author. Keep the
   old executable only until 07d so all existing callers work. Use the project contract
   in architecture.md; external projects may use explicit local Cargo and Unity
   dependencies.

2. Make `rt build`, `rt run`, and `rt author` resolve an explicit Reactant project root,
   defaulting to the current directory. Read `application`, `scene`, and optional
   `manifest-path` from the root's `reactant.toml`, using `rules/Cargo.toml` as the
   manifest default. The resolver is reusable by assets in 07c. Resolve file paths from
   the project root and let explicit flags override file values. Keep target, profile,
   output, and Web-server settings as invocation flags. Preserve `--web`, `--release`,
   and the Web-only `--port` option with its 8000 default. The remaining commands are
   introduced in 07c. Accept paths containing spaces without shell interpolation. The
   new rt commands remove repository-root discovery, `samples/<name>` lookup, chess
   defaults, and the `sample` subcommand.

3. Share one build operation across the commands. `rt build` performs Reactant asset
   preparation, Rust application-plugin construction, and Unity-player construction. `rt
   run` always invokes that operation before launching or serving; normal Cargo and
   Unity incremental behavior may reuse unchanged work. Authoring shares the project
   resolver, while Ditto receives its configuration directly rather than using
   sample-specific paths. Treat UI-only Reactant applications as ordinary application
   plugins without requiring a game session or game-specific metadata.

4. Package required Web initializer/server resources with their owning tool or package.
   Remove assumed-checkout reads from the new project path. No installer, SDK
   distribution, publishing, scaffold command, or dependency downloading belongs here.
   Existing supported target/profile/output options stay invocation flags; the plan does
   not add a new target matrix.

## Acceptance

- An external Reactant project builds, runs, and opens for authoring through explicit
  `rt` inputs without living under this repository's `samples/` directory.

- `reactant.toml` supplies the required application and scene plus the default
  application-plugin manifest. Explicit overrides win, relative paths use the project
  root, and target/profile/output/Web options do not become persistent project policy.

- `rt run` invokes the same preparation/build operation as `rt build`. A path containing
  spaces works, and a failed preparation or build stops before launch with a clear
  error. A UI-only Reactant project uses the same flow without game metadata.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](07-asset-tooling-boundary.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

From outside the checkout, build/run and author one project whose path contains spaces;
check native and Web launch plus preparation-failure-before-launch.
