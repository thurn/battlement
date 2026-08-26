use std::num::NonZeroU32;

use battlement::{
    Box, Button, Color, FilterFunction, Image, ImageScaleMode, Label, LanguageDirection, ObjectId,
    PickingMode, RepeatButton, TextElement, TextOverflowPosition, TransformOrigin, UiElement,
    UiEventKind, UiNode, UsageHint, VisualElement, WhiteSpace,
};

use crate::{
    appearance_styles, asset_catalog::ui::assets, asset_styles, background_styles, button_styles,
    component_styles, design_system, hierarchy_styles, interaction_styles, layout_styles,
    transform_styles, typography_styles,
};

pub(crate) struct NavigationIds {
    pub(crate) components: ObjectId,
    pub(crate) interactions: ObjectId,
    pub(crate) hierarchy: ObjectId,
    pub(crate) assets: ObjectId,
    pub(crate) layout: ObjectId,
    pub(crate) appearance: ObjectId,
    pub(crate) backgrounds: ObjectId,
    pub(crate) transforms: ObjectId,
    pub(crate) typography: ObjectId,
    pub(crate) buttons: ObjectId,
    pub(crate) containers: ObjectId,
    pub(crate) scroll: ObjectId,
    pub(crate) tabs: ObjectId,
    pub(crate) text_fields: ObjectId,
    pub(crate) boolean_controls: ObjectId,
    pub(crate) choice_groups: ObjectId,
    pub(crate) dropdowns: ObjectId,
    pub(crate) sliders: ObjectId,
    pub(crate) ranges: ObjectId,
    pub(crate) parts: ObjectId,
}

pub(crate) fn navigation(ids: &NavigationIds) -> UiNode {
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
    .child(navigation_item(ids.components, "01  COMPONENTS", true))
    .child(navigation_item(ids.interactions, "02  INTERACTIONS", false))
    .child(navigation_item(ids.hierarchy, "03  HIERARCHY", false))
    .child(navigation_item(ids.assets, "04  ASSETS", false))
    .child(navigation_item(ids.layout, "05  LAYOUT", false))
    .child(navigation_item(ids.appearance, "06  APPEARANCE", false))
    .child(navigation_item(ids.backgrounds, "07  BACKGROUNDS", false))
    .child(navigation_item(ids.transforms, "08  TRANSFORMS", false))
    .child(navigation_item(ids.typography, "09  TYPOGRAPHY", false))
    .child(navigation_item(ids.buttons, "10  BUTTONS", false))
    .child(navigation_item(ids.containers, "11  CONTAINERS", false))
    .child(navigation_item(ids.scroll, "12  SCROLL", false))
    .child(navigation_item(ids.tabs, "13  TABS", false))
    .child(navigation_item(ids.text_fields, "14  TEXT FIELDS", false))
    .child(navigation_item(
        ids.boolean_controls,
        "15  TOGGLE + RADIO",
        false,
    ))
    .child(navigation_item(
        ids.choice_groups,
        "16  CHOICE GROUPS",
        false,
    ))
    .child(navigation_item(ids.dropdowns, "17  DROPDOWNS", false))
    .child(navigation_item(ids.sliders, "18  SLIDERS", false))
    .child(navigation_item(ids.ranges, "19  RANGES + PROGRESS", false))
    .child(navigation_item(ids.parts, "20  PRIVATE PARTS", false))
}

pub(crate) struct ButtonIds {
    pub(crate) ordinary: ObjectId,
    pub(crate) icon: ObjectId,
    pub(crate) disabled: ObjectId,
    pub(crate) navigation: ObjectId,
    pub(crate) repeat: ObjectId,
    pub(crate) counter: ObjectId,
    pub(crate) status: ObjectId,
}

pub(crate) fn buttons_page(page_id: ObjectId, ids: &ButtonIds, repeat_count: u32) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("buttons-page"))
        .child(node(Label::new("BUTTONS").style(design_system::eyebrow())))
        .child(node(Label::new("Clear commands, every input").style(design_system::title())))
        .child(node(
            Label::new("Pointer, keyboard, icon, disabled, and press-and-hold behavior share one typed Rust contract.")
                .style(button_styles::intro()),
        ))
        .child(
            node(VisualElement::new().style(button_styles::gallery()))
                .child(button_card(
                    "ORDINARY COMMAND",
                    "A single pointer release submits once.",
                    UiNode::new(
                        ids.ordinary,
                        Button::new("Deploy squad")
                            .name("ordinary-command")
                            .events([UiEventKind::Click])
                            .style(button_styles::button()),
                    ),
                ))
                .child(button_card(
                    "ICON + TEXT",
                    "The icon is a prepared VectorImage lease.",
                    UiNode::new(
                        ids.icon,
                        Button::new("Loadout")
                            .name("icon-command")
                            .icon(assets::VECTOR.clone())
                            .events([UiEventKind::Click])
                            .style(button_styles::icon_button()),
                    ),
                ))
                .child(button_card(
                    "DISABLED",
                    "Native disabled state blocks every activation.",
                    UiNode::new(
                        ids.disabled,
                        Button::new("Mission locked")
                            .name("disabled-command")
                            .enabled(false)
                            .style(button_styles::button()),
                    ),
                ))
                .child(button_card(
                    "NAVIGATION SUBMIT",
                    "Tab to focus, then Space or gamepad submit.",
                    UiNode::new(
                        ids.navigation,
                        Button::new("Confirm selection")
                            .name("navigation-command")
                            .focusable(true)
                            .tab_index(0)
                            .events([UiEventKind::Click])
                            .style(button_styles::navigation_button()),
                    ),
                ))
                .child(
                    node(Box::new().style(button_styles::repeat_card()))
                        .child(node(Label::new("PRESS + HOLD").style(button_styles::caption())))
                        .child(
                            node(VisualElement::new().style(button_styles::repeat_row()))
                                .child(UiNode::new(
                                    ids.repeat,
                                    RepeatButton::new(
                                        "Reinforce",
                                        320,
                                        NonZeroU32::new(160).expect("constant interval is positive"),
                                    )
                                    .name("repeat-command")
                                    .events([UiEventKind::Click])
                                    .style(button_styles::repeat_button()),
                                ))
                                .child(UiNode::new(
                                    ids.counter,
                                    Label::new(repeat_count.to_string())
                                        .name("repeat-counter")
                                        .style(button_styles::counter()),
                                )),
                        )
                        .child(node(
                            Label::new(
                                "Immediate; 320/160 ms initially, then 200/100 ms after callback 4.",
                            )
                                .style(button_styles::help()),
                        )),
                ))
        .child(UiNode::new(
            ids.status,
            Label::new("Ready | choose a command")
                .name("button-status")
                .style(button_styles::status()),
        ))
}

fn button_card(caption: &str, help: &str, control: UiNode) -> UiNode {
    node(Box::new().style(button_styles::card()))
        .child(node(Label::new(caption).style(button_styles::caption())))
        .child(control)
        .child(node(Label::new(help).style(button_styles::help())))
}

pub(crate) fn typography_page(page_id: ObjectId) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("typography-page"))
        .child(node(Label::new("Typography").style(design_system::title())))
        .child(
            node(Box::new().style(typography_styles::matrix()))
                .child(typography_card(
                    "TextCore",
                    Label::new("Battlement").style(typography_styles::font_definition()),
                ))
                .child(typography_card(
                    "Weight",
                    Label::new("Bold italic").style(typography_styles::weight()),
                ))
                .child(typography_card(
                    "Alignment",
                    Label::new("Centered").style(typography_styles::alignment()),
                ))
                .child(typography_card(
                    "Auto size",
                    Label::new("Adaptive signal").style(typography_styles::auto_size()),
                ))
                .child(typography_card(
                    "Outline",
                    Label::new("Luminous").style(typography_styles::outline_shadow()),
                ))
                .child(typography_card(
                    "Spacing",
                    Label::new("Letter word\nParagraph").style(typography_styles::spacing()),
                ))
                .child(typography_card(
                    "Elision",
                    Label::new("Beginning middle ending signal transmission")
                        .tooltip_when_elided(true)
                        .style(typography_styles::elision(TextOverflowPosition::Middle)),
                ))
                .child(typography_card(
                    "Rich emoji",
                    Label::new("<b>Ready</b> 🚀\\nNext")
                        .rich_text(true)
                        .emoji_fallback(true)
                        .parse_escape_sequences(true)
                        .style(typography_styles::rich()),
                ))
                .child(typography_card(
                    "Selectable",
                    TextElement::new("<b>Select</b> this signal")
                        .name("selectable-rich-text")
                        .rich_text(true)
                        .selectable(true)
                        .focusable(true)
                        .double_click_selects_word(true)
                        .triple_click_selects_line(true)
                        .select_all_on_focus(false)
                        .select_all_on_mouse_up(false)
                        .style(typography_styles::selectable(WhiteSpace::NoWrap)),
                )),
        )
}

fn typography_card(label: &str, value: impl Into<UiElement>) -> UiNode {
    node(Box::new().style(typography_styles::card()))
        .child(node(Label::new(label).style(typography_styles::caption())))
        .child(node(value))
}

pub(crate) struct TransformIds {
    pub(crate) target: ObjectId,
    pub(crate) status: ObjectId,
    pub(crate) action: ObjectId,
}

pub(crate) fn transforms_page(page_id: ObjectId, ids: &TransformIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("transforms-page"))
        .child(node(Label::new("Transforms").style(design_system::title())))
        .child(
            node(Box::new().style(transform_styles::row()))
                .child(origin_card(
                    "Top left",
                    TransformOrigin::two_dimensional(0.into(), 0.into()),
                ))
                .child(origin_card(
                    "Center",
                    TransformOrigin::two_dimensional(
                        battlement::Length::Percent(50.0),
                        battlement::Length::Percent(50.0),
                    ),
                ))
                .child(origin_card(
                    "Bottom right",
                    TransformOrigin::two_dimensional(
                        battlement::Length::Percent(100.0),
                        battlement::Length::Percent(100.0),
                    ),
                )),
        )
        .child(
            node(Box::new().style(transform_styles::row()))
                .child(filter_slot(
                    "Tint",
                    FilterFunction::Tint(Color::rgb(1.0, 0.72, 0.3)),
                ))
                .child(filter_slot("Opacity", FilterFunction::Opacity(0.82)))
                .child(filter_slot("Invert", FilterFunction::Invert(0.65)))
                .child(filter_slot("Grayscale", FilterFunction::Grayscale(0.8)))
                .child(filter_slot("Sepia", FilterFunction::Sepia(0.75)))
                .child(filter_slot("Blur", FilterFunction::Blur(1.5)))
                .child(filter_slot("Contrast", FilterFunction::Contrast(1.35)))
                .child(filter_slot("Hue", FilterFunction::HueRotate(110.0))),
        )
        .child(
            node(Box::new().style(transform_styles::transition_stage()))
                .child(
                    UiNode::new(
                        ids.target,
                        Box::new()
                            .name("transition-target")
                            .usage_hints([UsageHint::DynamicTransform, UsageHint::DynamicColor])
                            .events([
                                UiEventKind::TransitionStart,
                                UiEventKind::TransitionEnd,
                                UiEventKind::TransitionCancel,
                            ])
                            .style(transform_styles::transition_initial()),
                    )
                    .child(node(Label::new("Signal").style(transform_styles::label()))),
                )
                .child(UiNode::new(
                    ids.status,
                    Label::new("Ready")
                        .name("transition-status")
                        .style(transform_styles::transition_status()),
                )),
        )
        .child(UiNode::new(
            ids.action,
            Button::new("Launch")
                .events([UiEventKind::Click])
                .style(design_system::command_button()),
        ))
}

fn origin_card(label: &str, origin: TransformOrigin) -> UiNode {
    node(Box::new().style(transform_styles::origin_card()))
        .child(node(
            Box::new().style(transform_styles::origin_mark(origin)),
        ))
        .child(node(Label::new(label).style(transform_styles::label())))
}

fn filter_slot(label: &str, filter: FilterFunction) -> UiNode {
    node(Box::new().style(transform_styles::filter_slot()))
        .child(node(
            Box::new().style(transform_styles::filter_swatch(filter)),
        ))
        .child(node(Label::new(label).style(transform_styles::label())))
}

pub(crate) struct BackgroundIds {
    pub(crate) texture: ObjectId,
    pub(crate) sprite: ObjectId,
    pub(crate) vector: ObjectId,
    pub(crate) render_texture: ObjectId,
    pub(crate) cursor_preview: ObjectId,
    pub(crate) action: ObjectId,
}

pub(crate) fn backgrounds_page(page_id: ObjectId, ids: &BackgroundIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("backgrounds-page"))
        .child(node(
            Label::new("Backgrounds").style(design_system::title()),
        ))
        .child(
            node(Box::new().style(background_styles::gallery()))
                .child(background_card(
                    ids.texture,
                    "Texture",
                    background_styles::interactive(
                        battlement::BackgroundSource::Texture(assets::TEXTURE.clone()),
                        assets::CURSOR.clone(),
                    ),
                ))
                .child(background_card(
                    ids.sprite,
                    "Sprite",
                    background_styles::source_card(
                        battlement::BackgroundSource::Sprite(assets::SPRITE.clone()),
                        1,
                    ),
                ))
                .child(background_card(
                    ids.vector,
                    "Vector",
                    background_styles::source_card(
                        battlement::BackgroundSource::VectorImage(assets::VECTOR.clone()),
                        2,
                    ),
                ))
                .child(background_card(
                    ids.render_texture,
                    "Render",
                    background_styles::source_card(
                        battlement::BackgroundSource::RenderTexture(assets::RENDER_TEXTURE.clone()),
                        3,
                    ),
                )),
        )
        .child(
            node(Box::new().style(background_styles::gallery()))
                .child(node(
                    Label::new("Auto Cover Contain Explicit").style(background_styles::label()),
                ))
                .child(node(
                    Label::new("Repeat Space Round No-repeat").style(background_styles::label()),
                ))
                .child(UiNode::new(
                    ids.cursor_preview,
                    Image::new()
                        .name("background-cursor-preview")
                        .source(assets::CURSOR.clone())
                        .style(background_styles::cursor_preview()),
                ))
                .child(node(
                    Label::new("Hover Texture").style(background_styles::label()),
                )),
        )
        .child(UiNode::new(
            ids.action,
            Button::new("Apply")
                .events([UiEventKind::Click])
                .style(design_system::command_button()),
        ))
}

fn background_card(object_id: ObjectId, label: &str, style: battlement::Style) -> UiNode {
    UiNode::new(
        object_id,
        Box::new()
            .name(format!("background-{}", label.to_lowercase()))
            .style(style),
    )
    .child(node(Label::new(label).style(background_styles::label())))
}

pub(crate) struct AppearanceIds {
    pub(crate) square: ObjectId,
    pub(crate) rounded: ObjectId,
    pub(crate) sliced: ObjectId,
    pub(crate) opacity: ObjectId,
    pub(crate) clipped: ObjectId,
    pub(crate) hidden: ObjectId,
    pub(crate) removed: ObjectId,
    pub(crate) action: ObjectId,
}

pub(crate) fn appearance_page(page_id: ObjectId, ids: &AppearanceIds) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("appearance-page"))
        .child(node(Label::new("Appearance").style(design_system::title())))
        .child(
            node(Box::new().style(appearance_styles::matrix()))
                .child(appearance_card(
                    ids.square,
                    "Square",
                    appearance_styles::square(),
                ))
                .child(appearance_card(
                    ids.rounded,
                    "Rounded",
                    appearance_styles::rounded(),
                ))
                .child(appearance_sliced_card(
                    ids.sliced,
                    "Sliced",
                    appearance_styles::sliced(assets::SPRITE.clone()),
                )),
        )
        .child(
            node(Box::new().style(appearance_styles::matrix()))
                .child(appearance_opacity_card(ids.opacity))
                .child(
                    UiNode::new(
                        ids.clipped,
                        Box::new()
                            .name("appearance-clipped")
                            .style(appearance_styles::clipped()),
                    )
                    .child(node(
                        Box::new().style(appearance_styles::overflow_content()),
                    ))
                    .child(node(
                        Label::new("Clipped").style(appearance_styles::overlay_label()),
                    )),
                ),
        )
        .child(
            node(Box::new().style(appearance_styles::matrix()))
                .child(appearance_visibility_slot("Hidden", ids.hidden, true))
                .child(appearance_visibility_slot("Removed", ids.removed, false)),
        )
        .child(UiNode::new(
            ids.action,
            Button::new("Show visibility")
                .events([UiEventKind::Click])
                .style(design_system::command_button()),
        ))
}

fn appearance_card(object_id: ObjectId, label: &str, style: battlement::Style) -> UiNode {
    UiNode::new(
        object_id,
        Box::new()
            .name(format!("appearance-{}", label.to_lowercase()))
            .style(style),
    )
    .child(node(Label::new(label).style(appearance_styles::label())))
}

fn appearance_sliced_card(object_id: ObjectId, label: &str, style: battlement::Style) -> UiNode {
    UiNode::new(
        object_id,
        Box::new()
            .name(format!("appearance-{}", label.to_lowercase()))
            .style(style),
    )
    .child(node(
        Label::new(label).style(appearance_styles::overlay_label()),
    ))
}

fn appearance_opacity_card(object_id: ObjectId) -> UiNode {
    node(Box::new().style(appearance_styles::opacity_card()))
        .child(UiNode::new(
            object_id,
            Box::new()
                .name("appearance-opacity")
                .style(appearance_styles::faded()),
        ))
        .child(node(
            Label::new("Opacity").style(appearance_styles::label()),
        ))
}

fn appearance_visibility_slot(label: &str, object_id: ObjectId, hidden: bool) -> UiNode {
    node(Box::new().style(appearance_styles::visibility_slot()))
        .child(node(Label::new(label).style(appearance_styles::label())))
        .child(UiNode::new(
            object_id,
            Box::new()
                .name(format!("appearance-{}", label.to_lowercase()))
                .style(if hidden {
                    appearance_styles::hidden()
                } else {
                    appearance_styles::removed()
                }),
        ))
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
                    .style(layout_styles::playground()),
            )
            .child(UiNode::new(
                ids.alpha,
                Label::new("Alpha").style(layout_styles::item()),
            ))
            .child(UiNode::new(
                ids.beta,
                Label::new("Beta").style(layout_styles::item()),
            ))
            .child(UiNode::new(
                ids.gamma,
                Label::new("Gamma").style(layout_styles::item()),
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
            node(Box::new().style(asset_styles::gallery()))
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
            node(Box::new().style(asset_styles::inspector()))
                .child(node(
                    Label::new("SWITCHED SOURCE").style(design_system::specimen_title()),
                ))
                .child(UiNode::new(
                    ids.switched,
                    Image::new()
                        .source(assets::TEXTURE.clone())
                        .scale_mode(ImageScaleMode::ScaleAndCrop)
                        .style(asset_styles::switched_image()),
                ))
                .child(UiNode::new(
                    ids.active_address,
                    Label::new(assets::TEXTURE.as_str()).style(asset_styles::address()),
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
    node(Box::new().style(asset_styles::card()))
        .child(node(
            Label::new(label).style(design_system::specimen_title()),
        ))
        .child(UiNode::new(image_id, image.style(asset_styles::image())))
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
                Label::new("Hello from Rust").style(component_styles::value()),
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
            .style(interaction_styles::result()),
    )
    .child(node(
        Label::new("Hello, world").style(interaction_styles::result_text()),
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
                    .style(hierarchy_styles::explorer()),
            )
            .child(
                UiNode::new(
                    ids.branch,
                    Box::new()
                        .name("logical-branch-a")
                        .class("hierarchy-branch")
                        .delegates_focus(true)
                        .style(hierarchy_styles::branch()),
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
                        .style(hierarchy_styles::item()),
                ))
                .child(UiNode::new(
                    ids.secondary,
                    Label::new("Beta")
                        .name("secondary-child")
                        .language_direction(LanguageDirection::Rtl)
                        .style(hierarchy_styles::item()),
                ))
                .child(UiNode::new(
                    ids.movable,
                    Label::new("Move")
                        .name("movable-child")
                        .picking_mode(PickingMode::Ignore)
                        .style(hierarchy_styles::item()),
                )),
            )
            .child(
                UiNode::new(
                    ids.destination,
                    Box::new()
                        .name("logical-branch-b")
                        .class("hierarchy-branch")
                        .style(hierarchy_styles::branch()),
                )
                .child(node(Label::new("Target").style(hierarchy_styles::item()))),
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
