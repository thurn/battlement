use std::sync::Arc;

use crate::{
  ChoicePolicy, Game, PromptData,
  publication::{Checkpoint, Publications},
};

/// The engine-owned publication connection used by interactive rules.
pub struct DisplayConnection<G: Game> {
  pub(crate) publications: Arc<Publications<G>>,
}

impl<G: Game> DisplayConnection<G> {
  pub(crate) fn present(
    &mut self,
    state: &G::State,
    animation: impl FnOnce() -> G::StateAnimation,
  ) {
    self.publications.check_active();
    let reservation = self.publications.reserve();
    let snapshot = G::logical_clone(state);
    self.publications.check_active();
    let animation = animation();
    self.publications.check_active();
    reservation.publish(Checkpoint {
      state: snapshot,
      animation: Some(animation),
      prompt: None,
      completion: None,
    });
  }

  pub(crate) fn choose<P>(&mut self, _state: &G::State, _prompt: P) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    self.publications.check_active();
    panic!("interactive prompts require typed request support");
  }

  pub(crate) fn choose_with_policy<P>(
    &mut self,
    _state: &G::State,
    _prompt: P,
    _policy: &mut impl ChoicePolicy<G>,
  ) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    self.publications.check_active();
    panic!("interactive prompts require typed request support");
  }
}
