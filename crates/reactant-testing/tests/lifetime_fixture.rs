use battlement::{ObjectId, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use reactant_testing::Display;

#[path = "../../../samples/reactant/rules/src/lifetime_proof.rs"]
mod lifetime_proof;

const CONTENT_SCENE: &str = "lifetime/fixture";
const ROOT_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000004");

fn semantic(display: &Display, label: &str) -> ObjectId {
  display
    .accessibility()
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing semantic {label}"))
    .object_id
}
fn click(display: &mut Display, label: &str) {
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
  let mut display = Display::mount(lifetime_proof::app, assets);
  display.poll();
  self::click(&mut display, "Increment card");
  self::click(&mut display, "Hide card");
  self::click(&mut display, "Show card");
  let card = display
    .ui_element(self::semantic(&display, "Card count: 1"))
    .parent_id()
    .unwrap();
  self::click(&mut display, "Destroy card");
  display.advance_time(std::time::Duration::from_millis(400));
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
