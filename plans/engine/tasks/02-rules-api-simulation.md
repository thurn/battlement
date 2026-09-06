# 02. Define the typed rules API and prove the simulation fast path

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Minimum interfaces](../interfaces.md)

- [Architecture contract](../architecture.md)
- [Execution contract](../execution.md)
- [Validation contract](../validation.md)

**Prerequisite:** [Task 01: Establish behavioral baselines and classify existing
tests](01-behavior-baseline.md) and all its required follow-ups must be
integrated.

**Source roles:** Existing asynchronous executor (for distinction only); chess
AI; Cargo workspace conventions. Resolve these through source-map.md; its links
track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

A new reactant-rules crate can run one synchronous generic rules function with
typed choices and no simulation publication overhead.

## Implementation

1. Define the game associated types, fork_state operation, typed ChoiceSpec
   conversion/validation contract, generic Executor<Game, Mode>, and concrete
   Simulation<Policy>. Keep this crate independent of Unity and Reactant
   components.

2. Make present accept lazy snapshot/change builders. In simulation, invoke
   neither. Evaluate ChoiceSpec through a statically dispatched concrete policy
   without constructing its owned UI prompt. Make check_cancelled an inline
   no-op.

3. Create a small neutral rules fixture with nested calls and two distinct typed
   choices. Run its real public entry point in simulation and a synchronous
   recording test mode to prove the same function works across modes; do not
   create a production pretend-interactive adapter.

4. Add public-entry allocation benchmarks and retained optimized-code inspection
   commands. Exclude game policy allocations from primitive measurements. Update
   execution.md with the final compiling authoring example and exact type names.

## Acceptance

- Two choice variants return their concrete answer types through the same rules
  function; wrong answer-envelope variants are rejected by validation.

- Panicking snapshot/change/prompt builders are never called in simulation.

- A measured loop of present, choose, and check_cancelled performs zero
  mandatory executor allocations and has no virtual dispatch or interactive
  cancellation path in optimized code.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Background workers and interactive waits begin in task 03. This task does not
change existing sample execution.

## Manual QA

Run the public fixture in recording and simulation modes and compare final
outcomes. Inspect the benchmark commands and optimized output from a clean
release build.
