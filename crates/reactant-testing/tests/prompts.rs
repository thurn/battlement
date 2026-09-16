#[path = "support/choice_game.rs"]
mod choice_game;

use std::{
  borrow::Cow,
  panic::{self, AssertUnwindSafe},
  sync::{Arc, atomic::Ordering},
  time::Duration,
};

use battlement_fake::assets::FakeAssetCatalog;
use choice_game::{Action, Empty, Number, Pair, Probe, Prompt};
use reactant_core::{app::App, host::ButtonHost, prelude::*};
use reactant_rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game, PromptData};
use reactant_testing::{Display, PublicationDisplay};
use trox::ls;

#[test]
fn typed_human_answers_resume_once_and_stale_payloads_are_ignored() {
  let probe = Probe::new(false);
  let mut display = choice_game::start(&probe, Action::Human, ChoiceOwner::Human);
  display.wait_for_publication(|o| o.published == 1);
  let early = display.response_handle().unwrap();
  let lookalike = Number {
    choices: [7, 99, 100],
    probe: Arc::clone(&probe),
  };
  assert!(panic::catch_unwind(AssertUnwindSafe(|| early.submit(&lookalike, 99))).is_err());
  assert!(early.is_active());
  assert!(
    panic::catch_unwind(AssertUnwindSafe(|| early.submit(&Pair([[1, 2]; 2]), [1, 2]))).is_err()
  );
  // The request can be answered while its display entry is still pending.
  early.submit(&lookalike, 7);
  early.submit(&Pair([[1, 2]; 2]), [999, 999]);
  let first = display.take_checkpoint();
  let presented = first.prompt().unwrap();
  assert!(matches!(&presented.prompt, Prompt::Number(p) if p.choices == [3, 7, 11]));
  assert!(!presented.handle.is_active());
  assert!(display.settle().prompt().unwrap().handle.is_active());
  let second = display.settle();
  assert_eq!(second.state().answers, [7]);
  let presented = second.prompt().unwrap();
  let Prompt::Pair(prompt) = &presented.prompt else {
    panic!("expected pair")
  };
  early.submit(&lookalike, 999);
  assert!(presented.handle.is_active());
  presented.handle.submit(prompt.as_ref(), [6, 8]);
  presented.handle.submit(prompt.as_ref(), [999, 999]);
  let final_output = display.settle();
  assert!(final_output.is_final());
  assert_eq!(final_output.state().answers, [7, 6, 8]);
  display.wait_for_worker_stopped();
  assert_eq!(probe.clones.load(Ordering::SeqCst), 1);
}

#[test]
fn five_policy_choices_finish_before_consumption_and_clone_only_for_display() {
  let probe = Probe::new(false);
  let mut display = choice_game::start(&probe, Action::Policies, ChoiceOwner::Policy);
  display.wait_for_worker_stopped();
  assert_eq!(probe.calls.load(Ordering::SeqCst), 5);
  assert_eq!(probe.clones.load(Ordering::SeqCst), 5);
  assert_eq!(display.publication_observation().consumed, 0);
  for answered in 0..5 {
    let checkpoint = display.take_checkpoint();
    assert_eq!(checkpoint.state().answers, vec![7; answered]);
    let presented = checkpoint.prompt().unwrap();
    let Prompt::Number(prompt) = &presented.prompt else {
      panic!("expected number")
    };
    assert!(matches!(prompt, Cow::Owned(_)));
    presented.handle.submit(prompt.as_ref(), 999); // already resolved AI request
  }
  assert_eq!(display.settle().state().answers, [7; 5]);
}

#[test]
fn human_wait_and_bounded_policy_leave_local_menu_and_selection_responsive() {
  for owner in [ChoiceOwner::Human, ChoiceOwner::Policy] {
    let probe = Probe::new(owner == ChoiceOwner::Policy);
    let display = choice_game::start(&probe, Action::Policies, owner);
    let checkpoint = display.take_checkpoint();
    if owner == ChoiceOwner::Policy {
      probe.wait_policy();
    }
    let presented = checkpoint.prompt().unwrap();
    let Prompt::Number(prompt) = &presented.prompt else {
      panic!("expected number")
    };
    if owner == ChoiceOwner::Policy {
      assert!(
        panic::catch_unwind(AssertUnwindSafe(|| presented
          .handle
          .submit(prompt.as_ref(), 7)))
        .is_err()
      );
    }
    let app = App::with_model("app/content", (0_usize, false)).root(|model| {
      View::new().child((
        Label::new(ls(format!("{}:{}", model.0, model.1))).name("local"),
        ButtonHost::new(ls("Select"))
          .name("select")
          .on_click(|model: &mut (usize, bool)| model.0 = 99),
        ButtonHost::new(ls("Settings"))
          .name("settings")
          .on_click(|model: &mut (usize, bool)| model.1 = !model.1),
      ))
    });
    let root = app.root_document().root_id;
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene("app/content");
    let mut menu = Display::connect(app, assets);
    menu.click_ui(menu.find_ui(root, "select"));
    menu.click_ui(menu.find_ui(root, "settings"));
    assert_eq!(
      menu.ui_element(menu.find_ui(root, "local")).text(),
      Some("99:true")
    );
    assert_eq!(menu.presentation_time(), Duration::ZERO);
    assert_eq!(menu.frame(), 0);
    assert!(checkpoint.state().answers.is_empty());
    assert!(presented.handle.is_active());
    display.stop();
    presented.handle.submit(prompt.as_ref(), 999);
    probe.release();
    display.wait_for_worker_stopped();
    assert!(display.worker_observation().cancelled);
    assert!(display.try_take_checkpoint().is_none());
    let mut replacement = choice_game::start(&probe, Action::Empty, ChoiceOwner::Human);
    let next = replacement.settle();
    presented.handle.submit(&Pair([[0; 2]; 2]), [999; 2]);
    let next = next.prompt().unwrap();
    let Prompt::Empty(prompt) = &next.prompt else {
      panic!("expected zero-sized prompt")
    };
    assert_eq!(std::mem::size_of::<Empty>(), 0);
    next.handle.submit(prompt.as_ref(), ());
    assert_eq!(replacement.settle().state().answers, [1]);
    replacement.wait_for_worker_stopped();
  }
}

struct OtherGame;
enum OtherPrompt<'a> {
  Confirm(Cow<'a, Confirmation>),
}
#[derive(Clone)]
struct Confirmation;
impl PromptData<OtherGame> for Confirmation {
  type ResponseType = bool;
  fn options(&self) -> impl Iterator<Item = bool> {
    [true].into_iter()
  }
  fn is_valid_response(&self, response: &bool) -> bool {
    *response
  }
  fn as_prompt(&self) -> OtherPrompt<'_> {
    OtherPrompt::Confirm(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> OtherPrompt<'static> {
    OtherPrompt::Confirm(Cow::Owned(self))
  }
}
struct OtherPolicy;
impl ChoicePolicy<OtherGame> for OtherPolicy {
  fn owner(&self, _: &bool, _: &OtherPrompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Human
  }
  fn choose(&mut self, _: &bool, _: &OtherPrompt<'_>) -> usize {
    panic!("human request")
  }
}
impl Game for OtherGame {
  type State = bool;
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = OtherPrompt<'a>;
  type Context = ExecutionMode<Self, OtherPolicy>;
  fn logical_clone(state: &bool) -> bool {
    *state
  }
  fn is_legal_action(_: &bool, _: &()) -> bool {
    true
  }
  fn execute(cx: &mut Self::Context, state: &mut bool, _: ()) {
    *state = cx.choose(state, Confirmation);
  }
}

#[test]
fn another_games_owned_enum_uses_the_same_typed_response_api() {
  let mut display =
    PublicationDisplay::<OtherGame>::start(&false, (), |connection| ExecutionMode::Interactive {
      connection,
      policy: OtherPolicy,
    });
  let checkpoint = display.settle();
  let presented = checkpoint.prompt().unwrap();
  let OtherPrompt::Confirm(prompt) = &presented.prompt;
  presented.handle.submit(prompt.as_ref(), true);
  assert!(*display.settle().state());
  display.wait_for_worker_stopped();
}
