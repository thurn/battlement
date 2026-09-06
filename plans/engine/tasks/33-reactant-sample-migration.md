# 33. Migrate the existing Reactant laboratory to the unified APIs

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Migration contract](../migration.md)
- [Architecture contract](../architecture.md)
- [Fixtures contract](../fixtures.md)

**Prerequisite:** [Task 32: Complete chess application integration and remove
the old engine](32-chess-cutover.md) and all its required follow-ups must be
integrated.

**Source roles:** samples/reactant/rules/src; its Ditto selections; extracted
facade and inspector. Resolve these through source-map.md; its links track the
current owner after crate moves. Inspect the concrete caller and host/fake
counterpart before editing.

## Result

Existing Reactant demonstrations retain their behavior and become the home for
the new engine specimens.

## Implementation

1. Replace remaining old App model/imperative host patterns with the unified
   facade, stores, and component APIs. UI-only specimens must not acquire
   unnecessary rules workers.

2. Preserve existing motion, resource, portal, input, and localization
   demonstrations and their review selectors.

3. Integrate task 29's inspector and completed specimens into a stable
   selection/reset surface. Ensure later specimens are clearly unavailable
   rather than fake examples.

4. Update generated asset declarations and sample preparation configuration to
   the new tooling owner. Remove temporary API adapters owned by this sample.

## Acceptance

- Existing Reactant native initial/changed/reset selections remain valid.

- Each completed engine specimen can be selected and reset without leaking
  subscriptions, workers, playbacks, or assets.

- UI-only demonstrations remain simple component code and do not require game
  state/action boilerplate.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

The richer matrix and missing engine demonstrations are tasks 44-45. Do not
claim their captions alone as evidence.

## Manual QA

Browse every existing demonstration, then run a new specimen and return to an
old one. Check reset and cleanup with the inspector.
