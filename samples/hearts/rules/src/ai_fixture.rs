use std::sync::{Arc, Condvar, Mutex};

use battlement::{ObjectId, object_id};
use reactant::{Application, hooks, prelude::*};
use trox::{SourceLocale, ls};

use crate::{
  ai::{sampling, search},
  assets, controller,
  domain::{HeartsState, Seat},
};

const HELD: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47070");
const STOPPED: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47071");
const COMPLETE: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47072");

#[derive(Clone)]
struct SearchGate {
  release: Arc<(Mutex<bool>, Condvar)>,
  phase: DisplayStore<&'static str>,
}

struct SearchExit(DisplayStore<&'static str>);
struct AiFixture;
struct LifecycleFixture;

impl Drop for SearchExit {
  fn drop(&mut self) {
    self.0.set("Search stopped");
  }
}

impl SearchGate {
  fn release(&self) {
    let (released, changed) = &*self.release;
    *released.lock().unwrap() = true;
    changed.notify_all();
  }

  fn hold(&self) {
    let (released, changed) = &*self.release;
    let released = released.lock().unwrap();
    if !*released {
      self.phase.set("Search held in rollout");
    }
    drop(changed.wait_while(released, |released| !*released).unwrap());
  }
}

pub(crate) fn application() -> Application {
  self::configured(AiFixture)
}

pub(crate) fn lifecycle_application() -> Application {
  self::configured(LifecycleFixture)
}

fn configured(root: impl Component) -> Application {
  Application::new(assets::hearts::CONTENT)
    .source_locale(SourceLocale::new("en-US").expect("source locale"))
    .child(root)
    .document(|mut document| {
      document.root_id = crate::app::ROOT;
      document
    })
}

impl Component for AiFixture {
  fn render(&self) -> impl Render {
    let (paused, pause) = hooks::use_state(false);
    let inactive = !reactant::application::use_application_state().is_active();
    let paused = paused || inactive;
    let (menu, update_menu) = hooks::use_state(0_u32);
    let gate = hooks::use_memo(
      || SearchGate {
        release: Arc::new((Mutex::new(false), Condvar::new())),
        phase: DisplayStore::new("Search starting"),
      },
      (),
    );
    let phase = hooks::use_external_store(gate.phase.clone());
    let work = gate.clone();
    let game = controller::use_hearts_with_policy(
      (),
      || HeartsState::new(43),
      Seat::South,
      paused,
      move |observation, stream, token| {
        let _exit = SearchExit(work.phase.clone());
        let mut sampling_boundaries = 0;
        sampling::sample_deals(&observation, stream, || {
          token.checkpoint();
          sampling_boundaries += 1;
        });
        let mut boundaries = 0;
        search::evaluate(observation, stream, || {
          token.checkpoint();
          boundaries += 1;
          if boundaries == sampling_boundaries + 8 {
            work.hold();
          }
          token.checkpoint();
        })
      },
    );
    let release = gate.clone();
    hooks::use_effect(
      move || {
        if paused {
          release.release();
        }
      },
      paused,
    );
    hooks::use_effect(move || move || gate.release(), ());
    let accepted = game.game.accepted();
    let original = HeartsState::new(43);
    let random = accepted.state.random();
    let unchanged = random == original.random();
    let all_advanced = Seat::ALL[1..]
      .iter()
      .all(|seat| random.ai[seat.index()] != original.random().ai[seat.index()]);
    let complete = accepted.version.revision == 3 && all_advanced;
    let status = if complete {
      "AI passes accepted"
    } else if paused && unchanged {
      "Paused with unchanged random streams"
    } else if accepted.version.revision == 0 {
      "No AI decision accepted"
    } else {
      "Opponent passes pending"
    };
    assert_eq!(random.deck, original.random().deck);
    assert_eq!(random.ai[0], original.random().ai[0]);
    let resume = pause.clone();
    View::new()
      .style(
        Style::new()
          .padding(24.px())
          .color(Color::rgb(0.97, 0.94, 0.83)),
      )
      .child((
        GameRoot::new(View::new()),
        Heading::new(ls("Fair opponent proof"), 1),
        (phase == "Search held in rollout").then(|| Label::new(ls(phase)).id(*HELD.as_uuid())),
        (phase == "Search stopped").then(|| Label::new(ls(phase)).id(*STOPPED.as_uuid())),
        Text::new(ls(status)),
        complete.then(|| Label::new(ls("All opponent streams advanced")).id(*COMPLETE.as_uuid())),
        Text::new(ls(format!("Menu {menu}"))),
        Button::new(ls("Open menu")).on_press(move || update_menu.set(menu + 1)),
        Button::new(ls("Pause search")).on_press(move || pause.set(true)),
        Button::new(ls("Resume search")).on_press(move || resume.set(false)),
      ))
  }
}

impl Component for LifecycleFixture {
  fn render(&self) -> impl Render {
    let (background, set_background) = hooks::use_state(false);
    let suspend = set_background.clone();
    reactant::application::provider(battlement::application::ApplicationState {
      focused: !background,
      paused: background,
    })
    .child((
      AiFixture,
      View::new()
        .style(
          Style::new()
            .position(Position::Absolute)
            .right(18.px())
            .top(18.px()),
        )
        .child((
          Button::new(ls("Background search")).on_press(move || suspend.set(true)),
          Button::new(ls("Foreground search")).on_press(move || set_background.set(false)),
        )),
    ))
  }
}
