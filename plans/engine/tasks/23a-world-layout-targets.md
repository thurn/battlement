# 23a. Compute world layout targets from explicit rest boxes

[Task group 23](23-world-layout.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Animation](../motion.md)
- [Identity and state](../identity.md)

**Prerequisite:** [Task 22: Build sequences with labels that follow actual
completion](22-sequence-dependencies.md) is integrated.

**Starting code:** World host descriptions; existing UI layout concepts; prepared rest
bounds and geometry observations.

## Implementation

1. Integrate Taffy for world Flexbox/Grid using explicit world-unit extents and
   authored/rest-metadata boxes. Preserve UI Toolkit ownership of UI layout.

2. Define the pure custom-layout interface over ordered IDs/boxes/parameters and
   implement fan, pile, and arc algorithms with deterministic output.

3. Expose live layout destination handles for each child, plus explicit
   orientation/scaling rules. Add fixed-plane and nested-layout fixtures with known
   expected positions.

## Acceptance

- Flex/Grid/fan/pile/arc produce deterministic target poses on two explicit planes using
  authored rest boxes. Include one nested layout and keep geometry facing/scaling
  explicit.

- Animating a visual's scale does not change rest-layout positions unless its authored
  layout box also changes.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](23-world-layout.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Resize a fan/grid and inspect deterministic targets without native measurement or
animation feedback.
