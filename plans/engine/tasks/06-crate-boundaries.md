# 06. Extract shared Reactant core, UI layer, and facade

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Architecture contract](../architecture.md)
- [Identity contract](../identity.md)
- [Migration contract](../migration.md)

**Prerequisite:** [Task 05: Prepare iOS and Android release validation
paths](05-mobile-build-paths.md) and all its required follow-ups must be
integrated.

**Source roles:** Application and engine integration; tree representation;
hooks/context; input/focus; asset declarations. Resolve these through
source-map.md; its links track the current owner after crate moves. Inspect the
concrete caller and host/fake counterpart before editing.

## Result

The existing UI behavior runs through the new crate boundaries without a second
component runtime.

## Implementation

1. Create reactant-core, reactant-ui, and reactant as specified in
   architecture.md. Move logical tree/hooks/context/stores/identity support into
   core and UI-specific controls/adapters into UI.

2. Introduce the typed host description/adapter boundary needed to remove the
   UI-only native field from core. Initially implement the existing UI adapter
   only; preserve its behavior and compiled public component examples.

3. Move application/world orchestration ownership into the facade, reexport
   ergonomic authoring surfaces, and connect reactant-rules without making every
   UI component generic over game state.

4. Mechanically update all workspace and standalone sample
   manifests/imports/macros in this task. Remove the old battlement-reactant
   package rather than keeping a compatibility facade. Update source-map.md to
   current locations.

5. Keep asset-tooling dependencies temporarily at their existing owner only
   until task 07; explicitly record that remaining edge there.

## Acceptance

- All existing component, localization, lifecycle, motion, and gallery behavior
  tests compile and pass through the extracted runtime.

- UI-only App creation starts no rules worker and needs no game-state type.

- Cargo dependency inspection shows UI -> core and facade -> core/UI/rules with
  no reverse facade dependency or duplicated hook runtime.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

The Reactant asset CLI reversal is task 07; world host construction is task 13.
Do not redesign sample behavior here.

## Manual QA

Build the current Reactant and chess-ui galleries. Change a context value and
exercise a portal/control to verify the extraction retained state and event
behavior.
