use std::sync::Arc;

use crate::{
  ChoiceOwner, ChoicePolicy, Game, PresentedPrompt, PromptData, ResponseHandle, execution,
  publication::{Checkpoint, Publications},
  response::{Reply, Request},
  session_context::PublicationTarget,
};

/// The engine-owned publication connection used by interactive rules.
pub struct DisplayConnection<G: Game> {
  pub(crate) target: PublicationTarget<G>,
}

impl<G: Game> DisplayConnection<G> {
  fn publications(&self) -> Arc<Publications<G>> {
    self
      .target
      .lock()
      .unwrap()
      .upgrade()
      .expect("display connection used outside an active rules action")
  }

  pub(crate) fn check_active(&self) {
    self.publications().check_active();
  }

  pub(crate) fn present(
    &mut self,
    state: &G::State,
    animation: impl FnOnce() -> G::StateAnimation,
  ) {
    self.publications().check_active();
    let publications = self.publications();
    let reservation = publications.reserve();
    let snapshot = G::logical_clone(state);
    self.publications().check_active();
    let animation = animation();
    self.publications().check_active();
    reservation.publish(Checkpoint {
      state: snapshot,
      animation: Some(animation),
      prompt: None,
      completion: None,
    });
  }

  pub(crate) fn choose<P>(&mut self, state: &G::State, prompt: P) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    let request = self.publish_prompt(state, prompt, ChoiceOwner::Human);
    let response = request.wait();
    self.publications().check_active();
    response
  }

  pub(crate) fn choose_with_policy<P>(
    &mut self,
    state: &G::State,
    prompt: P,
    policy: &mut impl ChoicePolicy<G>,
  ) -> P::ResponseType
  where
    P: PromptData<G>,
  {
    let request = self.publish_prompt(state, prompt, ChoiceOwner::Policy);
    let index = policy.choose(state, &request.prompt.as_prompt());
    self.publications().check_active();
    let response = execution::select_response::<G, P>(&request.prompt, index);
    request.finish_policy();
    self.publications().check_active();
    response
  }

  fn publish_prompt<P: PromptData<G>>(
    &self,
    state: &G::State,
    prompt: P,
    owner: ChoiceOwner,
  ) -> Arc<Request<G, P>> {
    self.publications().check_active();
    let publications = self.publications();
    let reservation = publications.reserve();
    let request = Arc::new(Request::new(
      prompt,
      owner,
      Arc::clone(&publications.abandoned),
    ));
    let snapshot = G::logical_clone(state);
    self.publications().check_active();
    let prompt = request.prompt.as_ref().clone().into_prompt();
    self.publications().check_active();
    let reply: Arc<dyn Reply> = request.clone();
    let handle = ResponseHandle::new(&reply);
    publications.register_request(reply);
    reservation.publish(Checkpoint {
      state: snapshot,
      animation: None,
      prompt: Some(PresentedPrompt { prompt, handle }),
      completion: None,
    });
    self.publications().check_active();
    request
  }
}
