use battlement::{
  ObjectId, UiBox, UiElement, UiEventKind, UiLabel, UiNode, UiTextField, UiVisualElement, object_id,
};
use battlement_native::{EngineError, UiEventActionView, UiValueView};

use crate::{design_system, native_ui::NativeUiResponseBuilder, text_field_styles};

pub(crate) const ACCEPTED_ID: ObjectId = object_id!("fd496f77-d46e-4bf9-8f5e-5cba8229d94f");
pub(crate) const NORMALIZED_ID: ObjectId = object_id!("df0c6d77-9ff1-40cb-8ae3-a01353df5c73");
pub(crate) const REJECTED_ID: ObjectId = object_id!("c20ac846-5730-48ab-89ea-9c943d5e385b");
pub(crate) const STATUS_ID: ObjectId = object_id!("8a83987f-581f-4f32-8ce8-e0a99c70174d");
pub(crate) const DRAFT_ID: ObjectId = object_id!("f93c739b-a044-44ed-89de-05a343937df6");
pub(crate) const COMMITTED_ID: ObjectId = object_id!("b6ce5ac8-1923-4470-a2a1-b9d9ad8fe7d1");
pub(crate) const SELECTION_ID: ObjectId = object_id!("d138cb1c-0d19-4a06-b96e-52acf0881f95");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("text-fields-page"))
    .child(node(
      UiLabel::new("CONTROLLED TEXT").style(design_system::eyebrow()),
    ))
    .child(node(
      UiLabel::new("Draft locally. Commit deliberately.").style(design_system::title()),
    ))
    .child(
      node(UiVisualElement::new().style(text_field_styles::main_layout()))
        .child(editor())
        .child(inspector()),
    )
    .child(specimen_row())
}

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, EngineError> {
  let target_id = ObjectId::from_bytes(event.target_id()).expect("validated UI target UUID");
  match event.event_kind() {
    UiEventKind::Input if target_id == ACCEPTED_ID => {
      let Some(value) = event.input_text() else {
        return Ok(false);
      };
      response.label(DRAFT_ID, &format!("LOCAL DRAFT  {value}"))?;
      response.label(STATUS_ID, "EDITING · no commit traffic")?;
    }
    UiEventKind::SelectionChanged if target_id == ACCEPTED_ID => {
      let Some(value) = event.selection() else {
        return Ok(false);
      };
      response.label(
        SELECTION_ID,
        &format!(
          "SELECTION  {} → {}",
          value.selection_index, value.cursor_index
        ),
      )?;
    }
    UiEventKind::ValueCommitted if target_id == ACCEPTED_ID => {
      let Some(commit) = event.value_commit() else {
        return Ok(false);
      };
      let UiValueView::Text(proposed) = commit.proposed() else {
        return Ok(false);
      };
      let field = response
        .writer()
        .text_field_builder()
        .text_value(proposed)
        .finish();
      response.update(ACCEPTED_ID, field)?;
      response.label(DRAFT_ID, &format!("LOCAL DRAFT  {proposed}"))?;
      response.label(COMMITTED_ID, &format!("RUST COMMITTED  {proposed}"))?;
      response.label(STATUS_ID, "ACCEPTED · exact value authored")?;
    }
    UiEventKind::ValueCommitted if target_id == NORMALIZED_ID => {
      let Some(commit) = event.value_commit() else {
        return Ok(false);
      };
      let UiValueView::Text(proposed) = commit.proposed() else {
        return Ok(false);
      };
      let normalized = proposed.trim().to_uppercase();
      let field = response
        .writer()
        .text_field_builder()
        .text_value(&normalized)
        .finish();
      response.update(NORMALIZED_ID, field)?;
      response.label(STATUS_ID, &format!("NORMALIZED · {normalized}"))?;
      response.label(COMMITTED_ID, &format!("RUST COMMITTED  {normalized}"))?;
    }
    UiEventKind::ValueCommitted if target_id == REJECTED_ID => {
      response.label(STATUS_ID, "REJECTED · kept prior value")?;
    }
    _ => return Ok(false),
  }
  Ok(true)
}

fn editor() -> UiNode {
  node(UiBox::new().style(text_field_styles::edit_surface()))
    .child(node(
      UiLabel::new("THREE COMMIT OUTCOMES").style(text_field_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Type freely; Rust decides only when the gesture commits.")
        .style(text_field_styles::lead()),
    ))
    .child(UiNode::new(
      ACCEPTED_ID,
      UiTextField::new()
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
      UiTextField::new()
        .name("normalized-text-field")
        .label("NORMALIZED")
        .value("alpha-7")
        .events([UiEventKind::ValueCommitted])
        .style(text_field_styles::field()),
    ))
    .child(UiNode::new(
      REJECTED_ID,
      UiTextField::new()
        .name("rejected-text-field")
        .label("REJECTED")
        .value("North Gate")
        .events([UiEventKind::ValueCommitted])
        .style(text_field_styles::field()),
    ))
}

fn inspector() -> UiNode {
  node(UiBox::new().style(text_field_styles::inspector()))
    .child(node(
      UiLabel::new("RUST STATE INSPECTOR").style(text_field_styles::caption()),
    ))
    .child(UiNode::new(
      STATUS_ID,
      UiLabel::new("IDLE · edit CALL SIGN")
        .name("text-commit-status")
        .style(text_field_styles::inspector_state()),
    ))
    .child(UiNode::new(
      DRAFT_ID,
      UiLabel::new("LOCAL DRAFT  Rook")
        .name("text-draft-status")
        .style(text_field_styles::inspector_value()),
    ))
    .child(UiNode::new(
      COMMITTED_ID,
      UiLabel::new("RUST COMMITTED  Rook")
        .name("text-committed-status")
        .style(text_field_styles::inspector_value()),
    ))
    .child(UiNode::new(
      SELECTION_ID,
      UiLabel::new("SELECTION  0 → 0")
        .name("text-selection-status")
        .style(text_field_styles::inspector_value()),
    ))
    .child(node(
      UiLabel::new(
        "Enter commits one proposal. Focus loss commits once. Escape restores silently.",
      )
      .style(text_field_styles::inspector_note()),
    ))
}

fn specimen_row() -> UiNode {
  node(UiVisualElement::new().style(text_field_styles::specimen_row()))
    .child(
      node(UiBox::new().style(text_field_styles::specimen()))
        .child(node(
          UiLabel::new("MULTILINE").style(text_field_styles::specimen_title()),
        ))
        .child(node(
          UiTextField::new()
            .value("Hold position\nAwait signal")
            .multiline(true)
            .style(text_field_styles::multiline_field()),
        )),
    )
    .child(
      node(UiBox::new().style(text_field_styles::specimen()))
        .child(node(
          UiLabel::new("PASSWORD").style(text_field_styles::specimen_title()),
        ))
        .child(node(
          UiTextField::new()
            .value("bastion")
            .password(true)
            .style(text_field_styles::compact_field()),
        ))
        .child(node(
          UiLabel::new("Native masking; Rust still owns the value.")
            .style(text_field_styles::specimen_note()),
        )),
    )
    .child(
      node(UiBox::new().style(text_field_styles::final_specimen()))
        .child(node(
          UiLabel::new("READ ONLY").style(text_field_styles::specimen_title()),
        ))
        .child(node(
          UiTextField::new()
            .value("COMMAND VERIFIED")
            .read_only(true)
            .style(text_field_styles::compact_field()),
        ))
        .child(node(
          UiLabel::new("Selectable context without edit traffic.")
            .style(text_field_styles::specimen_note()),
        )),
    )
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
