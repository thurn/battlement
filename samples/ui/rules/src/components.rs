use battlement::{Box, Button, Label, ObjectId, UiElement, UiEventKind, UiNode, VisualElement};

use crate::design_system;

pub(crate) fn navigation(components_id: ObjectId, interactions_id: ObjectId) -> UiNode {
    node(
        Box::new()
            .name("navigation")
            .style(design_system::navigation()),
    )
    .child(node(
        Label::new("BATTLEMENT")
            .name("brand")
            .style(design_system::brand()),
    ))
    .child(navigation_item(components_id, "01  COMPONENTS", true))
    .child(navigation_item(interactions_id, "02  INTERACTIONS", false))
}

pub(crate) fn canvas(canvas_id: ObjectId, page_id: ObjectId, label_id: ObjectId) -> UiNode {
    UiNode::new(
        canvas_id,
        VisualElement::new()
            .name("specimen-canvas")
            .style(design_system::canvas()),
    )
    .child(components_page(page_id, label_id))
}

pub(crate) fn components_page(page_id: ObjectId, label_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("components-page"))
        .child(node(
            Label::new("COMPONENTS").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Rust-authored UI").style(design_system::title()),
        ))
        .child(
            node(
                Box::new()
                    .name("label-component")
                    .style(design_system::specimen()),
            )
            .child(node(
                Label::new("Label component").style(design_system::specimen_title()),
            ))
            .child(UiNode::new(
                label_id,
                Label::new("Hello from Rust").style(design_system::component_value()),
            )),
        )
}

pub(crate) fn interactions_page(page_id: ObjectId, button_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("interactions-page"))
        .child(node(
            Label::new("INTERACTIONS").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Rust callbacks").style(design_system::title()),
        ))
        .child(
            node(
                Box::new()
                    .name("button-interaction")
                    .style(design_system::specimen()),
            )
            .child(node(
                Label::new("Button interaction").style(design_system::specimen_title()),
            ))
            .child(UiNode::new(
                button_id,
                Button::new("Click to run a Rust callback")
                    .events([UiEventKind::Click])
                    .style(design_system::command_button()),
            )),
        )
}

pub(crate) fn greeting(greeting_id: ObjectId) -> UiNode {
    UiNode::new(
        greeting_id,
        Box::new()
            .name("rust-callback-result")
            .style(design_system::success()),
    )
    .child(node(
        Label::new("Hello, world").style(design_system::success_text()),
    ))
}

fn navigation_item(object_id: ObjectId, text: &str, active: bool) -> UiNode {
    UiNode::new(
        object_id,
        Button::new(text)
            .events([UiEventKind::Click])
            .style(design_system::navigation_item(active)),
    )
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
