use battlement_types::{Color, MaterialAddress, ObjectId, SpriteAddress, TextureAddress};
use battlement_ui::{
  BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat, BackgroundRepeatMode,
  BackgroundSize, BackgroundSource, Cursor, CursorHotspot, Display, LengthUnits, Overflow,
  OverflowClipBox, Prop, SliceType, Style, StyleValue, UiBox, UiDocument, UiElement, UiNode,
  Visibility, VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn appearance_updates_merge_atomically_and_move_material_usage() {
  let target_id = ObjectId::new_v4();
  let initial_material = MaterialAddress::new("ui/material/initial");
  let replacement_material = MaterialAddress::new("ui/material/replacement");
  let initial_background = BackgroundSource::Sprite(SpriteAddress::new("ui/panel/initial"));
  let replacement_background =
    BackgroundSource::Texture(TextureAddress::new("ui/panel/replacement"));
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        target_id,
        UiBox::new().style(
          Style::new()
            .background_image(initial_background.clone())
            .border_color(Color::rgb(0.2, 0.8, 0.9))
            .border_radius((8, 16))
            .border_width(2)
            .display(Display::Flex)
            .overflow(Overflow::Hidden)
            .unity_material(initial_material.clone())
            .visibility(Visibility::Visible),
        ),
      )),
    ])
    .unwrap();

  assert_eq!(world.material_usage_count(&initial_material), 1);
  assert_eq!(world.background_usage_count(&initial_background), 1);
  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        UiBox::default().style(
          Style::new()
            .background_image(replacement_background.clone())
            .opacity(0.5)
            .unity_material(replacement_material.clone())
            .visibility(Visibility::Hidden),
        ),
      )
      .into(),
    })
    .unwrap();

  let committed = world.element(target_id).unwrap().style().clone();
  assert_eq!(
    committed.display,
    Prop::Set(StyleValue::Value(Display::Flex))
  );
  assert_eq!(committed.opacity, Prop::Set(StyleValue::Value(0.5.into())));
  assert_eq!(
    committed.visibility,
    Prop::Set(StyleValue::Value(Visibility::Hidden))
  );
  assert_eq!(world.material_usage_count(&initial_material), 0);
  assert_eq!(world.material_usage_count(&replacement_material), 1);
  assert_eq!(world.background_usage_count(&initial_background), 0);
  assert_eq!(world.background_usage_count(&replacement_background), 1);

  assert_eq!(
    world.update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(Style::new().border_left_width(-1))).into(),
    }),
    Err(UiWorldError::InvalidProperty)
  );
  assert_eq!(world.element(target_id).unwrap().style(), &committed);
  assert_eq!(world.material_usage_count(&replacement_material), 1);
  assert_eq!(world.background_usage_count(&replacement_background), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(Style::new().unity_material(Prop::Reset)))
        .into(),
    })
    .unwrap();
  assert_eq!(world.material_usage_count(&replacement_material), 0);
  assert_eq!(world.background_usage_count(&replacement_background), 1);

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(Style::new().background_image(Prop::Reset)))
        .into(),
    })
    .unwrap();
  assert_eq!(world.background_usage_count(&replacement_background), 0);

  world.destroy(target_id).unwrap();
  assert_eq!(world.material_usage_count(&replacement_material), 0);
  assert_eq!(world.background_usage_count(&replacement_background), 0);
}

macro_rules! assert_paint_fields {
  ($style:expr, $pattern:pat) => {
    assert!(matches!($style.background_color, $pattern));
    assert!(matches!($style.background_image, $pattern));
    assert!(matches!($style.background_position_x, $pattern));
    assert!(matches!($style.background_position_y, $pattern));
    assert!(matches!($style.background_repeat, $pattern));
    assert!(matches!($style.background_size, $pattern));
    assert!(matches!($style.border_bottom_color, $pattern));
    assert!(matches!($style.border_bottom_left_radius, $pattern));
    assert!(matches!($style.border_bottom_right_radius, $pattern));
    assert!(matches!($style.border_left_color, $pattern));
    assert!(matches!($style.border_right_color, $pattern));
    assert!(matches!($style.border_top_color, $pattern));
    assert!(matches!($style.border_top_left_radius, $pattern));
    assert!(matches!($style.border_top_right_radius, $pattern));
    assert!(matches!($style.color, $pattern));
    assert!(matches!($style.cursor, $pattern));
    assert!(matches!($style.opacity, $pattern));
    assert!(matches!($style.unity_background_image_tint_color, $pattern));
    assert!(matches!($style.unity_material, $pattern));
    assert!(matches!($style.unity_overflow_clip_box, $pattern));
    assert!(matches!($style.unity_slice_bottom, $pattern));
    assert!(matches!($style.unity_slice_left, $pattern));
    assert!(matches!($style.unity_slice_right, $pattern));
    assert!(matches!($style.unity_slice_scale, $pattern));
    assert!(matches!($style.unity_slice_top, $pattern));
    assert!(matches!($style.unity_slice_type, $pattern));
  };
}

#[test]
fn every_paint_family_sets_resets_and_preserves_omitted_layout() {
  let target_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(target_id, UiBox::new().style(assigned_paint().width(240))),
    )])
    .unwrap();
  assert_paint_fields!(world.element(target_id).unwrap().style(), Prop::Set(_));

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(reset_paint())).into(),
    })
    .unwrap();
  let reset = world.element(target_id).unwrap().style();
  assert_paint_fields!(reset, Prop::Reset);
  assert_eq!(
    reset.width,
    Prop::Set(StyleValue::Value(battlement_ui::LengthOrAuto::Px(240.0)))
  );
}

fn assigned_paint() -> Style {
  Style::new()
    .background_color(Color::rgb(0.1, 0.2, 0.3))
    .background_image(BackgroundSource::Texture(TextureAddress::new("ui/paint")))
    .background_position_x(BackgroundPosition::new(
      BackgroundPositionKeyword::Right,
      4.pct(),
    ))
    .background_position_y(BackgroundPosition::new(
      BackgroundPositionKeyword::Bottom,
      5,
    ))
    .background_repeat(BackgroundRepeat::new(
      BackgroundRepeatMode::Round,
      BackgroundRepeatMode::Space,
    ))
    .background_size(BackgroundSize::axes(50.pct(), 60))
    .border_color(Color::rgb(0.4, 0.5, 0.6))
    .border_radius((1, 2, 3, 4))
    .color(Color::rgb(0.7, 0.8, 0.9))
    .cursor(Cursor::texture(
      TextureAddress::new("ui/cursor"),
      CursorHotspot::new(1.0, 2.0),
    ))
    .opacity(0.75)
    .unity_background_image_tint_color(Color::rgb(0.8, 0.7, 0.6))
    .unity_material(MaterialAddress::new("ui/material"))
    .unity_overflow_clip_box(OverflowClipBox::ContentBox)
    .unity_slice_bottom(1)
    .unity_slice_left(2)
    .unity_slice_right(3)
    .unity_slice_scale(2)
    .unity_slice_top(4)
    .unity_slice_type(SliceType::Tiled)
}

fn reset_paint() -> Style {
  Style::new()
    .background_color(Prop::Reset)
    .background_image(Prop::Reset)
    .background_position_x(Prop::Reset)
    .background_position_y(Prop::Reset)
    .background_repeat(Prop::Reset)
    .background_size(Prop::Reset)
    .border_bottom_color(Prop::Reset)
    .border_bottom_left_radius(Prop::Reset)
    .border_bottom_right_radius(Prop::Reset)
    .border_left_color(Prop::Reset)
    .border_right_color(Prop::Reset)
    .border_top_color(Prop::Reset)
    .border_top_left_radius(Prop::Reset)
    .border_top_right_radius(Prop::Reset)
    .color(Prop::Reset)
    .cursor(Prop::Reset)
    .opacity(Prop::Reset)
    .unity_background_image_tint_color(Prop::Reset)
    .unity_material(Prop::Reset)
    .unity_overflow_clip_box(Prop::Reset)
    .unity_slice_bottom(Prop::Reset)
    .unity_slice_left(Prop::Reset)
    .unity_slice_right(Prop::Reset)
    .unity_slice_scale(Prop::Reset)
    .unity_slice_top(Prop::Reset)
    .unity_slice_type(Prop::Reset)
}
