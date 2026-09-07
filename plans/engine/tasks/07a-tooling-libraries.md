# 07a. Extract reusable tooling and Unity editor ownership

[Task group 07](07-asset-tooling-boundary.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Sample migration](../migration.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 06: Extract shared Reactant core, UI layer, and
facade](06-crate-boundaries.md) is integrated.

**Starting code:** Battlement CLI parser; standalone Ditto executable; Reactant asset
pipeline; plugin and Addressables commands; sample build/run and author preparation;
Unity editor generated assets; repository `justfile`.

## Implementation

1. Extract generic build/run, plugin, Addressables, and import mechanics into Battlement
   libraries. Reactant asset preparation stays in Reactant-owned code; make the existing
   CLI compose both sides. Preserve current commands during this leaf.

2. Split generic Unity editor import helpers from Reactant preparation into the
   corresponding assemblies/packages. Update callers immediately; preserve authoring
   open-and-enter-Play behavior.

3. Make Ditto's parser, builder, and executor callable as generic Battlement library
   code. Move Reactant preparation to the caller. Its old binary may temporarily call an
   explicitly documented migration adapter, removed by 07d; do not duplicate asset work.

## Acceptance

- Existing build, author, Ditto, plugin, and asset workflows still work. Generic
  library/assembly dependencies point from Reactant toward Battlement; any remaining
  legacy executable edge is explicitly owned by 07d.

- Reuse existing command tests and run a representative Reactant and direct-Battlement
  fixture; inspect dependency ownership without source-text snapshot tests.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](07-asset-tooling-boundary.md) identifies
the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Exercise current authoring and one existing Ditto fixture after the extraction.
