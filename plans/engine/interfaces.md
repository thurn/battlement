# Rules and session API

A game supplies ordinary state, synchronous rules, and a domain-specific
context. Reactant supplies the live session, a connection to the display, and
typed response delivery. The same rules run directly with a simulation context.

The [compiling contract
sketch](../../crates/battlement-reactant/src/proposal.rs) defines the complete
rules/session surface. Its private storage and method bodies are implementation
placeholders, not permission to omit public API. This page explains the
contract; [execution](execution.md) defines ordering and failure behavior. See
also [architecture](architecture.md), [presentation](presentation.md), and
[Hearts](hearts.md).

## The game owns its associated types

There is no separate display-data type. A **snapshot** is an independent logical
clone of `Game::State`, held immutable while components read it. A
`StateAnimation` describes an event such as collecting a trick; display code
chooses its movements, sounds, and effects.

```rust
trait Game: Sized + Send + 'static {
    type State: Send + 'static;
    type Action: Send + 'static;
    type StateAnimation: Send + 'static;
    type Prompt: Clone + Send + 'static;
    type Context: GameContext<Self> + Send + 'static;

    fn logical_clone(state: &Self::State) -> Self::State;
    fn is_legal_action(state: &Self::State, action: &Self::Action) -> bool;
    fn execute(
        context: &mut Self::Context,
        state: &mut Self::State,
        action: Self::Action,
    );
}
```

- `Action` starts rules execution. Answering an already active prompt does not
  dispatch another action.
- `Prompt` is one game-wide enum whose variants contain concrete prompt data.
  Display and simulation policies inspect the same enum.
- `Context` belongs to the game. It may hold an RNG, policies, and other
  domain-specific data alongside an interactive connection or simulation mode.
- `logical_clone` must not share mutable data with its source. Immutable shared
  data may use `Arc`. The engine does not deep-copy arbitrary Rust values.
- A choice-free game uses `Prompt = ()`. A game without semantic animation
  events uses `StateAnimation = ()`. There are no extra associated answer,
  decision, controller-message, or view types to register.

The context lives for the session and moves to the active worker for execution;
only one action borrows it at a time. Normal return makes it available for the
next action after acceptance. A failed or stopped run cannot reuse interrupted
context; restarting constructs a fresh context. State required for deterministic
restoration must be included in the game's saved data or supplied explicitly
when constructing that context.

## Start and replace a session

`App` owns session attachment. Starting a game also supplies the matching
connection to the game's context factory; there is no separate attachment call.

```rust
impl App {
    fn start_game<G: Game>(
        &mut self,
        initial_state: G::State,
        make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
    ) -> GameHandle<G>;
}
```

```rust
let game = app.start_game::<HeartsGame>(initial_state, |connection| {
    HeartsContext {
        human_player,
        mode: HeartsMode::Interactive { connection, policy: HeartsPolicy },
    }
});
```

The factory runs once on the app thread. Its returned context is `Send` and is
owned by the session. The initial state is accepted immediately and a clone is
queued for entry presentation. The session remains `Busy` until that initial
presentation finishes. Starting alone does not call `Game::execute`.

Calling `start_game` again stops the old session and attaches the new one to the
app's display and hooks. Existing old handles stay tied to the stopped session.
UI-only apps need no game session. Dropping an app stops its attached session;
dropping one cloned handle does not stop a session still owned by the app.

## Dispatch, stop, inspect, and save explicitly

`GameHandle<G>` is cloneable. Clones share one session without cloning its
state. The handle and component hooks are used on the app/display thread.

```rust
impl<G: Game> GameHandle<G> {
    fn dispatch(&self, action: G::Action) -> DispatchResult;
    fn accepted_state(&self) -> G::State;
    fn stop(&self);
    fn status(&self) -> GameStatus;
}

enum DispatchResult { Started, Busy }
enum GameStatus { Ready, Busy, Failed, Stopped }
```

- Dispatch returns `Busy` first if entry presentation, an action, a prompt, or
  final presentation is unfinished. It does not queue another action or run its
  validator in that case.
- Otherwise `is_legal_action` runs against accepted state. False is a
  programming error and panics before cloning state or starting a worker.
- `Started` means execution was scheduled, not completed. Session/run identities
  remain internal; `Started` carries no run ID.
- Dispatching to a `Failed` or `Stopped` session is a programming error.
- `accepted_state()` always returns a logical clone of the last accepted state,
  including while busy, failed, or stopped. It never returns in-progress rules
  state. Explicit saving can therefore save the previous completed action while
  a new action is being displayed.
- `stop()` is idempotent and immediately sets `Stopped`. It invalidates pending
  output and wakes waits without joining ordinary computation. It preserves
  accepted state for existing handles.
- Active worker or required-animation failure sets `Failed`, retains accepted
  state, and reports diagnostic details. The UI provides restart/exit controls.

There is no acceptance callback or autosave in v1. The application owns explicit
save/load operations using `accepted_state()` and `App::start_game()`.

## Publish and choose through the game context

Rules call this small interface regardless of the context's mode:

```rust
trait GameContext<G: Game> {
    fn present(
        &mut self,
        state: &G::State,
        animation: impl FnOnce() -> G::StateAnimation,
    );
    fn choose<P: PromptData<G::Prompt>>(
        &mut self, state: &G::State, prompt: P,
    ) -> P::ResponseType;
}
```

```rust
state.play(card);
context.present(state, || HeartsAnimation::CardPlayed(card));
let next = context.choose(state, PlayCardPrompt { choices: legal_cards });
```

Interactive `present` queues a state snapshot and one semantic animation event.
One event can describe several related changes; display code can start many
movements/effects for it. There is no grouped-publication method or final-event
callback. Normal return automatically publishes a final snapshot with no
semantic event. Default movement still applies to changes in that snapshot.

Interactive `choose` publishes a snapshot and owned prompt together, even when
rules did not call `present` first. Simulation creates no snapshots and invokes
no animation builder. Its `choose` calls the configured policy inline.

Cancellation is internal to interactive publication, choice, and action-return
boundaries. There is no explicit cancellation-check method. Ordinary rules or
policy computation runs until it reaches a boundary; abandoning a session makes
its late output harmless immediately.

## Choices remain typed

Prompt data owns the set of legal choices or the data needed to enumerate them.
The iterator can be lazy. Its order must remain stable for that prompt.

```rust
trait PromptData<T>: Sized + Send + 'static {
    type ResponseType: Send + 'static;
    fn options(&self) -> impl Iterator<Item = Self::ResponseType>;
    fn is_valid_response(&self, response: &Self::ResponseType) -> bool;
    fn into_prompt(self) -> T;
    fn from_prompt(prompt: &T) -> &Self;
}
```

```rust
#[derive(Clone)]
struct PlayCardPrompt { choices: Vec<CardId> }
#[derive(Clone)]
struct PassCardsPrompt { choices: Vec<[CardId; 3]> }
#[derive(Clone)]
enum HeartsPrompt {
    PlayCard(PlayCardPrompt),
    PassCards(PassCardsPrompt),
}
```

Each struct defines its fields and legality logic once. `into_prompt` moves it
into the enum without copying choices. `from_prompt` borrows that same data and
panics for a mismatched variant. The iterator yields owned response values.

Rules still receive a concrete answer:

```rust
let card: CardId = context.choose(state, PlayCardPrompt { choices });
```

No borrowed prompt or `Cow` conversion is required. Constructing a vector may
allocate once per constructed prompt; borrowing the resulting prompt for policy
evaluation does not copy it again. Simulations that construct new prompts pay
those construction costs, which must be measured rather than hidden.

## One prompt enum for display and policies

Interactive presentation wraps the enum and a response connection:

```rust
struct PresentedPrompt<T> {
    pub prompt: T,
    pub handle: ResponseHandle<T>,
}
impl<T> ResponseHandle<T> {
    fn submit<P: PromptData<T>>(&self, prompt: &P, response: P::ResponseType);
}
```

```rust
if let Some(presented) = use_game_prompt::<HeartsGame>() {
    if let HeartsPrompt::PlayCard(prompt) = &presented.prompt {
        presented.handle.submit(prompt, selected_card);
    }
}
```

The concrete prompt argument makes Rust require `CardId` in that branch. Passing
three cards there is a compile error. Runtime checks still bind the handle to
its exact session/run/request, verify the concrete prompt and response types,
and validate against the stored prompt. The supplied value determines the type;
it does not prove object identity. This also supports zero-sized prompt structs.
A caller-constructed prompt cannot broaden the stored request's legal choices. The UI may retain the
`Rc<PresentedPrompt<T>>` in its handler and match it again when submitting; it
need not clone the prompt data.

An invalid response to an active, human-owned request is a programming error and
panics. The UI uses the same validation logic to disable illegal choices.
Ended-request replies, including duplicate replies after resolution, are
ignored. An AI-owned request cannot be answered by a human response handle. A
valid human response resumes the waiting rules exactly once.

## Simulation and live AI use the same policy interface

Policies can inspect concrete prompt variants and state for tree statistics,
random choice, or fast rollout heuristics. They return an option index:

```rust
trait ChoicePolicy<G: Game> {
    fn choose(&mut self, state: &G::State, prompt: &G::Prompt) -> usize;
}
```

The context recovers the concrete prompt through `P::from_prompt`, selects the
indexed item from `options()`, validates it, and returns `P::ResponseType`.
Out-of-range indices or invalid selected responses panic. No answer enum is
needed. Stable option ordering is part of the prompt contract, not a policy
restriction on which heuristics it can use.

```rust
let mut state = HeartsGame::logical_clone(&accepted);
let mut context = HeartsContext {
    human_player,
    mode: HeartsMode::Simulation(rollout_policy),
};
HeartsGame::execute(&mut context, &mut state, action);
```

This call runs synchronously on the caller's thread and does not start or attach
a live session. Simulation can branch on the context mode; the API does not
require compile-time execution-mode specialization. Primitive dispatch must not
require a heap allocation, vtable, display connection, wait, or snapshot. Owned
prompt construction and game policy work have separate measured costs.

The policy receives `&G::State`. Hidden-state randomization and information-safe
heuristics belong to the game, not an engine observation/controller subsystem.
Hearts must still avoid using opponents' real hidden cards when choosing a move.

## The interactive connection

Reactant creates `DisplayConnection<G>` for the session; game contexts delegate
interactive mechanics to it. Its fields and constructor are private.

```rust
impl<G: Game> DisplayConnection<G> {
    fn present(&mut self, state: &G::State,
        animation: impl FnOnce() -> G::StateAnimation);
    fn choose<P: PromptData<G::Prompt>>(
        &mut self, state: &G::State, prompt: G::Prompt,
    ) -> P::ResponseType;
    fn choose_with_policy<P: PromptData<G::Prompt>>(
        &mut self, state: &G::State, prompt: G::Prompt,
        policy: &mut impl ChoicePolicy<G>,
    ) -> P::ResponseType;
}
```

In interactive Hearts, the context routes a human decision to `choose` and an AI
decision to `choose_with_policy`. Both publish the snapshot/prompt in order. The
latter clones the owned enum for display and retains the original for the policy
on the rules worker. This is why `Game::Prompt` requires `Clone`; no `Sync`
bound is needed. Human publication moves the prompt and simulation makes no
extra display copy. The policy runs only after presentation, without waiting for
human input. It validates the resulting option index and checks cancellation
before returning. The connection does not identify players.

A session owns one connection and one context. Helpers reject use outside that
session's current action rather than allowing unrelated publication. The app
owns normal-return publication; contexts do not expose or call a finish helper.

## Component access

Hooks refer to the session attached to their app. They panic when no matching
game is attached; `use_game_prompt` returns `None` when that game has no active
prompt. Replacement switches subscriptions atomically with the displayed state.

```rust
fn use_game_state<G: Game>() -> Rc<G::State>;
fn use_game_prompt<G: Game>() -> Option<Rc<PresentedPrompt<G::Prompt>>>;
fn use_game_status<G: Game>() -> GameStatus;
fn use_game_selector<G: Game, T: Clone + PartialEq + 'static>(
    selector: impl Fn(&G::State) -> T + 'static,
) -> T;
```

Selectors compare outputs between displayed snapshots. Equal results do not
trigger a component render through that subscription; selectors themselves may
still run to compute equality. Other props, hooks, and context can still render
the component. Props remain a complete way to pass selected data to children.

Pass a cloned `GameHandle<G>` through ordinary props/context when a component
needs to dispatch. There is no additional game-action hook. Status subscription
can drive next-action scheduling and failure controls; it is not autosave.

## Manual QA

Start a new state and a loaded state through the context factory. Dispatch while
busy, inject an illegal action, stop during a choice, and restart using the same
app. Verify old handles remain stopped and the accepted-state copy stays stable.
Run two typed choices through human and policy paths, deliberately misuse an
active reply in a fault fixture, and deliver a stale reply after replacement.
Compare simulation outcomes and confirm its snapshot/animation builders never
run. Exercise selectors, status-driven recovery, and an explicit save/load.
