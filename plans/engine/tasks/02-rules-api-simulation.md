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
tests](01-behavior-baseline.md) is integrated.

**Starting code:** Existing asynchronous executor (for distinction only); chess
AI; Cargo workspace conventions.

## Example

Rules receive the same concrete answer through human input or a policy:

```rust
let card: CardId = context.execution.choose(state, PlayCardPrompt { choices });
```

The policy reads the shared prompt enum and returns an option index. A second
prompt returns `[CardId; 3]`; there is no game-wide answer enum.

## Implementation

1. Implement the rules types and signatures in interfaces.md and the linked
   contract sketch: `Game`, `ExecutionMode<G, C>`, `PromptData<G>`, and
   `ChoicePolicy`. Keep them independent of Unity/components. Each game owns its
   context struct and embeds the reusable execution mode alongside arbitrary
   game-specific data and logic; there is no `GameContext` implementation.

2. Implement infallible `as_prompt`/`into_prompt` wrapping into one Cow-based
   prompt enum. Keep `P` for direct option-index mapping; never extract it back
   from the enum. Prompt data owns its choices and may enumerate lazily. Invalid
   selected indices or responses panic. Policy code receives state and the same
   enum the display will inspect; hidden-state sampling belongs to the game.

3. Make simulation `present` skip both snapshot cloning and the lazy animation
   builder. Simulation `choose` ignores live choice ownership and calls the
   policy inline with no display connection or wait. Borrowing the enum must
   neither allocate nor clone the prompt. There is no explicit
   cancellation-check primitive.

4. Compile a choice-free game (`Prompt<'a> = ()`), two distinct response types
   through nested rules, and a second game's generic prompt handling. Exercise
   simulation mode through nested calls. Declare the interactive API shape
   without inventing a public recording connection or claiming live execution.
   Tasks 09–11 supply interactive publication, replies, and App integration;
   task 11 owns end-to-end equivalence through the same game.

5. Measure primitive overhead separately from constructing owned prompt vectors
   and policy search. Retain one focused public-entry allocation measurement and
   review static dispatch. Inspect optimized code only if dispatch is uncertain;
   no compiler-output snapshots or general benchmark framework are required. Context-mode branching is permitted; do not claim the entire
   simulation or every game-owned prompt is allocation-free.

## Acceptance

- One Game implementation compiles for both modes and runs in simulation. Prompt fields/legality are
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

Run nested simulation choices through the public rules API. Inspect typed
responses, deterministic outcomes, and primitive allocation evidence. Actual
interactive equivalence is verified in task 11.
