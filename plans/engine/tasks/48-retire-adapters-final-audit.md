# 48. Remove transitional machinery and audit the finished architecture

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Architecture contract](../architecture.md)
- [Migration contract](../migration.md)
- [Validation contract](../validation.md)
- [Workflow contract](../workflow.md)

**Prerequisite:** [Task 47: Run final native, threaded-WebGL, and mobile build
conformance](47-release-conformance.md) and all its required follow-ups must be
integrated.

**Source roles:** Workspace dependency graph; all temporary adapter removal
notes; source-map; maintained guidance. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

The repository presents one coherent Reactant engine architecture with no
forgotten migration adapters or unsupported completion claims.

## Implementation

1. Search for remaining old component/runtime facades, game-specific imperative
   presentation paths, reverse Battlement-to-Reactant dependencies, and
   temporary adapters named by earlier tasks.

2. Remove obsolete machinery only after verifying all users have migrated. Keep
   generic low-level Battlement APIs needed by basic/ui and reusable host
   capabilities, routing shared animation behavior consistently.

3. Audit actual Hearts/tic-tac-toe/chess/chess-ui authoring for unnecessary
   framework plumbing. Fix confirmed ergonomic gaps within the fixed contracts
   and update relevant callers/examples.

4. Update source-map and incorrect maintained guidance by replacement, keeping
   this temporary plan separate from durable API docs. Verify every
   task/contract/source link.

5. Run affected checks and final aggregate CI after cleanup. Report any
   outstanding physical certification or advisory performance follow-up without
   claiming it passed.

## Acceptance

- The dependency graph satisfies architecture.md across runtime, tooling, tests,
  and Unity assemblies.

- There is one logical component runtime and one shared Motion execution model;
  no sample needs an old migration engine.

- All named transitional adapters have been removed or shown to be intentional
  generic Battlement capabilities.

- Final tests/CI pass after cleanup and the requirement coverage matrix has
  evidence for every in-scope v1 contract.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Dreamtides work, prefab binding, root motion, extra Hearts variants, and
physical certification remain outside this overhaul.

## Manual QA

Start each sample through its documented public command, inspect a minimal
game/component authoring example, and verify the current reading guide leads to
the actual code.
