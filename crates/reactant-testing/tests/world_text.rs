use std::panic::{self, AssertUnwindSafe};

use battlement::{
  Color, GameObjectKind, HorizontalAlignment, ObjectId, ParentScene, Quaternion, RenderOrder,
  RgbColor, Vector3, VerticalAlignment, object_id,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{app::App, hooks, host::ButtonHost, prelude::*, world};
use reactant_testing::Display;
use trox::ls;

const TEXT: ObjectId = object_id!("32200000-0000-4000-8000-000000000001");
const BACK: ObjectId = object_id!("32200000-0000-4000-8000-000000000002");

const GROUP: ObjectId = object_id!("32200000-0000-4000-8000-000000000003");
const SPRITE: ObjectId = object_id!("32200000-0000-4000-8000-000000000004");

struct Scene;

impl Component for Scene {
  fn render(&self) -> impl Render {
    let (changed, change) = hooks::use_state(false);
    let (missing, load_missing) = hooks::use_state(false);
    let group = world::Group::new().id(*GROUP.as_uuid());
    let group = if changed { group.sort_order(8) } else { group };
    let text = world::Text::new();
    let text = if changed { text.layer(3) } else { text };
    (
      ButtonHost::new(ls("Change text"))
        .name("change")
        .on_click(change.update_callback(|value| !value)),
      ButtonHost::new(ls("Missing font"))
        .name("missing")
        .on_click(move || load_missing.set(true)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        group.child((
          text
            .id(*TEXT.as_uuid())
            .text(if missing {
              "Unavailable font content"
            } else if changed {
              "<b>Wide</b> words"
            } else {
              "Plain words"
            })
            .font(if missing {
              "test/missing"
            } else if changed {
              "test/font-b"
            } else {
              "test/font-a"
            })
            .size(if changed { 12.0 } else { 10.0 })
            .rich_text(changed)
            .wrapping(changed.then_some(2.5))
            .alignment(
              if changed {
                HorizontalAlignment::Right
              } else {
                HorizontalAlignment::Center
              },
              if changed {
                VerticalAlignment::Top
              } else {
                VerticalAlignment::Middle
              },
            )
            .tint(if changed {
              RgbColor::rgb(0.2, 0.5, 0.8)
            } else {
              RgbColor::WHITE
            })
            .opacity(if changed { 0.4 } else { 1.0 })
            .position(Vector3::new(1.0, 2.0, 0.0))
            .scale(Vector3::new(0.5, 0.75, 1.0))
            .rotation(Quaternion::new(0.0, 0.0, 1.0, 0.0)),
          world::Sprite::new()
            .id(*SPRITE.as_uuid())
            .texture("test/texture")
            .layer(if changed { 2 } else { 4 }),
          world::Text::new()
            .id(*BACK.as_uuid())
            .text("Back")
            .font("test/font-a")
            .rotation(Quaternion::new(0.0, 1.0, 0.0, 0.0)),
        )),
      ),
    )
  }
}

fn fixture() -> (Display<App>, ObjectId) {
  let app = App::new("test/scene").ui(Scene);
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("test/scene");
  assets.add_texture("test/texture");
  assets.add_text_mesh_pro_font("test/font-a");
  assets.add_text_mesh_pro_font("test/font-b");
  (Display::connect(app, assets), root)
}

#[test]
fn rich_text_updates_preserve_host_geometry_and_independent_back() {
  let (mut display, root) = self::fixture();
  let pose = display.object(TEXT).unwrap().local_transform();
  let back = display.object(BACK).unwrap().kind().clone();
  let time = display.presentation_time();
  let frame = display.frame();
  display.click_ui(display.find_ui(root, "change"));
  let GameObjectKind::Text { text } = display.object(TEXT).unwrap().kind() else {
    panic!("world text host");
  };
  assert_eq!(text.text, "<b>Wide</b> words");
  assert_eq!(text.font.as_str(), "test/font-b");
  assert_eq!(text.size, 12.0);
  assert!(text.rich_text);
  assert_eq!(text.wrap_width, Some(2.5));
  assert_eq!(text.horizontal, HorizontalAlignment::Right);
  assert_eq!(text.vertical, VerticalAlignment::Top);
  assert_eq!(text.color, Color::rgba(0.2, 0.5, 0.8, 0.4));
  assert_eq!(display.object(TEXT).unwrap().local_transform(), pose);
  assert_eq!(display.object(BACK).unwrap().kind(), &back);
  assert_eq!(display.presentation_time(), time);
  assert_eq!(display.frame(), frame);
  display.reconnect();
  assert!(
    matches!(display.object(TEXT).unwrap().kind(), GameObjectKind::Text { text } if text.font.as_str() == "test/font-b" && text.wrap_width == Some(2.5))
  );
  display.click_ui(display.find_ui(root, "change"));
  assert!(
    matches!(display.object(TEXT).unwrap().kind(), GameObjectKind::Text { text } if !text.rich_text && text.wrap_width.is_none())
  );
}

#[test]
fn missing_font_fails_preparation_before_changing_visible_text() {
  let (mut display, root) = self::fixture();
  let before = display.object(TEXT).unwrap().kind().clone();
  let target = display.find_ui(root, "missing");
  let failure = panic::catch_unwind(AssertUnwindSafe(|| display.click_ui(target)));
  let failure = failure.expect_err("missing prepared font must fail");
  let message = failure
    .downcast_ref::<String>()
    .map(String::as_str)
    .or_else(|| failure.downcast_ref::<&str>().copied())
    .unwrap_or("");
  assert!(message.contains("unknown prepared asset"), "{message}");
  assert_eq!(display.object(TEXT).unwrap().kind(), &before);
  assert!(display.object(BACK).is_some());
}

#[test]
fn mixed_visual_order_updates_and_resets_survive_reconnect_without_remounting() {
  let (mut display, root) = self::fixture();
  assert_eq!(display.object(GROUP).unwrap().render_order(), None);
  assert_eq!(display.object(TEXT).unwrap().render_order(), None);
  assert_eq!(
    display.object(SPRITE).unwrap().render_order(),
    Some(RenderOrder::Layer(4))
  );
  display.click_ui(display.find_ui(root, "change"));
  for reconnect in [false, true] {
    if reconnect {
      display.reconnect();
    }
    assert_eq!(
      display.object(GROUP).unwrap().render_order(),
      Some(RenderOrder::Group(8))
    );
    assert_eq!(
      display.object(TEXT).unwrap().render_order(),
      Some(RenderOrder::Layer(3))
    );
    assert_eq!(
      display.object(SPRITE).unwrap().render_order(),
      Some(RenderOrder::Layer(2))
    );
    assert_eq!(display.object(TEXT).unwrap().parent_id(), Some(GROUP));
    assert_eq!(display.object(SPRITE).unwrap().parent_id(), Some(GROUP));
    assert_eq!(display.object(BACK).unwrap().render_order(), None);
  }
  display.click_ui(display.find_ui(root, "change"));
  assert_eq!(display.object(GROUP).unwrap().render_order(), None);
  assert_eq!(display.object(TEXT).unwrap().render_order(), None);
  assert_eq!(
    display.object(SPRITE).unwrap().render_order(),
    Some(RenderOrder::Layer(4))
  );
}
