# 06. Extract shared Reactant core, UI layer, and facade

The existing UI behavior runs through the new crate boundaries without a second
component runtime.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Architecture](../architecture.md)
- [Identity and state](../identity.md)
- [Sample migration](../migration.md)

**Prerequisite:** [Task 05: Prepare iOS and Android release validation
paths](05-mobile-build-paths.md) and all its required follow-ups must be
integrated.

**Starting code:** Application and engine integration; tree representation;
hooks/context; input/focus; asset declarations.

## Example

A local settings panel must remain independent of game execution after the crate
move:

```rust
App::new().root(SettingsPanel::new())
// No game state, action type, or rules worker is needed.
```

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
   package rather than keeping a compatibility facade. Update generated-asset
   source scanning to recognize the new exact declaration paths. Update
   source-map.md to current locations.

5. Keep project-tooling dependencies temporarily at their existing owner only
   until task 07; explicitly record that remaining edge there.

## Acceptance

- All existing component, localization, lifecycle, motion, and gallery behavior
  tests compile and pass through the extracted runtime.

- UI-only App creation starts no rules worker and needs no game-state type.

- Cargo dependency inspection shows UI -> core and facade -> core/UI/rules with
  no reverse facade dependency or duplicated hook runtime.

Run the public scenarios available at this task, affected regressions, native
checks for rendered claims, and staged aggregate CI described in
[validation](../validation.md).

## Scope of this task

The single `rt` command and project-tooling boundary are task 07; world host
construction is task 13. Do not redesign sample behavior here.

## Manual QA

Build the current Reactant and chess-ui galleries. Change a context value and
exercise a portal/control to verify the extraction retained state and event
behavior.
