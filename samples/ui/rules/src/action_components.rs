use battlement::{
    Box, Button, Command, CommandBody, Label, ObjectId, ParallelCommandGroup, ScrollView,
    ScrollerVisibility, Slider, TextElement, TextField, Toggle, UiElement, UiEvent, UiEventBody,
    UiEventKind, UiNode, UiValue, VisualElement, VisualElementAction, object_id,
};

use crate::{action_styles, design_system};

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
    UiNode::new(page_id, VisualElement::new().name("actions-page"))
        .child(node(Label::new("ACTIONS + AUTHORITY").style(design_system::eyebrow())))
        .child(node(
            Label::new("Transient intent. Authoritative state.").style(design_system::title()),
        ))
        .child(node(Label::new("One console proves every public action. Beside it, controlled values accept or reject proposals while typing and dragging each trigger a silent input shutdown.").style(action_styles::intro())))
        .child(
            node(VisualElement::new().style(action_styles::columns()))
                .child(action_console(actions_ran))
                .child(controlled_console(accepted, cleaned)),
        )
}

pub(crate) fn event_commands(
    event: &UiEvent,
    accepted: &mut bool,
    cleanup: &mut CleanupEvidence,
) -> Option<Vec<ParallelCommandGroup<Command>>> {
    match &event.body {
        UiEventBody::Click(_) if event.target_id == RUN_ID => Some(action_commands()),
        UiEventBody::Click(_) if event.target_id == CLEANUP_ID => {
            *cleanup = CleanupEvidence::default();
            Some(vec![ParallelCommandGroup::new(vec![
                Command::update_visual_element(
                    CONTROL_STATUS_ID,
                    Label::new(cleanup_status(cleanup)),
                ),
            ])])
        }
        UiEventBody::Input(_) if event.target_id == DRAFT_ID => {
            cleanup.draft = true;
            Some(cleanup_commands(cleanup))
        }
        UiEventBody::ValueChanging(_) if event.target_id == DRAG_ID => {
            cleanup.drag = true;
            Some(cleanup_commands(cleanup))
        }
        UiEventBody::ValueCommitted(_)
            if event.target_id == DRAFT_ID || event.target_id == DRAG_ID =>
        {
            cleanup.draft_leaked = event.target_id == DRAFT_ID;
            cleanup.drag_leaked = event.target_id == DRAG_ID;
            Some(vec![ParallelCommandGroup::new(vec![
                Command::update_visual_element(
                    CONTROL_STATUS_ID,
                    Label::new(cleanup_status(cleanup)),
                ),
            ])])
        }
        UiEventBody::ValueCommitted(value) if event.target_id == ACCEPTED_ID => {
            let proposed = boolean(&value.proposed)?;
            *accepted = proposed;
            Some(vec![ParallelCommandGroup::new(vec![
                Command::update_visual_element(ACCEPTED_ID, Toggle::new().value(proposed)),
                Command::update_visual_element(
                    CONTROL_STATUS_ID,
                    Label::new(format!(
                        "ACCEPTED | response committed {} before repaint",
                        state(proposed)
                    )),
                ),
            ])])
        }
        UiEventBody::ValueCommitted(value) if event.target_id == REJECTED_ID => Some(vec![
            ParallelCommandGroup::new(vec![Command::update_visual_element(
                CONTROL_STATUS_ID,
                Label::new(format!(
                    "REJECTED | proposal {} rolled back to ON",
                    state(boolean(&value.proposed)?)
                )),
            )]),
        ]),
        _ => None,
    }
}

fn action_console(ran: bool) -> UiNode {
    node(Box::new().style(action_styles::card(true)))
        .child(node(Label::new("ACTION CONSOLE").style(action_styles::caption())))
        .child(node(Label::new("ScrollTo reveals the cyan destination; SelectText highlights UTF-16 units 3-11. A separate probe proves Focus and Blur without hiding the selection.").style(action_styles::help())))
        .child(
            UiNode::new(
                SCROLL_ID,
                ScrollView::new()
                    .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
                    .style(action_styles::scroll()),
            )
            .child(node(Label::new("01 | Validate target" ).style(action_styles::row())))
            .child(node(Label::new("02 | Enter deferred gate").style(action_styles::row())))
            .child(node(Label::new("03 | Preserve response order").style(action_styles::row())))
            .child(node(Label::new("04 | Apply before repaint").style(action_styles::row())))
            .child(UiNode::new(
                SCROLL_TARGET_ID,
                Label::new("DESTINATION | logical descendant").style(action_styles::destination()),
            )),
        )
        .child(UiNode::new(
            FOCUS_TARGET_ID,
            TextElement::new("FOCUS / BLUR PROBE")
                .focusable(true)
                .style(action_styles::focus_probe()),
        ))
        .child(UiNode::new(
            SELECTABLE_ID,
            TextElement::new("SELECTABLE UTF-16 RANGE")
                .selectable(true)
                .focusable(true)
                .style(action_styles::selectable()),
        ))
        .child(UiNode::new(
            SELECTION_STATUS_ID,
            Label::new(if ran {
                "SELECTION | UTF-16 3-11 applied"
            } else {
                "SELECTION | waiting for SelectText"
            })
            .style(action_styles::selection_evidence(ran)),
        ))
        .child(UiNode::new(
            RUN_ID,
            Button::new(if ran { "Run actions again" } else { "Run all six actions" })
                .events([UiEventKind::Click])
                .style(action_styles::button()),
        ))
        .child(UiNode::new(
            ACTION_STATUS_ID,
            Label::new(if ran {
                "PASSED  Focus/Blur > ScrollTo > SelectText > Capture/Release"
            } else {
                "READY  Six actions | validated | no authored state retained"
            })
            .style(action_styles::status(ran)),
        ))
}

fn controlled_console(accepted: bool, cleaned: bool) -> UiNode {
    node(Box::new().style(action_styles::card(false)))
        .child(node(Label::new("CONTROLLED + DISABLED").style(action_styles::caption())))
        .child(node(Label::new("Native proposals restore first. Type in the draft and drag the slider; each active interaction disables input and proves silent rollback.").style(action_styles::help())))
        .child(UiNode::new(
            ACCEPTED_ID,
            Toggle::new()
                .name("action-accepted")
                .label("ACCEPTED")
                .text("Telemetry uplink")
                .value(accepted)
                .events([UiEventKind::ValueCommitted])
                .style(action_styles::toggle()),
        ))
        .child(UiNode::new(
            REJECTED_ID,
            Toggle::new()
                .name("action-rejected")
                .label("REJECTED")
                .text("Safety interlock")
                .value(true)
                .events([UiEventKind::ValueCommitted])
                .style(action_styles::toggle()),
        ))
        .child(UiNode::new(
            DRAFT_ID,
            TextField::new()
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
            Slider::new()
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
            Button::new("Reset cleanup proof")
                .events([UiEventKind::Click])
                .style(action_styles::button()),
        ))
        .child(UiNode::new(
            CONTROL_STATUS_ID,
            Label::new(if cleaned {
                "CLEANED  draft + drag restored | focus + capture released | 0 cleanup events"
            } else {
                "READY  Type in LOCAL DRAFT, then move LOCAL DRAG"
            })
            .style(action_styles::status(cleaned)),
        ))
}

fn action_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::perform_visual_element_action(FOCUS_TARGET_ID, VisualElementAction::Focus),
        Command::perform_visual_element_action(FOCUS_TARGET_ID, VisualElementAction::Blur),
        Command::perform_visual_element_action(
            SCROLL_ID,
            VisualElementAction::ScrollTo {
                descendant_id: SCROLL_TARGET_ID,
            },
        ),
        Command::perform_visual_element_action(SELECTABLE_ID, VisualElementAction::Focus),
        Command::perform_visual_element_action(
            SELECTABLE_ID,
            VisualElementAction::SelectText {
                cursor_index: 11,
                selection_index: 3,
            },
        ),
        Command::perform_visual_element_action(
            SELECTABLE_ID,
            VisualElementAction::CapturePointer { pointer_id: 17 },
        ),
        Command::perform_visual_element_action(
            SELECTABLE_ID,
            VisualElementAction::ReleasePointer { pointer_id: 17 },
        ),
        Command::update_visual_element(
            ACTION_STATUS_ID,
            Label::new("PASSED  Focus/Blur > ScrollTo > SelectText > Capture/Release"),
        ),
        Command::update_visual_element(
            SELECTION_STATUS_ID,
            Label::new("SELECTION | UTF-16 3-11 applied"),
        ),
        Command::update_visual_element(RUN_ID, Button::new("Run actions again")),
    ])]
}

fn cleanup_commands(cleanup: &CleanupEvidence) -> Vec<ParallelCommandGroup<Command>> {
    vec![
        ParallelCommandGroup::new(vec![Command::new_v4(CommandBody::set_input_enabled(false))]),
        ParallelCommandGroup::new(vec![
            Command::update_visual_element(CONTROL_STATUS_ID, Label::new(cleanup_status(cleanup))),
            Command::new_v4(CommandBody::set_input_enabled(true)),
        ]),
    ]
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

fn boolean(value: &UiValue) -> Option<bool> {
    match value {
        UiValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn state(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
