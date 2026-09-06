# 33. Migrate the existing Reactant laboratory to the unified APIs

Existing Reactant demonstrations retain their behavior and become the home for
the new engine test scenes.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Sample migration](../migration.md)
- [Architecture](../architecture.md)
- [Test scenes](../fixtures.md)

**Prerequisite:** [Task 32: Complete chess application integration and remove
the old engine](32-chess-cutover.md) and all its required follow-ups must be
integrated.

**Starting code:** samples/reactant/rules/src; its Ditto selections; extracted
facade and inspector.

## Example

Selection and reset must dispose of the previous demonstration:

```text
open an existing UI demo; change its setting
select draw-reflow; run its animation
reset; return to UI demo
no old workers, subscriptions, or effects continue in the new scene
```

## Implementation

1. Replace remaining old App model/imperative host patterns with the unified
   facade, stores, and component APIs. UI-only test scenes must not acquire
   unnecessary rules workers.

2. Preserve existing motion, resource, portal, input, and localization
   demonstrations and their review selectors.

3. Integrate task 29's inspector and completed test scenes into a stable
   selection/reset surface. Ensure later test scenes are clearly unavailable
   rather than fake examples.

4. Update generated asset declarations and sample preparation configuration to
   the new tooling owner. Remove temporary API adapters owned by this sample.

## Acceptance

- Existing Reactant native initial/changed/reset selections remain valid.

- Each completed engine test scene can be selected and reset without leaking
  subscriptions, workers, playbacks, or assets.

- UI-only demonstrations remain simple component code and do not require game
  state/action boilerplate.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

The richer matrix and missing engine demonstrations are tasks 44-45. Do not
claim their captions alone as evidence.

## Manual QA

Browse every existing demonstration, then run a new test scene and return to an
old one. Check reset and cleanup with the inspector.
