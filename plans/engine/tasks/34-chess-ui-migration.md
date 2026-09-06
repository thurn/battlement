# 34. Migrate the currently implemented chess UI gallery

The current chess-ui gallery uses the unified runtime with its implemented
behavior and appearance preserved.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Architecture](../architecture.md)
- [Animation](../motion.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 33: Migrate the existing Reactant laboratory to the
unified APIs](33-reactant-sample-migration.md) and all its required follow-ups
must be integrated.

**Starting code:** samples/chess-ui/rules/src, tests, and native gallery
scenarios.

## Example

Preserve existing gallery interactions through the new runtime:

```text
open an implemented settings page
change a controlled value; verify its visible state
rebind a key; activate the control using that binding
reset and compare the existing native baseline
```

## Implementation

1. Inventory the actually implemented gallery pages in the selected checkout.
   Use their current implementation, tests, and native baselines to establish
   behavior and appearance.

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

- Unimplemented gallery pages stay explicitly unimplemented; migrate the
  existing page behavior without inventing additional pages.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Unimplemented chess UI pages are outside the overhaul. No visual redesign
belongs here.

## Manual QA

Exercise implemented settings/input/effect pages with mouse and controller,
reset the gallery, and compare to its existing visual contracts.
