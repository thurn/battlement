# 10. Implement typed interactive prompts and validated answers

Synchronous rules can request typed choices while the public display exposes
only the currently actionable prompt.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [API examples and defaults](../interfaces.md)

- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 09: Publish immutable checkpoints with at most one
waiting](09-checkpoint-publication.md) and all its required follow-ups must be
integrated.

**Starting code:** Choice specifications/execution modes from task 02; worker
connection from task 09; display driver.

## Example

A component answers through a typed handle for the current presented prompt:

```rust
let presented = use_game_prompt::<CardGame>();
presented.answer.submit(Answer::Card(selected_card));
// An illegal card produces feedback and keeps this request active.
```

## Implementation

1. Publish prompt views in the checkpoint stream with run/request identities.
   Keep the concrete specification on the worker, construct its owned public
   representation, and attach the engine-owned typed answer handle outside the
   game prompt value.

2. Validate typed answer messages against the current request and immutable
   choice specification. Return a concrete typed answer only after successful
   validation and a final cancellation check.

3. Report invalid-answer feedback publicly while retaining the unanswered
   request. Reject stale identities without resuming the worker.

4. Build a neutral select/deselect cycle fixture and two different answer types.
   Keep display-only selection/menu stores independent from the immutable legal
   specification.

## Acceptance

- An invalid or wrong-type answer leaves the same request active; a valid answer
  resumes exactly once.

- A prompt behind an unfinished earlier checkpoint is not actionable.
  Replacement invalidates old answers even if request numbers repeat in another
  run.

- Cancellation wins over a queued answer when already observed at the final
  check, and a prompt cycle exits silently with cleanup.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Native animation gates are task 25. Use the existing public
presentation-consumer barrier to hold earlier checkpoints until the host
protocol exists.

## Manual QA

Wait at a prompt, open settings, submit an invalid answer, then replace the game
and submit the old answer. Confirm the replacement remains unchanged.
