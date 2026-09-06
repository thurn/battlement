# 03. Prove native worker cancellation and Rust cleanup

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 02: Define the typed rules API and prove the simulation
fast path](02-rules-api-simulation.md) and all its required follow-ups must be
integrated.

**Source roles:** Native C ABI and Engine; existing exported-engine fixture;
Unity runner. Resolve these through source-map.md; its links track the current
owner after crate moves. Inspect the concrete caller and host/fake counterpart
before editing.

## Result

A real native Unity-hosted action can be abandoned through silent Rust unwinding
while the host stays responsive.

## Implementation

1. Add the private cancellation payload and one worker boundary owning a forked
   state. Use resume_unwind for cancellation, catch/downcast at the worker
   boundary, and report genuine panic separately.

2. Implement endpoint closure/cancellation and worker lifecycle observations
   with mutex/condition-variable predicates. Start with a controlled wait
   fixture; later tasks add full checkpoint and prompt payloads.

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

- Closing the endpoint before or during the wait cannot lose a wakeup; Unity
  exit does not synchronously join the worker.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Full publication/prompt queues belong to tasks 09-10. Threaded WebGL proof is
task 04.

## Manual QA

Run the native release fixture, cancel during the controlled wait, immediately
interact with its replacement surface, then trigger a real panic separately.
