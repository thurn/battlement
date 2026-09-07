# 23b. Add cached rest measurement for dependent layouts

[Task group 23](23-world-layout.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Animation](../motion.md)
- [Identity and state](../identity.md)

**Prerequisite:** [23a: Compute world layout targets from explicit rest
boxes](23a-world-layout-targets.md) is integrated.

**Starting code:** World host descriptions; existing UI layout concepts; prepared rest
bounds and geometry observations.

## Implementation

1. Cache optional native rest measurements with request identity and invalidate only
   dependent ancestors when they change. Do not use animated current bounds as layout
   input.

2. Use existing geometry observations and identified requests to cache optional rest
   measurements. Hold only dependent pose commands while unmeasured; independent menus
   remain responsive. Treat rest metadata as the default path, not an obligation to
   measure every object.

## Acceptance

- Changing one subtree's box recomputes its dependent layouts without rebuilding
  unrelated arrangements.

- A deferred measurement prevents an unmeasured pose commit; a stale measurement cannot
  overwrite a newer result.

- Animating a visual's scale does not change rest-layout positions unless its authored
  layout box also changes.

Reuse existing evidence for covered behavior. Run affected checks and staged aggregate
CI as specified in [validation](../validation.md).

## Scope and handoff

This leaf owns only the behavior above. Keep existing callers working and update source
pointers for the next leaf. The [group index](23-world-layout.md) identifies the
remaining assignments; do not implement later acceptance criteria here.

## Manual QA

Delay a rest measurement, resize again, and verify the stale response cannot overwrite
the current target.
