use battlement::{
  ObjectId, UiBox, UiButton, UiDropdownField, UiElement, UiLabel, UiNode, UiProgressBar, UiToggle,
  UiVisualElement,
};

use crate::{asset_catalog::ui::assets, design_system, part_styles};

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("simple-parts-page"))
        .child(node(
            UiLabel::new("NATIVE PART STYLING").style(design_system::eyebrow()),
        ))
        .child(node(
            UiLabel::new("Style the anatomy, keep ownership intact").style(design_system::title()),
        ))
        .child(node(
            UiLabel::new(
                "Rust exposes named style methods, never selectors. Each declaration resolves one Unity-created descendant beneath its owning control and updates only the fields it names.",
            )
            .style(part_styles::intro()),
        ))
        .child(
            node(UiVisualElement::new().style(part_styles::gallery()))
                .child(action_card())
                .child(field_card())
                .child(progress_card()),
        )
        .child(
            node(UiVisualElement::new().style(part_styles::legend()))
                .child(node(UiLabel::new("CYAN OUTLINE  resolved native boundary")))
                .child(node(UiLabel::new("GOLD  authored inner part")))
                .child(node(UiLabel::new("OWNER SCOPE  no global query"))),
        )
}

fn action_card() -> UiNode {
  node(UiBox::new().style(part_styles::card(31.0)))
        .child(node(UiLabel::new("BUTTON ANATOMY").style(part_styles::caption())))
        .child(node(
            UiLabel::new("Outer icon button + Unity image slot").style(part_styles::help()),
        ))
        .child(
            node(UiVisualElement::new().style(part_styles::specimen_row()))
                .child(node(UiLabel::new("PART · icon").style(part_styles::anatomy_label())))
                .child(
                    node(UiVisualElement::new().style(part_styles::button_line()))
                        .child(node(
                            UiButton::new("")
                                .icon(assets::VECTOR.clone())
                                .style(part_styles::button())
                                .icon_style(part_styles::button_icon()),
                        ))
                        .child(node(
                            UiLabel::new("Analyze action").style(part_styles::action_label()),
                        )),
                ),
        )
        .child(node(
            UiLabel::new("The icon asset lease belongs to the native slot; replacement leaves the button's unrelated style untouched.")
                .style(part_styles::help()),
        ))
}

fn field_card() -> UiNode {
  node(UiBox::new().style(part_styles::card(36.0)))
    .child(node(
      UiLabel::new("FIELD ANATOMY").style(part_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Input, mark, text, and arrow remain native").style(part_styles::help()),
    ))
    .child(
      node(UiVisualElement::new().style(part_styles::specimen_row()))
        .child(node(
          UiLabel::new("PARTS · input / checkmark / text").style(part_styles::anatomy_label()),
        ))
        .child(node(
          UiToggle::new()
            .text("Include archive")
            .value(true)
            .style(part_styles::toggle())
            .input_style(part_styles::toggle_input())
            .checkmark_style(part_styles::toggle_checkmark())
            .text_style(part_styles::control_text()),
        )),
    )
    .child(
      node(UiVisualElement::new().style(part_styles::specimen_row()))
        .child(node(
          UiLabel::new("PARTS · input / text / arrow").style(part_styles::anatomy_label()),
        ))
        .child(node(
          UiDropdownField::new()
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
  node(UiBox::new().style(part_styles::card(33.0)))
        .child(node(UiLabel::new("PROGRESS ANATOMY").style(part_styles::caption())))
        .child(node(
            UiLabel::new("Container, track, fill, and title").style(part_styles::help()),
        ))
        .child(
            node(UiVisualElement::new().style(part_styles::specimen_row()))
                .child(node(
                    UiLabel::new("PARTS · container / track / fill / title")
                        .style(part_styles::anatomy_label()),
                ))
                .child(node(
                    UiProgressBar::new()
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
            UiLabel::new("Sparse updates can recolor only the fill while every neighboring declaration and lease remains stable.")
                .style(part_styles::help()),
        ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
