use battlement::{
  AccessibilitySnapshot, CommandBody, CurrentPage, ObjectId, SemanticRole, object_id,
};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
};

use crate::{
  assets,
  engine::{self, ChessUiEngine},
  pages,
};

const ROOT: ObjectId = object_id!("25310000-0000-4000-8000-000000000003");

#[test]
fn gallery_selection_recreates_each_harness_and_restores_heading_focus() {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(assets::addresses());
  let mut client = FakeClient::connect(engine::create_engine().unwrap(), assets);
  client.poll();
  self::assert_page(&mut client, 0);
  let change = self::named(&client.ui(), "demonstration-count");
  assert_eq!(client.ui().element(change).text(), Some("Changes: 0"));
  let button = self::snapshot(&client)
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Change demonstration"))
    .unwrap()
    .object_id;
  client.ui().click(button);
  client.poll();
  assert_eq!(client.ui().element(change).text(), Some("Changes: 1"));
  for index in 0..40 {
    for _ in 0..2 {
      let old_heading = self::named(&client.ui(), "page-heading");
      let target = self::named(&client.ui(), &format!("review-page-{}", index + 1));
      client.ui().click(target);
      client.poll();
      self::assert_page(&mut client, index);
      assert!(
        !client.ui().contains(old_heading),
        "selection must recreate the harness"
      );
      assert_eq!(client.ui().pointer_capture(0), None);
    }
  }
  client.reconnect();
  client.poll();
  self::assert_page(&mut client, 0);
  let count = self::named(&client.ui(), "demonstration-count");
  assert_eq!(client.ui().element(count).text(), Some("Changes: 0"));
}

fn assert_page(client: &mut FakeClient<ChessUiEngine>, index: usize) {
  let heading = self::named(&client.ui(), "page-heading");
  assert_eq!(client.ui().focused(), Some(heading));
  let semantics = self::snapshot(client);
  let current = semantics
    .nodes
    .iter()
    .filter(|node| node.state.current == Some(CurrentPage::Page))
    .collect::<Vec<_>>();
  assert_eq!(current.len(), 1);
  assert_eq!(
    current[0].label.as_deref(),
    Some(pages::ALL[index].semantic_target)
  );
  assert_eq!(
    semantics
      .nodes
      .iter()
      .filter(|node| node.role == SemanticRole::Button
        && node
          .label
          .as_deref()
          .is_some_and(|label| label.split_once(". ").is_some()))
      .count(),
    40
  );
  let navigation = semantics
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Navigation)
    .unwrap();
  assert_eq!(navigation.label.as_deref(), Some("Chess UI review pages"));
  assert_eq!(current[0].parent_id, Some(navigation.object_id));
  let region = semantics
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Region)
    .unwrap();
  assert_eq!(region.label.as_deref(), Some(pages::ALL[index].title));
}

fn snapshot(client: &FakeClient<ChessUiEngine>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("gallery semantics")
}

fn named(ui: &UiClient<'_, ChessUiEngine>, name: &str) -> ObjectId {
  let mut pending = vec![ROOT];
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing {name}");
}
