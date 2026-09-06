# 12. Add prepared host commits and rendered-frame acknowledgement

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Minimum interfaces](../interfaces.md)

- [Presentation contract](../presentation.md)
- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 11: Integrate action admission, accepted state, and
failure surfaces](11-accepted-action-runtime.md) and all its required follow-ups
must be integrated.

**Source roles:** Protocol messages/commands; native batch/snapshot handling;
runtime commit receipt; world/UI fakes. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

Host preparation, visible commit, Rust acknowledgement, and rendered opportunity
have correlated identities and atomic input behavior.

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

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

New world primitives and full animation registration are later tasks, but their
transaction protocol must be real and usable now.

## Manual QA

Delay preparation across frames, change a setting to supersede it, and verify
only the newest proposal commits. Step the zero-duration two-checkpoint fixture
frame by frame.
