use battlement_types::ObjectId;
use battlement_ui::{
    Box, ScrollView, TextElement, TextField, UiDocument, UiNode, VisualElementAction,
};
use battlement_ui_fake::{UiJournalEntry, UiWorld, UiWorldError};

#[test]
fn actions_validate_targets_and_cleanup_transient_state() {
    let document_id = ObjectId::new_v4();
    let root_id = ObjectId::new_v4();
    let focus_id = ObjectId::new_v4();
    let scroll_id = ObjectId::new_v4();
    let child_id = ObjectId::new_v4();
    let text_id = ObjectId::new_v4();
    let outside_id = ObjectId::new_v4();
    let field_id = ObjectId::new_v4();
    let mut world = UiWorld::default();
    world
        .replace(vec![
            UiDocument::with_root_id(document_id, root_id)
                .child(UiNode::new(focus_id, Box::new().focusable(true)))
                .child(
                    UiNode::new(scroll_id, ScrollView::new())
                        .child(UiNode::new(child_id, Box::new())),
                )
                .child(UiNode::new(
                    text_id,
                    TextElement::new("A🚀B").selectable(true),
                ))
                .child(UiNode::new(outside_id, Box::new()))
                .child(UiNode::new(
                    field_id,
                    TextField::new().value("native default"),
                )),
        ])
        .unwrap();

    world
        .perform_action(focus_id, &VisualElementAction::Focus)
        .unwrap();
    world
        .perform_action(
            focus_id,
            &VisualElementAction::CapturePointer { pointer_id: 7 },
        )
        .unwrap();
    world
        .perform_action(
            scroll_id,
            &VisualElementAction::ScrollTo {
                descendant_id: child_id,
            },
        )
        .unwrap();
    world
        .perform_action(
            text_id,
            &VisualElementAction::SelectText {
                cursor_index: 3,
                selection_index: 1,
            },
        )
        .unwrap();

    assert_eq!(world.focused(), Some(focus_id));
    assert_eq!(world.pointer_capture(7), Some(focus_id));
    assert_eq!(world.selection(text_id), Some((3, 1)));
    assert!(
        matches!(world.journal().last(), Some(UiJournalEntry::Action(id, VisualElementAction::SelectText { .. })) if *id == text_id)
    );
    world
        .perform_action(focus_id, &VisualElementAction::Blur)
        .unwrap();
    world
        .perform_action(field_id, &VisualElementAction::Focus)
        .unwrap();
    assert_eq!(world.focused(), Some(field_id));
    assert_eq!(
        world.perform_action(
            scroll_id,
            &VisualElementAction::ScrollTo {
                descendant_id: outside_id,
            },
        ),
        Err(UiWorldError::InvalidHierarchy)
    );
    assert_eq!(
        world.perform_action(
            text_id,
            &VisualElementAction::SelectText {
                cursor_index: 5,
                selection_index: 0,
            },
        ),
        Err(UiWorldError::InvalidProperty)
    );

    world.clear_interaction_state();
    assert_eq!(world.focused(), None);
    assert_eq!(world.pointer_capture(7), None);
    assert_eq!(world.selection(text_id), Some((3, 1)));
}

#[test]
fn release_blur_and_destroy_require_current_ownership() {
    let document_id = ObjectId::new_v4();
    let root_id = ObjectId::new_v4();
    let target_id = ObjectId::new_v4();
    let mut world = UiWorld::default();
    world
        .replace(vec![
            UiDocument::with_root_id(document_id, root_id)
                .child(UiNode::new(target_id, Box::new().focusable(true))),
        ])
        .unwrap();

    assert_eq!(
        world.perform_action(target_id, &VisualElementAction::Blur),
        Err(UiWorldError::InvalidProperty)
    );
    assert_eq!(
        world.perform_action(
            target_id,
            &VisualElementAction::ReleasePointer { pointer_id: 3 },
        ),
        Err(UiWorldError::InvalidProperty)
    );
    world
        .perform_action(target_id, &VisualElementAction::Focus)
        .unwrap();
    world
        .perform_action(
            target_id,
            &VisualElementAction::CapturePointer { pointer_id: 3 },
        )
        .unwrap();
    world.destroy(target_id).unwrap();

    assert_eq!(world.focused(), None);
    assert_eq!(world.pointer_capture(3), None);
}
