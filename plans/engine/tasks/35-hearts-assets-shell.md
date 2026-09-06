# 35. Prepare Hearts assets and its 3D sample shell

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Hearts contract](../hearts.md)
- [Architecture contract](../architecture.md)
- [World contract](../world.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 34: Migrate the currently implemented chess UI
gallery](34-chess-ui-migration.md) and all its required follow-ups must be
integrated.

**Source roles:** Existing sample authoring/build patterns; KayKit
subset/license; new Reactant asset tooling. Resolve these through source-map.md;
its links track the current owner after crate moves. Inspect the concrete caller
and host/fake counterpart before editing.

## Result

A native Hearts sample displays the licensed table/card assets and native UI
shell through the real engine.

## Implementation

1. Locate or obtain the user's KayKit EXTRA archive, record its
   source/hash/license, and inventory a complete regular 52-card texture set
   plus one back. If access is missing, report that concrete prerequisite; do
   not purchase or substitute secretly.

2. Create samples/hearts with the existing standalone
   sample/rules/authoring/Addressables conventions. Generate assets through
   tooling rather than editing generated Unity outputs.

3. Build a fixed angled table scene, four seat labels, score/menu UI shell, and
   a static Rust-composed front/back card specimen. Keep game-specific scene
   composition in Rust except reusable authored environment assets.

4. Register deterministic native review entrypoints and initial/reset capture
   inputs. Ensure the viewport has usable portrait/landscape framing before
   filling it with gameplay.

## Acceptance

- All required deck textures are mapped exactly once to rank/suit and the back
  is available, with license/source provenance retained.

- The sample builds through the same native/threaded-web pipeline and uses no
  Dreamtides files or prefab-bound card hierarchy.

- Front/back rendering, card dimensions, camera framing, and UI shell are
  visible and resettable in a native capture.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Rules begin in task 36 and dynamic hands in task 37. Do not label static cards
as a playable game.

## Manual QA

Open the native shell, inspect representative red/black faces and back,
resize/reorient, and verify assets are readable and correctly assigned.
