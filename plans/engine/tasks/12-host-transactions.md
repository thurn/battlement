# 12. Prepare native updates and acknowledge rendered frames

Unity prepares a complete update, applies its objects and handlers together, and
tells Rust when that checkpoint has had a rendering opportunity.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [API examples and defaults](../interfaces.md)

- [Presentation timing](../presentation.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 11: Accept completed actions and recover from
failures](11-accepted-action-runtime.md) and all its required follow-ups must be
integrated.

**Starting code:** Protocol messages/commands; native batch/snapshot handling;
runtime commit receipt; world/UI fakes.

## Example

A delayed card face must not expose its new handler before the face appears:

```text
prepare: load front texture while back remains visible
commit: swap face and handler together
acknowledge: Rust installs the committed tree
rendered frame: checkpoint may advance if animation is also complete
```

## Implementation

1. Add protocol prepare/ready/commit/discard records with session, preparation,
   run/checkpoint, base/desired revision, and commit generation. Prepare
   existing world/UI descriptors inactive as the first vertical slice.

2. Implement host resource preparation and discard. Commit validated host
   changes without yielding, then report the committed generation. Maintain
   pending and acknowledged Rust trees separately.

3. Tie handler/ref installation and input generation validation to that commit.
   Reject stale preparations immediately before application; unexpected
   application failure stops the session instead of claiming rollback.

4. Emit a post-render/end-of-frame acknowledgement and forward it through the
   fake's advance_frame. Connect task 11's acceptance interface to the actual
   protocol.

5. Keep the previously visible generation interactive during delayed
   preparation; never emit effects during prepare.

## Acceptance

- Delayed or superseded assets do not flash, play effects, or accept
  new-generation input.

- Two empty-gate checkpoints cannot replace one another in one rendered frame;
  old/pre-gate frame acknowledgements cannot accept the final state.

- Duplicate commit delivery is idempotent; failed/stale preparation cannot
  mutate the committed tree.

- Input during a commit observes one complete generation, and an injected
  unexpected host failure shows the session failure surface.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

New world primitives and full animation registration are later tasks, but their
transaction protocol must be real and usable now.

## Manual QA

Delay preparation across frames, change a setting to supersede it, and verify
only the newest prepared update commits. Step the two-checkpoint, zero-duration
fixture frame by frame.
