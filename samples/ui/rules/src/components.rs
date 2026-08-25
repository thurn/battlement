use battlement::{
    Box, Button, Image, ImageScaleMode, Label, LanguageDirection, ObjectId, PickingMode, UiElement,
    UiEventKind, UiNode, UsageHint, VisualElement,
};

use crate::{asset_catalog::ui::assets, design_system};

pub(crate) fn navigation(
    components_id: ObjectId,
    interactions_id: ObjectId,
    hierarchy_id: ObjectId,
    assets_id: ObjectId,
    layout_id: ObjectId,
) -> UiNode {
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
    .child(navigation_item(hierarchy_id, "03  HIERARCHY", false))
    .child(navigation_item(assets_id, "04  ASSETS", false))
    .child(navigation_item(layout_id, "05  LAYOUT", false))
}

pub(crate) struct LayoutIds {
    pub(crate) playground: ObjectId,
    pub(crate) alpha: ObjectId,
    pub(crate) beta: ObjectId,
    pub(crate) gamma: ObjectId,
    pub(crate) action: ObjectId,
}

pub(crate) fn layout_page(page_id: ObjectId, ids: &LayoutIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("layout-page"))
        .child(node(Label::new("Layout").style(design_system::title())))
        .child(
            UiNode::new(
                ids.playground,
                Box::new()
                    .name("layout-playground")
                    .style(design_system::layout_playground()),
            )
            .child(UiNode::new(
                ids.alpha,
                Label::new("Alpha").style(design_system::layout_item()),
            ))
            .child(UiNode::new(
                ids.beta,
                Label::new("Beta").style(design_system::layout_item()),
            ))
            .child(UiNode::new(
                ids.gamma,
                Label::new("Gamma").style(design_system::layout_item()),
            )),
        )
        .child(UiNode::new(
            ids.action,
            Button::new("Column layout")
                .events([UiEventKind::Click])
                .style(design_system::command_button()),
        ))
}

pub(crate) struct AssetIds {
    pub(crate) texture: ObjectId,
    pub(crate) sprite: ObjectId,
    pub(crate) vector: ObjectId,
    pub(crate) render_texture: ObjectId,
    pub(crate) switched: ObjectId,
    pub(crate) active_address: ObjectId,
    pub(crate) switch_action: ObjectId,
}

pub(crate) fn assets_page(page_id: ObjectId, ids: &AssetIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("assets-page"))
        .child(node(Label::new("ASSETS").style(design_system::eyebrow())))
        .child(node(
            Label::new("Addressed sources").style(design_system::title()),
        ))
        .child(
            node(Box::new().style(design_system::asset_gallery()))
                .child(asset_card(
                    "Texture",
                    ids.texture,
                    Image::new().source(assets::TEXTURE.clone()),
                ))
                .child(asset_card(
                    "Sprite",
                    ids.sprite,
                    Image::new().source(assets::SPRITE.clone()),
                ))
                .child(asset_card(
                    "Vector",
                    ids.vector,
                    Image::new().source(assets::VECTOR.clone()),
                ))
                .child(asset_card(
                    "Render",
                    ids.render_texture,
                    Image::new()
                        .source(assets::RENDER_TEXTURE.clone())
                        .tint_color(battlement::Color::rgb(0.32, 0.92, 0.96)),
                )),
        )
        .child(
            node(Box::new().style(design_system::source_inspector()))
                .child(node(
                    Label::new("SWITCHED SOURCE").style(design_system::specimen_title()),
                ))
                .child(UiNode::new(
                    ids.switched,
                    Image::new()
                        .source(assets::TEXTURE.clone())
                        .scale_mode(ImageScaleMode::ScaleAndCrop)
                        .style(design_system::switched_image()),
                ))
                .child(UiNode::new(
                    ids.active_address,
                    Label::new(assets::TEXTURE.as_str()).style(design_system::address_value()),
                ))
                .child(UiNode::new(
                    ids.switch_action,
                    Button::new("Show sprite")
                        .events([UiEventKind::Click])
                        .style(design_system::command_button()),
                )),
        )
}

fn asset_card(label: &str, image_id: ObjectId, image: Image) -> UiNode {
    node(Box::new().style(design_system::asset_card()))
        .child(node(
            Label::new(label).style(design_system::specimen_title()),
        ))
        .child(UiNode::new(
            image_id,
            image.style(design_system::gallery_image()),
        ))
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

pub(crate) struct HierarchyIds {
    pub(crate) branch: ObjectId,
    pub(crate) primary: ObjectId,
    pub(crate) secondary: ObjectId,
    pub(crate) movable: ObjectId,
    pub(crate) destination: ObjectId,
    pub(crate) action: ObjectId,
}

pub(crate) fn hierarchy_page(page_id: ObjectId, ids: &HierarchyIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("hierarchy-page"))
        .child(node(Label::new("Hierarchy").style(design_system::title())))
        .child(
            node(
                Box::new()
                    .name("hierarchy-specimen")
                    .class("hierarchy-explorer")
                    .picking_mode(PickingMode::Position)
                    .language_direction(LanguageDirection::Ltr)
                    .focusable(true)
                    .tab_index(0)
                    .delegates_focus(true)
                    .usage_hints([UsageHint::DynamicTransform, UsageHint::DynamicColor])
                    .style(design_system::hierarchy_explorer()),
            )
            .child(
                UiNode::new(
                    ids.branch,
                    Box::new()
                        .name("logical-branch-a")
                        .class("hierarchy-branch")
                        .delegates_focus(true)
                        .style(design_system::hierarchy_branch()),
                )
                .child(UiNode::new(
                    ids.primary,
                    Label::new("Alpha")
                        .name("primary-child")
                        .enabled(true)
                        .picking_mode(PickingMode::Position)
                        .focusable(true)
                        .tab_index(1)
                        .class("ready")
                        .style(design_system::hierarchy_item()),
                ))
                .child(UiNode::new(
                    ids.secondary,
                    Label::new("Beta")
                        .name("secondary-child")
                        .language_direction(LanguageDirection::Rtl)
                        .style(design_system::hierarchy_item()),
                ))
                .child(UiNode::new(
                    ids.movable,
                    Label::new("Move")
                        .name("movable-child")
                        .picking_mode(PickingMode::Ignore)
                        .style(design_system::hierarchy_item()),
                )),
            )
            .child(
                UiNode::new(
                    ids.destination,
                    Box::new()
                        .name("logical-branch-b")
                        .class("hierarchy-branch")
                        .style(design_system::hierarchy_branch()),
                )
                .child(node(
                    Label::new("Target").style(design_system::hierarchy_item()),
                )),
            )
            .child(UiNode::new(
                ids.action,
                Button::new("Reorder children")
                    .focusable(true)
                    .tab_index(2)
                    .events([UiEventKind::Click])
                    .style(design_system::command_button()),
            )),
        )
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
