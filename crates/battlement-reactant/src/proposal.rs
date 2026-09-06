use std::marker::PhantomData;
use std::rc::Rc;

trait GameContext<G: Game> {
  // Interactive publication pairs an independent logical_clone(state) with the
  // animation. Simulation calls neither logical_clone nor the animation builder.
  fn present(&mut self, state: &G::State, animation: impl FnOnce() -> G::StateAnimation);
  fn choose<P>(&mut self, state: &G::State, prompt: P) -> P::ResponseType
  where
    P: PromptData<G::Prompt>;
}

trait Game: Sized + Send + 'static {
  type State: Send + 'static;
  type Action: Send + 'static;
  type StateAnimation: Send + 'static;
  type Prompt: Clone + Send + 'static;
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
  fn choose(&mut self, state: &G::State, prompt: &G::Prompt) -> usize;
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

fn use_game_prompt<G: Game>() -> Option<Rc<PresentedPrompt<G::Prompt>>> {
  // State and prompt become visible together; ended prompts stop being active.
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
    // Queue them together. At most one snapshot waits, including preparation.
    // Display code sequences entries and determines required animation completion.
    // Cancellation wakes blocked publication and unwinds inside the Rust worker.
    todo!()
  }

  fn choose<P: PromptData<G::Prompt>>(
    &mut self,
    state: &G::State,
    prompt: G::Prompt,
  ) -> P::ResponseType {
    // Queue a state snapshot and PresentedPrompt together, after earlier entries.
    // Enable its response handle only when its snapshot is displayed. Wait on
    // this worker while menus, hovering, and inspection remain responsive.
    // Validate replies against the actual request's P using is_valid_response.
    // Invalid current replies panic; ended-request replies are ignored.
    // Cancellation unwinds without a response and discards the working state.
    todo!()
  }

  fn choose_with_policy<P: PromptData<G::Prompt>>(
    &mut self,
    state: &G::State,
    prompt: G::Prompt,
    policy: &mut impl ChoicePolicy<G>,
  ) -> P::ResponseType {
    // Publish the snapshot and prompt in display order before asking the policy
    // on this worker. Clone the enum for display and retain the original for the
    // policy, so prompt data need not be Sync. The AI owns this request; human
    // replies cannot resolve it. Simulation does not make this display copy.
    // Convert the selected index with select_response. Check cancellation before
    // returning. The game chooses this route; Reactant does not identify players
    // or randomize hidden state for the policy.
    todo!()
  }
}

struct HeartsPolicy;

impl ChoicePolicy<HeartsGame> for HeartsPolicy {
  fn choose(&mut self, state: &HeartsState, prompt: &HeartsPrompt) -> usize {
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
  type Prompt = HeartsPrompt;
  type Context = HeartsContext;

  fn logical_clone(state: &HeartsState) -> HeartsState {
    state.clone()
  }

  fn is_legal_action(state: &HeartsState, action: &HeartsAction) -> bool {
    true
  }

  fn execute(e: &mut HeartsContext, state: &mut HeartsState, action: HeartsAction) {}
}

trait PromptData<T>: Sized + Send + 'static {
  type ResponseType: Send + 'static;

  // Own the data needed to enumerate legal responses. Enumeration may be lazy,
  // but repeated calls must preserve the option order while the prompt exists.
  fn options(&self) -> impl Iterator<Item = Self::ResponseType>;

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool;

  fn into_prompt(self) -> T;

  // Retrieves this prompt's data from the shared enum without copying choices.
  // A mismatched variant is a programming error.
  fn from_prompt(prompt: &T) -> &Self;
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
  fn submit<P>(&self, prompt: &P, response: P::ResponseType)
  where
    P: PromptData<T>,
  {
    // Ignore replies to ended requests. The handle identifies the exact request;
    // the prompt argument determines the answer type, not request identity.
    // Verify P and its response type match the waiting choose call. Validate
    // against the stored prompt, never the supplied value. Panic if invalid and
    // resume once. Do not compare prompt addresses, including for zero-sized P.
    todo!()
  }
}

#[derive(Clone)]
enum HeartsPrompt {
  PlayCard(PlayCardPrompt),
  PassCards(PassCardsPrompt),
}

fn submit_selected_card(presented: &PresentedPrompt<HeartsPrompt>, selected_card: CardId) {
  if let HeartsPrompt::PlayCard(prompt) = &presented.prompt {
    // The UI can use this same check to disable illegal choices.
    if prompt.is_valid_response(&selected_card) {
      // Rust requires a CardId here because prompt is a PlayCardPrompt.
      presented.handle.submit(prompt, selected_card);
    }
  }
}

fn submit_selected_pass(presented: &PresentedPrompt<HeartsPrompt>, selected_cards: [CardId; 3]) {
  if let HeartsPrompt::PassCards(prompt) = &presented.prompt {
    if prompt.is_valid_response(&selected_cards) {
      // This variant requires three cards rather than one CardId.
      presented.handle.submit(prompt, selected_cards);
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
    P: PromptData<HeartsPrompt>,
  {
    let prompt = prompt.into_prompt();
    match &mut self.mode {
      HeartsMode::Simulation(policy) => {
        let index = policy.choose(state, &prompt);
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

fn select_response<G: Game, P: PromptData<G::Prompt>>(
  prompt: &G::Prompt,
  index: usize,
) -> P::ResponseType {
  let data = P::from_prompt(prompt);
  let response = data
    .options()
    .nth(index)
    .expect("Policy selected an invalid option index");
  assert!(
    data.is_valid_response(&response),
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

impl PromptData<HeartsPrompt> for PlayCardPrompt {
  type ResponseType = CardId;

  fn options(&self) -> impl Iterator<Item = CardId> {
    self.choices.iter().copied()
  }

  fn is_valid_response(&self, response: &CardId) -> bool {
    self.choices.contains(response)
  }

  fn into_prompt(self) -> HeartsPrompt {
    HeartsPrompt::PlayCard(self)
  }

  fn from_prompt(prompt: &HeartsPrompt) -> &Self {
    match prompt {
      HeartsPrompt::PlayCard(prompt) => prompt,
      _ => panic!("Expected a play-card prompt"),
    }
  }
}

#[derive(Clone)]
struct PassCardsPrompt {
  choices: Vec<[CardId; 3]>,
}

impl PromptData<HeartsPrompt> for PassCardsPrompt {
  type ResponseType = [CardId; 3];

  fn options(&self) -> impl Iterator<Item = Self::ResponseType> {
    self.choices.iter().copied()
  }

  fn is_valid_response(&self, response: &Self::ResponseType) -> bool {
    self.choices.contains(response)
  }

  fn into_prompt(self) -> HeartsPrompt {
    HeartsPrompt::PassCards(self)
  }

  fn from_prompt(prompt: &HeartsPrompt) -> &Self {
    match prompt {
      HeartsPrompt::PassCards(prompt) => prompt,
      _ => panic!("Expected a pass-cards prompt"),
    }
  }
}
