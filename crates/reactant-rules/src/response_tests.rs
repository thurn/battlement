use std::{
  any::TypeId,
  panic::{self, AssertUnwindSafe},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use crate::{
  ChoiceOwner, Game, PromptData,
  response::{Reply, Request},
};

struct TestGame;
#[derive(Clone)]
struct Prompt;

impl Game for TestGame {
  type State = ();
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = ();
  fn logical_clone(_: &()) {}
  fn is_legal_action(_: &(), _: &()) -> bool {
    true
  }
  fn execute(_: &mut (), _: &mut (), _: ()) {}
}
impl PromptData<TestGame> for Prompt {
  type ResponseType = u8;
  fn options(&self) -> impl Iterator<Item = u8> {
    [7].into_iter()
  }
  fn is_valid_response(&self, response: &u8) -> bool {
    *response == 7
  }
  fn as_prompt(&self) {}
  fn into_prompt(self) {}
}

#[test]
fn erased_transport_checks_game_prompt_and_response_types_only_while_active() {
  let request =
    Request::<TestGame, Prompt>::new(Prompt, ChoiceOwner::Human, Arc::new(AtomicBool::new(false)));
  let game = TypeId::of::<TestGame>();
  let prompt = TypeId::of::<Prompt>();
  for (g, p, value) in [
    (
      TypeId::of::<()>(),
      prompt,
      Box::new(7_u8) as Box<dyn std::any::Any + Send>,
    ),
    (game, TypeId::of::<()>(), Box::new(7_u8)),
    (game, prompt, Box::new("wrong payload")),
  ] {
    assert!(panic::catch_unwind(AssertUnwindSafe(|| request.submit(g, p, value))).is_err());
    assert!(request.active());
  }
  request.submit(game, prompt, Box::new(7_u8));
  request.submit(TypeId::of::<()>(), TypeId::of::<()>(), Box::new("ignored"));
  assert_eq!(request.wait(), 7);
}

#[test]
fn cancellation_observed_at_wakeup_wins_over_an_already_queued_answer() {
  let abandoned = Arc::new(AtomicBool::new(false));
  let request =
    Request::<TestGame, Prompt>::new(Prompt, ChoiceOwner::Human, Arc::clone(&abandoned));
  request.submit(
    TypeId::of::<TestGame>(),
    TypeId::of::<Prompt>(),
    Box::new(7_u8),
  );
  abandoned.store(true, Ordering::Release);
  request.cancel();
  assert!(panic::catch_unwind(AssertUnwindSafe(|| request.wait())).is_err());
}
