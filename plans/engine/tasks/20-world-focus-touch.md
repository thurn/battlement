# 20. Extend focus, controller navigation, and touch across domains

Keyboard and controller users can visibly focus and activate world controls.
Touch follows the same pointer capture and modal rules as mouse input.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Validation](../validation.md)
- [Sample migration](../migration.md)

**Prerequisite:** [Task 19: Unify UI/world hit testing, propagation, and modal
capture](19-unified-pointer-routing.md) and all its required follow-ups must be
integrated.

**Starting code:** Existing focus/control behavior; Unity keyboard/controller
input; task 19 pointer routing.

## Example

Exercise navigation and activation without converting them to pointer clicks:

```text
Right: focus the next eligible world control
Activate: open its modal
Back: close modal and restore visible focus
remove focused control: select another target without activating it
```

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

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Hearts card navigation and touch inspection UX are tasks 41-42. This task
establishes reusable engine capabilities.

## Manual QA

Operate the mixed-input fixture without a mouse, then repeat capture/modal cases
with touch and verify visible focus throughout.
