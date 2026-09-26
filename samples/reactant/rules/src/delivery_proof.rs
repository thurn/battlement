use battlement::{AccessibilitySnapshot, AccessibilityUpdate, Command, CommandBody};
use reactant::{
  Application, GameHandle, GameStatus,
  delivery_diagnostics::DeliveryDiagnostics,
  prelude::*,
  rules::{ChoiceOwner, ChoicePolicy, ExecutionMode, Game},
};
use trox::ls;

struct Counter;
struct Policy;
struct Root;
struct Board(GameHandle<Counter>);

pub(crate) fn app() -> Application {
  self::with_diagnostics(DeliveryDiagnostics::default())
}

pub(crate) fn with_diagnostics(diagnostics: DeliveryDiagnostics) -> Application {
  Application::new(crate::CONTENT_SCENE)
    .delivery_diagnostics(diagnostics)
    .child(Root)
    .document(|mut document| {
      document.root_id = crate::ROOT_ID;
      document
    })
}

impl Component for Root {
  fn render(&self) -> impl Render {
    let game = reactant::use_game::<Counter, _>((), 0, |connection| ExecutionMode::Interactive {
      connection,
      policy: Policy,
    });
    View::new().child((
      Text::new(ls(if game.status() == GameStatus::Stopped {
        "Delivery canceled"
      } else {
        "Delivery ready"
      })),
      GameRoot::new(Board(game)),
    ))
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let app = reactant::app_context::use_app();
    let game = self.0.clone();
    Button::new(ls("Cancel queued observation")).on_press(move || {
      app.send(Command::new_v4(CommandBody::AccessibilityUpdate(
        AccessibilityUpdate {
          snapshot: Some(AccessibilitySnapshot {
            commit_sequence: 31337,
            ..AccessibilitySnapshot::default()
          }),
          announcements: vec!["private announcement must not be logged".to_owned()],
        },
      )));
      game.stop();
    })
  }
}

impl ChoicePolicy<Counter> for Policy {
  fn owner(&self, _: &u32, _: &()) -> ChoiceOwner {
    ChoiceOwner::Policy
  }

  fn choose(&mut self, _: &u32, _: &()) -> usize {
    unreachable!()
  }
}

impl Game for Counter {
  type State = u32;
  type Action = ();
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = ExecutionMode<Counter, Policy>;

  fn logical_clone(state: &u32) -> u32 {
    *state
  }
  fn is_legal_action(_: &u32, _: &()) -> bool {
    true
  }
  fn execute(_: &mut Self::Context, _: &mut u32, _: ()) {}
}
