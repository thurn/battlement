# 07d. Switch repository callers and remove superseded executables

[Task group 07](07-asset-tooling-boundary.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Sample migration](../migration.md)
- [Validation](../validation.md)

**Prerequisite:** [07c: Expose assets, Addressables, plugins, and Ditto through
rt](07c-rt-support-commands.md) is integrated.

**Starting code:** Battlement CLI parser; standalone Ditto executable; Reactant asset
pipeline; plugin and Addressables commands; sample build/run and author preparation;
Unity editor generated assets; repository `justfile`.

## Implementation

1. Move Battlement repository policy into [justfile](../../../justfile). Only its
   recipes may select sample names, configuration files, default scenes, and review
   modes. They pass explicit inputs to `rt` or to parameterized lower-level scripts.
   Those scripts must accept project paths and options; they must not maintain another
   sample registry. Direct Battlement fixtures such as basic and ui do not create
   another public CLI.

2. Convert the existing Reactant and chess-ui projects from `sample.toml` to
   `reactant.toml`, mark their Ditto players as Reactant, and route their `just` recipes
   through `rt`. Mark direct Battlement Ditto players as non-Reactant. At this point in
   the task order, basic, ui, tic-tac-toe, and chess remain direct or legacy Battlement
   projects; keep their `just` recipes on parameterized repository build/run mechanics.
   Tasks 30 and 32 move tic-tac-toe and chess to `rt` when those applications migrate to
   Reactant.

3. Remove cargo-battlement and the old battlement-cli package after routing every
   caller. Make battlement-ditto library-only and remove the executable-only migration
   adapter from 07a. rt is the sole project-tool binary. Update CI, source-map, and
   build guidance in the same leaf.

4. Preserve parameterized repository mechanics for basic, ui, tic-tac-toe, and chess
   until their migrations. Only justfile selects sample names, default scenes, or review
   modes.

## Acceptance

- Cargo metadata shows one `rt` package and binary for these workflows, no
  `battlement-cli`/`cargo-battlement` target, and a library-only `battlement-ditto`
  package.

- The repository's documented `just` recipes run all samples through explicit paths.
  Reactant and chess-ui use `rt`; basic, ui, tic-tac-toe, and chess use parameterized
  repository mechanics until their assigned migrations. No script outside `justfile`
  selects a sample name or default.

- Cargo and Unity dependency inspection shows Reactant/`rt` depending on Battlement's
  reusable tooling, with no reverse Reactant dependency in Battlement crates or
  assemblies.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](07-asset-tooling-boundary.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Use the documented recipes for Reactant, chess, and one direct fixture. Inspect final
Cargo metadata and help; reuse earlier command evidence where unchanged.
