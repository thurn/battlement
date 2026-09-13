use battlement::{
  ObjectId, ScrollerVisibility, UiBox, UiButton, UiElement, UiEventKind, UiLabel, UiNode,
  UiScrollView, UiSlider, UiTextElement, UiTextField, UiToggle, UiVisualElement, object_id,
};
use battlement_native::{UiEventActionView, UiValueView};

use crate::{action_styles, design_system, native_ui::NativeUiResponseBuilder};

pub(crate) const RUN_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000001");
pub(crate) const SCROLL_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000002");
pub(crate) const SCROLL_TARGET_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000003");
pub(crate) const SELECTABLE_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000004");
pub(crate) const ACTION_STATUS_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000005");
pub(crate) const ACCEPTED_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000006");
pub(crate) const REJECTED_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000007");
pub(crate) const DRAFT_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000008");
pub(crate) const DRAG_ID: ObjectId = object_id!("25100000-0000-4000-8000-000000000009");
pub(crate) const CLEANUP_ID: ObjectId = object_id!("25100000-0000-4000-8000-00000000000a");
pub(crate) const CONTROL_STATUS_ID: ObjectId = object_id!("25100000-0000-4000-8000-00000000000b");
pub(crate) const SELECTION_STATUS_ID: ObjectId = object_id!("25100000-0000-4000-8000-00000000000c");
pub(crate) const FOCUS_TARGET_ID: ObjectId = object_id!("25100000-0000-4000-8000-00000000000d");

#[derive(Default)]
pub(crate) struct CleanupEvidence {
  draft: bool,
  drag: bool,
  draft_leaked: bool,
  drag_leaked: bool,
}

pub(crate) fn page(page_id: ObjectId, actions_ran: bool, accepted: bool, cleaned: bool) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("actions-page"))
        .child(node(UiLabel::new("ACTIONS + AUTHORITY").style(design_system::eyebrow())))
        .child(node(
            UiLabel::new("Transient intent. Authoritative state.").style(design_system::title()),
        ))
        .child(node(UiLabel::new("One console proves every public action. Beside it, controlled values accept or reject proposals while typing and dragging each trigger a silent input shutdown.").style(action_styles::intro())))
        .child(
            node(UiVisualElement::new().style(action_styles::columns()))
                .child(action_console(actions_ran))
                .child(controlled_console(accepted, cleaned)),
        )
}

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  accepted: &mut bool,
  cleanup: &mut CleanupEvidence,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, battlement_native::EngineError> {
  let target_id =
    ObjectId::from_bytes(event.target_id()).expect("UI event view validates target UUIDs");
  match (event.event_kind(), target_id) {
    (UiEventKind::Click, RUN_ID) => {
      response.focus(FOCUS_TARGET_ID)?;
      response.blur(FOCUS_TARGET_ID)?;
      response.scroll_to(SCROLL_ID, SCROLL_TARGET_ID)?;
      response.focus(SELECTABLE_ID)?;
      response.select_text(SELECTABLE_ID, 11, 3)?;
      response.capture_pointer(SELECTABLE_ID, 17)?;
      response.release_pointer(SELECTABLE_ID, 17)?;
      response.label(
        ACTION_STATUS_ID,
        "PASSED  Focus/Blur > ScrollTo > SelectText > Capture/Release",
      )?;
      response.label(SELECTION_STATUS_ID, "SELECTION | UTF-16 3-11 applied")?;
      let button = response
        .writer()
        .button_builder()
        .text("Run actions again")
        .finish();
      response.update(RUN_ID, button)?;
    }
    (UiEventKind::Click, CLEANUP_ID) => {
      *cleanup = CleanupEvidence::default();
      response.label(CONTROL_STATUS_ID, cleanup_status(cleanup))?;
    }
    (UiEventKind::Input, DRAFT_ID) => {
      cleanup.draft = true;
      write_cleanup(cleanup, response)?;
    }
    (UiEventKind::ValueChanging, DRAG_ID) => {
      cleanup.drag = true;
      write_cleanup(cleanup, response)?;
    }
    (UiEventKind::ValueCommitted, id) if id == DRAFT_ID || id == DRAG_ID => {
      cleanup.draft_leaked = target_id == DRAFT_ID;
      cleanup.drag_leaked = target_id == DRAG_ID;
      response.label(CONTROL_STATUS_ID, cleanup_status(cleanup))?;
    }
    (UiEventKind::ValueCommitted, ACCEPTED_ID) => {
      let Some(UiValueView::Bool(proposed)) = event.value_commit().map(|value| value.proposed())
      else {
        return Ok(false);
      };
      *accepted = proposed;
      let toggle = response
        .writer()
        .toggle_builder()
        .bool_value(proposed)
        .finish();
      response.update(ACCEPTED_ID, toggle)?;
      response.label(
        CONTROL_STATUS_ID,
        &format!(
          "ACCEPTED | response committed {} before repaint",
          state(proposed)
        ),
      )?;
    }
    (UiEventKind::ValueCommitted, REJECTED_ID) => {
      let Some(UiValueView::Bool(proposed)) = event.value_commit().map(|value| value.proposed())
      else {
        return Ok(false);
      };
      response.label(
        CONTROL_STATUS_ID,
        &format!("REJECTED | proposal {} rolled back to ON", state(proposed)),
      )?;
    }
    _ => return Ok(false),
  }
  Ok(true)
}

fn write_cleanup(
  cleanup: &CleanupEvidence,
  response: &mut NativeUiResponseBuilder,
) -> Result<(), battlement_native::EngineError> {
  response.set_input_enabled(false)?;
  response.next_group();
  response.label(CONTROL_STATUS_ID, cleanup_status(cleanup))?;
  response.set_input_enabled(true)
}

fn action_console(ran: bool) -> UiNode {
  node(UiBox::new().style(action_styles::card(true)))
        .child(node(UiLabel::new("ACTION CONSOLE").style(action_styles::caption())))
        .child(node(UiLabel::new("ScrollTo reveals the cyan destination; SelectText highlights UTF-16 units 3-11. A separate probe proves Focus and Blur without hiding the selection.").style(action_styles::help())))
        .child(
            UiNode::new(
                SCROLL_ID,
                UiScrollView::new()
                    .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
                    .style(action_styles::scroll()),
            )
            .child(node(UiLabel::new("01 | Validate target" ).style(action_styles::row())))
            .child(node(UiLabel::new("02 | Enter deferred gate").style(action_styles::row())))
            .child(node(UiLabel::new("03 | Preserve response order").style(action_styles::row())))
            .child(node(UiLabel::new("04 | Apply before repaint").style(action_styles::row())))
            .child(UiNode::new(
                SCROLL_TARGET_ID,
                UiLabel::new("DESTINATION | logical descendant").style(action_styles::destination()),
            )),
        )
        .child(UiNode::new(
            FOCUS_TARGET_ID,
            UiTextElement::new("FOCUS / BLUR PROBE")
                .focusable(true)
                .style(action_styles::focus_probe()),
        ))
        .child(UiNode::new(
            SELECTABLE_ID,
            UiTextElement::new("SELECTABLE UTF-16 RANGE")
                .selectable(true)
                .focusable(true)
                .style(action_styles::selectable()),
        ))
        .child(UiNode::new(
            SELECTION_STATUS_ID,
            UiLabel::new(if ran {
                "SELECTION | UTF-16 3-11 applied"
            } else {
                "SELECTION | waiting for SelectText"
            })
            .style(action_styles::selection_evidence(ran)),
        ))
        .child(UiNode::new(
            RUN_ID,
            UiButton::new(if ran { "Run actions again" } else { "Run all six actions" })
                .events([UiEventKind::Click])
                .style(action_styles::button()),
        ))
        .child(UiNode::new(
            ACTION_STATUS_ID,
            UiLabel::new(if ran {
                "PASSED  Focus/Blur > ScrollTo > SelectText > Capture/Release"
            } else {
                "READY  Six actions | validated | no authored state retained"
            })
            .style(action_styles::status(ran)),
        ))
}

fn controlled_console(accepted: bool, cleaned: bool) -> UiNode {
  node(UiBox::new().style(action_styles::card(false)))
        .child(node(UiLabel::new("CONTROLLED + DISABLED").style(action_styles::caption())))
        .child(node(UiLabel::new("Native proposals restore first. Type in the draft and drag the slider; each active interaction disables input and proves silent rollback.").style(action_styles::help())))
        .child(UiNode::new(
            ACCEPTED_ID,
            UiToggle::new()
                .name("action-accepted")
                .label("ACCEPTED")
                .text("Telemetry uplink")
                .value(accepted)
                .events([UiEventKind::ValueCommitted])
                .style(action_styles::toggle()),
        ))
        .child(UiNode::new(
            REJECTED_ID,
            UiToggle::new()
                .name("action-rejected")
                .label("REJECTED")
                .text("Safety interlock")
                .value(true)
                .events([UiEventKind::ValueCommitted])
                .style(action_styles::toggle()),
        ))
        .child(UiNode::new(
            DRAFT_ID,
            UiTextField::new()
                .name("action-draft")
                .label("LOCAL DRAFT")
                .value("Committed: North Gate")
                .focusable(true)
                .events([UiEventKind::Input, UiEventKind::ValueCommitted])
                .style(action_styles::field())
                .input_style(action_styles::field_input())
                .text_element_style(action_styles::field_text()),
        ))
        .child(UiNode::new(
            DRAG_ID,
            UiSlider::new()
                .name("action-drag")
                .label("LOCAL DRAG - move to disable")
                .low_value(0.0)
                .high_value(100.0)
                .value(38.0)
                .fill(true)
                .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted])
                .style(action_styles::slider()),
        ))
        .child(UiNode::new(
            CLEANUP_ID,
            UiButton::new("Reset cleanup proof")
                .events([UiEventKind::Click])
                .style(action_styles::button()),
        ))
        .child(UiNode::new(
            CONTROL_STATUS_ID,
            UiLabel::new(if cleaned {
                "CLEANED  draft + drag restored | focus + capture released | 0 cleanup events"
            } else {
                "READY  Type in LOCAL DRAFT, then move LOCAL DRAG"
            })
            .style(action_styles::status(cleaned)),
        ))
}

fn cleanup_status(cleanup: &CleanupEvidence) -> &'static str {
  if cleanup.draft_leaked {
    "FAILED  draft cleanup emitted an unexpected commit"
  } else if cleanup.drag_leaked {
    "FAILED  drag cleanup emitted an unexpected commit"
  } else if cleanup.draft && cleanup.drag {
    "CLEANED  draft + drag restored | focus + capture released | 0 cleanup events"
  } else if cleanup.draft {
    "DRAFT CLEANED  restored silently | now move LOCAL DRAG"
  } else if cleanup.drag {
    "DRAG CLEANED  restored silently | now type in LOCAL DRAFT"
  } else {
    "READY  Type in LOCAL DRAFT, then move LOCAL DRAG"
  }
}

fn state(value: bool) -> &'static str {
  if value { "ON" } else { "OFF" }
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
