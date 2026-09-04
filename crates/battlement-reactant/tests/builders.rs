mod app_support;

use std::{cell::Cell, rc::Rc};
use trox::ls;

use battlement_fake::client::FakeClient;
use battlement_reactant::{app::App, prelude::*};

#[builder]
struct Action {
  #[builder(required)]
  clicked: EventCallback<()>,
  label: String,
}

#[builder]
struct Forward {
  #[builder(required)]
  clicked: EventCallback<()>,
}

#[builder]
struct OptionalAction {
  clicked: Option<EventCallback<()>>,
  changed: Option<EventCallback<u32>>,
}

#[builder]
struct SlottedCard {
  #[builder(required, into)]
  title: Child,
  #[builder(required, into)]
  children: Children,
}

impl Component for Action {
  fn render(&self) -> impl Render {
    Button::new(ls(self.label.clone()))
      .name("generated-action")
      .on_click(Forward::new().clicked(self.clicked.clone()).clicked)
  }
}

impl Component for OptionalAction {
  fn render(&self) -> impl Render {
    let button = Button::new(ls("Optional")).name("optional-action");
    match &self.clicked {
      Some(callback) => button.on_click(callback.clone()),
      None => button,
    }
  }
}

impl Component for SlottedCard {
  fn render(&self) -> impl Render {
    View::new().child((self.title.render(), self.children.render()))
  }
}

#[test]
fn child_slots_accept_and_replay_arbitrary_render_values() {
  let app = App::new("app/content").ui(
    SlottedCard::new()
      .title(Label::new(ls("Title")).name("slot-title"))
      .children((
        Label::new(ls("First")).name("slot-first"),
        Button::new(ls("Second")).name("slot-second"),
      )),
  );
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect_with(app, app_support::catalog(), app_support::connect());

  assert_eq!(app_support::text(&mut client, root, "slot-title"), "Title");
  assert_eq!(app_support::text(&mut client, root, "slot-first"), "First");
  assert_eq!(
    app_support::text(&mut client, root, "slot-second"),
    "Second"
  );
}

#[test]
fn optional_accessible_callbacks_have_a_builder_default() {
  let dialog = DialogOptions::new().name(ls("Settings"));

  assert!(dialog.on_dismiss.is_none());
}

#[test]
fn generated_event_props_forward_model_and_ordinary_callbacks_once() {
  let app = App::with_model("app/content", 0_u32).root(|value| {
    Action::new()
      .label(value.to_string())
      .clicked(|model: &mut u32| *model += 1)
  });
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect_with(app, app_support::catalog(), app_support::connect());
  let button = app_support::named(&mut client, root, "generated-action");
  client.ui().click(button);
  assert_eq!(
    app_support::text(&mut client, root, "generated-action"),
    "1"
  );
  client.ui().click(button);
  assert_eq!(
    app_support::text(&mut client, root, "generated-action"),
    "2"
  );

  let calls = Rc::new(Cell::new(0));
  let counter = Rc::clone(&calls);
  let action = Action::new().clicked(move || counter.set(counter.get() + 1));
  assert_eq!(calls.get(), 0);
  let app = App::new("app/content").ui(action);
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect_with(app, app_support::catalog(), app_support::connect());
  let button = app_support::named(&mut client, root, "generated-action");
  client.ui().click(button);
  assert_eq!(calls.get(), 1);
}

#[test]
fn optional_event_props_clear_without_invoking_or_subscribing() {
  let calls = Rc::new(Cell::new(0));
  let counter = Rc::clone(&calls);
  let props = OptionalAction::new()
    .clicked(move || counter.set(counter.get() + 1))
    .changed(|model: &mut u32, value: u32| *model += value)
    .clear_changed()
    .clear_clicked();
  assert!(props.changed.is_none());
  let app = App::new("app/content").ui(props);
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect_with(app, app_support::catalog(), app_support::connect());
  let button = app_support::named(&mut client, root, "optional-action");
  client.ui().click(button);
  assert_eq!(calls.get(), 0);
}

#[test]
#[should_panic(expected = "Reactant handler model type does not match its runtime")]
fn forwarded_event_props_preserve_model_mismatch_validation() {
  let action = Action::new().clicked(|_model: &mut String| {});
  let app = App::with_model("app/content", 0_u32)
    .root(move |_| Action::new().clicked(action.clicked.clone()));
  let _ = FakeClient::connect_with(app, app_support::catalog(), app_support::connect());
}
