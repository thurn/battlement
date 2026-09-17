use std::{borrow::Cow, cell::RefCell, rc::Rc, time::Duration};

use battlement::{ClickEvent, Command, CommandBody, ObjectId, UiEvent, WaitPayload};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{GameConsumer, GameHandle, GameStatus, app::App, host::ButtonHost, prelude::*};
use reactant_rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game, PresentedPrompt, PromptData};
use reactant_testing::Display;
use trox::ls;

const TIMEOUT: Duration = Duration::from_secs(10);

type CurrentPrompt = Rc<RefCell<Option<Rc<PresentedPrompt<Prompt<'static>>>>>>;

struct Decisions;
struct Policy;
#[derive(Clone)]
struct Confirm;
enum Prompt<'a> {
  Confirm(Cow<'a, Confirm>),
}
struct Panel(CurrentPrompt, usize);

impl PromptData<Decisions> for Confirm {
  type ResponseType = usize;
  fn options(&self) -> impl Iterator<Item = usize> {
    [1].into_iter()
  }
  fn is_valid_response(&self, response: &usize) -> bool {
    *response == 1
  }
  fn as_prompt(&self) -> Prompt<'_> {
    Prompt::Confirm(Cow::Borrowed(self))
  }
  fn into_prompt(self) -> Prompt<'static> {
    Prompt::Confirm(Cow::Owned(self))
  }
}
impl ChoicePolicy<Decisions> for Policy {
  fn owner(&self, _: &usize, _: &Prompt<'_>) -> ChoiceOwner {
    ChoiceOwner::Human
  }
  fn choose(&mut self, _: &usize, _: &Prompt<'_>) -> usize {
    unreachable!()
  }
}
impl Game for Decisions {
  type State = usize;
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = Prompt<'a>;
  type Context = ExecutionMode<Self, Policy>;
  fn logical_clone(state: &usize) -> usize {
    *state
  }
  fn is_legal_action(_: &usize, _: &()) -> bool {
    true
  }
  fn execute(cx: &mut Self::Context, state: &mut usize, _: ()) {
    for _ in 0..2 {
      *state += cx.choose(state, Confirm);
    }
  }
}
impl Component for Panel {
  fn render(&self) -> impl Render {
    let state = *reactant::use_game_state::<Decisions>();
    let prompt = reactant::use_game_prompt::<Decisions>();
    *self.0.borrow_mut() = prompt.clone();
    let app = reactant::app_context::use_app();
    reactant::hooks::use_effect(
      move || {
        app.send(Command::new_v4(CommandBody::TimeWait(WaitPayload {
          duration_ms: 1000,
        })));
      },
      prompt.as_ref().map(|prompt| prompt.handle.clone()),
    );
    View::new().child((
      Label::new(ls(state.to_string())).name("state"),
      Label::new(ls(self.1.to_string())).name("game-menu"),
      prompt.map(|prompt| {
        ButtonHost::new(ls("Choose one"))
          .key(prompt.handle.clone())
          .name("answer")
          .on_click(move |_: &mut usize| {
            let Prompt::Confirm(value) = &prompt.prompt;
            prompt.handle.submit(value.as_ref(), 1);
          })
      }),
    ))
  }
}

fn setup() -> (
  Display<App<usize>>,
  GameHandle<Decisions>,
  GameConsumer<Decisions>,
  CurrentPrompt,
  ObjectId,
) {
  let current = Rc::new(RefCell::new(None));
  let panel = current.clone();
  let mut app = App::with_model("input/content", 0_usize).root(move |menu| {
    View::new().child((
      ButtonHost::new(ls("Settings"))
        .name("menu")
        .on_click(|menu: &mut usize| *menu += 1),
      Label::new(ls(menu.to_string())).name("menu-state"),
      GameRoot::new(Panel(panel.clone(), *menu)),
    ))
  });
  let game = app.start_game::<Decisions>(0, |connection| ExecutionMode::Interactive {
    connection,
    policy: Policy,
  });
  let consumer = app.game_consumer::<Decisions>();
  consumer.resume_automatic_submission();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("input/content");
  let mut display = Display::connect(app, assets);
  display.poll();
  display.settle();
  (display, game, consumer, current, root)
}

#[test]
fn request_keys_keep_old_pointer_and_navigation_targets_out_of_new_prompt() {
  for navigation in [false, true] {
    let (mut display, game, consumer, current, root) = self::setup();
    game.dispatch(());
    assert!(consumer.wait_for_output(TIMEOUT));
    display.poll();
    let first = display.find_ui(root, "answer");
    let menu = display.find_ui(root, "menu");
    let old = current.borrow().clone().unwrap();
    let Prompt::Confirm(value) = &old.prompt;
    // Resolve through the public request API while its native target is still visible.
    old.handle.submit(value.as_ref(), 1);
    assert!(consumer.wait_for_publication(TIMEOUT, |o| o.published == 2));
    display.poll();
    let next = current.borrow().clone().unwrap();
    assert!(old.handle != next.handle);
    assert!(next.handle == next.handle.clone());
    assert!(next.handle.is_waiting_for_human());
    assert_eq!(display.find_ui(root, "answer"), first);
    assert_eq!(
      display.ui_element(display.find_ui(root, "state")).text(),
      Some("0")
    );
    let time = display.presentation_time();
    display.click_ui(menu);
    if navigation {
      display.navigation_submit_ui(first);
    } else {
      display.click_ui(first);
    }
    assert!(next.handle.is_waiting_for_human());
    assert_eq!(
      display
        .ui_element(display.find_ui(root, "menu-state"))
        .text(),
      Some("1")
    );
    assert_eq!(
      display.ui_element(display.find_ui(root, "state")).text(),
      Some("0")
    );
    assert_eq!(
      display
        .ui_element(display.find_ui(root, "game-menu"))
        .text(),
      Some("0")
    );
    assert_eq!(display.presentation_time(), time);
    assert_eq!(display.frame(), 0);
    display.advance_time(Duration::from_secs(1));
    let second = display.find_ui(root, "answer");
    assert_ne!(first, second);
    assert_eq!(
      display.ui_element(display.find_ui(root, "state")).text(),
      Some("1")
    );
    display.deliver_ui_event(UiEvent::click(first, ClickEvent::NavigationSubmit));
    assert!(next.handle.is_waiting_for_human());
    if navigation {
      display.navigation_submit_ui(second);
    } else {
      display.click_ui(second);
    }
    assert!(consumer.wait_for_worker_stopped(TIMEOUT));
    display.poll();
    assert_eq!(game.status(), GameStatus::Ready);
    assert_eq!(game.accepted_state(), 2);
    display.settle();
    assert_eq!(
      display.ui_element(display.find_ui(root, "state")).text(),
      Some("2")
    );
    assert_eq!(display.find_ui(root, "menu"), menu);
    assert_eq!(
      display
        .ui_element(display.find_ui(root, "game-menu"))
        .text(),
      Some("1")
    );
  }
}

#[test]
fn replacement_discards_queued_prompt_hosts_and_delayed_input_without_remounting_menu() {
  let (mut display, game, consumer, current, root) = self::setup();
  game.dispatch(());
  assert!(consumer.wait_for_output(TIMEOUT));
  display.poll();
  let old_target = display.find_ui(root, "answer");
  let old = current.borrow().clone().unwrap();
  let Prompt::Confirm(value) = &old.prompt;
  old.handle.submit(value.as_ref(), 1);
  assert!(consumer.wait_for_publication(TIMEOUT, |o| o.published == 2));
  display.poll();
  let menu = display.find_ui(root, "menu");
  display.click_ui(menu);
  let replacement = display.with_engine(|app| {
    app.start_game::<Decisions>(17, |connection| ExecutionMode::Interactive {
      connection,
      policy: Policy,
    })
  });
  display.poll();
  display.poll();
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  display.deliver_ui_event(UiEvent::click(old_target, ClickEvent::NavigationSubmit));
  old.handle.submit(value.as_ref(), 999);
  display.advance_time(Duration::from_secs(30));
  assert_eq!(game.status(), GameStatus::Stopped);
  assert_eq!(replacement.status(), GameStatus::Ready);
  assert_eq!(replacement.accepted_state(), 17);
  assert!(current.borrow().is_none());
  assert_eq!(
    display.ui_element(display.find_ui(root, "state")).text(),
    Some("17")
  );
  assert_eq!(display.find_ui(root, "menu"), menu);
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "menu-state"))
      .text(),
    Some("1")
  );
  assert_eq!(display.with_engine(|app| app.retained_gameplay_bytes()), 0);
}
