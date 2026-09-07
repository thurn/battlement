# 06a. Extract the shared host interface inside Reactant

[Task group 06](06-crate-boundaries.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Identity and state](../identity.md)
- [Sample migration](../migration.md)

**Prerequisite:** [Task 05: Prepare iOS and Android release validation
paths](05-mobile-build-paths.md) is integrated.

**Starting code:** Application and engine integration; tree representation;
hooks/context; input/focus; asset declarations.

## Implementation

1. Introduce the typed host description/adapter boundary within the existing
   battlement-reactant package. Remove UI-specific native fields from shared
   tree/reconciliation ownership while initially implementing only the existing UI
   adapter. Keep public imports and sample behavior intact.

2. Identify core versus UI versus app/facade ownership in actual modules. Move useful
   implementations internally instead of starting a second runtime. The crate rename and
   all public caller updates belong to 06b.

## Acceptance

- All existing component, localization, lifecycle, motion, and gallery behavior tests
  compile and pass through the extracted runtime.

- The existing UI adapter runs through the extracted host interface. Hooks, refs,
  portals, effects, localization, and Motion retain their existing behavior. No world
  implementation or rules worker is required for this slice.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](06-crate-boundaries.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Run existing UI/gallery behavior and inspect one portal/context interaction after the
internal extraction.
