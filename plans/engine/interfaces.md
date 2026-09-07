# Rules and session API

A game supplies ordinary state, synchronous rules, and a domain-specific
context. Reactant supplies the live session, a connection to the display, and
typed response delivery. The same rules run directly with a simulation context.

The [complete contract sketch](#complete-contract-sketch) below defines the
complete rules/session surface. Its private storage and method bodies are
implementation placeholders, not permission to omit public API. This page
explains the contract; [execution](execution.md) defines ordering and failure
behavior. See also [architecture](architecture.md),
[presentation](presentation.md), and [Hearts](hearts.md).

## The game owns its associated types

There is no separate display-data type. A **snapshot** is an independent logical
clone of `Game::State`, held immutable while components read it. A
`StateAnimation` describes an event such as collecting a trick; display code
chooses its movements, sounds, and effects.

- `Action` starts rules execution. Answering an already active prompt does not
  dispatch another action.
- `Prompt` is one game-wide enum whose variants contain concrete prompt data.
  Display and simulation policies inspect the same enum.
- `Context` belongs to the game. It may hold an RNG, policies, and other
  domain-specific data alongside an interactive connection or simulation mode.
- `logical_clone` must not share mutable data with its source. Immutable shared
  data may use `Arc`. The engine does not deep-copy arbitrary Rust values.
- A choice-free game uses `Prompt<'a> = ()`. A game without semantic animation
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
state. The handle and component hooks are used on the app/display thread. AI
policies receive state and a prompt, never a handle. A dispatched action may
include a complete AI turn with many choices; each choice invokes the policy
inside `execute()`, rather than returning to UI dispatch between card plays.

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

Rules call `GameContext` regardless of the context's mode:

```rust
state.play(card);
context.present(state, || HeartsAnimation::CardPlayed(card));
let next = context.choose(state, PlayCardPrompt { choices: legal_cards });
```

Interactive `present` queues a state snapshot and one semantic animation event.
The fixed FIFO holds 32 pending snapshots, including builders and native
preparation but excluding the displayed snapshot. Reserve capacity before
cloning/building; wait only when all 32 slots are occupied. Otherwise return
after enqueueing without waiting for animation. Commit releases a slot; merely
starting preparation does not. For example, one UI-dispatched EndTurn action can
run five successive AI searches and card plays while the display is held, as
long as their combined prompt/present/final entries fit in the queue. The 33rd
pending entry waits for a slot. Simulation has no queue and never waits.

Normal action completion still waits for final presentation before another UI
dispatch. Keep consecutive AI plays within one execution so this UI boundary
does not gate each search. One event can describe several related changes;
display code can start many movements/effects for it. There is no
grouped-publication method or final-event callback. Normal return automatically
publishes a final snapshot with no semantic event. Default movement still
applies to changes in that snapshot.

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

Each struct defines its fields and legality logic once. `as_prompt` wraps a
reference with `Cow::Borrowed`; `into_prompt` moves the data into `Cow::Owned`.
Both wrap a known concrete type and are infallible. There is no reverse
conversion and no separate reference enum. The iterator yields owned responses.

`PromptData` names the game so its methods can use the lifetime-parameterized
`Game::Prompt<'a>`. For Hearts this is `HeartsPrompt<'a>`; display stores
`HeartsPrompt<'static>`. The lifetime belongs to the wrapper, not to the vector
of legal choices. Concrete prompt data still owns that vector.

The two forms must describe identical options, in identical order. Cloning
concrete prompt data preserves those choices and their validation rules. Prompt
data is immutable for the request lifetime, including through shared interior
state. `Sync` allows an internal typed request to share it safely with the
response validator; it also makes the borrowed arm of the `Cow` enum `Send`.
Game state and the rest of the context do not gain a `Sync` bound.

Rules still receive a concrete answer:

```rust
let card: CardId = context.choose(state, PlayCardPrompt { choices });
```

Constructing a vector may allocate once per constructed prompt. Simulation wraps
the resulting prompt in `Cow::Borrowed` without cloning or allocating; it never
needs `into_owned`. Interactive requests clone concrete prompt data once for the
owned display enum while retaining the typed original for validation and policy
evaluation. Measure these live costs separately from simulation primitive
overhead.

## One prompt enum for display and policies

Interactive presentation uses `PresentedPrompt<T>` to wrap the enum and its
`ResponseHandle<T>`:

```rust
if let Some(presented) = use_game_prompt::<HeartsGame>() {
    if let HeartsPrompt::PlayCard(prompt) = &presented.prompt {
        presented.handle.submit(prompt.as_ref(), selected_card);
    }
}
```

The concrete prompt argument makes Rust require `CardId` in that branch. Passing
three cards there is a compile error. Runtime checks still bind the handle to
its exact session/run/request, verify the concrete prompt and response types,
and validate against the stored prompt. The supplied value determines the type;
it does not prove object identity. This also supports zero-sized prompt structs.
A caller-constructed prompt cannot broaden the stored request's legal choices.
The UI may retain the `Rc<PresentedPrompt<T>>` in its handler and match it again
when submitting; it need not clone the prompt data.

An invalid response to an active, human-owned request is a programming error and
panics. The UI uses the same validation logic to disable illegal choices.
Ended-request replies, including duplicate replies after resolution, are
ignored. An AI-owned request cannot be answered by a human response handle. A
valid human response resumes the waiting rules exactly once.

## Simulation and live AI use the same policy interface

Policies can inspect concrete prompt variants and state for tree statistics,
random choice, or fast rollout heuristics. They return an option index.

The context retains the concrete prompt and borrows an enum wrapper for the
policy. It then selects the indexed item directly from the original typed
prompt's `options()`, validates it, and returns `P::ResponseType`:

```rust
let index = policy.choose(state, &prompt.as_prompt());
let response = select_response::<G, P>(&prompt, index);
```

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

In interactive Hearts, the context routes a human decision to `choose` and an AI
decision to `choose_with_policy`. Both receive the concrete `P`, retain it in an
internal typed request, and publish `prompt.clone().into_prompt()` with the
snapshot in display order. Reserve publication capacity before cloning. This one
copy is required for human requests as well as live AI: the display owns an enum
while the request retains `P` for validation without reverse conversion.
Internal `Arc<P>` storage lets the display validate against the original and
lets the live worker borrow it for policy evaluation. `Clone + Send + Sync`
apply to concrete prompt data; the request and its erased response transport are
private engine details.

The policy borrows `prompt.as_prompt()` immediately after enqueueing its prompt,
without waiting for animation, display readiness, or human input. Full-queue
backpressure can delay enqueueing, but visibility is not an AI prerequisite.
Select its response directly from retained `P`, then check cancellation before
returning. The connection does not identify players. Simulation allocates no
request and makes no display copy.

A session owns one connection and one context. Helpers reject use outside that
session's current action rather than allowing unrelated publication. The app
owns normal-return publication; contexts do not expose or call a finish helper.

## Component access

Hooks refer to the session attached to their app. They panic when no matching
game is attached. `use_game_prompt` exposes the displayed snapshot's prompt, not
a newer queued request. Human prompts close after their response. Resolved AI
prompts may appear informationally until their next checkpoint; their ended
handles cannot resume rules. Return `None` when no prompt is displayed.
Replacement switches subscriptions atomically with the displayed state.

Selectors compare outputs between displayed snapshots. Equal results do not
trigger a component render through that subscription; selectors themselves may
still run to compute equality. Other props, hooks, and context can still render
the component. Props remain a complete way to pass selected data to children.

Pass a cloned `GameHandle<G>` through ordinary props/context when a component
needs to dispatch. There is no additional game-action hook. Status subscription
can drive next-action scheduling and failure controls; it is not autosave.

## Complete contract sketch

This single Rust block contains all rules/session declarations and the connected
Hearts examples. It can be extracted and compiled as a standalone Rust 2024
library. Private storage and engine bodies are placeholders; the Hearts state,
action, and policy bodies illustrate API wiring rather than complete game rules.
The proposed additions to `App` are shown without its existing UI API.

```rust
use std::borrow::Cow;
use std::marker::PhantomData;
use std::rc::Rc;

trait GameContext<G: Game> {
  // Interactive publication pairs an independent logical_clone(state) with the
  // animation. Simulation calls neither logical_clone nor the animation builder.
  fn present(&mut self, state: &G::State, animation: impl FnOnce() -> G::StateAnimation);
  fn choose<P>(&mut self, state: &G::State, prompt: P) -> P::ResponseType
  where
    P: PromptData<G>;
}

trait Game: Sized + Send + 'static {
  type State: Send + 'static;
  type Action: Send + 'static;
  type StateAnimation: Send + 'static;
  type Prompt<'a>: Send + 'a;
  type Context: GameContext<Self> + Send + 'static;

  // Copies must not share mutable data with the source. Used for the worker's
  // private state and for snapshots read by display components.
  fn logical_clone(state: &Self::State) -> Self::State;

  fn is_legal_action(state: &Self::State, action: &Self::Action) -> bool;

  // Interactive execution runs on a private worker state. Normal return queues
  // a final snapshot automatically. Accept that state for saving and another
  // gameplay action only after the display finishes its final presentation
  // and a qualifying rendered frame. Normal return reuses the context for the
  // next action; failure or stop discards it. Restart constructs a fresh context.
  // Cancellation takes effect at present, choose, or return. There is no
  // explicit cancellation-check API; ordinary computation runs to a boundary.
  fn execute(e: &mut Self::Context, state: &mut Self::State, action: Self::Action);
}

trait ChoicePolicy<G: Game> {
  // Returns an index into the prompt's stable option order. Policies may inspect
  // the concrete prompt variant and state for tree search or rollout heuristics.
  // Hidden-state randomization is the game's responsibility, outside Reactant.
  fn choose(&mut self, state: &G::State, prompt: &G::Prompt<'_>) -> usize;
}

// Existing Reactant App; only its game-session entry point is shown here.
struct App;

impl App {
  fn start_game<G: Game>(
    &mut self,
    initial_state: G::State,
    make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
  ) -> GameHandle<G> {
    // Stop the previously attached session, if any. Create the new connection,
    // construct the context, and attach the session to this app's display and
    // hooks internally. Accept initial_state and publish a logical clone.
    // Starting does not execute an action. New and loaded states use this API.
    todo!()
  }
}

struct GameHandle<G: Game> {
  session: Rc<GameSession<G>>,
}

// Private session storage and synchronization are implementation placeholders.
struct GameSession<G: Game> {
  game_type: PhantomData<G>,
}

impl<G: Game> Clone for GameHandle<G> {
  fn clone(&self) -> Self {
    Self {
      session: Rc::clone(&self.session),
    }
  }
}

impl<G: Game> GameHandle<G> {
  fn dispatch(&self, action: G::Action) -> DispatchResult {
    // Busy includes entry presentation, execution, prompt waits, and final presentation.
    // Return Busy first, without queueing the action or running its validator.
    // Otherwise panic if is_legal_action returns false. Clone accepted state,
    // schedule execute with the session's context, and return Started without
    // waiting. Dispatch to a Failed or Stopped session is a programming error.
    todo!()
  }

  fn accepted_state(&self) -> G::State {
    // Return logical_clone of the last fully completed state, including while
    // busy, failed, or stopped. Applications can save this owned copy explicitly.
    todo!()
  }

  fn stop(&self) {
    // Idempotently mark the session Stopped, discard pending display work, and
    // wake blocked rules. Old replies and results cannot affect a replacement.
    // Do not wait for ordinary computation to reach its next execution boundary.
    // Retain accepted state for explicit saving through existing handles.
    todo!()
  }

  fn status(&self) -> GameStatus {
    // Rules or required-presentation failure discards unfinished state, retains
    // accepted state, and sets Failed. Diagnostics receive the error details;
    // display code can show restart/exit controls. Expected cancellation is not
    // a failure. Busy ends only after final presentation has completed.
    todo!()
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchResult {
  Started,
  Busy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameStatus {
  Ready,
  Busy,
  Failed,
  Stopped,
}

// Hooks require a matching session attached to their app and panic otherwise.
// Replacement switches their session. Components receive GameHandle via props
// or context for dispatch; there is no extra action hook or autosave callback.
// Component hooks read the displayed snapshot, never the mutable worker state.
// Rc shares the received snapshot on the display thread without cloning state.
fn use_game_state<G: Game>() -> Rc<G::State> {
  todo!()
}

fn use_game_prompt<G: Game>() -> Option<Rc<PresentedPrompt<G::Prompt<'static>>>> {
  // State and prompt become visible together. Human prompts close after reply.
  // Already-resolved AI prompts can appear informationally in queued display;
  // their ended handles cannot resume rules. The next checkpoint replaces them.
  todo!()
}

fn use_game_status<G: Game>() -> GameStatus {
  // Subscribe to status changes for the session attached to this app.
  todo!()
}

fn use_game_selector<G: Game, T: Clone + PartialEq + 'static>(
  selector: impl Fn(&G::State) -> T + 'static,
) -> T {
  // Evaluate against displayed snapshots. This subscription triggers a render
  // only when the selected value changes; other component inputs may also render.
  todo!()
}

struct HeartsGame;

struct HeartsContext {
  mode: HeartsMode,
  human_player: PlayerId,
  // Other domain-specific data, such as the game's RNG, can live here.
}

fn start_hearts(
  app: &mut App,
  initial_state: HeartsState,
  human_player: PlayerId,
) -> GameHandle<HeartsGame> {
  app.start_game::<HeartsGame>(initial_state, |connection| HeartsContext {
    human_player,
    mode: HeartsMode::Interactive {
      connection,
      policy: HeartsPolicy,
    },
  })
}

enum HeartsMode {
  Interactive {
    connection: DisplayConnection<HeartsGame>,
    policy: HeartsPolicy,
  },
  Simulation(HeartsPolicy),
}

// Placeholder for Reactant's worker/display connection.
struct DisplayConnection<G: Game> {
  game_type: PhantomData<G>,
}

impl<G: Game> DisplayConnection<G> {
  fn present(&mut self, state: &G::State, animation: impl FnOnce() -> G::StateAnimation) {
    // Reserve capacity before cloning state and constructing the animation.
    // Queue them together in the session's 32-slot pending FIFO. The displayed
    // snapshot is excluded; reserved builders and native preparation count.
    // Wait only if all 32 slots are occupied, otherwise return after enqueueing.
    // Commit releases a slot. Never drop or coalesce checkpoints.
    // Display code sequences entries and determines required animation completion.
    // Cancellation wakes blocked publication and unwinds inside the Rust worker.
    todo!()
  }

  fn choose<P: PromptData<G>>(&mut self, state: &G::State, prompt: P) -> P::ResponseType {
    // Queue a state snapshot and PresentedPrompt together, after earlier entries.
    // Enable its response handle only when its snapshot is displayed. Wait on
    // this worker while menus, hovering, and inspection remain responsive.
    // Keep P in an internal Arc-backed typed request. After reserving capacity,
    // publish prompt.clone().into_prompt(); the retained P validates replies.
    // The enum owns its copy, so there is no self-reference or reverse conversion.
    // Invalid current replies panic; ended-request replies are ignored.
    // Cancellation unwinds without a response and discards the working state.
    todo!()
  }

  fn choose_with_policy<P: PromptData<G>>(
    &mut self,
    state: &G::State,
    prompt: P,
    policy: &mut impl ChoicePolicy<G>,
  ) -> P::ResponseType {
    // Enqueue the snapshot and prompt in display order, then ask the policy on
    // this worker immediately. Do not wait for prompt visibility or animation. Retain the typed P in the request as for human input;
    // clone P into the owned display enum and borrow P.as_prompt() for the policy.
    // P is Sync so the typed validator can be shared with the display thread.
    // The AI owns this request; human replies cannot resolve it. Its display
    // snapshot may arrive after computation has already resolved the request.
    // Simulation does not make this display copy or allocate a request.
    // Convert the selected index with select_response. Check cancellation before
    // returning. The game chooses this route; Reactant does not identify players
    // or randomize hidden state for the policy.
    todo!()
  }
}

struct HeartsPolicy;

impl ChoicePolicy<HeartsGame> for HeartsPolicy {
  fn choose(&mut self, state: &HeartsState, prompt: &HeartsPrompt<'_>) -> usize {
    match prompt {
      HeartsPrompt::PlayCard(prompt) => {
        // Inspect state and prompt.choices; return the selected option's index.
        todo!()
      }
      HeartsPrompt::PassCards(prompt) => {
        // The same policy can apply a passing heuristic to these choices.
        todo!()
      }
    }
  }
}

#[derive(Clone)]
struct HeartsState {
  current_player: PlayerId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PlayerId(u8);

struct HeartsAction;

struct HeartsAnimation;

impl Game for HeartsGame {
  type State = HeartsState;
  type Action = HeartsAction;
  type StateAnimation = HeartsAnimation;
  type Prompt<'a> = HeartsPrompt<'a>;
  type Context = HeartsContext;

  fn logical_clone(state: &HeartsState) -> HeartsState {
    state.clone()
  }

  fn is_legal_action(state: &HeartsState, action: &HeartsAction) -> bool {
    true
  }

  fn execute(e: &mut HeartsContext, state: &mut HeartsState, action: HeartsAction) {}
}

trait PromptData<G: Game>: Clone + Send + Sync + 'static {
  type ResponseType: Send + 'static;

  // Own the data needed to enumerate legal responses. Enumeration may be lazy,
  // but repeated calls must preserve the option order while the prompt exists.
  // Clones and both enum wrappers preserve these choices and their order.
  // Request data stays immutable, including through shared interior state.
  fn options(&self) -> impl Iterator<Item = Self::ResponseType>;

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool;

  // Both conversions wrap a known concrete type; neither extracts an enum variant.
  fn as_prompt(&self) -> G::Prompt<'_>;
  fn into_prompt(self) -> G::Prompt<'static>;
}

struct PresentedPrompt<T> {
  pub prompt: T,
  pub handle: ResponseHandle<T>,
}

struct ResponseHandle<T> {
  // Placeholder for the engine-owned connection to one specific request.
  prompt_type: PhantomData<T>,
}

impl<T> ResponseHandle<T> {
  fn submit<G, P>(&self, prompt: &P, response: P::ResponseType)
  where
    G: Game<Prompt<'static> = T>,
    P: PromptData<G>,
  {
    // Ignore replies to ended requests. The handle identifies the exact request;
    // the prompt argument determines the answer type, not request identity.
    // Verify P and its response type match the waiting choose call. Validate
    // against the retained typed prompt, never the supplied value. Panic if invalid and
    // resume once. Do not compare prompt addresses, including for zero-sized P.
    todo!()
  }
}

#[derive(Clone)]
enum HeartsPrompt<'a> {
  PlayCard(Cow<'a, PlayCardPrompt>),
  PassCards(Cow<'a, PassCardsPrompt>),
}

fn submit_selected_card(presented: &PresentedPrompt<HeartsPrompt<'static>>, selected_card: CardId) {
  if let HeartsPrompt::PlayCard(prompt) = &presented.prompt {
    // The UI can use this same check to disable illegal choices.
    if prompt.is_valid_response(&selected_card) {
      // Rust requires a CardId here because prompt is a PlayCardPrompt.
      presented.handle.submit(prompt.as_ref(), selected_card);
    }
  }
}

fn submit_selected_pass(
  presented: &PresentedPrompt<HeartsPrompt<'static>>,
  selected_cards: [CardId; 3],
) {
  if let HeartsPrompt::PassCards(prompt) = &presented.prompt {
    if prompt.is_valid_response(&selected_cards) {
      // This variant requires three cards rather than one CardId.
      presented.handle.submit(prompt.as_ref(), selected_cards);
    }
  }
}

impl GameContext<HeartsGame> for HeartsContext {
  fn present(&mut self, state: &HeartsState, animation: impl FnOnce() -> HeartsAnimation) {
    if let HeartsMode::Interactive { connection, .. } = &mut self.mode {
      connection.present(state, animation);
    }
    // Simulation skips both snapshot creation and the animation builder.
  }

  fn choose<P>(&mut self, state: &HeartsState, prompt: P) -> P::ResponseType
  where
    P: PromptData<HeartsGame>,
  {
    match &mut self.mode {
      HeartsMode::Simulation(policy) => {
        let index = policy.choose(state, &prompt.as_prompt());
        select_response::<HeartsGame, P>(&prompt, index)
      }
      HeartsMode::Interactive { connection, policy } => {
        // Hearts owns human/AI routing. Simulation always uses its own policy.
        if state.current_player == self.human_player {
          connection.choose::<P>(state, prompt)
        } else {
          connection.choose_with_policy::<P>(state, prompt, policy)
        }
      }
    }
  }
}

fn select_response<G: Game, P: PromptData<G>>(prompt: &P, index: usize) -> P::ResponseType {
  let response = prompt
    .options()
    .nth(index)
    .expect("Policy selected an invalid option index");
  assert!(
    prompt.is_valid_response(&response),
    "Policy selected an invalid response"
  );
  response
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CardId(u8);

#[derive(Clone)]
struct PlayCardPrompt {
  choices: Vec<CardId>,
}

impl PromptData<HeartsGame> for PlayCardPrompt {
  type ResponseType = CardId;

  fn options(&self) -> impl Iterator<Item = CardId> {
    self.choices.iter().copied()
  }

  fn is_valid_response(&self, response: &CardId) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> HeartsPrompt<'_> {
    HeartsPrompt::PlayCard(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> HeartsPrompt<'static> {
    HeartsPrompt::PlayCard(Cow::Owned(self))
  }
}

#[derive(Clone)]
struct PassCardsPrompt {
  choices: Vec<[CardId; 3]>,
}

impl PromptData<HeartsGame> for PassCardsPrompt {
  type ResponseType = [CardId; 3];

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.choices.iter().copied()
  }

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.choices.contains(response)
  }

  fn as_prompt(&self) -> HeartsPrompt<'_> {
    HeartsPrompt::PassCards(Cow::Borrowed(self))
  }

  fn into_prompt(self) -> HeartsPrompt<'static> {
    HeartsPrompt::PassCards(Cow::Owned(self))
  }
}
```

## Manual QA

Start a new state and a loaded state through the context factory. Dispatch while
busy, inject an illegal action, stop during a choice, and restart using the same
app. Verify old handles remain stopped and the accepted-state copy stays stable.
Run two typed choices through human and policy paths, deliberately misuse an
active reply in a fault fixture, and deliver a stale reply after replacement.
Compare simulation outcomes and confirm its snapshot/animation builders never
run. Exercise selectors, status-driven recovery, and an explicit save/load.
