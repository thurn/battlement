# Rules API: one Game trait

A game exposes one coherent contract instead of registering state, view,
animation, validation, and rules callbacks separately. The `Game` trait gives
the engine enough information to run the same action interactively or in a
headless simulation.

Read this for tasks 02 and 09–11. Related pages: [application
composition](architecture.md), the complete [action execution
sequence](execution.md#from-dispatch-to-accepted-state), and [presentation
timing](presentation.md). These are proposed interfaces. Task 02 must compile
the trait, executor, and rules examples. Task 11 must compile the `App`
registration example.

## The game owns its associated types

The application registers a concrete game value. Its implementation names every
type that belongs to the game, so no type-only builder calls are necessary.

```rust
App::new()
    .game(HeartsGame::new(saved_or_new))
    .root(HeartsDisplay::new())
```

The engine infers `HeartsState`, `Action`, `HeartsView`, and `StateAnimation`
from `HeartsGame`. In particular, `StateAnimation` is not the action type:

- `Action` describes what the player asks the rules to do.
- `StateAnimation` describes how the display should present a published state.

For example, `Action::PlayCard(card)` enters the game once. That action may
publish several checkpoints with animations such as
`StateAnimation::CardPlayed(card)` and
`StateAnimation::TrickCollected(winner)`.

A suitable starting interface is:

```rust
trait Game: Sized + Send + 'static {
    type State: Send + 'static;
    type View: Send + 'static;
    type Action: Send + 'static;
    type StateAnimation: Send + 'static;
    type Prompt: Send + 'static;
    type ControllerPrompt: Send + 'static;
    type Answer: Send + 'static;
    type Decision<'a> where Self: 'a;

    fn into_state(self) -> Self::State;
    fn logical_clone(state: &Self::State) -> Self::State;
    fn view(state: &Self::State) -> Self::View;
    fn validate_action(state: &Self::State, action: &Self::Action)
        -> Result<(), ActionRejection> { Ok(()) }
    fn apply_action<M: ExecutionMode<Self>>(
        state: &mut Self::State, action: Self::Action,
        cx: &mut Executor<Self, M>,
    );
    fn final_state_animations(
        _state: &Self::State,
    ) -> Vec<Self::StateAnimation> { Vec::new() }
}
```

`Game` is implemented by game code, not supplied as a registration builder.
The implementation may use `Clone` inside `logical_clone`, but the trait also
supports persistent or otherwise custom state copies. A logical clone and every
returned view must not share mutable interior state with the accepted state.

Games without prompts use `()` for `Prompt`, `Answer`, and `Decision`. Games
without checkpoint-specific presentation use `()` for `StateAnimation`. No
builder call is needed to register any of those defaults.

## A small complete game

This counter shows how registration, rules, and display fit together. The game
value owns the initial state, while the associated state remains an ordinary
domain type.

```rust
struct CounterGame(Counter);

impl Game for CounterGame {
    type State = Counter;
    type View = Counter;
    type Action = Add;
    type StateAnimation = ();
    type Prompt = ();
    type ControllerPrompt = ();
    type Answer = ();
    type Decision<'a> = ();

    fn into_state(self) -> Counter { self.0 }
    fn logical_clone(state: &Counter) -> Counter { state.clone() }
    fn view(state: &Counter) -> Counter { state.clone() }

    fn apply_action<M: ExecutionMode<Self>>(
        state: &mut Counter, action: Add, cx: &mut Executor<Self, M>,
    ) {
        state.total += action.amount;
        cx.present(state, || ());
    }
}
```

`cx.present` receives the current state by immutable borrow. Interactive mode
calls `CounterGame::view` and retains the resulting immutable value. Simulation
mode calls neither the view method nor the lazy animation builder.

The display reads the associated view and dispatches the associated action:

```rust
fn counter_display() -> impl Render {
    let view = use_game_view::<CounterGame>();
    let actions = use_game_actions::<CounterGame>();
    (
        Label::new().text(view.total.to_string()),
        Button::new().text("Add one")
            .on_click(move |_| actions.dispatch(Add { amount: 1 })),
    )
}
```

The action handle returns `Started`, `Busy`, or `Invalid`. It never mutates the
presented view directly. A new value appears only when the worker publishes and
the display commits a checkpoint.

## Hearts separates rules state from the visible view

`HeartsState` contains every player's cards, so display components must never
receive it. `HeartsGame::view` returns a `HeartsView` containing only
information visible to the south player.

```rust
fn view(state: &HeartsState) -> HeartsView {
    HeartsView {
        south_hand: state.hand(SOUTH).clone(),
        hand_sizes: state.hand_sizes(),
        trick: state.current_trick.clone(),
        scores: state.scores,
    }
}
```

`HeartsDisplay` reads the complete `HeartsView`. A `CardView` is an individual
component rendered from one visible card within that game-wide view.

The trait method contains the game's action routing:

```rust
fn apply_action<M: ExecutionMode<Self>>(
    state: &mut HeartsState, action: Action,
    cx: &mut Executor<Self, M>,
) {
    match action {
        Action::PlayTurn => play_turn(state, cx),
        Action::PassCards(cards) => pass_cards(state, cards, cx),
    }
}
```

Private helper functions may keep the rules readable. There is no separately
registered rules callback.

## Publish state animations with views

A **checkpoint** contains an immutable `Game::View` and an ordered collection of
`Game::StateAnimation` values. The view says what is visible now. Each state
animation says how the display should explain the transition to that view.

```rust
state.play(card);
cx.present(state, || StateAnimation::CardPlayed(card));

state.collect_trick();
cx.present_many(state, || [
    StateAnimation::TrickCollected(winner),
    StateAnimation::ScoreChanged(winner),
]);
```

The animation builders remain lazy so simulation does not construct display
data. Single-animation publication assigns index zero. Group indices follow
iteration order. The engine publishes a final view automatically when
`apply_action` returns. Its animation list comes from
`Game::final_state_animations`, whose default is empty.

`StateAnimation` is semantic animation input, not an imperative Unity command.
Presentation code decides which motion, sound, particle, or gate corresponds to
each variant.

## Choices remain typed

A **choice specification** describes one decision and validates its answer.
Selecting one card can return `CardId`; selecting three cards can return a
three-card collection. The game-wide `Prompt`, `Answer`, and `Decision` enums
provide the worker boundary, while each specification exposes its concrete
answer type to rules code.

```rust
trait ChoiceSpec<G: Game> {
    type Output;
    fn prompt(&self, state: &G::State) -> G::Prompt;
    fn controller_prompt(
        &self, state: &G::State,
    ) -> Option<G::ControllerPrompt> { None }
    fn decision<'a>(&'a self, state: &'a G::State) -> G::Decision<'a>;
    fn validate(
        &self, state: &G::State, answer: G::Answer,
    ) -> Result<Self::Output, ChoiceRejection>;
}
```

`Game::Prompt` contains public display data only. The engine wraps it in a
`PresentedPrompt<G>` containing an `AnswerHandle<G::Answer>` bound to the
current run and request. Game code therefore does not manufacture or store
engine handles inside its prompt enum.

```rust
struct PresentedPrompt<G: Game> {
    value: G::Prompt,
    answer: AnswerHandle<G::Answer>,
}
```

`Game::ControllerPrompt` is a separate owned, controller-only message. A choice
returns one when an application-owned AI job needs private observation data.
The application controller may receive it; components and `Game::View` may not.
Simulation uses the borrowed `Game::Decision` directly and never constructs
either owned prompt.

`ChoicePolicy<G>` consumes `G::Decision<'_>` and returns `G::Answer`.
Interactive execution validates the answer submitted through the presented
prompt; simulation obtains an answer from the concrete policy and runs the same
validation. Wrong policy output is a developer error. Wrong player input leaves
the interactive prompt open with public feedback.

```rust
let legal = state.legal_cards(player);
let card = cx.choose(state, SelectOne::new().options(&legal));
state.play(card);
```

Each specification must:

- Borrow legal options and observation data while the synchronous call runs.
- Build an owned `Game::Prompt` only for interactive display.
- Convert a `Game::Answer` into its concrete answer type and validate it.
- Return invalid-answer feedback without consuming the pending request.
- Prevent the policy from observing hidden authoritative state.

The executor calls `Game::view` when an interactive choice needs a presented
view. It never retains a hidden pointer to mutable worker state. The rules
function cannot mutate that state while `choose` is waiting.

The display reads the engine-owned wrapper around the game-wide prompt enum:

```rust
let presented = use_game_prompt::<HeartsGame>();
if let Prompt::PlayCard { legal } = &presented.value {
    let answer = presented.answer.clone();
    Button::new().text("Play")
        .enabled(legal.contains(&selected))
        .on_click(move |_| answer.submit(Answer::PlayCard(selected)))
}
```

The handle accepts the `Game::Answer` type for `HeartsGame`, so unrelated Rust
types cannot be submitted. Transport decoding and the retained choice
specification reject the wrong answer variant, stale handles, and illegal
values. A private controller prompt carries a clone of this same answer handle
through its controller-only envelope.

## Simulation uses the same Game implementation

Simulation invokes `Game::apply_action` directly on an independent state value.
It does not start an interactive worker unless its caller schedules the whole
simulation on one.

```rust
let mut simulated = HeartsGame::logical_clone(accepted);
let policy = HeartsPolicy::new().seed(seed);
let mut cx = Executor::simulation(policy);
HeartsGame::apply_action(&mut simulated, Action::PlayTurn, &mut cx);
display.preview(HeartsGame::view(&simulated));
```

The simulation specialization has these requirements:

- `present` builds neither a view nor a `StateAnimation`.
- `choose` calls the concrete policy inline.
- `check_cancelled` is an inline no-op.
- No primitive requires allocation, dynamic dispatch, or blocking.

`display.preview` renders an independent view and never installs simulated state
as the accepted live state. Hearts policies receive only the acting player's
observation, public history, and inferred constraints—not opponents' real
hands.

## Prove the API before expanding it

Task 02 must include compiling examples for a choice-free game, a game with two
choice types, and the same nested rules body in simulation. The examples must
show application registration, associated-type inference, and direct simulation
through the same `Game` implementation.

Verify that simulation skips panicking view and animation builders and performs
no mandatory allocation, dynamic dispatch, blocking, or interactive
cancellation checks in `present`, `choose`, and `check_cancelled`. Retain
allocation measurements and optimized-code inspection rather than inferring
zero overhead from generic signatures.

Host protocol records are engine implementation details described in
[presentation](presentation.md). Game authors never construct those records
when publishing checkpoints or answering prompts.

## Manual QA

Implement the counter and card-selection examples from this page. Confirm that
the application infers the action, view, and state-animation types from the
registered game. Run the same nested rules in simulation. Submit an illegal card
through the display and verify the player can correct it without losing the
prompt.
