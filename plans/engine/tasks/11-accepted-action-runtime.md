# 11. Start game sessions, accept actions, and expose recovery

App-owned sessions construct domain contexts, expose cloneable handles, and keep
rendered snapshots separate from accepted and worker state.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and session API](../interfaces.md)
- [Rules and choices](../execution.md)
- [Presentation timing](../presentation.md)
- [Architecture](../architecture.md)

**Prerequisite:** [Task 10: Implement typed interactive prompts and validated
responses](10-typed-prompts.md) is integrated.

**Starting code:** Application/engine integration; worker completion records; UI
failure surface; existing chess saves.

## Example

Start and attachment are one operation:

```rust
let game = app.start_game::<HeartsGame>(initial_state, |connection| {
    HeartsContext {
        execution: ExecutionMode::Interactive {
            connection,
            policy: HeartsPolicy { human_player },
        },
    }
});
```

Handle clones refer to the same session. Calling start again stops/replaces it.

## Implementation

1. Implement the complete session surface in interfaces.md: the context factory,
   `GameHandle` methods, `DispatchResult`, `GameStatus`, state/prompt/status
   hooks, and app-owned attachment. Accept initial state immediately, hold Busy
   until initial publication is consumed, and execute nothing merely because
   a game starts. Unity playback never determines rules readiness.

2. Dispatch returns Busy before validation during initial publication, rules
   execution, a human wait, or unconsumed final publication. Otherwise call is_legal_action; false panics before cloning or
   worker creation. Legal work returns Started. Failed/stopped dispatch is a
   programming error. Run IDs are internal.

3. Transfer the session context to one active worker at a time. Retain final
   state until the Rust consumer submits final output; automatic final publication has no semantic
   event. Return the context after normal completion; discard interrupted
   context on failure/stop and construct a new one for replacement.

4. Implement idempotent nonjoining stop, old-handle isolation, fresh game-subtree presentation lifetime on replacement, and
   Ready/Busy/Failed/Stopped status. App teardown stops its session. Distinguish
   immediate public Stopped from the later cleanup-complete worker observation.

5. Return an independent logical clone from accepted_state in every session
   status. It returns the previous completed state while busy. Expose status
   subscriptions for UI recovery/next-action scheduling, with no acceptance
   callback or autosave service. Preserve detailed failures in diagnostics.

## Acceptance

- The same deterministic rules fixture reaches equivalent state through
  interactive App dispatch/typed replies and direct simulation. This completes
  task 02's deferred live-path proof without a test-only execution facade.

- Startup and replacement construct exactly one context with the correct
  connection. Old handles remain stopped; cloned current handles share state.

- Busy dispatch queues no work and invokes no validator. Illegal idle dispatch
  panics. Started does not imply completion.

- Holding final publication in Rust keeps the previous accepted state and Busy.
  Consuming it installs the new state and Ready even with Unity playback paused.
  Another legal action may submit commands behind the earlier action.

- Worker failure retains the previous accepted state. Host failure retains the
  latest accepted state, including actions completed ahead of playback. Restart
  reconstructs from it; ended-session output is ignored. No automatic save occurs.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Task 12 connects the consumer to existing ordered batches and task 25 adds Motion
commands. Use a public Rust-consumer fixture until then. Do not tie acceptance
or status to native completion.

## Manual QA

Start, dispatch, hold final acceptance, explicitly copy accepted state, and
stop. Restart in the same app. Inject a worker failure and observe status-driven
recovery controls without losing the last accepted state.
