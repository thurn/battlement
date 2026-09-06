# 07. Move Reactant asset preparation out of Battlement

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Architecture contract](../architecture.md)
- [Migration contract](../migration.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 06: Extract shared Reactant core, UI layer, and
facade](06-crate-boundaries.md) and all its required follow-ups must be
integrated.

**Source roles:** Reactant asset pipeline; sample build/author preparation;
Unity editor generated assets. Resolve these through source-map.md; its links
track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

Battlement tooling can build direct samples without depending on Reactant, while
Reactant samples retain automatic asset preparation.

## Implementation

1. Create reactant-cli and move Reactant-specific generated-asset
   commands/libraries to their Reactant ownership. Retain generic Addressables
   and native plugin/player commands in Battlement.

2. Add the optional executable-plus-argument-array preparation entry to
   sample.toml, resolved relative to the sample with a deterministic working
   directory. Spawn directly without shell interpolation and stop the build on
   failure.

3. Configure Reactant sample preparation to call the checkout's Reactant CLI
   with explicit project/manifest inputs. Update author/build flows and CI
   entrypoints; remove the direct battlement-cli dependency on Reactant asset
   crates.

4. Split Unity editor code so generic import helpers stay in Battlement and
   Reactant-specific preparation stays in its own assembly/package layer. Update
   source-map and relevant build guidance rather than preserving old
   contradictory commands.

## Acceptance

- basic and ui prepare/build without Reactant tooling dependencies.

- Reactant generated-asset discovery, generation, check, and preview retain
  their behavior from the new command.

- A sample path with spaces is handled correctly; a failing preparation
  executable prevents plugin/player build and reports its failure.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

New Hearts assets belong to task 35. This task changes ownership and invocation,
not artwork.

## Manual QA

Run generation/check on the existing Reactant sample, then intentionally use a
failing preparation command in an untracked fixture and verify clear failure
before build.
