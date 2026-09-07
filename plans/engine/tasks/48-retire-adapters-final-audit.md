# 48. Remove transitional machinery and audit the finished architecture

The repository presents one coherent Reactant engine architecture with no
forgotten migration adapters or unsupported completion claims.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Sample migration](../migration.md)
- [Validation](../validation.md)
- [Workflow](../workflow.md)

**Prerequisite:** [Task 47: Validate native, threaded WebGL, and mobile
builds](47-release-conformance.md) and all its required follow-ups must be
integrated.

**Starting code:** Workspace dependency graph; all temporary adapter removal
notes; source-map; maintained guidance.

## Example

Check application code as well as the dependency graph:

```rust
// UI-only app uses ordinary component setup, with no game session.
// A game app starts and attaches its session in one call:
let game = app.start_game::<HeartsGame>(initial_state, make_context);
```

## Implementation

1. Search for remaining old component/runtime facades, game-specific imperative
   presentation paths, reverse Battlement-to-Reactant dependencies, and
   temporary adapters named by earlier tasks.

2. Remove obsolete machinery only after verifying all users have migrated. Keep
   generic low-level Battlement APIs needed by basic/ui and reusable host
   capabilities, routing shared animation behavior consistently.

3. Audit actual Hearts/tic-tac-toe/chess/chess-ui authoring for unnecessary
   framework plumbing. Verify snapshot rendering uses Battlement's existing
   scheduler and operation registry, with no separate presentation transactions
   or animation-progress evaluator. Fix confirmed ergonomic gaps within the contracts
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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Typed prefab binding, humanoid root motion, and extra Hearts variants are
separate work. Physical device execution follows its certification checklist.

## Manual QA

Start each sample through its documented public command, inspect a minimal
game/component authoring example, and verify the current reading guide leads to
the actual code.
