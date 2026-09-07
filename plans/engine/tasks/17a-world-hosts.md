# 17a. Compose sprites, meshes, cameras, and lights

[Task group 17](17-world-rendering-primitives.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Presentation timing](../presentation.md)
- [Identity and state](../identity.md)

**Prerequisite:** [Task 16: Add stable state selectors and queued display
stores](16-view-selectors-stores.md) is integrated.

**Starting code:** World adapter; object protocol/builders; Unity world creation;
prepared assets; fake asset catalog.

## Implementation

1. Add typed world::Sprite, Mesh, Camera, and Light builders using existing Rust
   protocol, Unity hosts, and fake observations. Keep world::Group from task 13; adapt
   existing host capabilities rather than defining parallel ones.

2. Extend prepared mesh/material asset dependencies as needed. Specify geometry
   scale/orientation and independent front/back child replacement. Camera/light creation
   must be available before the motion writers and Hearts shell.

## Acceptance

- Missing required assets fail existing asset loading before dependent commands;
  prepared assets are reused for replacement children through ordered commands.

- A native fixture shows a Rust-composed textured surface and prepared mesh with
  explicit scale/orientation under a Rust-declared camera/light. Replacement and
  missing-asset behavior use the existing queue.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](17-world-rendering-primitives.md)
identifies the remaining assignments; do not implement later acceptance criteria here.

## Manual QA

View a surface and mesh, switch a face, and verify explicit geometry and camera framing.
