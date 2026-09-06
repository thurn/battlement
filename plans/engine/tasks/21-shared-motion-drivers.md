# 21. Use shared Motion drivers for UI, world, and native properties

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Motion contract](../motion.md)
- [World contract](../world.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 20: Extend focus, controller navigation, and touch
across domains](20-world-focus-touch.md) and all its required follow-ups must be
integrated.

**Source roles:** UI Motion sampler/timeline; Reactant targets/values; world
property writers; fake scheduling. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

The same transition and playback controls animate UI properties, world
transforms, and typed native parameters.

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

5. Route retained generic low-level tween execution through shared drivers where
   semantics match; do not break basic/ui protocol contracts.

## Acceptance

- One UI opacity and one world scalar with the same transition sample
  equivalently at intermediate times.

- Pause/resume/speed/stop and interrupted/completed/failed outcomes behave
  consistently across adapters.

- A hover offset composes with a moving base and returns smoothly to the latest
  underlying target.

- Existing UI motion regression tests and native captures remain valid,
  including reduced-motion behavior.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Completion-relative sequences are task 22 and layout ownership is tasks 23-24.
Static parameter support from task 17 now becomes animatable.

## Manual QA

Run motion-equivalence and compare UI/world controls at slow speed, including
replacement during a spring and reduced-motion settings.
