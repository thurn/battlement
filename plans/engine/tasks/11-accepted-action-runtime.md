# 11. Integrate action admission, accepted state, and failure surfaces

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read and start

- [Minimum interfaces](../interfaces.md)

- [Execution contract](../execution.md)
- [Presentation contract](../presentation.md)
- [Architecture contract](../architecture.md)

**Prerequisite:** [Task 10: Implement typed interactive prompts and validated
answers](10-typed-prompts.md) and all its required follow-ups must be
integrated.

**Source roles:** Application/engine integration; worker completion records; UI
failure surface; existing chess saves. Resolve these through source-map.md; its
links track the current owner after crate moves. Inspect the concrete caller and
host/fake counterpart before editing.

## Result

A game App coordinates private worker actions while retaining a safe accepted
state and responsive display stores.

## Implementation

1. Add the game application adapter with fork_state, action dispatch, presented
   snapshot provider, accepted-state notification, and active-run lifecycle.

2. Return Busy while an action/prompt is unresolved. Otherwise run the game's
   pure bounded validate_action against accepted state; return Invalid(reason)
   without a worker for illegal actions, or Started(run_id) for admitted work.
   Handle prompt answers through their typed request path. Keep menus and
   inspection available.

3. Retain final worker state pending presentation acceptance; make acceptance
   depend on a presentation-completion interface that will receive real host
   acknowledgements in task 12 and motion gates in task 25.

4. Route worker panic to an active-run failure surface with exit/restart from
   accepted state. Never poison a replacement run for abandoned output.

5. Add persistence notification outside the worker; a failing fixture store
   leaves accepted state usable. No game-specific save format belongs in the
   engine.

## Acceptance

- A worker completing computation does not immediately enable a next action or
  save; explicit presentation acceptance does.

- Invalid action dispatch starts no worker and changes no accepted/presented
  state; Busy and Started are distinct from final state acceptance.

- An abandoned or panicking action leaves the previous accepted state available
  for restart.

- Menus/store updates remain responsive while the worker publishes or waits, and
  only one interactive action is active.

Use standalone public scenarios for these assertions and the appropriate native
specimen for rendered claims. Run affected regressions and the required staged
aggregate CI as described in validation.md. Preserve concrete evidence for each
bullet; a compiling API or placeholder specimen is not acceptance.

## Named deferrals

Do not ship a fake 'rendered immediately' implementation: until task 12,
exercise acceptance through the explicit public fixture host. Motion-dependent
acceptance is task 25.

## Manual QA

Hold final presentation acceptance, observe saving/next action disabled, then
release it. Trigger a worker failure and restart from the last accepted state.
