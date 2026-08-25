use battlement_types::ObjectId;
use battlement_ui::{
    Box, FlexDirection, FlexWrap, LengthUnits, Style, StyleValue, UiDocument, UiElement, UiNode,
    VisualElementUpdate,
};
use battlement_ui_fake::{UiWorld, UiWorldError};

#[test]
fn layout_updates_merge_sparse_fields_and_reject_invalid_values_atomically() {
    let target_id = ObjectId::new_v4();
    let mut world = UiWorld::default();
    world
        .replace(vec![
            UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
                target_id,
                Box::new().style(
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
                Box::default().style(Style::new().flex_direction(FlexDirection::ColumnReverse)),
            )
            .into(),
        })
        .unwrap();
    let committed = world.element(target_id).unwrap().style().clone();
    assert_eq!(
        committed.flex_direction,
        Some(StyleValue::Value(FlexDirection::ColumnReverse))
    );
    assert_eq!(committed.flex_wrap, Some(StyleValue::Value(FlexWrap::Wrap)));

    assert_eq!(
        world.update(VisualElementUpdate::Properties {
            object_id: target_id,
            element: UiElement::from(Box::default().style(Style::new().padding_left(-1))).into(),
        }),
        Err(UiWorldError::InvalidProperty)
    );
    assert_eq!(world.element(target_id).unwrap().style(), &committed);
}
