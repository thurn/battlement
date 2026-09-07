# 43a. Provide native and WebGL durable save operations

[Task group 43](43-hearts-save-resume.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 42: Finish Hearts keyboard/controller navigation and
menus](42-hearts-navigation-menus.md) is integrated.

**Starting code:** GameHandle::accepted_state and explicit menu actions; generic
persistent-data host support; chess save integration; Hearts state.

## Implementation

1. Reuse game-owned native Rust file I/O for temporary-write/atomic-replace. Add
   only the missing generic WebGL durable-flush acknowledgement at the existing
   Battlement host boundary, correlated to the captured write. No native file-I/O
   command protocol is needed. Keep the game schema and save triggers game-owned.

2. Use a tiny explicit byte payload to verify read/write, acknowledged durability, and
   write/flush failure preserving the prior durable value. Do not build a save migration
   framework, autosave service, or storage abstraction beyond the actual native/browser
   requirements.

## Acceptance

- A native restart and actual threaded-WebGL reload read the exact acknowledged payload.
  A failed write/flush reports failure and preserves the previously durable payload.
  If termination loses the acknowledgement, load whichever complete write
  actually reached durable storage. Reuse existing storage paths where sufficient.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](43-hearts-save-resume.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Write, acknowledge, reload, then inject a failed replacement write in native and WebGL.
