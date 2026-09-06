# 35. Prepare Hearts assets and its 3D sample shell

A native Hearts sample displays the licensed table/card assets and native UI
shell through the real engine.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Architecture](../architecture.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 34: Migrate the currently implemented chess UI
gallery](34-chess-ui-migration.md) and all its required follow-ups must be
integrated.

**Starting code:** Existing sample authoring/build patterns; KayKit
subset/license; new Reactant asset tooling.

## Example

Verify the complete asset mapping before building dynamic gameplay:

```text
52 distinct rank/suit faces + one shared back
front texture on Rust-created surface
back texture on separate Rust-created surface
independent hit region; readable portrait and landscape framing
```

## Implementation

1. Locate or obtain the user's KayKit EXTRA archive, record its
   source/hash/license, and inventory a complete regular 52-card texture set
   plus one back. If access is missing, report that concrete prerequisite; do
   not purchase or substitute secretly.

2. Create samples/hearts with the existing standalone
   sample/rules/authoring/Addressables conventions. Generate assets through
   tooling rather than editing generated Unity outputs.

3. Build a fixed angled table scene, four seat labels, score/menu UI shell, and
   a static Rust-composed front/back card test scene. Keep game-specific scene
   composition in Rust except reusable authored environment assets.

4. Register deterministic native review entrypoints and initial/reset capture
   inputs. Ensure the viewport has usable portrait/landscape framing before
   filling it with gameplay.

## Acceptance

- All required deck textures are mapped exactly once to rank/suit and the back
  is available, with license/source provenance retained.

- The sample builds through the same native/threaded-web pipeline and constructs
  its card hierarchy in Rust from the prepared assets.

- Front/back rendering, card dimensions, camera framing, and UI shell are
  visible and resettable in a native capture.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Rules begin in task 36 and dynamic hands in task 37. Do not label static cards
as a playable game.

## Manual QA

Open the native shell, inspect representative red/black faces and back,
resize/reorient, and verify assets are readable and correctly assigned.
