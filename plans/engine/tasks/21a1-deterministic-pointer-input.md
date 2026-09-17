# 21a.1. Prove deterministic native hover and drag delivery

[Task group 21](21-shared-motion-drivers.md) · [Workflow](../workflow.md) ·
[Validation](../validation.md)

## Read before implementing

- [World objects and input](../world.md)
- [Animation](../motion.md)
- [Validation](../validation.md), especially host conformance and platform evidence

**Prerequisite:** [21a: Shared Motion drivers](21a-motion-drivers.md) is integrated.
This is the first assignment when continuing from 21a. Complete and promote it
before [21b](21b-motion-parameters.md); existing task numbers remain unchanged.

**Starting code:** `BattlementPointerDevices`, `BattlementPointerInput`,
`BattlementLogicalPointerInput`, `BattlementPanelInputCoordinator`,
`BattlementUiSyntheticInputAdapter`, and `BattlementRunner`; Ditto's
`DittoInputTargets`, `DittoInputTransaction`, `DittoScenarioExecutor`, job
validation/codecs, and player reset. Locate these through the
[source map](../source-map.md). Read both the actual device path and controlled
player loop, including focus, suspension, panel capture, and cleanup callbacks.

## Reliability gate

Ship only if ordinary supported hover/drag delivery has no dependency on OS
cursor position, foreground focus, Unity Input System event queues, or another
test's scheduling. A correct action must be consumed exactly once in order and
reach the shared production routing path. Merely detecting frequent dropped
actions and failing the test does not satisfy this requirement.

Do not promise universal fault-free software from a finite stress run. Establish
the guarantee through explicit ownership and ordering invariants, adversarial
tests, and retained native evidence. Crashes, actual application suspension,
unsupported environments, and invalid actions must terminate explicitly within
a bounded watchdog; they cannot become successful or silently skipped input.
Ordinary desktop focus loss and concurrent players are supported conditions,
not acceptable failure excuses. If this contract cannot be met, retain the
blocker, leave the capability disabled, and do not advance to 21b. Do not relax
the contract to finish the task.

## Implementation

1. Separate device acquisition from production pointer processing. Both real
   devices and an exclusively owned Ditto source feed the same hit testing,
   UI/world arbitration, propagation, capture, gesture recognition, and native
   drag execution. Preserve ordinary device behavior. No OS input synthesis,
   cursor warping, synthetic Unity devices, `QueueStateEvent`/`QueueEvent`, or
   manual `InputSystem.Update` calls in the controlled delivery path. Calling
   handlers directly, fabricating enter/drag events, or setting the resulting
   transform is not a valid substitute for exercising production routing.

2. Give each player session an exclusive input lease and generation. Suppress
   competing physical input and duplicate automatic UI module delivery while
   controlled. Audit focus/reset/device-change callbacks so they cannot clear
   controlled buttons, capture, or hover. Independent player processes must
   operate concurrently without a global focus/cursor lock or shared mutable
   pointer state. Reject a second owner in one session before mutation. Isolate
   transport endpoints, request identities, and artifacts between runs.

3. Define the controlled loop's ordering: apply ready host changes, make picking
   geometry current, consume the ordered pointer samples, process their effects,
   then expose the resulting presentation boundary. Specify coordinate space,
   viewport/camera identity, sample sequence, pointer identity, buttons,
   cancellation, and capture transitions. Reuse existing frame/time controls;
   worker barriers, virtual time, and rendered frames remain separate. Never
   synchronize by sleeping, stealing focus, or retrying an action until it lands.

4. Add authorable hover, leave, and press/move/release/cancel trajectories. Resolve
   semantic targets to reachable geometry using actual native picking. Do not
   force the requested target through an occluder or modal. Fix a stationary
   pointer in screen space while objects move underneath it; target tracking
   must be a separately explicit operation if supported. Sample drag paths at
   declared virtual/frame boundaries, retaining presses and capture across
   steps, including UI/world crossings. Define rejection for missing targets,
   ambiguous hit ties, stale generations, duplicate/out-of-order samples, and
   unsupported actions before they can cause unintended delivery.

5. Produce a consumed-input receipt and trace with session/generation, sample
   sequence, actual hit/route, capture owner, and associated presentation
   boundary. A receipt proves processing, not the expected game outcome; assert
   that outcome separately. Hover that only changes native Motion must succeed
   without a Rust evaluation or causal command batch. Missing receipts cannot
   be treated as success. Extend Ditto's existing job/result contracts and exact
   replay inputs; preserve verified FlatBuffers wherever engine transport is
   involved. Keep queues and retained diagnostic buffers bounded.

6. Revoke the lease on completion, cancellation, failure, timeout, or disconnect.
   Clear buttons, hover and capture; reject late messages from the old generation;
   restore the previous real-input configuration. Preserve intentional explicit
   focus-loss/cancellation test operations independently from ambient OS focus.
   Keep existing semantic activation and ordinary input tests passing.

## Acceptance and validation

- Native Ditto demonstrates hover composing with a moving base, smooth release
  to its latest pose, a stationary pointer crossed by a moving target, drag
  capture outside bounds and across UI/world regions, release, cancellation,
  modal interception, and target removal during drag. Assert intermediate poses,
  routing/capture outcomes, and terminal state; retain inspected screenshots.
- Prove the same production behavior through public display scenarios and
  native adapter checks. UI Toolkit defaults/capture must not be bypassed by a
  test-only imitation. Keep a small separate real-device adapter conformance
  suite; deterministic playback does not certify OS/hardware delivery.
- On the supported desktop native release player, run the same fixed seeded
  trajectories alone and in at least two genuinely concurrent independent
  players. Cover startup without focus, real foreground switches during held
  drags, background execution, and unrelated physical input/device activity.
  The focus perturbation harness may manipulate focus; input delivery may not.
  Record proof that players actually overlapped and focus changes occurred.
- Retain at least 1,000 complete trajectories per player across at least 20 fresh
  launches, including warm session reuse and varied host load/interleaving.
  Require zero dropped, duplicated, reordered, misrouted, or cross-session
  samples, zero unexpected cancellations, and matching normalized traces and
  presentation outcomes versus isolated execution. Normalize only declared
  incidental IDs/wall times. Do not hide failed attempts with retries or accept
  a percentage-based flake budget. This supplements the ownership proof.
- Fault probes cover duplicate/out-of-order/stale messages, full queues, lease
  conflicts, transport loss, and player termination; failures are bounded and
  distinguish infrastructure failure from application rejection. A subsequent
  clean session inherits no pointer state. Reproduce a failed seed directly.
- Keep a small deterministic regression set in ordinary CI; retain the bounded
  stress command and its full certification evidence without imposing the whole
  stress matrix on every unrelated change. Follow required platform selection;
  advertise capabilities only for backends with proven delivery, explicitly
  rejecting others. Do not weaken performance-run focus validity checks to make
  functional background testing pass. Stage intended files before aggregate CI.

## Scope and handoff

This leaf owns reliable native controlled pointer delivery and its evidence,
including the UI/world routing needed for real hover and drag. It does not add
new application gestures, layout/effect features, or OS automation as a product
capability. Preserve direct samples, input responsiveness, bounded scheduling,
and existing Motion semantics. Report the exact supported execution envelope,
ownership proof, stress results, failure boundaries, and any unavailable backend.
Update source pointers. Only after successful promotion continue with 21b.

## Manual QA

Run two native fixtures concurrently, switch focus elsewhere during each held
drag, and move the physical mouse. Both declared trajectories must continue
unchanged. Cancel one scenario, then start another and verify clean input state.
