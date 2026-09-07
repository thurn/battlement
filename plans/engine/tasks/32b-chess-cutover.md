# 32b. Switch chess to Reactant and remove its old engine

[Task group 32](32-chess-cutover.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Rules and choices](../execution.md)
- [Animation](../motion.md)
- [Validation](../validation.md)

**Prerequisite:** [32a: Complete chess flow in the Reactant
fixture](32a-chess-app-integration.md) is integrated.

**Starting code:** Task 31 chess fixture; existing chess AI, persistence, diagnostics,
audio, spawn and input code.

## Implementation

1. Switch the default factory and Ditto fixtures to the Reactant implementation only
   after the complete old behavior suite passes on it. Remove the old command-building
   engine and task 31's transition-only factory.

2. Keep opaque piece prefabs, but remove game-specific imperative presentation decisions
   outside Rust components/policies.

3. Move chess project metadata and recipes to rt as required by the tooling group. Reuse
   32a's evidence and rerun affected default-entrypoint/native checks after switching.

## Acceptance

- All established chess gameplay/input/audio/persistence/diagnostic assertions pass
  through the new default entrypoint.

- No legacy chess engine or temporary adapter remains necessary for normal gameplay.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](32-chess-cutover.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Launch chess through its public recipe, verify start/move/capture/save/reset, and
inspect that the alternate factory and old engine are removed.
