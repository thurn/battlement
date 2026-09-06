# 34. Migrate the currently implemented chess UI gallery

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Migration contract](../migration.md)
- [Architecture contract](../architecture.md)
- [Motion contract](../motion.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 33: Migrate the existing Reactant laboratory to the
unified APIs](33-reactant-sample-migration.md) and all its required follow-ups
must be integrated.

**Source roles:** samples/chess-ui/rules/src and tests; retained chess UI plan's
applicable contracts/pages. Resolve these through source-map.md; its links track
the current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

The current chess-ui gallery uses the unified runtime with its implemented
behavior and appearance preserved.

## Implementation

1. Inventory the actually implemented gallery pages in the selected checkout.
   Use the retained chess UI plan only for those pages' behavior/fidelity
   requirements.

2. Migrate imports, component composition, stores, portals, focus, Motion
   controls, and asset setup to the new facade without introducing worker
   execution for local settings.

3. Replace unnecessary application wiring with reusable APIs where needed,
   updating their owning contracts/callers in this task.

4. Preserve keyboard rebinding, controls, effects, localization, gallery
   navigation/reset, and existing page snapshots. Remove this sample's remaining
   legacy adapters.

## Acceptance

- All currently implemented page tests and native initial/changed/reset captures
  remain valid.

- Controlled props, inline child composition, focus/default prevention, and
  motion do not require extra per-sample framework plumbing.

- Unimplemented retained-plan pages stay explicitly unimplemented; this
  migration does not invent or waive their acceptance.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Remaining original chess UI port pages are outside the overhaul. No visual
redesign belongs here.

## Manual QA

Exercise implemented settings/input/effect pages with mouse and controller,
reset the gallery, and compare to its existing visual contracts.
