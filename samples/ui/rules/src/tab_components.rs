use battlement::{
    Box, Command, Label, ObjectId, Tab, TabView, UiElement, UiEvent, UiEventBody, UiEventKind,
    UiNode, VisualElement, object_id,
};

use crate::{asset_catalog::ui::assets, design_system, tab_styles};

const VIEW_ID: ObjectId = object_id!("aa1bd60d-71e5-4f3a-a7ba-13f456621b9c");
const BOARD_ID: ObjectId = object_id!("e7491a26-c97e-4668-9b72-0aba2f8920c1");
const NOTES_ID: ObjectId = object_id!("1560af93-b7eb-489e-983b-768747b9db49");
const LOADOUT_ID: ObjectId = object_id!("9fca8e31-3f73-4245-8fbf-523b1094ef0a");
const TIMELINE_ID: ObjectId = object_id!("d3f27972-0998-4e83-ad01-3125540ad95a");
const SIGNAL_ID: ObjectId = object_id!("abbb5697-bb75-4f18-85ca-f5bb706dc59f");
const STATUS_ID: ObjectId = object_id!("752743e9-cb89-4148-ad40-e5076f78f6e1");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("tabs-page"))
        .child(node(Label::new("WORKSPACE TABS").style(design_system::eyebrow())))
        .child(node(
            Label::new("Order, choose, close with intent").style(design_system::title()),
        ))
        .child(
            node(VisualElement::new().style(tab_styles::layout()))
                .child(
                    node(Box::new().style(tab_styles::workspace()))
                        .child(node(
                            Label::new("CONTROLLED WORKSPACE").style(tab_styles::caption()),
                        ))
                        .child(
                            UiNode::new(
                                VIEW_ID,
                                TabView::new()
                                    .name("controlled-tab-view")
                                    .selected_tab_index(0)
                                    .reorderable(true)
                                    .events([
                                        UiEventKind::TabSelectionRequested,
                                        UiEventKind::TabCloseRequested,
                                        UiEventKind::TabReorderRequested,
                                    ])
                                    .style(tab_styles::tab_view()),
                            )
                            .child(tab(BOARD_ID, "BOARD", "Pinned overview", "3 squads ready"))
                            .child(tab(NOTES_ID, "NOTES", "Mission notes", "Ridge route confirmed"))
                            .child(
                                UiNode::new(
                                    LOADOUT_ID,
                                    Tab::new("LOADOUT")
                                        .name("workspace-tab-loadout")
                                        .icon(assets::VECTOR.clone())
                                        .closeable(true),
                                )
                                .child(content("Prepared gear", "Icon lease is active")),
                            )
                            .child(tab(
                                TIMELINE_ID,
                                "TIMELINE",
                                "Deployment sequence",
                                "T–12 minutes",
                            ))
                            .child(tab(SIGNAL_ID, "SIGNAL", "Comms channel", "Encrypted and clear")),
                        ),
                )
                .child(
                    node(Box::new().style(tab_styles::inspector()))
                        .child(node(Label::new("EVENT INSPECTOR").style(tab_styles::caption())))
                        .child(node(
                            Label::new("Rust owns the final state")
                                .style(tab_styles::inspector_title()),
                        ))
                        .child(UiNode::new(
                            STATUS_ID,
                            Label::new("Ready | 5 tabs · BOARD pinned")
                                .name("tab-event-status")
                                .style(tab_styles::status()),
                        ))
                        .child(node(
                            Label::new(
                                "Drag headers to propose order. Close requests are vetoed until Rust destroys the tab.",
                            )
                            .style(tab_styles::help()),
                        )),
                ),
        )
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
    match &event.body {
        UiEventBody::TabSelectionRequested(value) if event.target_id == VIEW_ID => Some(vec![
            Command::update_visual_element(
                VIEW_ID,
                TabView::new().selected_tab_index(value.proposed_index),
            ),
            Command::update_visual_element(
                STATUS_ID,
                Label::new(format!("Selected tab {}", value.proposed_index + 1)),
            ),
        ]),
        UiEventBody::TabReorderRequested(value) if event.target_id == VIEW_ID => Some(vec![
            Command::update_visual_element_index(value.tab_id, value.proposed_index),
            Command::update_visual_element(
                STATUS_ID,
                Label::new(format!(
                    "Reordered {} → {}",
                    value.previous_index + 1,
                    value.proposed_index + 1
                )),
            ),
        ]),
        UiEventBody::TabCloseRequested(value)
            if event.target_id == VIEW_ID && value.tab_id == BOARD_ID =>
        {
            Some(vec![Command::update_visual_element(
                STATUS_ID,
                Label::new("Rejected close | BOARD is pinned"),
            )])
        }
        UiEventBody::TabCloseRequested(value) if event.target_id == VIEW_ID => Some(vec![
            Command::destroy_visual_element(value.tab_id),
            Command::update_visual_element(STATUS_ID, Label::new("Closed | 4 tabs remain")),
        ]),
        _ => None,
    }
}

fn tab(object_id: ObjectId, label: &str, heading: &str, detail: &str) -> UiNode {
    UiNode::new(
        object_id,
        Tab::new(label)
            .name(format!("workspace-tab-{}", label.to_lowercase()))
            .closeable(true),
    )
    .child(content(heading, detail))
}

fn content(heading: &str, detail: &str) -> UiNode {
    node(Box::new().style(tab_styles::content()))
        .child(node(Label::new(heading).style(tab_styles::content_title())))
        .child(node(Label::new(detail).style(tab_styles::content_detail())))
        .child(node(
            Label::new("Selected content remains aligned with its identified Tab.")
                .style(tab_styles::content_note()),
        ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
