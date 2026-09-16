//! Typed synchronous game rules and simulation primitives for Reactant.

use std::marker::PhantomData;

#[cfg(feature = "platform-proof")]
#[doc(hidden)]
pub mod platform_proof;
#[cfg(any(test, feature = "platform-proof"))]
mod worker;

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
  fn as_prompt(&self) -> G::Prompt<'_>;

  /// Wraps this concrete data in the game's owned prompt enum.
  fn into_prompt(self) -> G::Prompt<'static>;
}

/// The engine-owned connection used by interactive rules execution.
///
/// Session construction and live publication are introduced with the worker
/// runtime. Keeping the storage private prevents games from manufacturing an
/// unattached connection.
pub struct DisplayConnection<G: Game> {
  game: PhantomData<fn() -> G>,
}

impl<G: Game> DisplayConnection<G> {
  fn present(&mut self, _state: &G::State, _animation: impl FnOnce() -> G::StateAnimation) {
    panic!("interactive rules execution is not available without a session runtime");
  }

  fn choose<P>(&mut self, _state: &G::State, _prompt: P) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    panic!("interactive rules execution is not available without a session runtime");
  }

  fn choose_with_policy<P>(
    &mut self,
    _state: &G::State,
    _prompt: P,
    _policy: &mut impl ChoicePolicy<G>,
  ) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    panic!("interactive rules execution is not available without a session runtime");
  }
}

/// Selects between a live display connection and direct simulation.
pub enum ExecutionMode<G: Game, C: ChoicePolicy<G>> {
  /// Publishes states and resolves prompts through a live game session.
  Interactive {
    /// The connection owned by the active game session.
    connection: DisplayConnection<G>,
    /// The policy used for policy-owned live prompts.
    policy: C,
  },
  /// Runs policies synchronously without display publication.
  Simulation {
    /// The policy used for every simulated prompt.
    policy: C,
  },
}

impl<G, C> ExecutionMode<G, C>
where
  G: Game,
  C: ChoicePolicy<G>,
{
  /// Presents a snapshot and semantic event during interactive execution.
  ///
  /// Simulation skips both logical cloning and the lazy event builder.
  pub fn present(&mut self, state: &G::State, animation: impl FnOnce() -> G::StateAnimation) {
    if let Self::Interactive { connection, .. } = self {
      connection.present(state, animation);
    }
  }

  /// Resolves a typed prompt through the active execution mode.
  pub fn choose<P>(&mut self, state: &G::State, prompt: P) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    match self {
      Self::Simulation { policy } => {
        let wrapped = prompt.as_prompt();
        let index = policy.choose(state, &wrapped);
        self::select_response::<G, P>(&prompt, index)
      }
      Self::Interactive { connection, policy } => {
        let owner = {
          let wrapped = prompt.as_prompt();
          policy.owner(state, &wrapped)
        };
        match owner {
          ChoiceOwner::Human => connection.choose(state, prompt),
          ChoiceOwner::Policy => connection.choose_with_policy(state, prompt, policy),
        }
      }
    }
  }
}

fn select_response<G, P>(prompt: &P, index: usize) -> P::ResponseType
where
  G: Game,
  P: PromptData<G>,
{
  let response = prompt
    .options()
    .nth(index)
    .expect("policy selected an invalid option index");
  assert!(
    prompt.is_valid_response(&response),
    "policy selected an invalid response"
  );
  response
}
