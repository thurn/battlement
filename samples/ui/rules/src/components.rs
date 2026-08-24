use battlement::{Box, Label, ObjectId, VisualElement};

use crate::design_system;

pub(crate) fn navigation() -> Box {
    Box::new()
        .name("navigation")
        .style(design_system::navigation())
        .child(
            Label::new("BATTLEMENT")
                .name("brand")
                .style(design_system::brand()),
        )
        .children([
            navigation_item("01  OVERVIEW", true),
            navigation_item("02  HIERARCHY", false),
            navigation_item("03  ASSETS", false),
            navigation_item("04  STYLING", false),
            navigation_item("05  CONTROLS", false),
            navigation_item("06  EVENTS", false),
            navigation_item("07  RENDER MODES", false),
        ])
}

pub(crate) fn canvas() -> VisualElement {
    VisualElement::new()
        .name("specimen-canvas")
        .style(design_system::canvas())
        .child(eyebrow("UI FOUNDATION / OVERVIEW"))
        .child(title("COMMAND DECK"))
        .child(first_specimen())
}

pub(crate) fn inspector(root_id: ObjectId, source: Option<&str>) -> Box {
    Box::new()
        .name("inspector")
        .style(design_system::inspector())
        .child(eyebrow("STATE / EVENT / COMMAND"))
        .child(Label::new("DOCUMENT ROOT").style(design_system::inspector_heading()))
        .child(Label::new(root_id.to_string()).style(design_system::inspector_identity()))
        .optional_child(source.map(|value| {
            Label::new(format!(
                "type   VisualElement\nmode   ScreenSpaceOverlay\nsource {value}"
            ))
        }))
}

fn navigation_item(text: &str, active: bool) -> Label {
    Label::new(text).style(design_system::navigation_item(active))
}

fn eyebrow(text: impl Into<String>) -> Label {
    Label::new(text).style(design_system::eyebrow())
}

fn title(text: impl Into<String>) -> Label {
    Label::new(text).style(design_system::title())
}

fn first_specimen() -> Box {
    Box::new()
        .name("first-specimen")
        .style(design_system::specimen())
        .child(Label::new("FIRST RUST-AUTHORED LABEL").style(design_system::specimen_title()))
        .child(Label::new(
            "VisualElement → Box → Label\nScreen-space document online",
        ))
}
