# 21. Animate UI, world objects, and effects with shared drivers

The same transition and playback controls animate UI properties, world
transforms, and typed native parameters.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Animation](../motion.md)
- [World objects and input](../world.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 20: Extend focus, controller navigation, and touch
across domains](20-world-focus-touch.md) and all its required follow-ups must be
integrated.

**Starting code:** UI Motion sampler/timeline; Reactant targets/values; world
property writers; fake scheduling.

## Example

UI and world properties use the same transition and control behavior:

```rust
let transition = Transition::spring();
Button::new().animate(target().opacity(1.0)).transition(transition.clone());
WorldGroup::new().animate(target().scale(1.0)).transition(transition);
```

## Implementation

1. Extract host-neutral transition sampling, playback identity/generation, and
   property ownership from UI Motion. Add typed writers for world transform,
   material scalar, light intensity, particle emission, and audio volume.

2. Preserve existing UI animate/initial/exit/variants/gesture semantics and
   MotionConfig inheritance. Ordinary host playback must not trigger Rust
   component evaluation.

3. Implement separate base transform and local gesture-offset composition. Make
   replacement start from actual displayed values and preserve spring velocity.

4. Extend the fake with the same observable interpolation/control/outcome
   contract, using independent code paths where needed rather than calling Unity
   or production tree internals.

5. Expose running Motion as existing `IBattlementCommandOperation` instances,
   including finite/infinite behavior, completion, cancellation, and failures.
   Preserve generated command blocking flags; verify blocking waits and
   nonblocking continuation through the existing batch scheduler. Reuse generic
   low-level tween drivers where semantics match; preserve direct sample contracts.

## Acceptance

- One UI opacity and one world scalar with the same transition sample
  equivalently at intermediate times.

- Pause/resume/speed/stop and interrupted/completed/failed outcomes behave
  consistently across adapters.

- A hover offset composes with a moving base and returns smoothly to the latest
  underlying target.

- Existing UI motion regression tests and native captures remain valid,
  including reduced-motion behavior.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Completion-relative sequences are task 22 and layout ownership is tasks 23-24.
Static parameter support from task 17 now becomes animatable.

## Manual QA

Run motion-equivalence and compare UI/world controls at slow speed, including
replacement during a spring and reduced-motion settings.
