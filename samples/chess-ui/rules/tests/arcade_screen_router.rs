use battlement::{
  AccessibilitySnapshot, AudioClipAddress, CheckedState, CommandBody, GameObjectKind, KeyEvent,
  KeyModifiers, MotionDescriptor, MotionEventBatch, MotionEventKind, MotionLayer,
  MotionLifecycleEvent, MotionSequence, NavigationEvent, ObjectId, PhysicalKey, Prop, SemanticRole,
  StyleValue, UiEvent, UiEventBody, UiFontAddress, UiVisualElementProperties, VisualElementAction,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::engine;

const ACTION_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/action");
const BACKGROUND_MUSIC: AudioClipAddress =
  AudioClipAddress::from_static("chess-ui/audio/drag-and-dread");
const DISPLAY_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/display");
const VALUE_FONT: UiFontAddress = UiFontAddress::from_static("chess-ui/fonts/control");

#[test]
fn complete_mockup_launches_directly_and_preserves_settings() {
  self::with_render_stack(self::full_screen_router_scenario);
}

#[test]
fn action_buttons_emit_native_streaks_and_respect_reduced_motion() {
  self::with_render_stack(|| {
    let mut client = self::client();
    self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
    let bursts = client
      .commands()
      .iter()
      .filter_map(|entry| match &entry.command.body {
        CommandBody::VisualElementPerformAction(action) => match &action.action {
          VisualElementAction::ParticleStreaks { streaks } => Some(streaks),
          _ => None,
        },
        _ => None,
      })
      .collect::<Vec<_>>();
    assert_eq!(bursts.len(), 1);
    assert_eq!(bursts[0].len(), 10);
    assert!(bursts[0].iter().all(|streak| streak.is_valid()));

    self::toggle(&mut client, "Reduce Motion");
    let start = client.commands().len();
    self::click_semantic(&mut client, SemanticRole::Button, "RETURN");
    self::click_semantic(&mut client, SemanticRole::Button, "ABOUT");
    assert!(client.commands()[start..].iter().any(|entry| matches!(
      &entry.command.body,
      CommandBody::VisualElementPerformAction(action)
        if matches!(&action.action, VisualElementAction::ParticleStreaks { streaks } if streaks.is_empty())
    )));
  });
}

fn full_screen_router_scenario() {
  let mut client = self::client();

  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
  let settings_heading = self::named(&mut client, "screen-header-heading");
  assert_eq!(
    client.ui().focused(),
    Some(settings_heading),
    "settings heading should receive route focus"
  );
  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  client.poll();
  self::semantic(&client, SemanticRole::Heading, "Settings");
  assert!(
    client.ui().focused().is_some(),
    "settings route should keep focus inside the application"
  );
  self::choose(&mut client, "Text Size 100%", "200%");
  self::toggle(&mut client, "Reduce Motion");
  self::click_semantic(&mut client, SemanticRole::Button, "RETURN");
  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
  self::assert_px(&mut client, "main-menu-actions", "top", 540.0);

  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  self::semantic(&client, SemanticRole::Button, "Text Size 200%");
  self::assert_checkbox(&client, "Reduce Motion", true);
  let return_button = self::semantic(&client, SemanticRole::Button, "RETURN");
  self::escape(&mut client, return_button);
  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
}

#[test]
fn controller_cancel_returns_to_main_menu_without_dismissing_the_app() {
  self::with_render_stack(self::controller_cancel_scenario);
}

fn controller_cancel_scenario() {
  let mut client = self::client();
  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");

  let return_button = self::semantic(&client, SemanticRole::Button, "RETURN");
  self::cancel(&mut client, return_button);
  let play = self::semantic(&client, SemanticRole::Button, "PLAY");
  self::cancel(&mut client, play);

  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
}

#[test]
fn play_and_quit_reach_terminal_black() {
  self::with_render_stack(|| {
    for (sequence, action) in ["PLAY", "QUIT"].into_iter().enumerate() {
      let mut client = self::client();
      self::click_semantic(&mut client, SemanticRole::Button, action);
      self::complete_exit(&mut client, (sequence + 1) as u64);
      self::semantic(&client, SemanticRole::Region, "Dismissed arcade stage");
    }
  });
}

fn with_render_stack(scenario: fn()) {
  std::thread::Builder::new()
    .name("complete-chess-ui-render".to_owned())
    .stack_size(16 * 1024 * 1024)
    .spawn(scenario)
    .expect("complete UI test thread")
    .join()
    .expect("complete UI scenario");
}

fn complete_exit(client: &mut FakeClient<App>, sequence: u64) {
  let descriptor = self::descriptor_named(client, "arcade-exit-content-surface");
  let slot = descriptor
    .slots
    .iter()
    .find(|slot| slot.layer == MotionLayer::Animate)
    .expect("exit animation slot");
  let sequence = MotionSequence(sequence);
  client.submit_motion(MotionEventBatch {
    first_sequence: sequence,
    last_sequence: sequence,
    events: vec![MotionLifecycleEvent {
      sequence,
      descriptor_id: descriptor.descriptor_id,
      slot: slot.slot,
      generation: slot.generation,
      elapsed_micros: 620_000,
      kind: MotionEventKind::Completed,
    }],
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  client.poll();
  client.poll();
}

fn descriptor_named(client: &mut FakeClient<App>, name: &str) -> MotionDescriptor {
  let id = self::named(client, name);
  let Prop::Set(descriptor) = client
    .ui()
    .element(id)
    .element()
    .visual_element()
    .motion
    .clone()
  else {
    panic!("missing motion descriptor")
  };
  descriptor
}

fn choose(client: &mut FakeClient<App>, trigger: &str, option: &str) {
  self::click_semantic(client, SemanticRole::Button, trigger);
  self::click_semantic(client, SemanticRole::Option, option);
}

fn toggle(client: &mut FakeClient<App>, label: &str) {
  let target = self::semantic(client, SemanticRole::Checkbox, label);
  client.ui().toggle_click(target);
  client.poll();
}

fn escape(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::KeyDown(KeyEvent {
      physical_key: Some(PhysicalKey::Escape),
      text: String::new(),
      modifiers: KeyModifiers::default(),
    }),
  ));
  client.poll();
}

fn cancel(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::NavigationCancel(NavigationEvent::default()),
  ));
  client.poll();
}

fn assert_checkbox(client: &FakeClient<App>, label: &str, expected: bool) {
  let node = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Checkbox && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing checkbox {label}"));
  assert_eq!(
    node.state.checked,
    Some(if expected {
      CheckedState::True
    } else {
      CheckedState::False
    })
  );
}

fn assert_px(client: &mut FakeClient<App>, name: &str, property: &str, expected: f32) {
  let id = self::named(client, name);
  let ui = client.ui();
  let style = ui.element(id).style();
  let value = match property {
    "top" => &style.top,
    _ => panic!("unknown property"),
  };
  assert!(matches!(
    value,
    Prop::Set(StyleValue::Value(battlement::LengthOrAuto::Px(actual)))
      if (*actual - expected).abs() < 0.01
  ));
}

fn click_semantic(client: &mut FakeClient<App>, role: SemanticRole, label: &str) {
  let target = self::semantic(client, role, label);
  client.ui().click(target);
  client.poll();
}

fn semantic(client: &FakeClient<App>, role: SemanticRole, label: &str) -> ObjectId {
  self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == role && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing {role:?} {label}"))
    .object_id
}

fn snapshot(client: &FakeClient<App>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("arcade screen semantics")
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  while let Some(id) = pending.pop() {
    let ui = client.ui();
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing {name}");
}

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_audio_clip(BACKGROUND_MUSIC);
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(DISPLAY_FONT);
  assets.add_ui_font(VALUE_FONT);
  assets.add_ui_font(ACTION_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}
