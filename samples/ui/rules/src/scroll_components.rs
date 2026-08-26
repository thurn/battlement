use battlement::{
    Box, Command, Label, NestedInteraction, ObjectId, ScrollView, ScrollViewMode, Scroller,
    ScrollerVisibility, SliderDirection, TouchScrollBehavior, UiElement, UiEvent, UiEventBody,
    UiEventKind, UiNode, Vector, VisualElement, object_id,
};

use crate::{design_system, scroll_styles};

const PRIMARY_ID: ObjectId = object_id!("d24fec17-cb8a-4b9c-a604-da4113d6ef9b");
const NESTED_ID: ObjectId = object_id!("9f5d82b0-561b-49a5-8ca5-eb493a8f0419");
const SCROLLER_ID: ObjectId = object_id!("df12adf3-3a6c-4900-bb15-1f53117f1a8e");
const SCROLL_STATUS_ID: ObjectId = object_id!("898a986b-893d-48d8-bd68-5d39ef58c086");
const SCROLLER_STATUS_ID: ObjectId = object_id!("a7338149-f968-40a3-9bdd-e7640546e2fe");

pub(crate) struct ScrollIds {
    pub(crate) primary: ObjectId,
    pub(crate) nested: ObjectId,
    pub(crate) scroller: ObjectId,
    pub(crate) scroll_status: ObjectId,
    pub(crate) scroller_status: ObjectId,
}

pub(crate) fn ids() -> ScrollIds {
    ScrollIds {
        primary: PRIMARY_ID,
        nested: NESTED_ID,
        scroller: SCROLLER_ID,
        scroll_status: SCROLL_STATUS_ID,
        scroller_status: SCROLLER_STATUS_ID,
    }
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
    match &event.body {
        UiEventBody::ScrollChanged(_) if event.target_id == PRIMARY_ID => {
            Some(vec![Command::update_visual_element(
                SCROLL_STATUS_ID,
                Label::new("Moving"),
            )])
        }
        UiEventBody::ScrollSettled(value) if event.target_id == PRIMARY_ID => {
            Some(vec![Command::update_visual_element(
                SCROLL_STATUS_ID,
                Label::new(format!(
                    "Settled {:.0} × {:.0}",
                    value.offset.x, value.offset.y
                )),
            )])
        }
        UiEventBody::ValueChanging(value) if event.target_id == SCROLLER_ID => {
            let battlement::UiValue::F32(proposed) = value.proposed;
            Some(vec![Command::update_visual_element(
                SCROLLER_STATUS_ID,
                Label::new(format!("Preview {proposed:.0}")),
            )])
        }
        UiEventBody::ValueCommitted(value) if event.target_id == SCROLLER_ID => {
            let battlement::UiValue::F32(proposed) = value.proposed;
            Some(vec![
                Command::update_visual_element(SCROLLER_ID, Scroller::default().value(proposed)),
                Command::update_visual_element(
                    SCROLLER_STATUS_ID,
                    Label::new(format!("Committed {proposed:.0}")),
                ),
            ])
        }
        _ => None,
    }
}

pub(crate) fn scroll_page(page_id: ObjectId, ids: &ScrollIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("scroll-page"))
        .child(node(
            Label::new("SCROLL CONTROLS").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Motion, bounded and owned").style(design_system::title()),
        ))
        .child(
            node(VisualElement::new().style(scroll_styles::layout()))
                .child(
                    node(Box::new().style(scroll_styles::scroll_specimen()))
                        .child(node(
                            Label::new("NESTED TWO-AXIS").style(scroll_styles::caption()),
                        ))
                        .child(
                            UiNode::new(
                                ids.primary,
                                ScrollView::new()
                                    .name("primary-scroll")
                                    .mode(ScrollViewMode::VerticalAndHorizontal)
                                    .nested_interaction(NestedInteraction::ForwardScrolling)
                                    .horizontal_scroller_visibility(
                                        ScrollerVisibility::AlwaysVisible,
                                    )
                                    .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
                                    .scroll_offset(Vector::new(48.0, 132.0))
                                    .horizontal_page_size(0.8)
                                    .vertical_page_size(0.8)
                                    .mouse_wheel_scroll_size(36.0)
                                    .touch_scroll_behavior(TouchScrollBehavior::Elastic)
                                    .scroll_deceleration_rate(0.135)
                                    .elasticity(0.1)
                                    .elastic_animation_interval(16)
                                    .events([
                                        UiEventKind::ScrollChanged,
                                        UiEventKind::ScrollSettled,
                                    ])
                                    .style(scroll_styles::primary_scroll()),
                            )
                            .child(
                                node(VisualElement::new().style(scroll_styles::map()))
                                    .child(node(
                                        Label::new("SECTOR GRID").style(scroll_styles::map_title()),
                                    ))
                                    .child(nested_gallery(ids.nested))
                                    .child(node(
                                        Label::new("Beyond the viewport")
                                            .style(scroll_styles::map_note()),
                                    )),
                            ),
                        )
                        .child(UiNode::new(
                            ids.scroll_status,
                            Label::new("Settled 48 × 132")
                                .name("scroll-settlement-status")
                                .style(scroll_styles::status()),
                        )),
                )
                .child(
                    node(Box::new().style(scroll_styles::control_specimen()))
                        .child(node(
                            Label::new("CONTROLLED VALUE").style(scroll_styles::caption()),
                        ))
                        .child(node(
                            Label::new("Rust owns release").style(scroll_styles::control_heading()),
                        ))
                        .child(UiNode::new(
                            ids.scroller,
                            Scroller::new()
                                .name("controlled-scroller")
                                .low_value(0.0)
                                .high_value(100.0)
                                .value(42.0)
                                .direction(SliderDirection::Horizontal)
                                .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted])
                                .style(scroll_styles::scroller()),
                        ))
                        .child(UiNode::new(
                            ids.scroller_status,
                            Label::new("Committed 42")
                                .name("scroller-value-status")
                                .style(scroll_styles::value()),
                        ))
                        .child(node(
                            Label::new("Drag and release").style(scroll_styles::control_note()),
                        )),
                ),
        )
}

fn nested_gallery(object_id: ObjectId) -> UiNode {
    UiNode::new(
        object_id,
        ScrollView::new()
            .name("nested-horizontal-gallery")
            .mode(ScrollViewMode::Horizontal)
            .nested_interaction(NestedInteraction::ForwardScrolling)
            .horizontal_scroller_visibility(ScrollerVisibility::AlwaysVisible)
            .vertical_scroller_visibility(ScrollerVisibility::Hidden)
            .style(scroll_styles::nested_scroll()),
    )
    .child(
        node(VisualElement::new().style(scroll_styles::gallery())).children([
            gallery_card("ALPHA", "Ready"),
            gallery_card("BRAVO", "Moving"),
            gallery_card("CHARLIE", "Holding"),
            gallery_card("DELTA", "Clear"),
        ]),
    )
}

fn gallery_card(title: &str, status: &str) -> UiNode {
    node(Box::new().style(scroll_styles::card()))
        .child(node(Label::new(title).style(scroll_styles::card_title())))
        .child(node(Label::new(status).style(scroll_styles::card_status())))
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
