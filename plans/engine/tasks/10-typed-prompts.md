# 10. Implement typed interactive prompts and validated answers

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Minimum interfaces](../interfaces.md)

- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 09: Implement immutable checkpoint publication and
backpressure](09-checkpoint-publication.md) and all its required follow-ups must
be integrated.

**Source roles:** ChoiceSpec/modes from task 02; bounded endpoint from task 09;
display driver. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

Synchronous rules can request typed choices while the public display exposes
only the currently actionable prompt.

## Implementation

1. Publish prompt snapshots in the checkpoint stream with run/request
   identities. Keep the concrete specification on the worker and construct only
   its owned public representation for display.

2. Validate answer envelopes against the current request and immutable choice
   specification. Return a concrete typed answer only after successful
   validation and a final cancellation check.

3. Report invalid-answer feedback publicly while retaining the unanswered
   request. Reject stale identities without resuming the worker.

4. Build a neutral select/deselect cycle fixture and two different answer types.
   Keep display-only selection/menu stores independent from the immutable legal
   specification.

## Acceptance

- An invalid or wrong-variant answer leaves the same request active; a valid
  answer resumes exactly once.

- A prompt behind an unfinished earlier checkpoint is not actionable.
  Replacement invalidates old answers even if request numbers repeat in another
  run.

- Cancellation wins over a queued answer when already observed at the final
  check, and a prompt cycle exits silently with cleanup.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Native animation gates are task 25. Use the existing public
presentation-consumer barrier to hold earlier checkpoints until the host
protocol exists.

## Manual QA

Wait at a prompt, open settings, submit an invalid answer, then replace the game
and submit the old answer. Confirm the replacement remains unchanged.
