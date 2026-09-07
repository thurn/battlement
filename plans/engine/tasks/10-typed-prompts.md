# 10. Implement typed interactive prompts and validated responses

Synchronous rules return typed responses while display code examines the same
owned prompt enum used by policies.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and session API](../interfaces.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 09: Publish immutable checkpoints through a 32-slot
queue](09-checkpoint-publication.md) and all its required follow-ups must be
integrated.

**Starting code:** PromptData and ExecutionMode from task 02; worker
connection from task 09; display driver.

## Example

A matched prompt determines the required answer type:

```rust
if let Some(presented) = use_game_prompt::<HeartsGame>() {
    if let HeartsPrompt::PlayCard(prompt) = &presented.prompt {
        presented.handle.submit(prompt.as_ref(), selected_card);
    }
}
```

A three-card answer in that branch is a compile error. An illegal card submitted
to a current request is a programming error; an ended-request reply is ignored.

## Implementation

1. Publish snapshot and owned `PresentedPrompt<T>` in checkpoint order. The
   wrapper contains `G::Prompt<'static>` and its `ResponseHandle`. Retain the
   original concrete P in a private typed request and publish one owned clone;
   keep both alive through resolution. The handle identifies the request; the
   prompt argument selects the response type. Do not compare prompt addresses.

2. Implement the generic `ResponseHandle<T>::submit<G, P>` signature from
   interfaces.md, tying T to the game's owned prompt enum. Validate
   session/run/request, concrete prompt/response type, human/AI ownership, and
   legality against the stored request. Valid input resumes once. Worker wakeup
   checks request/type and cancellation before returning.

3. Ignore ended-request replies before decoding or checking their payload. Panic
   on active illegal/mismatched replies through a Rust boundary that never
   unwinds across the C ABI. UI uses the same validator to disable illegal
   input; do not add a recoverable invalid-response result type.

4. Implement connection helpers for human input and policy-owned live requests.
   Both retain P in internal Arc-backed storage and publish one owned clone
   after reserving capacity. Concrete data is Clone + Send + Sync. The typed
   validator uses retained P without enum extraction. Live AI borrows
   P.as_prompt() on the rules worker immediately after enqueueing, without
   waiting for Unity; only full 32-slot capacity delays publication. Then map its index
   directly through P. Human handles cannot resolve AI requests. Player routing
   remains in game context.

5. Exercise two response types and another game's prompt enum. Retain compile
   checks for wrong response types, fault-injected transport mismatch, stable
   indices, zero-sized prompt data, and local selection/menu state independent
   of rules. Verify borrowed policy calls do not clone and temporary wrappers
   cannot escape into owned display storage.

## Acceptance

- Valid responses resume once. Active invalid replies panic; stopped, replaced,
  and already-resolved request replies are ignored without resuming anything.

- Native prompt controls are queued after earlier blocking commands. Request
  handles are valid in Rust before display; distinct native targets prevent an
  old visible prompt from invoking a newer handle. Cover all activation modes.

- A caller-created prompt cannot broaden legal choices. An AI-owned request
  cannot be answered by human UI even if the value would be legal.

- Cancellation already observed before returning an answer wins. Menus remain
  responsive during both human waits and bounded live AI computation.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Task 12 connects queued native prompt controls. Use the public input/consumer
fixture until then; never add a presentation-ready wait for live AI.

## Manual QA

Wait at a human prompt, change selection and settings, answer once, then submit
a duplicate. In a fault fixture submit an illegal current reply and observe
failure. Restart and deliver an old reply; the replacement remains unchanged.
