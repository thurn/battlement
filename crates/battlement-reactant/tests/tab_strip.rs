use battlement::{
  AccessibilitySnapshot, ClickEvent, CommandBody, ObjectId, SemanticRole, UiAccessibilityAction,
  UiAccessibilityActionEvent, UiEvent, UiEventBody,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, control_behavior, hooks, prelude::*};
use trox::ls;

struct ControlledStrip;

impl Component for ControlledStrip {
  fn render(&self) -> impl Render {
    let (proposals, set_proposals) = hooks::use_state(Vec::<u32>::new());
    View::new().child((
      TabStrip::new()
        .label(ls("Sections"))
        .selected_index(0)
        .on_select(move |index| {
          set_proposals.update(move |mut values| {
            values.push(index);
            values
          })
        })
        .children((
          TabButton::new().label(ls("First")).index(0),
          TabButton::new().label(ls("Second")).index(1),
          TabButton::new()
            .label(ls("Unavailable"))
            .index(2)
            .disabled(true),
        )),
      control_behavior::static_label(ls(format!("Proposals: {proposals:?}"))),
    ))
  }
}

#[test]
fn tab_buttons_propose_once_per_route_without_changing_parent_selection() {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("test/content");
  let mut client = FakeClient::connect(App::new("test/content").ui(ControlledStrip), assets);
  client.poll();
  let list = self::node(&client, "Sections");
  assert_eq!(
    self::snapshot(&client)
      .nodes
      .iter()
      .find(|node| node.object_id == list)
      .unwrap()
      .role,
    SemanticRole::TabList
  );
  for name in ["First", "Second", "Unavailable"] {
    let target = self::node(&client, name);
    if name != "Unavailable" {
      client.ui().click(target);
    }
    client.poll();
    client
      .ui()
      .send_event(UiEvent::click(target, ClickEvent::NavigationSubmit));
    client.poll();
    client.ui().send_event(UiEvent {
      target_id: target,
      cancelable: true,
      default_prevented: false,
      body: UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
        action: UiAccessibilityAction::Activate,
        backend_generation: 1,
      }),
    });
    client.poll();
  }
  self::node(&client, "Proposals: [0, 0, 0, 1, 1, 1]");
  let snapshot = self::snapshot(&client);
  assert_eq!(
    snapshot
      .nodes
      .iter()
      .filter(|node| node.role == SemanticRole::Tab)
      .count(),
    3
  );
  assert!(
    !snapshot
      .nodes
      .iter()
      .any(|node| node.role == SemanticRole::TabPanel)
  );
  for (name, selected, disabled) in [
    ("First", true, false),
    ("Second", false, false),
    ("Unavailable", false, true),
  ] {
    let node = snapshot
      .nodes
      .iter()
      .find(|node| node.label.as_deref() == Some(name))
      .unwrap();
    assert_eq!(node.parent_id, Some(list));
    assert_eq!(node.state.selected, Some(selected));
    assert_eq!(node.state.disabled, disabled);
  }
}

fn node(client: &FakeClient<App>, name: &str) -> ObjectId {
  self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some(name))
    .unwrap_or_else(|| panic!("missing {name}"))
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
    .expect("committed semantics")
}
