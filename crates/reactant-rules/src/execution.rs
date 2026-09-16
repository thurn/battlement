use crate::{ChoiceOwner, ChoicePolicy, DisplayConnection, Game, PromptData};

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
