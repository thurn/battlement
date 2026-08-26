use battlement::{
    Box, Command, Label, ObjectId, RadioButton, Toggle, UiElement, UiEvent, UiEventBody,
    UiEventKind, UiNode, UiValue, VisualElement, object_id,
};

use crate::{boolean_styles, design_system};

pub(crate) const ACCEPTED_TOGGLE_ID: ObjectId = object_id!("93ecbf8e-5be7-4087-b292-6f68903436c1");
pub(crate) const REJECTED_TOGGLE_ID: ObjectId = object_id!("d18a9439-619d-4ca8-ac58-d82d999b3bf1");
pub(crate) const ACCEPTED_RADIO_ID: ObjectId = object_id!("bfe98ac4-cfa5-4f56-8e6a-253837c66c05");
pub(crate) const REJECTED_RADIO_ID: ObjectId = object_id!("174b5d07-dd4f-4fe6-a264-3863ea6bc318");
pub(crate) const STATUS_ID: ObjectId = object_id!("1745a91d-06f7-460c-bd3b-bd1f432332c0");
pub(crate) const HISTORY_ID: ObjectId = object_id!("65cba5dd-fc33-49e3-a636-a6d4fc59e73d");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("boolean-controls-page"))
        .child(node(
            Label::new("CONTROLLED BOOLEAN").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Propose immediately. Rust decides.").style(design_system::title()),
        ))
        .child(node(
            Label::new(
                "Every click rolls native state back first; only the Rust response can change the committed value.",
            )
            .style(boolean_styles::intro()),
        ))
        .child(
            node(VisualElement::new().style(boolean_styles::gallery()))
                .child(settings_card())
                .child(radios_card()),
        )
        .child(inspector())
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
    let UiEventBody::ValueCommitted(value) = &event.body else {
        return None;
    };
    let (previous, proposed) = values(&value.previous, &value.proposed)?;
    let history = Command::update_visual_element(
        HISTORY_ID,
        Label::new(format!(
            "PROPOSAL  {} → {}  |  committed before callback: {}",
            state(previous),
            state(proposed),
            state(previous),
        )),
    );
    match event.target_id {
        ACCEPTED_TOGGLE_ID => Some(vec![
            Command::update_visual_element(ACCEPTED_TOGGLE_ID, Toggle::new().value(proposed)),
            Command::update_visual_element(
                STATUS_ID,
                Label::new(format!(
                    "ACCEPTED · threat alerts committed {}",
                    state(proposed)
                )),
            ),
            history,
        ]),
        REJECTED_TOGGLE_ID => Some(vec![
            Command::update_visual_element(
                STATUS_ID,
                Label::new("REJECTED · safety interlock remains ON"),
            ),
            history,
        ]),
        ACCEPTED_RADIO_ID => Some(vec![
            Command::update_visual_element(ACCEPTED_RADIO_ID, RadioButton::new().value(proposed)),
            Command::update_visual_element(
                STATUS_ID,
                Label::new("ACCEPTED · command channel committed"),
            ),
            history,
        ]),
        REJECTED_RADIO_ID => Some(vec![
            Command::update_visual_element(
                STATUS_ID,
                Label::new("REJECTED · restricted channel stays OFF"),
            ),
            history,
        ]),
        _ => None,
    }
}

fn settings_card() -> UiNode {
    node(Box::new().style(boolean_styles::card()))
        .child(node(
            Label::new("SETTINGS TOGGLES").style(boolean_styles::caption()),
        ))
        .child(node(
            Label::new("Accepted, rejected, and disabled states share one Boolean contract.")
                .style(boolean_styles::help()),
        ))
        .child(UiNode::new(
            ACCEPTED_TOGGLE_ID,
            Toggle::new()
                .name("accepted-toggle")
                .label("ACCEPTED")
                .text("Threat alerts")
                .value(false)
                .events([UiEventKind::ValueCommitted])
                .style(boolean_styles::control()),
        ))
        .child(UiNode::new(
            REJECTED_TOGGLE_ID,
            Toggle::new()
                .name("rejected-toggle")
                .label("REJECTED")
                .text("Safety interlock")
                .value(true)
                .events([UiEventKind::ValueCommitted])
                .style(boolean_styles::control()),
        ))
        .child(node(
            Toggle::new()
                .name("disabled-toggle")
                .label("DISABLED")
                .text("Remote override")
                .value(false)
                .enabled(false)
                .events([UiEventKind::ValueCommitted])
                .style(boolean_styles::control()),
        ))
}

fn radios_card() -> UiNode {
    node(Box::new().style(boolean_styles::final_card()))
        .child(node(
            Label::new("STANDALONE RADIO BUTTONS").style(boolean_styles::caption()),
        ))
        .child(node(
            Label::new(
                "Standalone radio proposals remain independent until grouping arrives next.",
            )
            .style(boolean_styles::help()),
        ))
        .child(UiNode::new(
            ACCEPTED_RADIO_ID,
            RadioButton::new()
                .name("accepted-radio")
                .label("ACCEPTED")
                .text("Command channel")
                .value(false)
                .events([UiEventKind::ValueCommitted])
                .style(boolean_styles::control()),
        ))
        .child(UiNode::new(
            REJECTED_RADIO_ID,
            RadioButton::new()
                .name("rejected-radio")
                .label("REJECTED")
                .text("Restricted channel")
                .value(false)
                .events([UiEventKind::ValueCommitted])
                .style(boolean_styles::control()),
        ))
}

fn inspector() -> UiNode {
    node(Box::new().style(boolean_styles::inspector()))
        .child(UiNode::new(
            STATUS_ID,
            Label::new("READY · choose any enabled specimen")
                .name("boolean-status")
                .style(boolean_styles::status()),
        ))
        .child(UiNode::new(
            HISTORY_ID,
            Label::new("PROPOSAL  —  |  no callback yet")
                .name("boolean-history")
                .style(boolean_styles::history()),
        ))
}

fn values(previous: &UiValue, proposed: &UiValue) -> Option<(bool, bool)> {
    match (previous, proposed) {
        (UiValue::Bool(previous), UiValue::Bool(proposed)) => Some((*previous, *proposed)),
        _ => None,
    }
}

fn state(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
