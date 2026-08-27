use battlement::{
    Box, Command, Label, ObjectId, TextField, UiElement, UiEvent, UiEventBody, UiEventKind, UiNode,
    UiValue, VisualElement, object_id,
};

use crate::{design_system, text_field_styles};

pub(crate) const ACCEPTED_ID: ObjectId = object_id!("fd496f77-d46e-4bf9-8f5e-5cba8229d94f");
pub(crate) const NORMALIZED_ID: ObjectId = object_id!("df0c6d77-9ff1-40cb-8ae3-a01353df5c73");
pub(crate) const REJECTED_ID: ObjectId = object_id!("c20ac846-5730-48ab-89ea-9c943d5e385b");
pub(crate) const STATUS_ID: ObjectId = object_id!("8a83987f-581f-4f32-8ce8-e0a99c70174d");
pub(crate) const DRAFT_ID: ObjectId = object_id!("f93c739b-a044-44ed-89de-05a343937df6");
pub(crate) const COMMITTED_ID: ObjectId = object_id!("b6ce5ac8-1923-4470-a2a1-b9d9ad8fe7d1");
pub(crate) const SELECTION_ID: ObjectId = object_id!("d138cb1c-0d19-4a06-b96e-52acf0881f95");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("text-fields-page"))
        .child(node(
            Label::new("CONTROLLED TEXT").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Draft locally. Commit deliberately.").style(design_system::title()),
        ))
        .child(
            node(VisualElement::new().style(text_field_styles::main_layout()))
                .child(editor())
                .child(inspector()),
        )
        .child(specimen_row())
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
    match &event.body {
        UiEventBody::Input(value) if event.target_id == ACCEPTED_ID => Some(vec![
            Command::update_visual_element(
                DRAFT_ID,
                Label::new(format!("LOCAL DRAFT  {}", value.value)),
            ),
            Command::update_visual_element(STATUS_ID, Label::new("EDITING · no commit traffic")),
        ]),
        UiEventBody::SelectionChanged(value) if event.target_id == ACCEPTED_ID => {
            Some(vec![Command::update_visual_element(
                SELECTION_ID,
                Label::new(format!(
                    "SELECTION  {} → {}",
                    value.selection_index, value.cursor_index
                )),
            )])
        }
        UiEventBody::ValueCommitted(value) if event.target_id == ACCEPTED_ID => {
            let proposed = text(value.proposed.clone())?;
            Some(vec![
                Command::update_visual_element(ACCEPTED_ID, TextField::new().value(&proposed)),
                Command::update_visual_element(
                    DRAFT_ID,
                    Label::new(format!("LOCAL DRAFT  {proposed}")),
                ),
                Command::update_visual_element(
                    COMMITTED_ID,
                    Label::new(format!("RUST COMMITTED  {proposed}")),
                ),
                Command::update_visual_element(
                    STATUS_ID,
                    Label::new("ACCEPTED · exact value authored"),
                ),
            ])
        }
        UiEventBody::ValueCommitted(value) if event.target_id == NORMALIZED_ID => {
            let normalized = text(value.proposed.clone())?.trim().to_uppercase();
            Some(vec![
                Command::update_visual_element(NORMALIZED_ID, TextField::new().value(&normalized)),
                Command::update_visual_element(
                    STATUS_ID,
                    Label::new(format!("NORMALIZED · {normalized}")),
                ),
                Command::update_visual_element(
                    COMMITTED_ID,
                    Label::new(format!("RUST COMMITTED  {normalized}")),
                ),
            ])
        }
        UiEventBody::ValueCommitted(_) if event.target_id == REJECTED_ID => {
            Some(vec![Command::update_visual_element(
                STATUS_ID,
                Label::new("REJECTED · kept prior value"),
            )])
        }
        _ => None,
    }
}

fn editor() -> UiNode {
    node(Box::new().style(text_field_styles::edit_surface()))
        .child(node(
            Label::new("THREE COMMIT OUTCOMES").style(text_field_styles::caption()),
        ))
        .child(node(
            Label::new("Type freely; Rust decides only when the gesture commits.")
                .style(text_field_styles::lead()),
        ))
        .child(UiNode::new(
            ACCEPTED_ID,
            TextField::new()
                .name("accepted-text-field")
                .label("ACCEPTED")
                .value("Rook")
                .placeholder("Type a call sign")
                .hide_placeholder_on_focus(true)
                .select_all_on_focus(false)
                .select_all_on_mouse_up(false)
                .events([
                    UiEventKind::Input,
                    UiEventKind::ValueCommitted,
                    UiEventKind::SelectionChanged,
                ])
                .style(text_field_styles::emphasized_field()),
        ))
        .child(UiNode::new(
            NORMALIZED_ID,
            TextField::new()
                .name("normalized-text-field")
                .label("NORMALIZED")
                .value("alpha-7")
                .events([UiEventKind::ValueCommitted])
                .style(text_field_styles::field()),
        ))
        .child(UiNode::new(
            REJECTED_ID,
            TextField::new()
                .name("rejected-text-field")
                .label("REJECTED")
                .value("North Gate")
                .events([UiEventKind::ValueCommitted])
                .style(text_field_styles::field()),
        ))
}

fn inspector() -> UiNode {
    node(Box::new().style(text_field_styles::inspector()))
        .child(node(
            Label::new("RUST STATE INSPECTOR").style(text_field_styles::caption()),
        ))
        .child(UiNode::new(
            STATUS_ID,
            Label::new("IDLE · edit CALL SIGN")
                .name("text-commit-status")
                .style(text_field_styles::inspector_state()),
        ))
        .child(UiNode::new(
            DRAFT_ID,
            Label::new("LOCAL DRAFT  Rook")
                .name("text-draft-status")
                .style(text_field_styles::inspector_value()),
        ))
        .child(UiNode::new(
            COMMITTED_ID,
            Label::new("RUST COMMITTED  Rook")
                .name("text-committed-status")
                .style(text_field_styles::inspector_value()),
        ))
        .child(UiNode::new(
            SELECTION_ID,
            Label::new("SELECTION  0 → 0")
                .name("text-selection-status")
                .style(text_field_styles::inspector_value()),
        ))
        .child(node(
            Label::new(
                "Enter commits one proposal. Focus loss commits once. Escape restores silently.",
            )
            .style(text_field_styles::inspector_note()),
        ))
}

fn specimen_row() -> UiNode {
    node(VisualElement::new().style(text_field_styles::specimen_row()))
        .child(
            node(Box::new().style(text_field_styles::specimen()))
                .child(node(
                    Label::new("MULTILINE").style(text_field_styles::specimen_title()),
                ))
                .child(node(
                    TextField::new()
                        .value("Hold position\nAwait signal")
                        .multiline(true)
                        .style(text_field_styles::multiline_field()),
                )),
        )
        .child(
            node(Box::new().style(text_field_styles::specimen()))
                .child(node(
                    Label::new("PASSWORD").style(text_field_styles::specimen_title()),
                ))
                .child(node(
                    TextField::new()
                        .value("bastion")
                        .password(true)
                        .style(text_field_styles::compact_field()),
                ))
                .child(node(
                    Label::new("Native masking; Rust still owns the value.")
                        .style(text_field_styles::specimen_note()),
                )),
        )
        .child(
            node(Box::new().style(text_field_styles::final_specimen()))
                .child(node(
                    Label::new("READ ONLY").style(text_field_styles::specimen_title()),
                ))
                .child(node(
                    TextField::new()
                        .value("COMMAND VERIFIED")
                        .read_only(true)
                        .style(text_field_styles::compact_field()),
                ))
                .child(node(
                    Label::new("Selectable context without edit traffic.")
                        .style(text_field_styles::specimen_note()),
                )),
        )
}

fn text(value: UiValue) -> Option<String> {
    match value {
        UiValue::String(value) => Some(value),
        UiValue::Bool(_)
        | UiValue::Choice(_)
        | UiValue::F32(_)
        | UiValue::I32(_)
        | UiValue::Index(_)
        | UiValue::Indices(_) => None,
        UiValue::F32Range(_) => None,
    }
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
