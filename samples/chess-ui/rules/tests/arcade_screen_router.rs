use std::time::Duration;

use battlement::{
  AudioClipAddress, CheckedState, CommandBody, GameObjectKind, KeyEvent, KeyModifiers,
  NavigationEvent, ObjectId, PhysicalKey, Prop, SemanticRole, StyleValue, UiAccessibilityAction,
  UiAccessibilityActionEvent, UiEvent, UiEventBody, UiFontAddress, UiVisualElementProperties,
  VisualElementAction,
};
use battlement_fake::assets::FakeAssetCatalog;
use battlement_rules::engine;
use reactant::{app::App, asset_generator};
use reactant_testing::Display;

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
    client.focused(),
    Some(settings_heading),
    "settings heading should receive route focus"
  );
  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  client.poll();
  self::semantic(&client, SemanticRole::Heading, "Settings");
  assert!(
    client.focused().is_some(),
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

#[test]
fn keyboard_rebinding_uses_accessible_actions_and_native_key_input() {
  self::with_render_stack(|| {
    let mut client = self::client();
    self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
    self::click_semantic(&mut client, SemanticRole::Tab, "Input");
    self::click_semantic(
      &mut client,
      SemanticRole::Button,
      "Change Left keyboard binding",
    );

    self::semantic(
      &client,
      SemanticRole::StaticText,
      "Waiting for keyboard input",
    );
    let capture = self::named(&mut client, "shortcut-waiting-marker");
    client.deliver_ui_event(UiEvent::new(
      capture,
      true,
      false,
      UiEventBody::KeyDown(KeyEvent {
        physical_key: Some(PhysicalKey::KeyA),
        text: "a".to_owned(),
        modifiers: KeyModifiers::default(),
      }),
    ));
    for _ in 0..4 {
      client.poll();
    }

    let label = self::named(&mut client, "keyboard-binding-label-0");
    assert_eq!(client.ui_element(label).text(), Some("A"));
  });
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
    for action in ["PLAY", "QUIT"] {
      let mut client = self::client();
      self::click_semantic(&mut client, SemanticRole::Button, action);
      client.advance_time(Duration::from_millis(620));
      client.poll();
      self::semantic(&client, SemanticRole::Region, "Dismissed arcade stage");
    }
  });
}

#[test]
fn menu_actions_pulse_from_music_and_respect_reduce_motion() {
  self::with_render_stack(|| {
    let mut client = self::client();
    for name in ["play", "settings", "about", "quit", "music-indicator"] {
      let host = if name == "music-indicator" {
        "main-menu-music-indicator".to_owned()
      } else {
        format!("main-menu-action-{name}")
      };
      let container = self::named(&mut client, &host);
      self::assert_music_pulse(&mut client, container, true);
    }
    self::click_semantic(&mut client, SemanticRole::Button, "Mute background music");
    let play = self::named(&mut client, "main-menu-action-play");
    self::assert_music_pulse(&mut client, play, false);
    self::click_semantic(&mut client, SemanticRole::Button, "Enable background music");
    self::assert_music_pulse(&mut client, play, true);
    self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
    let late = self::semantic(&client, SemanticRole::Button, "RETURN");
    self::assert_music_pulse(&mut client, late, true);
    self::toggle(&mut client, "Reduce Motion");
    self::assert_music_pulse(&mut client, late, false);
    self::click_semantic(&mut client, SemanticRole::Button, "RETURN");
    let play = self::named(&mut client, "main-menu-action-play");
    self::assert_music_pulse(&mut client, play, false);
  });
}

fn assert_music_pulse(client: &mut Display<App>, root: ObjectId, enabled: bool) {
  let mut pending = vec![root];
  while let Some(id) = pending.pop() {
    let element = client.ui_element(id);
    if matches!(
      element.name(),
      Some("action-button" | "music-playback-indicator")
    ) {
      let properties = element.element().visual_element();
      let Prop::Set(descriptor) = &properties.motion else {
        assert!(!enabled, "playing controls must have a heartbeat");
        return;
      };
      let composed = descriptor
        .value_bindings
        .iter()
        .filter(|binding| binding.composition == battlement::MotionBindingComposition::Compose)
        .collect::<Vec<_>>();
      assert_eq!(composed.len(), if enabled { 1 } else { 0 });
      if enabled {
        assert!(descriptor.values.iter().any(|value| matches!(
          value.source,
          battlement::MotionValueSource::Time(battlement::MotionClockSource::Audio(_))
        )));
        assert!(
          descriptor.value_subscriptions.is_empty(),
          "presentation must not subscribe Rust to every frame"
        );
        assert!(
          composed
            .iter()
            .any(|binding| binding.property == battlement::MotionProperty::Scale)
        );
      }
      return;
    }
    pending.extend(element.children());
  }
  panic!("control must contain a production button host");
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

fn choose(client: &mut Display<App>, trigger: &str, option: &str) {
  self::click_semantic(client, SemanticRole::Button, trigger);
  self::click_semantic(client, SemanticRole::Option, option);
}

fn toggle(client: &mut Display<App>, label: &str) {
  let target = self::semantic(client, SemanticRole::Checkbox, label);
  self::activate_semantic(client, target);
  client.poll();
}

fn escape(client: &mut Display<App>, target: ObjectId) {
  client.deliver_ui_event(UiEvent::new(
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

fn cancel(client: &mut Display<App>, target: ObjectId) {
  client.deliver_ui_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::NavigationCancel(NavigationEvent::default()),
  ));
  client.poll();
}

fn assert_checkbox(client: &Display<App>, label: &str, expected: bool) {
  let node = client
    .accessibility()
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

fn assert_px(client: &mut Display<App>, name: &str, property: &str, expected: f32) {
  let id = self::named(client, name);
  let style = client.ui_element(id).style();
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

fn click_semantic(client: &mut Display<App>, role: SemanticRole, label: &str) {
  let target = self::semantic(client, role, label);
  self::activate_semantic(client, target);
  client.poll();
}

fn activate_semantic(client: &mut Display<App>, target: ObjectId) {
  client.deliver_ui_event(UiEvent {
    target_id: target,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::Activate,
    }),
  });
}

fn semantic(client: &Display<App>, role: SemanticRole, label: &str) -> ObjectId {
  client
    .accessibility()
    .nodes
    .iter()
    .find(|node| node.role == role && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing {role:?} {label}"))
    .object_id
}

fn named(client: &mut Display<App>, name: &str) -> ObjectId {
  let root = client
    .objects()
    .find_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .expect("chess UI document");
  client.find_ui(root, name)
}

fn client() -> Display<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_audio_clip(BACKGROUND_MUSIC);
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(DISPLAY_FONT);
  assets.add_ui_font(VALUE_FONT);
  assets.add_ui_font(ACTION_FONT);
  let mut client = Display::connect(engine::create_engine(), assets);
  client.poll();
  client
}
