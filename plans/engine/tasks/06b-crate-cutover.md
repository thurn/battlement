# 06b. Move core, UI, and facade ownership and update callers

[Task group 06](06-crate-boundaries.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Identity and state](../identity.md)
- [Sample migration](../migration.md)

**Prerequisite:** [06a: Extract the shared host interface inside
Reactant](06a-host-boundary.md) is integrated.

**Starting code:** Application and engine integration; tree representation;
hooks/context; input/focus; asset declarations.

## Implementation

1. Create reactant-core, reactant-ui, and reactant as specified in architecture.md. Move
   logical tree/hooks/context/stores/identity support into core and UI-specific
   controls/adapters into UI.

2. Move application/world orchestration ownership into the facade, reexport ergonomic
   authoring surfaces, and connect reactant-rules without making every UI component
   generic over game state.

3. Mechanically update all workspace and standalone sample manifests/imports/macros in
   this task. Remove the old battlement-reactant package rather than keeping a
   compatibility facade. Update generated-asset source scanning to recognize the new
   exact declaration paths. Update source-map.md to current locations.

4. Keep project-tooling dependencies temporarily at their existing owner only until task
   07; explicitly record that remaining edge there.

## Acceptance

- All existing component, localization, lifecycle, motion, and gallery behavior tests
  compile and pass through the extracted runtime.

- UI-only App creation starts no rules worker and needs no game-state type.

- Cargo dependency inspection shows UI -> core and facade -> core/UI/rules with no
  reverse facade dependency or duplicated hook runtime.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](06-crate-boundaries.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Build Reactant and chess-ui through the renamed crates, verify UI-only App setup, and
inspect dependency direction.
