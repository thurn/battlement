# 02. Define the typed rules API and prove the simulation path

The rules crate runs the agreed `Game::execute` contract with owned prompt data,
index-returning policies, and no display work during simulation.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Rules and session API](../interfaces.md)
- [Architecture](../architecture.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 01: Establish behavioral baselines and classify existing
tests](01-behavior-baseline.md) and all its required follow-ups must be
integrated.

**Starting code:** Existing asynchronous executor (for distinction only); chess
AI; Cargo workspace conventions.

## Example

Rules receive the same concrete answer through human input or a policy:

```rust
let card: CardId = context.choose(state, PlayCardPrompt { choices });
```

The policy reads the shared prompt enum and returns an option index. A second
prompt returns `[CardId; 3]`; there is no game-wide answer enum.

## Implementation

1. Implement the rules types and signatures in interfaces.md and the linked
   contract sketch: `Game`, `GameContext`, `PromptData<T>`, and `ChoicePolicy`.
   Keep them independent of Unity/components. Context is game-owned and may
   branch on its mode. Do not introduce a required generic execution-mode type.

2. Implement prompt enum conversion and stable option-index mapping. Prompt data
   owns its choices and may enumerate lazily. Invalid selected indices or
   responses panic. Policy code receives state and the same enum the display
   will inspect; hidden-state sampling belongs to the game.

3. Make simulation `present` skip both snapshot cloning and the lazy animation
   builder. Simulation `choose` calls the policy inline with no display
   connection or wait. There is no explicit cancellation-check primitive.

4. Compile a choice-free game (`Prompt = ()`), two distinct response types
   through nested rules, and a second game's generic prompt handling. Exercise
   the same rules with a recording context and a simulation context. Task 11
   supplies actual App startup and worker integration.

5. Measure primitive overhead separately from constructing owned prompt vectors
   and policy search. Retain public-entry allocation measurements and optimized
   code inspection. Context-mode branching is permitted; do not claim the entire
   simulation or every game-owned prompt is allocation-free.

## Acceptance

- One Game implementation runs in both contexts. Prompt fields/legality are
  defined once; stable option order maps policy indices to the correct typed
  response. An out-of-range index panics.

- A panicking snapshot-clone or animation builder is never invoked by simulation
  `present`. Simulation choice makes no display publication or worker wait.

- After prompt construction, primitives require no extra heap allocation or
  virtual dispatch. Report prompt-construction and rollout allocations
  separately.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Background workers begin in task 03. Interactive response transport is task 10;
App startup, handles, and attachment are task 11. Do not migrate samples here.

## Manual QA

Run the same nested fixture in recording and simulation contexts. Inspect its
selected response types and compare final outcomes and allocation evidence.
