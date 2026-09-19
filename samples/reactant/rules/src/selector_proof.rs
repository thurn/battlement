use std::{cell::Cell, rc::Rc};

use battlement::{ObjectId, object_id};
use reactant::{component, hooks, prelude::*};
use trox::ls;

use crate::ROOT_ID;

const CARD: ObjectId = object_id!("25300000-0000-4000-8000-000000000091");

#[derive(Clone, PartialEq)]
struct Values {
  score: u32,
  settings: u32,
}
#[derive(PartialEq)]
struct Screen {
  left: DisplayStore<Values>,
  right: DisplayStore<Values>,
  props: bool,
}
#[derive(PartialEq)]
struct SelectedCard;
#[derive(PartialEq)]
struct PropsCard(u32);
#[derive(PartialEq)]
struct PropsReader;
#[derive(PartialEq)]
struct Settings(DisplayStore<Values>);

pub(crate) fn app(props: bool) -> crate::ReactantEngine {
  reactant::app::App::new(crate::CONTENT_SCENE)
    .ui(Screen {
      left: DisplayStore::new(Values {
        score: 1,
        settings: 0,
      }),
      right: DisplayStore::new(Values {
        score: 9,
        settings: 0,
      }),
      props,
    })
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document
    })
}

impl Component for Screen {
  fn render(&self) -> impl Render {
    let (moved, set_moved) = hooks::use_state(false);
    let source = if moved {
      self.right.clone()
    } else {
      self.left.clone()
    };
    let score = source.clone();
    let settings = source.clone();
    View::new()
      .style(
        Style::new()
          .width(800.px())
          .padding(28.px())
          .color(Color::WHITE)
          .background_color(Color::rgb(0.06, 0.09, 0.15)),
      )
      .child((
        Heading::new(ls("Stable score and settings"), 1),
        Label::new(ls(
          "Move the card between providers. Its local count follows it.",
        )),
        Button::new(ls("Change score")).on_press(move || score.update(|value| value.score += 1)),
        Button::new(ls("Change settings"))
          .on_press(move || settings.update(|value| value.settings += 1)),
        Button::new(ls("Move card")).on_press(set_moved.update_callback(|value| !value)),
        component::memo(Settings(source)),
        self::provider("Source", self.left.clone(), !moved, self.props),
        self::provider("Destination", self.right.clone(), moved, self.props),
      ))
  }
}

fn provider(
  name: &'static str,
  store: DisplayStore<Values>,
  visible: bool,
  props: bool,
) -> impl Render {
  View::new().style(Style::new().padding(12.px())).child((
    Heading::new(ls(name), 2),
    ContextProvider::new()
      .context(store)
      .child(visible.then(|| {
        if props {
          Node::new(component::memo(PropsReader).id(*CARD.as_uuid()))
        } else {
          Node::new(component::memo(SelectedCard).id(*CARD.as_uuid()))
        }
      })),
  ))
}

impl Component for Settings {
  fn render(&self) -> impl Render {
    let settings = hooks::use_external_store_selector(self.0.clone(), |value| value.settings);
    Heading::new(ls(format!("Settings: {settings}")), 2)
  }
}
impl Component for SelectedCard {
  fn render(&self) -> impl Render {
    let source = hooks::use_required_context::<DisplayStore<Values>>();
    let score = hooks::use_external_store_selector(source, |value| value.score);
    self::card(score)
  }
}
impl Component for PropsReader {
  fn render(&self) -> impl Render {
    let source = hooks::use_required_context::<DisplayStore<Values>>();
    let values = hooks::use_external_store(source);
    component::memo(PropsCard(values.score))
  }
}
impl Component for PropsCard {
  fn render(&self) -> impl Render {
    self::card(self.0)
  }
}

fn card(score: u32) -> impl Render {
  let (count, set_count) = hooks::use_state(0_u32);
  let renders = hooks::use_memo(|| Rc::new(Cell::new(0_u32)), ());
  renders.set(renders.get() + 1);
  View::new()
    .style(
      Style::new()
        .padding(18.px())
        .background_color(Color::rgb(0.14, 0.23, 0.34)),
    )
    .child((
      Heading::new(ls(format!("Score {score} / local {count}")), 2),
      Heading::new(ls(format!("Card evaluations: {}", renders.get())), 3),
      Button::new(ls("Increment card")).on_press(set_count.update_callback(|value| value + 1)),
    ))
}
