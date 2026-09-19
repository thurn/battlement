use std::{cell::RefCell, rc::Rc};

use battlement::{ObjectId, object_id};
use reactant::{app::App, element_ref, hooks, prelude::*};
use trox::ls;

use crate::{CONTENT_SCENE, ROOT_ID};

const CARD: ObjectId = object_id!("25300000-0000-4000-8000-000000000111");

#[derive(Clone, Default)]
struct Probe {
  presence: Rc<RefCell<Option<Presence>>>,
  retained: Rc<RefCell<Vec<RetainedVisual>>>,
  reference: Rc<RefCell<Option<ElementRef>>>,
  setter: Rc<RefCell<Option<StateSetter<u32>>>>,
}
struct Screen(Probe);
struct Card {
  shown: bool,
  manual: bool,
  active: StateSetter<u32>,
  probe: Probe,
}

pub(crate) fn app() -> App {
  App::new(CONTENT_SCENE)
    .ui(Screen(Probe::default()))
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
}

impl Component for Screen {
  fn render(&self) -> impl Render {
    let (shown, set_shown) = hooks::use_state(true);
    let (open, set_open) = hooks::use_state(true);
    let (held_open, set_held_open) = hooks::use_state(true);
    let (active, set_active) = hooks::use_state(0_u32);
    let (releases, set_releases) = hooks::use_state(0_u32);
    let hold = self.0.clone();
    let release = self.0.clone();
    let stale = self.0.clone();
    View::new()
      .style(
        Style::new()
          .padding(32.px())
          .width(720.px())
          .color(Color::WHITE)
          .background_color(Color::rgb(0.05, 0.08, 0.14)),
      )
      .child((
        Heading::new(ls("Visibility and terminal visuals"), 1),
        Heading::new(
          ls(format!(
            "Active {active} / releases {releases} / uses {}",
            self.0.retained.borrow().len()
          )),
          2,
        ),
        Button::new(ls(if shown { "Hide card" } else { "Show card" }))
          .on_press(move || set_shown.set(!shown)),
        Button::new(ls("Destroy card")).on_press(move || set_open.set(false)),
        Button::new(ls("Hold and destroy")).on_press(move || {
          if held_open {
            let held = hold.presence.borrow().as_ref().unwrap().retain_visual();
            hold.retained.borrow_mut().extend([held.clone(), held]);
            set_held_open.set(false);
          }
        }),
        Button::new(ls("Release one use")).on_press(move || {
          if release.retained.borrow_mut().pop().is_some() {
            set_releases.update(|n| n + 1);
          }
        }),
        Button::new(ls("Deliver stale callback")).on_press(move || {
          if let Some(setter) = &*stale.setter.borrow() {
            setter.set(999);
          }
        }),
        AnimatePresence::new().child(open.then(|| {
          Card {
            shown,
            manual: false,
            active: set_active.clone(),
            probe: Probe::default(),
          }
          .id(*CARD.as_uuid())
          .key("card")
        })),
        AnimatePresence::new().child(held_open.then(|| {
          Card {
            shown: true,
            manual: true,
            active: set_active.clone(),
            probe: self.0.clone(),
          }
          .key("held")
        })),
      ))
  }
}

impl Component for Card {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    self.probe.setter.replace(Some(set_count.clone()));
    if self.manual {
      self.probe.presence.replace(Some(hooks::use_presence()));
    } else {
      let _ = hooks::use_is_present();
    }
    let active = self.active.clone();
    hooks::use_effect(
      move || {
        active.update(|n| n + 1);
        move || active.update(|n| n - 1)
      },
      (),
    );
    let reference = element_ref::use_element_ref();
    self.probe.reference.replace(Some(reference.clone()));
    let card = View::new()
      .element_ref(reference)
      .style(
        Style::new()
          .height(150.px())
          .padding(18.px())
          .margin_top(14.px())
          .background_color(if self.manual {
            Color::rgb(0.30, 0.15, 0.45)
          } else {
            Color::rgb(0.06, 0.35, 0.40)
          }),
      )
      .animate(StyleTarget::new().opacity(if self.shown { 1.0 } else { 0.0 }))
      .transition(Transition::tween().duration_secs(0.4))
      .child((
        Heading::new(
          ls(format!(
            "{} count: {count}",
            if self.manual { "Held" } else { "Card" }
          )),
          2,
        ),
        Button::new(ls(if self.manual {
          "Increment held"
        } else {
          "Increment card"
        }))
        .on_press(move || set_count.update(|n| n + 1)),
      ));
    let card = if self.manual {
      card
    } else {
      card.exit(
        MotionTarget::new(StyleTarget::new().opacity(0.0))
          .transition(Transition::tween().duration_secs(0.4)),
      )
    };
    VisibilityScope::new(self.shown).child(card)
  }
}
