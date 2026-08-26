use battlement::{
    Box, Button, Command, DropdownField, Label, ObjectId, UiElement, UiEvent, UiEventBody,
    UiEventKind, UiNode, UiValue, VisualElement, object_id,
};

use crate::{design_system, dropdown_styles};

pub(crate) const THEME_ID: ObjectId = object_id!("ae31830c-672e-4e99-b409-02ba8383d452");
pub(crate) const LOADOUT_ID: ObjectId = object_id!("2d5a2b47-1e52-45c2-b454-a178157133f0");
pub(crate) const CLEAR_ID: ObjectId = object_id!("c1834769-2048-40f4-953d-0268561883b5");
pub(crate) const THEME_SUMMARY_ID: ObjectId = object_id!("727e62a9-ebce-48cb-876f-20f86784b8cc");
pub(crate) const LOADOUT_SUMMARY_ID: ObjectId = object_id!("2e8fdeee-8310-4173-9daf-87905506c15c");
pub(crate) const STATUS_ID: ObjectId = object_id!("4c864234-bb34-43fd-bf0a-634a44111156");
pub(crate) const HISTORY_ID: ObjectId = object_id!("948fd3dd-ac76-4761-8831-e9abb02db7d5");

const THEMES: [&str; 3] = ["DUSK", "SOLAR", "VOID"];
const LOADOUTS: [&str; 3] = ["SCOUT", "HEAVY", "MEDIC"];

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("dropdown-page"))
        .child(node(Label::new("DROPDOWN FIELD").style(design_system::eyebrow())))
        .child(node(
            Label::new("Choose clearly. Commit deliberately.").style(design_system::title()),
        ))
        .child(node(
            Label::new(
                "Each proposal carries a matching index and value; Rust accepts, rejects, or clears it explicitly.",
            )
            .style(dropdown_styles::intro()),
        ))
        .child(
            node(VisualElement::new().style(dropdown_styles::gallery()))
                .child(theme_card())
                .child(loadout_card()),
        )
        .child(inspector())
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
    if event.target_id == CLEAR_ID && matches!(event.body, UiEventBody::Click(_)) {
        return Some(vec![
            Command::update_visual_element(LOADOUT_ID, DropdownField::new().clear_selection()),
            Command::update_visual_element(
                LOADOUT_SUMMARY_ID,
                Label::new("CLEARED · no selected index or value"),
            ),
            Command::update_visual_element(STATUS_ID, Label::new("LOADOUT · cleared by Rust")),
            Command::update_visual_element(
                HISTORY_ID,
                Label::new("CLEARED  SCOUT → NONE  |  (none, none)"),
            ),
            Command::update_visual_element(CLEAR_ID, Button::new("Loadout cleared").enabled(false)),
        ]);
    }
    let UiEventBody::ValueCommitted(commit) = &event.body else {
        return None;
    };
    let (previous, proposed) = choices(&commit.previous, &commit.proposed)?;
    match event.target_id {
        THEME_ID => {
            let index = proposed.index?;
            let value = proposed.value.as_deref()?;
            Some(vec![
                Command::update_visual_element(
                    THEME_ID,
                    DropdownField::new().selection(index, value),
                ),
                Command::update_visual_element(
                    THEME_SUMMARY_ID,
                    Label::new(format!("COMMITTED · {value} (index {index})")),
                ),
                Command::update_visual_element(
                    STATUS_ID,
                    Label::new(format!("THEME · {value} committed")),
                ),
                Command::update_visual_element(
                    HISTORY_ID,
                    Label::new(format!(
                        "ACCEPTED  {} → {value}  |  matching index + value",
                        value_or_none(previous.value.as_deref())
                    )),
                ),
            ])
        }
        LOADOUT_ID => Some(vec![
            Command::update_visual_element(
                STATUS_ID,
                Label::new(format!(
                    "REJECTED · {} remains uncommitted",
                    value_or_none(proposed.value.as_deref())
                )),
            ),
            Command::update_visual_element(
                LOADOUT_SUMMARY_ID,
                Label::new("COMMITTED · SCOUT (index 0)"),
            ),
            Command::update_visual_element(
                HISTORY_ID,
                Label::new(format!(
                    "REJECTED  {} → {}  |  native proposal rolled back",
                    value_or_none(previous.value.as_deref()),
                    value_or_none(proposed.value.as_deref())
                )),
            ),
        ]),
        _ => None,
    }
}

fn theme_card() -> UiNode {
    node(Box::new().style(dropdown_styles::card()))
        .child(node(
            Label::new("ACCEPTED THEME").style(dropdown_styles::caption()),
        ))
        .child(node(
            Label::new("Open the menu and choose SOLAR; Rust authors the matching pair.")
                .style(dropdown_styles::help()),
        ))
        .child(UiNode::new(
            THEME_ID,
            DropdownField::new()
                .name("theme-selector")
                .label("THEME")
                .choices(THEMES)
                .selection(0, THEMES[0])
                .events([UiEventKind::ValueCommitted])
                .style(dropdown_styles::dropdown()),
        ))
        .child(UiNode::new(
            THEME_SUMMARY_ID,
            Label::new("COMMITTED · DUSK (index 0)")
                .name("theme-summary")
                .style(dropdown_styles::selection_summary()),
        ))
}

fn loadout_card() -> UiNode {
    node(Box::new().style(dropdown_styles::final_card()))
        .child(node(
            Label::new("REJECTED + CLEARED LOADOUT").style(dropdown_styles::caption()),
        ))
        .child(node(
            Label::new("HEAVY is rejected and restored; the separate command clears both fields.")
                .style(dropdown_styles::help()),
        ))
        .child(UiNode::new(
            LOADOUT_ID,
            DropdownField::new()
                .name("loadout-selector")
                .label("LOADOUT")
                .choices(LOADOUTS)
                .selection(0, LOADOUTS[0])
                .events([UiEventKind::ValueCommitted])
                .style(dropdown_styles::dropdown()),
        ))
        .child(UiNode::new(
            CLEAR_ID,
            Button::new("Clear loadout")
                .name("clear-loadout")
                .events([UiEventKind::Click])
                .style(dropdown_styles::clear_button()),
        ))
        .child(UiNode::new(
            LOADOUT_SUMMARY_ID,
            Label::new("COMMITTED · SCOUT (index 0)")
                .name("loadout-summary")
                .style(dropdown_styles::selection_summary()),
        ))
}

fn inspector() -> UiNode {
    node(Box::new().style(dropdown_styles::inspector()))
        .child(UiNode::new(
            STATUS_ID,
            Label::new("READY · open a selector")
                .name("dropdown-status")
                .style(dropdown_styles::status()),
        ))
        .child(UiNode::new(
            HISTORY_ID,
            Label::new("DUSK [0]  |  SCOUT [0]")
                .name("dropdown-history")
                .style(dropdown_styles::history()),
        ))
}

fn choices<'a>(
    previous: &'a UiValue,
    proposed: &'a UiValue,
) -> Option<(&'a battlement::Choice, &'a battlement::Choice)> {
    match (previous, proposed) {
        (UiValue::Choice(previous), UiValue::Choice(proposed)) => Some((previous, proposed)),
        _ => None,
    }
}

fn value_or_none(value: Option<&str>) -> &str {
    value.unwrap_or("NONE")
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
