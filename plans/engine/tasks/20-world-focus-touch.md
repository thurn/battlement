# 20. Extend focus, controller navigation, and touch across domains

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [World contract](../world.md)
- [Validation contract](../validation.md)
- [Migration contract](../migration.md)

**Prerequisite:** [Task 19: Unify UI/world hit testing, propagation, and modal
capture](19-unified-pointer-routing.md) and all its required follow-ups must be
integrated.

**Source roles:** Existing focus/control behavior; Unity keyboard/controller
input; task 19 pointer routing. Resolve these through source-map.md; its links
track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

World controls participate in visible focus and semantic activation, while touch
uses the same capture/modal contract.

## Implementation

1. Add explicit world focus/activation properties and a directional navigation
   policy using logical eligibility and current displayed geometry. Reuse
   existing UI focus behavior.

2. Expose semantic activate/cancel/navigation in the public driver and native
   host without synthesizing mouse coordinates.

3. Handle touch pointer IDs, capture loss, and UI/world modal exclusion. Provide
   a sample world control with visible focus and return-focus behavior.

4. Preserve existing keyboard/controller mappings in all samples; mechanically
   adapt APIs now rather than deferring compile failures to migration tasks.

## Acceptance

- Keyboard/controller can focus and activate a world control, enter a modal, and
  restore focus on close.

- Removed or ineligible targets do not retain active focus/capture, and stale
  events cannot activate replacements.

- Two touch IDs do not share capture state; canceled touch dispatches no
  activation.

- Existing chess-ui input and keyboard-rebinding tests still pass.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Hearts card navigation and touch inspection UX are tasks 41-42. This task
establishes reusable engine capabilities.

## Manual QA

Operate the mixed-input fixture without a mouse, then repeat capture/modal cases
with touch and verify visible focus throughout.
