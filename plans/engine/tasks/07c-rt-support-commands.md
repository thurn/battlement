# 07c. Expose assets, Addressables, plugins, and Ditto through rt

[Task group 07](07-asset-tooling-boundary.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Sample migration](../migration.md)
- [Validation](../validation.md)

**Prerequisite:** [07b: Add standalone rt project resolution and shared
build/run/author](07b-project-build-run.md) is integrated.

**Starting code:** Battlement CLI parser; standalone Ditto executable; Reactant asset
pipeline; plugin and Addressables commands; sample build/run and author preparation;
Unity editor generated assets; repository `justfile`.

## Implementation

1. Preserve typed Addressables generation and checking under `rt addressables`,
   native-plugin inspect/install/restore/verify under `rt plugin`, and Reactant asset
   discovery/generation/check/preview under `rt assets`. Preserve the plugin commands'
   current macOS application and signing scope. Keep their generic mechanics in reusable
   Battlement libraries. Remove Reactant dependencies from those Battlement layers; the
   Reactant-owned `rt` executable depends on both sides and composes them.

2. Expose the generic Ditto parser/build/execution through rt ditto. For reactant=true,
   resolve the project from the explicit Ditto config and run shared asset preparation
   once before generic execution. Direct Battlement players skip Reactant preparation.
   Preserve forwarding, exit status, and interruption; binary removal belongs to 07d.

3. Expose assets through the shared resolver from 07b. Remove repository discovery and
   sample defaults from each new rt command; keep compatibility entrypoints only until
   07d.

## Acceptance

- `rt --help` exposes build, run, author, Ditto, plugin, Addressables, and Reactant
  asset workflows with no nested Battlement/Reactant namespace or sample-name command.

- Reactant generated-asset discovery, generation, check, and preview retain their
  behavior under `rt assets`.

- Typed Addressables generation and plugin inspect/install/restore/verify retain their
  behavior through `rt`, and `rt ditto` preserves its exit and interruption behavior.

- A Reactant Ditto player runs shared asset preparation exactly once before the generic
  Ditto path. A direct Battlement player performs no Reactant work, and the
  generic Ditto library code performs no Reactant preparation. The temporary
  legacy executable dependency from 07a may remain until 07d, which owns the
  final Reactant-free package dependency graph.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](07-asset-tooling-boundary.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Exercise asset preparation, one direct and one Reactant Ditto configuration,
Addressables check, and plugin inspect/install/verify/restore in a disposable
application. Interrupt one representative long command.
