# Rules API: start with state and a function

A simple game should need ordinary state and a rules function, not a collection
of framework-specific types. Register that function with the application, then
use an executor to present intermediate states or request choices. This page
specifies the public authoring contract and the small amount of type machinery
needed to support both interactive play and simulation.

Read this for tasks 02 and 09–11. Related pages: [application
composition](architecture.md), [worker behavior](execution.md), and
[presentation timing](presentation.md). The API examples describe the interface
to implement. Task 02 must make the representative examples compile before other
tasks build on them.

## The default case

A rules function receives state, an action, and its execution context. The same
function is generic over interactive or simulation execution:

```rust
fn apply<M: ExecutionMode<Counter>>(
    state: &mut Counter, action: Add, cx: &mut Executor<Counter, M>,
) {
    state.total += action.amount;
    cx.present(|| state.clone(), || ());
}
```

Registration infers the state and action types. The interactive function is
specialized when passed to `.rules`; type inference should make an explicit mode
argument unnecessary in the normal application call:

```rust
let game = Game::new().state(Counter::default()).rules(apply);
App::new().game(game).root(CounterDisplay::new())
```

Implement these defaults:

- Clone state to create the worker's private copy and the final snapshot.
- Use `()` when the game has no semantic change descriptions.
- Accept actions when no validator is supplied. The application still returns
  `Busy` while another action is running or awaiting presentation.
- Publish a final snapshot automatically when the rules function returns.
- Require no prompt or answer types for a game that never asks a question.
- Use the engine's default movement transition without application setup.

`Game` is a builder supplied by the engine. Game authors do not implement it.
There is no game-wide associated-type list for state, snapshot, action, changes,
prompt, answer, or policy decisions.

The display reads the current snapshot and dispatches actions through context.
For example, clicking Add updates the counter through the registered function:

```rust
fn counter_display() -> impl Render {
    let view = use_game_snapshot::<Counter>();
    let actions = use_game_actions::<Add>();
    (
        Label::new().text(view.total.to_string()),
        Button::new().text("Add one")
            .on_click(move |_| actions.dispatch(Add { amount: 1 })),
    )
}
```

The action handle returns `Started`, `Busy`, or `Invalid`. It does not directly
mutate the snapshot. The new value appears when the worker's checkpoint is
presented. `CounterDisplay` is the component wrapping this render function.

## Customize only what the game needs

A larger game can choose cheaper or safer copies and typed change descriptions.
These are independent callbacks on the registration, inferred from their values:

```rust
Game::new()
    .state(saved_or_new)
    .fork(HeartsState::clone)
    .snapshot(HeartsState::visible_to_south)
    .changes::<Change>()
    .validate(validate_action)
    .rules(resolve)
```

Omitting `.fork()` uses `Clone`. A custom fork must produce an independent
mutable state; sharing immutable data through `Arc` is fine. A state that is not
`Clone` must provide its own fork and snapshot callbacks. Builder methods can
change the builder's Rust type to express these choices on stable Rust; do not
rely on unstable associated-type defaults.

The `.snapshot()` callback also supplies snapshots for prompts and normal
completion. Explicit `present` calls must produce that same snapshot type. The
optional `.changes::<Change>()` selects the typed changes for this game;
omitting it uses `()`. An implemented API may infer that type from a callback
instead, provided ordinary call sites remain clear. The final checkpoint has an
empty change list by default. Provide an optional final-change builder for games
that need a distinct completion animation, without duplicating the final
snapshot.

For example, a game can publish a single change or an ordered group. Builders
are lazy in both cases:

```rust
cx.present(|| state.visible_to_south(), || Change::CardPlayed(card));
cx.present_many(|| state.visible_to_south(), || [
    Change::TrickCollected(winner), Change::ScoreChanged(winner),
]);
```

Single-change publication assigns index zero. Group indices follow iteration
order. The executor must accept game-selected immutable snapshots and change
collections; it must not require a separate collection type on a game trait.

## Choices have local types

A **choice specification** describes one decision and how to validate its
answer. For example, selecting a card returns `CardId`; selecting three cards
returns a three-card collection. These types belong to that choice, not a
mandatory game-wide prompt/answer enum.

Provide built-in single- and multiple-selection specifications. They borrow
eligible values for simulation and create an owned display prompt only when
interactive execution needs it:

```rust
let legal = state.legal_cards(player);
let card = cx.choose(state, SelectOne::new().options(&legal));
state.play(card);
```

`choose` receives an immutable borrow of the current rules state explicitly. The
executor passes that borrow to the registered snapshot callback when it needs to
publish an interactive prompt. It never keeps a hidden pointer to the worker's
mutable state. The choice can borrow legal options while the call waits; the
synchronous rules function cannot mutate that state during the wait. Simulation
receives the same borrow but does not construct a snapshot from it.

A custom specification can supply its own owned prompt, answer type, and
validator. For Hearts it also describes the acting player's observation. Its
implementation contract is:

- Borrow legal options and observation data while the synchronous call runs.
- Build an owned, `Send + 'static` prompt lazily for interactive display.
- Validate answers against the unchanged decision, returning invalid-answer
  feedback without consuming a pending request.
- Expose a typed answer handle with the prompt, so a component can answer
  without assembling IDs or converting an answer to a game-wide enum.
- Allow a game to use an enum when useful, but do not require one.

For example, the human card picker uses the currently presented prompt:

```rust
let prompt = use_prompt::<SelectOne<CardId>>();
Button::new().text("Play")
    .enabled(prompt.allows(selected))
    .on_click(move |_| prompt.answer(selected))
```

The handle carries the run and request IDs internally. Transport decoding must
reject the wrong answer type, stale handles, and illegal values. Normal typed
callers should be unable to send an answer of the wrong Rust type.

## Static simulation without a large game trait

Simulation specializes the same function for a concrete policy. It never builds
owned prompts, snapshots, or changes merely to satisfy an interactive interface.
A choice policy receives the choice's borrowed decision data and returns its
typed answer inline.

The implementation can express this using an `ExecutionMode<State, View,
Change>` bound for publication and a `ResolveChoice<State, Spec>` bound for each
choice. `View` and `Change` default to `State` and `()` on the engine trait, so
the choice-free example keeps its short bound. A rules module with several
choices can give their combined bounds a local trait name to keep its function
signatures short. The trait is a convenience over ordinary bounds, not another
state model.

For Hearts, define the combined mode bound once beside its rules entrypoint. The
bound below uses ordinary stable Rust supertraits and a blanket implementation;
it does not require each game to implement another adapter:

```rust
trait HeartsMode: ExecutionMode<HeartsState, HeartsView, Change>
    + for<'a> ResolveChoice<HeartsState, PassCards<'a>>
    + for<'a> ResolveChoice<HeartsState, PlayCard<'a>> {}
impl<M> HeartsMode for M where
    M: ExecutionMode<HeartsState, HeartsView, Change>
        + for<'a> ResolveChoice<HeartsState, PassCards<'a>>
        + for<'a> ResolveChoice<HeartsState, PlayCard<'a>> {}
```

The rules functions then use `M: HeartsMode`. For example, the card-playing
branch of `resolve` calls this nested function:

```rust
fn play_card<M: HeartsMode>(state: &mut HeartsState,
                           cx: &mut Executor<HeartsState, M>) {
    let choice = PlayCard::new().observation(state.observe(state.turn));
    let card = cx.choose(state, choice);
    state.play(card);
    cx.present(|| state.visible_to_south(), || Change::CardPlayed(card));
}
```

The choice structs borrow only the acting player's observation and options.
Their policy implementations take a choice, not the full state. For example:

```rust
impl ChoicePolicy<PlayCard<'_>> for HeartsPolicy {
    fn choose(&mut self, choice: &PlayCard<'_>) -> CardId {
        self.evaluate_legal_cards(choice.observation(), choice.options())
    }
}
```

Provide the corresponding `ChoicePolicy<PassCards<'_>>` implementation returning
three cards. `Simulation<HeartsPolicy>` implements the resolution bounds by
calling these concrete methods directly. Its resolution adapter receives state
for the shared method signature but does not pass it into the policy.

A caller runs the same rules entrypoint without an application or Unity:

```rust
let mut simulated = accepted.clone();
let policy = HeartsPolicy::new().seed(seed);
let mut cx = Executor::new().mode(Simulation::new().policy(policy));
resolve(&mut simulated, Action::PlayTurn, &mut cx);
let preview = simulated.visible_to_south();
display.preview(preview);
```

This shows the call shape Task 02 must prove with a small two-choice fixture;
Hearts implements its policy in task 40. `display.preview` renders an
independent view and never installs simulated state as the accepted live state.

Interactive mode may erase prompt and callback types at its thread-message
boundary and recover them through checked typed handles. This cost is absent
from the simulation specialization. Do not use a runtime registry or vtable for
each simulated choice. Invalid simulation-policy answers are developer errors;
invalid human answers produce feedback and keep the prompt waiting.

For Hearts, the policy sees only the acting player's cards, public history, and
inferred constraints. Construct that observation before calling the policy.
Never hand the policy the authoritative state containing opponents' real hands.
Game-owned candidate enumeration and observation construction have their own
costs; benchmark them separately from executor overhead.

## Prove the API before expanding it

Task 02 must include compiling examples for a choice-free game, a game with two
choice types, and the same nested rules body in simulation. The examples must
show ordinary application registration, not just isolated generic helpers. Task
10 adds actual typed UI answers through the worker.

Verify that simulation skips panicking display builders and performs no
mandatory allocation, dynamic dispatch, blocking, or interactive cancellation
checks in `present`, `choose`, and `check_cancelled`. A generic signature alone
is not proof: retain allocation measurements and optimized-code inspection.

Host protocol records are engine implementation details and are described with
worked examples in [presentation](presentation.md). Game authors never construct
those records when publishing a checkpoint or answering a prompt.

## Manual QA

Write the counter example and a card-selection example from this page. Check
that defaults work without a game trait, movement setup, prompt enum, or answer
enum. Run the same nested rules in simulation. Submit an illegal card through
the display and verify the player can correct it without losing the prompt.
