# 36. Implement fixed Hearts rules through the shared context contract

One synchronous Hearts implementation handles dealing, passing, legal play,
tricks, scoring, and match completion in live games and simulation.

[Plan and order](../README.md) · [Workflow](../workflow.md) · [Source
map](../source-map.md) · [Validation](../validation.md)

## Read before implementing

- [Hearts rules and behavior](../hearts.md)
- [Rules and choices](../execution.md)
- [Validation](../validation.md)

**Prerequisite:** [Task 35: Prepare Hearts assets and its 3D sample
shell](35-hearts-assets-shell.md) is integrated.

**Starting code:** reactant-rules public API; Hearts shell; public display
driver.

## Example

Use explicit deals for rare rules:

```text
first trick, unable to follow clubs:
  a non-penalty discard exists -> penalty choices are excluded
  only penalty cards are available -> allow a penalty discard
queen of spades alone -> hearts remain unbroken
```

## Implementation

1. Define HeartsGame, complete HeartsState, actions, semantic animations, owned
   prompt structs/enum, and HeartsContext. Implement logical_clone,
   is_legal_action, and execute. Use stable rank/suit order and a documented
   seeded shuffle with enough saved data for deterministic restoration.

2. Implement ResolvePassing and PlayTurn with typed responses and lazy semantic
   events. Collect passing choices against unchanged hands and commit all
   transfers together. PlayTurn includes leading AI plays, one human choice, and
   subsequent AI plays until the next human choice or hand boundary. All those
   AI choices run within one execute call. Include trick/hand resolution when
   the last card is played. End the action at a completed hand even if no human
   choice was reached in that action.

3. Route human/live-AI choices in HeartsContext; identify the deciding seat
   during passing as well as play. Simulation always uses its configured policy.
   Policies receive state and the shared prompt enum. Keep hidden-state sampling
   game-owned; the engine does not construct a filtered view or observation.

4. Implement the fixed first-trick, broken-heart, queen, moon, 100-point, and
   shared-tie rules from hearts.md without new house-rule settings.

5. Add a minimal text display from state snapshots for complete scripted hands.
   Keep opponent cards hidden through display logic. UI legality prevents
   illegal dispatch/replies; fault-injected illegal requests panic.

## Acceptance

- Explicit deals cover all pass directions, follow-suit restrictions, forced
  first-trick penalty, queen not breaking hearts, only-hearts lead, trick
  leadership, moon scoring, and shared winners.

- The same seeded scripted action sequence reaches the same state through
  interactive and simulation contexts, with stable policy option ordering.

- Full snapshots remain immutable. Player-facing text and inspection do not
  reveal hidden cards despite having access to the full state.

- A hand ends after thirteen tricks; the match ends only after hand scoring.

Run the public scenarios, affected regressions, native checks for rendered
claims, and staged aggregate CI described in [validation](../validation.md).

## Scope of this task

Card layout/interaction follows in tasks 37-39; Monte Carlo policies in task
40. Use a deterministic legal policy here.

## Manual QA

Run explicit rare-rule deals, inspect enabled choices and totals, then route one
human and one AI choice through the same rules function.
