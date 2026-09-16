/// Defines a game's state, actions, prompts, and synchronous rules entry point.
pub trait Game: Sized + Send + 'static {
  /// Mutable rules state and the immutable display snapshot source.
  type State: Send + 'static;
  /// An input that starts one rules execution.
  type Action: Send + 'static;
  /// A semantic event associated with a presented state.
  type StateAnimation: Send + 'static;
  /// The game-wide prompt enum inspected by both displays and policies.
  type Prompt<'a>: Send + 'a;
  /// Game-owned services and data used during rules execution.
  type Context: Send + 'static;

  /// Makes an independent logical copy of game state.
  ///
  /// Copies must not share mutable data with their source. Immutable data may
  /// be shared; the engine does not validate or deep-copy arbitrary Rust values.
  fn logical_clone(state: &Self::State) -> Self::State;

  /// Reports whether an action can begin from the supplied state.
  fn is_legal_action(state: &Self::State, action: &Self::Action) -> bool;

  /// Runs one action synchronously against private state.
  fn execute(context: &mut Self::Context, state: &mut Self::State, action: Self::Action);
}

/// Selects typed prompt responses by their stable option index.
pub trait ChoicePolicy<G: Game> {
  /// Classifies who owns a choice during interactive execution.
  fn owner(&self, state: &G::State, prompt: &G::Prompt<'_>) -> ChoiceOwner;

  /// Returns an index into the prompt's stable option order.
  fn choose(&mut self, state: &G::State, prompt: &G::Prompt<'_>) -> usize;
}

/// Ownership of a prompt during interactive execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceOwner {
  /// A display response resolves the prompt.
  Human,
  /// The configured policy resolves the prompt.
  Policy,
}

/// A typed prompt and the rules response it produces.
pub trait PromptData<G: Game>: Clone + Send + Sync + 'static {
  /// The concrete response returned to game rules.
  type ResponseType: Send + 'static;

  /// Enumerates owned responses in a stable order.
  fn options(&self) -> impl Iterator<Item = Self::ResponseType>;

  /// Checks a response against this prompt's rules.
  fn is_valid_response(&self, response: &Self::ResponseType) -> bool;

  /// Wraps this concrete data in the game's borrowed prompt enum.
  ///
  /// A temporary wrapper cannot become owned display storage:
  /// ```compile_fail
  /// use reactant_rules::{Game, PromptData};
  /// fn escape<G: Game, P: PromptData<G>>(prompt: P) -> G::Prompt<'static> {
  ///   prompt.as_prompt()
  /// }
  /// ```
  fn as_prompt(&self) -> G::Prompt<'_>;

  /// Wraps this concrete data in the game's owned prompt enum.
  fn into_prompt(self) -> G::Prompt<'static>;
}
