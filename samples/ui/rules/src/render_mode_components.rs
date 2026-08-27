use battlement::{Box, Image, Label, ObjectId, UiElement, UiNode, VisualElement};

use crate::{asset_catalog::ui::assets, design_system, render_mode_styles};

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("render-modes-page"))
        .child(node(Label::new("PANEL SETTINGS + TARGET TEXTURE").style(design_system::eyebrow())))
        .child(node(
            Label::new("Three scale contracts. One live panel target.")
                .style(render_mode_styles::page_title()),
        ))
        .child(node(Label::new("The screen-space lab remains ConstantPixelSize. Explicit specimens document the other scale modes, while the monitor shows a second UIDocument rendered into a leased RenderTexture.").style(render_mode_styles::intro())))
        .child(
            node(VisualElement::new().style(render_mode_styles::columns()))
                .child(scale_card())
                .child(target_card()),
        )
}

pub(crate) fn target_document(root_id: ObjectId) -> UiNode {
    UiNode::new(
        root_id,
        VisualElement::new()
            .name("target-texture-document")
            .style(render_mode_styles::target_root()),
    )
    .child(
        node(Box::new().style(render_mode_styles::target_panel()))
            .child(node(
                Label::new("BATTLEMENT SIGNAL").style(render_mode_styles::target_title()),
            ))
            .child(node(
                Label::new("TARGET TEXTURE | LIVE").style(render_mode_styles::target_status()),
            ))
            .child(node(
                Label::new("256 x 192  |  leased document output")
                    .style(render_mode_styles::detail()),
            )),
    )
}

fn scale_card() -> UiNode {
    node(Box::new().style(render_mode_styles::card(45.0, false)))
        .child(node(
            Label::new("SCREEN-SPACE SCALE CONTRACTS").style(render_mode_styles::caption()),
        ))
        .child(mode(
            "CONSTANT PIXEL SIZE | ACTIVE",
            "Scale 1.0; stable 1280 x 720 evidence baseline.",
            true,
        ))
        .child(mode(
            "CONSTANT PHYSICAL SIZE",
            "Reference DPI 96; fallback DPI 110.",
            false,
        ))
        .child(mode(
            "SCALE WITH SCREEN SIZE",
            "Reference 1280 x 720; width/height match 0.5.",
            false,
        ))
        .child(node(
            Label::new("Display 0 | depth clear ON | color clear OFF")
                .style(render_mode_styles::detail()),
        ))
        .child(node(
            Label::new("Atlas 64 -> 4096 | sub-texture 64 | five filters")
                .style(render_mode_styles::detail()),
        ))
}

fn target_card() -> UiNode {
    node(Box::new().style(render_mode_styles::card(55.0, true)))
        .child(node(
            Label::new("RENDER TARGET MONITOR | LIVE TEXTURE")
                .style(render_mode_styles::caption()),
        ))
        .child(node(Label::new("This image is the output of a separate UIDocument, not a duplicate authored hierarchy.").style(render_mode_styles::detail())))
        .child(
            node(Box::new().style(render_mode_styles::monitor())).child(node(
                Image::new()
                    .source(assets::RENDER_TEXTURE.clone())
                    .style(render_mode_styles::monitor_image()),
            )),
        )
        .child(node(Label::new("Panel inspector | ConstantPixelSize | target display 0").style(render_mode_styles::detail())))
        .child(node(Label::new("Target: RenderTexture | explicit pointer mapping required").style(render_mode_styles::detail())))
}

fn mode(title: &str, detail: &str, active: bool) -> UiNode {
    node(Box::new())
        .child(node(
            Label::new(title).style(render_mode_styles::mode_name(active)),
        ))
        .child(node(Label::new(detail).style(render_mode_styles::detail())))
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
