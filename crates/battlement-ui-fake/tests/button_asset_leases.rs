use battlement_types::{ObjectId, SpriteAddress, TextureAddress};
use battlement_ui::{
  IconSource, Prop, UiButton, UiDocument, UiNode, UiTab, UiTabView, VisualElementUpdate,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld};

#[test]
fn button_icon_usage_follows_sparse_replacement_and_destruction() {
  let button_id = ObjectId::new_v4();
  let texture = IconSource::Texture(TextureAddress::new("ui/icon-texture"));
  let sprite = IconSource::Sprite(SpriteAddress::new("ui/icon-sprite"));
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(button_id, UiButton::new("Command").icon(texture.clone())),
    )])
    .unwrap();

  assert_eq!(world.icon_usage_count(&texture), 1);
  world
    .update(VisualElementUpdate::Properties {
      object_id: button_id,
      element: std::boxed::Box::new(UiButton::default().icon(sprite.clone()).into()),
    })
    .unwrap();
  assert_eq!(world.icon_usage_count(&texture), 0);
  assert_eq!(world.icon_usage_count(&sprite), 1);

  world.destroy(button_id).unwrap();
  assert_eq!(world.icon_usage_count(&sprite), 0);
}

#[test]
fn tab_icon_usage_follows_sparse_replacement_and_destruction() {
  let tab_view_id = ObjectId::new_v4();
  let tab_id = ObjectId::new_v4();
  let texture = IconSource::Texture(TextureAddress::new("ui/tab-texture"));
  let sprite = IconSource::Sprite(SpriteAddress::new("ui/tab-sprite"));
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(tab_view_id, UiTabView::new().selected_tab_index(0)).child(UiNode::new(
        tab_id,
        UiTab::new("Loadout").icon(texture.clone()),
      )),
    )])
    .unwrap();

  assert_eq!(world.icon_usage_count(&texture), 1);
  world
    .update(VisualElementUpdate::Properties {
      object_id: tab_id,
      element: std::boxed::Box::new(UiTab::default().icon(sprite.clone()).into()),
    })
    .unwrap();
  assert_eq!(world.icon_usage_count(&texture), 0);
  assert_eq!(world.icon_usage_count(&sprite), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: tab_id,
      element: std::boxed::Box::new(UiTab::default().icon(Prop::Reset).into()),
    })
    .unwrap();
  assert_eq!(world.element(tab_id).unwrap().icon_source(), None);
  assert_eq!(world.icon_usage_count(&sprite), 0);

  world.destroy(tab_id).unwrap();
  assert_eq!(world.icon_usage_count(&sprite), 0);
}

#[test]
fn button_content_resets_without_recreating_the_element() {
  let button_id = ObjectId::new_v4();
  let icon = IconSource::Texture(TextureAddress::new("ui/resettable-icon"));
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(button_id, UiButton::new("Deploy").icon(icon.clone())),
    )])
    .unwrap();

  world
    .update(VisualElementUpdate::Properties {
      object_id: button_id,
      element: std::boxed::Box::new(UiButton::default().into()),
    })
    .unwrap();
  assert_eq!(world.element(button_id).unwrap().text(), Some("Deploy"));

  world
    .update(VisualElementUpdate::Properties {
      object_id: button_id,
      element: std::boxed::Box::new(
        UiButton::default()
          .text(Prop::Reset)
          .icon(Prop::Reset)
          .into(),
      ),
    })
    .unwrap();

  let state = world.element(button_id).unwrap();
  assert_eq!(state.object_id(), button_id);
  assert_eq!(state.text(), None);
  assert_eq!(state.icon_source(), None);
  assert_eq!(world.icon_usage_count(&icon), 0);
  assert!(matches!(
    world.journal().last(),
    Some(UiJournalEntry::Update(_))
  ));
}
