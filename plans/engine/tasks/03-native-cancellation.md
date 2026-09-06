# 03. Prove native worker cancellation and Rust cleanup

A real native Unity-hosted action can be abandoned through silent Rust unwinding
while the host stays responsive.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 02: Define the typed rules API and prove the simulation
fast path](02-rules-api-simulation.md) and all its required follow-ups must be
integrated.

**Starting code:** Native C ABI and Engine; existing exported-engine fixture;
Unity runner.

## Example

Use a fixture with nested Rust calls and a controlled wait so cleanup has a
deterministic observation:

```text
start action; wait until nested function is blocked
cancel; immediately interact with the replacement display
observe nested destructors finish, then worker-stopped
expected cancellation produces no panic report
```

## Implementation

1. Add the private cancellation payload and one worker boundary owning a cloned
   state. Use resume_unwind for cancellation, catch/downcast at the worker
   boundary, and report genuine panic separately.

2. Implement worker connection closure and cancellation and worker lifecycle
   observations with mutex/condition-variable predicates. Start with a
   controlled wait fixture; later tasks add full checkpoint and prompt payloads.

3. Drop communication guards before initiating unwind. Ensure worker-stopped is
   emitted only after private state/destructors are gone. Document any
   AssertUnwindSafe ownership argument at its use.

4. Extend the exported native fixture and Unity integration runner to execute
   nested Rust rules calls, abandon the worker, and observe cleanup. Validate
   actual release panic strategy and keep all unwinding within Rust frames.

## Acceptance

- Cancellation wakes a waiting worker, calls nested drop probes exactly once,
  and produces worker-stopped after cleanup without an expected-cancellation
  panic report.

- An ordinary rules panic is distinguishable from cancellation and does not
  poison an unrelated replacement run.

- Closing the worker connection before or during the wait cannot lose a wakeup;
  Unity exit does not synchronously join the worker.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Full publication/prompt queues belong to tasks 09-10. Threaded WebGL proof is
task 04.

## Manual QA

Run the native release fixture, cancel during the controlled wait, immediately
interact with its replacement surface, then trigger a real panic separately.
