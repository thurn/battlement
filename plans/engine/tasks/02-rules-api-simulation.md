# 02. Define the typed rules API and prove the simulation fast path

A new reactant-rules crate can run one synchronous generic rules function with
typed choices and no simulation publication overhead.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [API examples and defaults](../interfaces.md)

- [Architecture](../architecture.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 01: Establish behavioral baselines and classify existing
tests](01-behavior-baseline.md) and all its required follow-ups must be
integrated.

**Starting code:** Existing asynchronous executor (for distinction only); chess
AI; Cargo workspace conventions.

## Example

The simplest game should need state and a function. Compile this style of
registration, then compile a second example with two typed choices:

```rust
let game = Game::new().state(Counter::default()).rules(apply);
App::new().game(game).root(CounterDisplay::new())
```

## Implementation

1. Implement registration using state and a rules function with inferred action
   types, default Clone fork/snapshot behavior, optional callbacks, and no
   mandatory Game trait. Add Executor<State, Mode>, typed choice specifications
   and handles, built-in selections, and concrete simulation policies as
   described in interfaces.md. Keep the rules crate independent of
   Unity/components; use a registration-only harness until App integration in
   task 11.

2. Make present accept lazy snapshot/change builders. In simulation, invoke
   neither. Evaluate choice specifications through a statically dispatched
   concrete policy without constructing its owned UI prompt. Make
   check_cancelled an inline no-op.

3. Create a small neutral rules fixture with nested calls and two distinct typed
   choices, including borrowed data through nested calls. Run its real public
   entry point in simulation and a synchronous recording test mode to prove the
   same function works across modes; do not create a production
   pretend-interactive adapter.

4. Add public-entry allocation benchmarks and retained optimized-code inspection
   commands. Exclude game policy allocations from primitive measurements. Update
   execution.md with the final compiling authoring example and exact type names.

## Acceptance

- A choice-free game needs no framework trait, prompt enum, answer enum,
  movement setup, or custom snapshot type. Two typed choices work in the same
  nested function; wrong transport answer types are rejected by validation.

- Panicking snapshot/change/prompt builders are never called in simulation.

- A measured loop of present, choose, and check_cancelled performs zero
  mandatory executor allocations and has no virtual dispatch or interactive
  cancellation path in optimized code.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Background workers and interactive waits begin in task 03. This task does not
change existing sample execution.

## Manual QA

Run the public fixture in recording and simulation modes and compare final
outcomes. Inspect the benchmark commands and optimized output from a clean
release build.
