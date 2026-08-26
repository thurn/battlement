use battlement::{
    Box, Button, DropdownField, Label, ObjectId, ProgressBar, Toggle, UiElement, UiNode,
    VisualElement,
};

use crate::{asset_catalog::ui::assets, design_system, part_styles};

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("simple-parts-page"))
        .child(node(
            Label::new("PRIVATE PART STYLING").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Style the anatomy, keep ownership intact").style(design_system::title()),
        ))
        .child(node(
            Label::new(
                "Rust exposes named style methods, never selectors. Each declaration resolves one Unity-created descendant beneath its owning control and updates only the fields it names.",
            )
            .style(part_styles::intro()),
        ))
        .child(
            node(VisualElement::new().style(part_styles::gallery()))
                .child(action_card())
                .child(field_card())
                .child(progress_card()),
        )
        .child(
            node(VisualElement::new().style(part_styles::legend()))
                .child(node(Label::new("CYAN OUTLINE  resolved native boundary")))
                .child(node(Label::new("GOLD  authored inner part")))
                .child(node(Label::new("OWNER SCOPE  no global query"))),
        )
}

fn action_card() -> UiNode {
    node(Box::new().style(part_styles::card(31.0)))
        .child(node(Label::new("BUTTON ANATOMY").style(part_styles::caption())))
        .child(node(
            Label::new("Outer icon button + Unity image slot").style(part_styles::help()),
        ))
        .child(
            node(VisualElement::new().style(part_styles::specimen_row()))
                .child(node(Label::new("PART · icon").style(part_styles::anatomy_label())))
                .child(
                    node(VisualElement::new().style(part_styles::button_line()))
                        .child(node(
                            Button::new("")
                                .icon(assets::VECTOR.clone())
                                .style(part_styles::button())
                                .icon_style(part_styles::button_icon()),
                        ))
                        .child(node(
                            Label::new("Analyze action").style(part_styles::action_label()),
                        )),
                ),
        )
        .child(node(
            Label::new("The icon asset lease belongs to the private slot; replacement leaves the button's unrelated style untouched.")
                .style(part_styles::help()),
        ))
}

fn field_card() -> UiNode {
    node(Box::new().style(part_styles::card(36.0)))
        .child(node(
            Label::new("FIELD ANATOMY").style(part_styles::caption()),
        ))
        .child(node(
            Label::new("Input, mark, text, and arrow remain native").style(part_styles::help()),
        ))
        .child(
            node(VisualElement::new().style(part_styles::specimen_row()))
                .child(node(
                    Label::new("PARTS · input / checkmark / text")
                        .style(part_styles::anatomy_label()),
                ))
                .child(node(
                    Toggle::new()
                        .text("Include archive")
                        .value(true)
                        .style(part_styles::toggle())
                        .input_style(part_styles::toggle_input())
                        .checkmark_style(part_styles::toggle_checkmark())
                        .text_style(part_styles::control_text()),
                )),
        )
        .child(
            node(VisualElement::new().style(part_styles::specimen_row()))
                .child(node(
                    Label::new("PARTS · input / text / arrow").style(part_styles::anatomy_label()),
                ))
                .child(node(
                    DropdownField::new()
                        .choices(["Balanced", "Compact", "Spacious"])
                        .selection(0, "Balanced")
                        .style(part_styles::dropdown())
                        .input_style(part_styles::dropdown_input())
                        .text_style(part_styles::control_text())
                        .arrow_style(part_styles::dropdown_arrow()),
                )),
        )
}

fn progress_card() -> UiNode {
    node(Box::new().style(part_styles::card(33.0)))
        .child(node(Label::new("PROGRESS ANATOMY").style(part_styles::caption())))
        .child(node(
            Label::new("Container, track, fill, and title").style(part_styles::help()),
        ))
        .child(
            node(VisualElement::new().style(part_styles::specimen_row()))
                .child(node(
                    Label::new("PARTS · container / track / fill / title")
                        .style(part_styles::anatomy_label()),
                ))
                .child(node(
                    ProgressBar::new()
                        .low_value(0.0)
                        .high_value(100.0)
                        .value(68.0)
                        .title("INDEXED  68%")
                        .style(part_styles::progress())
                        .container_style(part_styles::progress_container())
                        .background_style(part_styles::progress_background())
                        .progress_style(part_styles::progress_fill())
                        .title_style(part_styles::progress_title()),
                )),
        )
        .child(node(
            Label::new("Sparse updates can recolor only the fill while every neighboring declaration and lease remains stable.")
                .style(part_styles::help()),
        ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
