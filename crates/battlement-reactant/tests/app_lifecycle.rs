mod app_support;

use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::{
  ActionId, Command, GameObjectKind, ResponseMessage, SessionId, UiEventAction, UiEventDisposition,
  Validate,
};
use battlement_fake::client::FakeClient;
use battlement_native::Engine;
use battlement_reactant::{app::App, prelude::*};

struct Counter {
  cleanups: Rc<Cell<u32>>,
  handle: Rc<RefCell<Option<AppHandle>>>,
}

struct Mixed {
  value: u32,
}

impl Component for Counter {
  fn render(&self) -> impl Render {
    let (count, set) = use_state(0);
    let cleanup = Rc::clone(&self.cleanups);
    let capture = Rc::clone(&self.handle);
    let app = use_app();
    use_effect(
      move || {
        *capture.borrow_mut() = Some(app);
        move || cleanup.set(cleanup.get() + 1)
      },
      (),
    );
    View::new().child((
      Label::new(format!("{count}")).name("count"),
      Button::new("Increment")
        .name("increment")
        .on_click(move || set.update(|value| value + 1)),
      Button::new("Prevent")
        .name("prevent")
        .on_click_event(|event| event.prevent_default()),
    ))
  }
}

impl Component for Mixed {
  fn render(&self) -> impl Render {
    let (enabled, set) = use_state(false);
    let input = use_element_ref();
    let toggle = use_checkbox(ToggleOptions {
      name: text("Enabled"),
      description: None,
      checked: enabled,
      is_disabled: false,
      on_change: move |value| set.set(value),
    });
    let label = toggle.label_interaction(&input);
    View::new().child((
      Label::new(format!("{} {enabled}", self.value)).name("value"),
      Button::new("Game")
        .name("game")
        .on_click(|model: &mut u32| *model += 1),
      View::new()
        .name("enabled-label")
        .interaction_props(label)
        .child(
          Button::new("Enabled")
            .name("enabled")
            .element_ref(input)
            .semantic(toggle.semantic)
            .focus_props(toggle.focus)
            .interaction_props(toggle.interaction),
        ),
    ))
  }
}

#[test]
fn generated_application_snapshot_and_mixed_callbacks_work_without_a_custom_engine() {
  let app = App::with_model("app/content", 0_u32).root(|value| Mixed { value: *value });
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect_with(app, app_support::catalog(), app_support::connect());
  let game = app_support::named(&mut client, root, "game");
  let toggle = app_support::named(&mut client, root, "enabled");
  client.ui().click(game);
  client.ui().click(toggle);
  assert_eq!(app_support::text(&mut client, root, "value"), "1 true");
  let label = app_support::named(&mut client, root, "enabled-label");
  client.ui().send_event(app_support::click(label));
  assert_eq!(app_support::text(&mut client, root, "value"), "1 false");
  client.ui().click(toggle);
  client.reconnect();
  assert_eq!(app_support::text(&mut client, root, "value"), "1 true");
  assert_eq!(
    client
      .world()
      .objects()
      .filter(|object| matches!(object.kind(), GameObjectKind::UiDocument(_)))
      .count(),
    1
  );
}

#[test]
fn reconnect_policy_controls_remounts_and_drop_runs_cleanup_once() {
  for reset in [false, true] {
    let cleanups = Rc::new(Cell::new(0));
    let handle = Rc::new(RefCell::new(None));
    let app = App::new("app/content").ui(Counter {
      cleanups: Rc::clone(&cleanups),
      handle: Rc::clone(&handle),
    });
    let root = app.root_document().root_id;
    let app = if reset { app.reset_on_reconnect() } else { app };
    let mut client = FakeClient::connect(app, app_support::catalog());
    client.poll();
    let increment = app_support::named(&mut client, root, "increment");
    client.ui().click(increment);
    let old_handle = handle.borrow().clone().unwrap();
    client.reconnect();
    client.poll();
    assert_eq!(
      app_support::text(&mut client, root, "count"),
      if reset { "0" } else { "1" }
    );
    assert_eq!(cleanups.get(), u32::from(reset));
    old_handle.send(Command::open_external_url("https://example.com/stale"));
    client.poll();
    assert!(!client.commands().iter().any(|entry| matches!(
      entry.command.body,
      battlement::CommandBody::ApplicationOpenUrl(_)
    )));
    drop(client);
    assert_eq!(cleanups.get(), 1 + u32::from(reset));
  }
}

#[test]
fn ui_disposition_is_synchronous_and_old_session_events_are_rejected() {
  let mut app = App::new("app/content").ui(Counter {
    cleanups: Rc::default(),
    handle: Rc::default(),
  });
  let response = app.connect(app_support::connect()).unwrap();
  let ResponseMessage::Snapshot(snapshot) = &response.messages[0] else {
    panic!("initial snapshot");
  };
  snapshot.validate().unwrap();
  let button = snapshot.ui[0].children[0].children[2].object_id;
  let action = UiEventAction::new(
    ActionId::new_v4(),
    response.session_id,
    app_support::click(button),
  );
  let event = app.submit_ui_event(action.clone()).unwrap();
  assert_eq!(event.disposition, UiEventDisposition::PreventDefault);
  let next = app.connect(app_support::connect()).unwrap();
  assert_ne!(next.session_id, response.session_id);
  assert!(app.submit_ui_event(action).is_err());
  assert!(
    app
      .submit_ui_event(UiEventAction::new(
        ActionId::new_v4(),
        SessionId::new_v4(),
        app_support::click(button)
      ))
      .is_err()
  );
}

struct Commands;

impl Component for Commands {
  fn render(&self) -> impl Render {
    let (count, set) = use_state(0);
    let app = use_app();
    let deferred = app.clone();
    use_effect(
      move || {
        if count > 0 {
          deferred.send(Command::open_external_url("https://example.com/effect"));
        }
      },
      count,
    );
    Button::new("Send").on_click(move || {
      app.send(Command::open_external_url("https://example.com/click"));
      set.update(|count| count + 1);
    })
  }
}

#[test]
fn native_commands_keep_action_attribution_through_deferred_effects() {
  let mut app = App::new("app/content").ui(Commands);
  let initial = app.connect(app_support::connect()).unwrap();
  let ResponseMessage::Snapshot(snapshot) = &initial.messages[0] else {
    panic!("snapshot");
  };
  let button = snapshot.ui[0].children[0].object_id;
  app.poll().unwrap();
  for _ in 0..2 {
    let action = ActionId::new_v4();
    let event = app
      .submit_ui_event(UiEventAction::new(
        action,
        initial.session_id,
        app_support::click(button),
      ))
      .unwrap();
    self::assert_command_action(&event.response, action);
    self::assert_command_action(&app.poll().unwrap().expect("effect response"), action);
  }
}

fn assert_command_action(response: &battlement::Response, action: ActionId) {
  let batch = response
    .messages
    .iter()
    .find_map(|message| match message {
      ResponseMessage::Batch(batch)
        if batch
          .groups
          .iter()
          .flat_map(|group| &group.commands)
          .any(|command| {
            matches!(command.body, battlement::CommandBody::ApplicationOpenUrl(_))
          }) =>
      {
        Some(batch)
      }
      _ => None,
    })
    .expect("native command");
  assert_eq!(batch.caused_by_action_id, Some(action));
}

#[test]
fn back_to_back_actions_do_not_steal_deferred_effect_attribution() {
  let mut app = App::new("app/content").ui(Commands);
  let initial = app.connect(app_support::connect()).unwrap();
  let ResponseMessage::Snapshot(snapshot) = &initial.messages[0] else {
    panic!("snapshot")
  };
  let button = snapshot.ui[0].children[0].object_id;
  app.poll().unwrap();
  let first = ActionId::new_v4();
  let second = ActionId::new_v4();
  app
    .submit_ui_event(UiEventAction::new(
      first,
      initial.session_id,
      app_support::click(button),
    ))
    .unwrap();
  let response = app
    .submit_ui_event(UiEventAction::new(
      second,
      initial.session_id,
      app_support::click(button),
    ))
    .unwrap()
    .response;
  let actions: Vec<_> = response
    .messages
    .iter()
    .filter_map(|message| match message {
      ResponseMessage::Batch(batch)
        if batch
          .groups
          .iter()
          .flat_map(|group| &group.commands)
          .any(|command| {
            matches!(command.body, battlement::CommandBody::ApplicationOpenUrl(_))
          }) =>
      {
        Some(batch.caused_by_action_id)
      }
      _ => None,
    })
    .collect();
  assert_eq!(actions, vec![Some(first), Some(second)]);
}

struct RefreshFocus;

impl Component for RefreshFocus {
  fn render(&self) -> impl Render {
    let target = use_element_ref();
    let app = use_app();
    View::new().child((
      Button::new("Target")
        .name("target")
        .element_ref(target.clone()),
      Button::new("Refresh").name("refresh").on_click(move || {
        target.focus();
        app.refresh_snapshot();
      }),
    ))
  }
}

#[test]
fn snapshot_refresh_delivers_focus_after_the_replacement_document() {
  let app = App::new("app/content").ui(RefreshFocus);
  let root = app.root_document().root_id;
  let mut client = FakeClient::connect(app, app_support::catalog());
  let refresh = app_support::named(&mut client, root, "refresh");
  let target = app_support::named(&mut client, root, "target");
  client.ui().click(refresh);
  assert_eq!(client.ui().focused(), Some(target));
}
