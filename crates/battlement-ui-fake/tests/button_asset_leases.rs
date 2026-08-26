use battlement_types::{ObjectId, SpriteAddress, TextureAddress};
use battlement_ui::{Button, IconSource, UiDocument, UiNode, VisualElementUpdate};
use battlement_ui_fake::UiWorld;

#[test]
fn button_icon_usage_follows_sparse_replacement_and_destruction() {
    let button_id = ObjectId::new_v4();
    let texture = IconSource::Texture(TextureAddress::new("ui/icon-texture"));
    let sprite = IconSource::Sprite(SpriteAddress::new("ui/icon-sprite"));
    let mut world = UiWorld::default();
    world
        .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
            UiNode::new(button_id, Button::new("Command").icon(texture.clone())),
        )])
        .unwrap();

    assert_eq!(world.icon_usage_count(&texture), 1);
    world
        .update(VisualElementUpdate::Properties {
            object_id: button_id,
            element: std::boxed::Box::new(Button::default().icon(sprite.clone()).into()),
        })
        .unwrap();
    assert_eq!(world.icon_usage_count(&texture), 0);
    assert_eq!(world.icon_usage_count(&sprite), 1);

    world.destroy(button_id).unwrap();
    assert_eq!(world.icon_usage_count(&sprite), 0);
}
