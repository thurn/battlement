use battlement::{
  MotionEventBatch, MotionEventKind, MotionLifecycleEvent, MotionSequence, ObjectId, Prop,
  UiVisualElementProperties, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::app::App;
use reactant_testing::Display;

#[path = "../../../samples/reactant/rules/src/lifetime_proof.rs"]
mod lifetime_proof;

type Game = u32;
const CONTENT_SCENE: &str = "lifetime/fixture";
const ROOT_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000004");
mod model {
  pub fn new() -> u32 {
    0
  }
}

fn semantic(display: &Display<App<Game>>, label: &str) -> ObjectId {
  display
    .accessibility()
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing semantic {label}"))
    .object_id
}
fn click(display: &mut Display<App<Game>>, label: &str) {
  let id = self::semantic(display, label);
  display.deliver_ui_event(battlement::UiEvent {
    target_id: id,
    cancelable: true,
    default_prevented: false,
    body: battlement::UiEventBody::AccessibilityAction(battlement::UiAccessibilityActionEvent {
      backend_generation: 1,
      action: battlement::UiAccessibilityAction::Activate,
    }),
  });
  display.poll();
}

#[test]
fn native_fixture_shared_holds_survive_stale_callback_and_ancestor_updates() {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene(CONTENT_SCENE);
  let mut display = Display::connect(lifetime_proof::app(), assets);
  display.poll();
  self::click(&mut display, "Increment card");
  self::click(&mut display, "Hide card");
  self::click(&mut display, "Show card");
  let card = display
    .ui_element(self::semantic(&display, "Card count: 1"))
    .parent_id()
    .unwrap();
  self::click(&mut display, "Destroy card");
  let Prop::Set(descriptor) = display
    .ui_element(card)
    .element()
    .visual_element()
    .motion
    .clone()
  else {
    panic!("exit motion missing")
  };
  let events = descriptor
    .slots
    .iter()
    .enumerate()
    .map(|(index, slot)| MotionLifecycleEvent {
      sequence: MotionSequence(index as u64 + 1),
      descriptor_id: descriptor.descriptor_id,
      slot: slot.slot,
      generation: slot.generation,
      elapsed_micros: 400_000,
      kind: MotionEventKind::Completed,
    })
    .collect::<Vec<_>>();
  display.deliver_motion_events(MotionEventBatch {
    first_sequence: MotionSequence(1),
    last_sequence: MotionSequence(events.len() as u64),
    events,
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  display.poll();
  assert!(!display.contains_ui(card));
  self::click(&mut display, "Increment held");
  let held = self::semantic(&display, "Held count: 1");
  self::click(&mut display, "Hold and destroy");
  assert!(display.contains_ui(held));
  self::click(&mut display, "Deliver stale callback");
  assert!(
    display.contains_ui(held),
    "stale callback destroyed retained visual"
  );
  self::click(&mut display, "Release one use");
  assert!(
    display.contains_ui(held),
    "first release destroyed shared visual"
  );
  self::click(&mut display, "Release one use");
  assert!(!display.contains_ui(held));
}
