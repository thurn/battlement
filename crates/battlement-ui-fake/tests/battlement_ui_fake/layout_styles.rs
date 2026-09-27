use battlement_types::{Color, ObjectId};
use battlement_ui::{
  Align, AspectRatio, Display, FlexDirection, FlexWrap, Justify, LengthOrAuto, LengthUnits,
  Overflow, Position, Prop, Style, StyleValue, UiBox, UiDocument, UiElement, UiNode,
  VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

macro_rules! assert_layout_fields {
  ($style:expr, $pattern:pat) => {
    assert!(matches!($style.align_content, $pattern));
    assert!(matches!($style.align_items, $pattern));
    assert!(matches!($style.align_self, $pattern));
    assert!(matches!($style.aspect_ratio, $pattern));
    assert!(matches!($style.border_bottom_width, $pattern));
    assert!(matches!($style.border_left_width, $pattern));
    assert!(matches!($style.border_right_width, $pattern));
    assert!(matches!($style.border_top_width, $pattern));
    assert!(matches!($style.bottom, $pattern));
    assert!(matches!($style.display, $pattern));
    assert!(matches!($style.flex_basis, $pattern));
    assert!(matches!($style.flex_direction, $pattern));
    assert!(matches!($style.flex_grow, $pattern));
    assert!(matches!($style.flex_shrink, $pattern));
    assert!(matches!($style.flex_wrap, $pattern));
    assert!(matches!($style.height, $pattern));
    assert!(matches!($style.justify_content, $pattern));
    assert!(matches!($style.left, $pattern));
    assert!(matches!($style.margin_bottom, $pattern));
    assert!(matches!($style.margin_left, $pattern));
    assert!(matches!($style.margin_right, $pattern));
    assert!(matches!($style.margin_top, $pattern));
    assert!(matches!($style.max_height, $pattern));
    assert!(matches!($style.max_width, $pattern));
    assert!(matches!($style.min_height, $pattern));
    assert!(matches!($style.min_width, $pattern));
    assert!(matches!($style.overflow, $pattern));
    assert!(matches!($style.padding_bottom, $pattern));
    assert!(matches!($style.padding_left, $pattern));
    assert!(matches!($style.padding_right, $pattern));
    assert!(matches!($style.padding_top, $pattern));
    assert!(matches!($style.position, $pattern));
    assert!(matches!($style.right, $pattern));
    assert!(matches!($style.top, $pattern));
    assert!(matches!($style.width, $pattern));
  };
}

#[test]
fn layout_updates_merge_sparse_fields_and_reject_invalid_values_atomically() {
  let target_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        target_id,
        UiBox::new().style(
          Style::new()
            .flex_direction(FlexDirection::Row)
            .flex_wrap(FlexWrap::Wrap)
            .width(80.pct())
            .padding((12, 20)),
        ),
      )),
    ])
    .unwrap();

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(
        UiBox::default().style(Style::new().flex_direction(FlexDirection::ColumnReverse)),
      )
      .into(),
    })
    .unwrap();
  let committed = world.element(target_id).unwrap().style().clone();
  assert_eq!(
    committed.flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::ColumnReverse))
  );
  assert_eq!(
    committed.flex_wrap,
    Prop::Set(StyleValue::Value(FlexWrap::Wrap))
  );

  assert_eq!(
    world.update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(Style::new().padding_left(-1))).into(),
    }),
    Err(UiWorldError::InvalidProperty)
  );
  assert_eq!(world.element(target_id).unwrap().style(), &committed);
}

#[test]
fn every_layout_family_sets_resets_and_preserves_omitted_state() {
  let target_id = ObjectId::new_v4();
  let mut world = UiWorld::default();
  world
    .replace(vec![UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(
        target_id,
        UiBox::new().style(Style::new().background_color(Color::rgb(0.1, 0.2, 0.3))),
      ),
    )])
    .unwrap();

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(assigned_layout())).into(),
    })
    .unwrap();
  let assigned = world.element(target_id).unwrap().style();
  assert_layout_fields!(assigned, Prop::Set(_));

  world
    .update(VisualElementUpdate::Properties {
      object_id: target_id,
      element: UiElement::from(UiBox::default().style(reset_layout())).into(),
    })
    .unwrap();
  let reset = world.element(target_id).unwrap().style();
  assert_layout_fields!(reset, Prop::Reset);
  assert_eq!(
    reset.background_color,
    Prop::Set(StyleValue::Value(Color::rgb(0.1, 0.2, 0.3)))
  );
}

fn assigned_layout() -> Style {
  Style::new()
    .align_content(Align::Center)
    .align_items(Align::FlexEnd)
    .align_self(Align::Stretch)
    .aspect_ratio(AspectRatio::new(4.0, 3.0))
    .border_width((1, 2, 3, 4))
    .bottom(5)
    .display(Display::None)
    .flex_basis(LengthOrAuto::Auto)
    .flex_direction(FlexDirection::Row)
    .flex_grow(2)
    .flex_shrink(3)
    .flex_wrap(FlexWrap::Wrap)
    .height(70)
    .justify_content(Justify::SpaceBetween)
    .left(7)
    .margin((8, 9, 10, 11))
    .max_height(120)
    .max_width(130)
    .min_height(20)
    .min_width(30)
    .overflow(Overflow::Hidden)
    .padding((12, 13, 14, 15))
    .position(Position::Absolute)
    .right(16)
    .top(17)
    .width(140)
}

fn reset_layout() -> Style {
  Style::new()
    .align_content(Prop::Reset)
    .align_items(Prop::Reset)
    .align_self(Prop::Reset)
    .aspect_ratio(Prop::Reset)
    .border_bottom_width(Prop::Reset)
    .border_left_width(Prop::Reset)
    .border_right_width(Prop::Reset)
    .border_top_width(Prop::Reset)
    .bottom(Prop::Reset)
    .display(Prop::Reset)
    .flex_basis(Prop::Reset)
    .flex_direction(Prop::Reset)
    .flex_grow(Prop::Reset)
    .flex_shrink(Prop::Reset)
    .flex_wrap(Prop::Reset)
    .height(Prop::Reset)
    .justify_content(Prop::Reset)
    .left(Prop::Reset)
    .margin_bottom(Prop::Reset)
    .margin_left(Prop::Reset)
    .margin_right(Prop::Reset)
    .margin_top(Prop::Reset)
    .max_height(Prop::Reset)
    .max_width(Prop::Reset)
    .min_height(Prop::Reset)
    .min_width(Prop::Reset)
    .overflow(Prop::Reset)
    .padding_bottom(Prop::Reset)
    .padding_left(Prop::Reset)
    .padding_right(Prop::Reset)
    .padding_top(Prop::Reset)
    .position(Prop::Reset)
    .right(Prop::Reset)
    .top(Prop::Reset)
    .width(Prop::Reset)
}
