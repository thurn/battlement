use battlement_types::ObjectId;
use battlement_ui::{
  Align, FlexDirection, FlexWrap, GridAutoFlow, GridItem, GridTrack, Justify, OverlayLayer,
  OverlayPlacement, PlacementAlign, PlacementSide, PopoverPlacement, Position, Prop, StackItem,
  Sticky, Style, UiDocument, UiElement, UiFlex, UiGrid, UiNode, UiStack, UiValidationError,
  UiVisualElement, validate_element_state,
};

#[test]
fn layout_catalog_round_trips_every_variant_through_json() {
  let anchor = ObjectId::new_v4();
  let initial_focus = ObjectId::new_v4();
  let restore_focus = ObjectId::new_v4();
  let values = [
    UiElement::from(UiFlex {
      direction: Prop::Set(FlexDirection::RowReverse),
      wrap: Prop::Set(FlexWrap::WrapReverse),
      align_items: Prop::Set(Align::Center),
      justify_content: Prop::Set(Justify::SpaceEvenly),
      row_gap: Prop::Set(2.0),
      column_gap: Prop::Set(3.0),
      ..UiFlex::new()
    }),
    UiElement::from(UiGrid {
      columns: Prop::Set(vec![
        GridTrack::px(12.0),
        GridTrack::fr(2.0),
        GridTrack::auto(),
      ]),
      rows: Prop::Set(Vec::new()),
      auto_columns: Prop::Set(GridTrack::Fraction(1.0)),
      auto_rows: Prop::Set(GridTrack::Auto),
      auto_flow: Prop::Set(GridAutoFlow::Column),
      row_gap: Prop::Set(4.0),
      column_gap: Prop::Set(5.0),
      align_items: Prop::Set(Align::FlexStart),
      justify_items: Prop::Set(Align::FlexEnd),
      ..UiGrid::new()
    }),
    UiElement::from(UiStack {
      align_items: Prop::Set(Align::Stretch),
      justify_items: Prop::Set(Align::Center),
      ..UiStack::new()
    }),
    UiElement::from(UiVisualElement {
      grid_item: Prop::Set(GridItem {
        row: Some(1),
        column: Some(2),
        row_span: 3,
        column_span: 4,
        align_self: Align::Auto,
        justify_self: Align::Center,
      }),
      stack_item: Prop::Set(StackItem {
        order: -7,
        align_self: Align::FlexEnd,
        justify_self: Align::Stretch,
        top: Some(1.0),
        right: Some(2.0),
        bottom: Some(3.0),
        left: Some(4.0),
        contributes_to_size: false,
      }),
      sticky: Prop::Set(Sticky {
        top: Some(-3.0),
        right: Some(4.0),
        bottom: None,
        left: None,
        order: 8,
      }),
      overlay_placement: Prop::Set(OverlayPlacement::Popover {
        anchor,
        placement: PopoverPlacement {
          side: PlacementSide::Left,
          align: PlacementAlign::End,
          main_offset: -2.0,
          cross_offset: 3.0,
          collision_padding: 9.0,
          flip: false,
          shift: false,
        },
      }),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      overlay_placement: Prop::Set(OverlayPlacement::Layer(OverlayLayer::Popover)),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      overlay_placement: Prop::Set(OverlayPlacement::Modal {
        initial_focus: Some(initial_focus),
        restore_focus: Some(restore_focus),
      }),
      ..UiVisualElement::new()
    }),
  ];

  for value in values {
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(serde_json::from_str::<UiElement>(&json).unwrap(), value);
  }
}

#[test]
fn layout_validation_rejects_every_numeric_category() {
  let invalid = [
    UiElement::from(UiGrid {
      columns: Prop::Set(vec![GridTrack::Px(-1.0)]),
      ..UiGrid::new()
    }),
    UiElement::from(UiGrid {
      columns: Prop::Set(vec![GridTrack::Fraction(0.0)]),
      ..UiGrid::new()
    }),
    UiElement::from(UiFlex {
      row_gap: Prop::Set(f32::NAN),
      ..UiFlex::new()
    }),
    UiElement::from(UiGrid {
      align_items: Prop::Set(Align::Auto),
      ..UiGrid::new()
    }),
    UiElement::from(UiVisualElement {
      grid_item: Prop::Set(GridItem {
        row: Some(0),
        ..GridItem::default()
      }),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      grid_item: Prop::Set(GridItem {
        row: Some(1),
        row_span: 0,
        ..GridItem::default()
      }),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      grid_item: Prop::Set(GridItem {
        column: Some(1),
        column_span: 0,
        ..GridItem::default()
      }),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      stack_item: Prop::Set(StackItem {
        left: Some(-1.0),
        ..StackItem::default()
      }),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      sticky: Prop::Set(Sticky {
        left: Some(0.0),
        right: Some(0.0),
        ..Sticky::default()
      }),
      ..UiVisualElement::new()
    }),
    UiElement::from(UiVisualElement {
      overlay_placement: Prop::Set(OverlayPlacement::Popover {
        anchor: ObjectId::new_v4(),
        placement: PopoverPlacement {
          collision_padding: -1.0,
          ..PopoverPlacement::default()
        },
      }),
      ..UiVisualElement::new()
    }),
  ];

  for value in invalid {
    assert_eq!(
      validate_element_state(&value),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn snapshots_containing_each_layout_host_are_valid_protocol_state() {
  let document = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(ObjectId::new_v4(), UiFlex::new()))
    .child(UiNode::new(ObjectId::new_v4(), UiGrid::new()))
    .child(UiNode::new(ObjectId::new_v4(), UiStack::new()));

  assert!(battlement_ui::validate_documents(&[document]).is_ok());
}

#[test]
fn grid_items_require_a_grid_parent_and_default_positioning() {
  let outside = UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
    ObjectId::new_v4(),
    UiVisualElement {
      grid_item: Prop::Set(GridItem::new()),
      ..UiVisualElement::new()
    },
  ));
  assert_eq!(
    battlement_ui::validate_documents(&[outside]),
    Err(UiValidationError::InvalidProperty)
  );

  for style in [
    Style::new().position(Position::Absolute),
    Style::new().top(1),
  ] {
    let grid = UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(ObjectId::new_v4(), UiGrid::new()).child(UiNode::new(
        ObjectId::new_v4(),
        UiVisualElement {
          style,
          ..UiVisualElement::new()
        },
      )),
    );
    assert_eq!(
      battlement_ui::validate_documents(&[grid]),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn grid_item_builder_authors_each_axis_and_alignment() {
  assert_eq!(
    GridItem::new()
      .row(2)
      .column(3)
      .span_rows(4)
      .span_columns(5)
      .align_self(Align::Center)
      .justify_self(Align::FlexEnd),
    GridItem {
      row: Some(2),
      column: Some(3),
      row_span: 4,
      column_span: 5,
      align_self: Align::Center,
      justify_self: Align::FlexEnd,
    }
  );
}

#[test]
fn stack_item_builder_and_context_author_the_closed_contract() {
  let item = StackItem::new()
    .order(-3)
    .align_self(Align::Center)
    .justify_self(Align::FlexEnd)
    .top(1.0)
    .right(2.0)
    .bottom(3.0)
    .left(4.0)
    .contributes_to_size(false);
  assert_eq!(
    item,
    StackItem {
      order: -3,
      align_self: Align::Center,
      justify_self: Align::FlexEnd,
      top: Some(1.0),
      right: Some(2.0),
      bottom: Some(3.0),
      left: Some(4.0),
      contributes_to_size: false,
    }
  );

  let outside = UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
    ObjectId::new_v4(),
    UiVisualElement {
      stack_item: Prop::Set(item),
      ..UiVisualElement::new()
    },
  ));
  assert_eq!(
    battlement_ui::validate_documents(&[outside]),
    Err(UiValidationError::InvalidProperty)
  );

  for style in [
    Style::new().position(Position::Absolute),
    Style::new().left(1),
  ] {
    let stack = UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(ObjectId::new_v4(), UiStack::new()).child(UiNode::new(
        ObjectId::new_v4(),
        UiVisualElement {
          style,
          ..UiVisualElement::new()
        },
      )),
    );
    assert_eq!(
      battlement_ui::validate_documents(&[stack]),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn sticky_rejects_stack_and_overlay_placement_combinations() {
  for visual in [
    UiVisualElement {
      stack_item: Prop::Set(StackItem::new()),
      sticky: Prop::Set(Sticky {
        top: Some(0.0),
        ..Sticky::default()
      }),
      ..UiVisualElement::new()
    },
    UiVisualElement {
      sticky: Prop::Set(Sticky {
        top: Some(0.0),
        ..Sticky::default()
      }),
      overlay_placement: Prop::Set(OverlayPlacement::Layer(OverlayLayer::Popover)),
      ..UiVisualElement::new()
    },
  ] {
    assert_eq!(
      validate_element_state(&UiElement::from(visual)),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn sticky_builders_accept_only_orthogonal_edges() {
  assert_eq!(
    Sticky::top(-2.0).with_left(3.0).order(7),
    Sticky {
      top: Some(-2.0),
      right: None,
      bottom: None,
      left: Some(3.0),
      order: 7,
    }
  );
  assert_eq!(
    Sticky::right(4.0).with_bottom(5.0),
    Sticky {
      top: None,
      right: Some(4.0),
      bottom: Some(5.0),
      left: None,
      order: 0,
    }
  );
  assert!(std::panic::catch_unwind(|| Sticky::top(0.0).with_bottom(1.0)).is_err());
  assert!(std::panic::catch_unwind(|| Sticky::left(0.0).with_right(1.0)).is_err());
}

#[test]
fn sticky_requires_scroll_ancestry_and_normal_flow_positioning() {
  let sticky = UiVisualElement {
    sticky: Prop::Set(Sticky::top(0.0)),
    ..UiVisualElement::new()
  };
  let outside =
    UiDocument::new(ObjectId::new_v4()).child(UiNode::new(ObjectId::new_v4(), sticky.clone()));
  assert_eq!(
    battlement_ui::validate_documents(&[outside]),
    Err(UiValidationError::InvalidProperty)
  );

  let inside = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(ObjectId::new_v4(), battlement_ui::UiScrollView::new())
      .child(UiNode::new(ObjectId::new_v4(), sticky)),
  );
  assert!(battlement_ui::validate_documents(&[inside]).is_ok());

  assert_eq!(
    validate_element_state(&UiElement::from(UiVisualElement {
      sticky: Prop::Set(Sticky::left(0.0)),
      style: Style::new().position(Position::Absolute),
      ..UiVisualElement::new()
    })),
    Err(UiValidationError::InvalidProperty)
  );
}
