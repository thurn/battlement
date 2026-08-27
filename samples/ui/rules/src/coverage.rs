use std::collections::HashSet;

use crate::coverage_parts;

pub(crate) struct CoverageGroup {
    pub(crate) title: &'static str,
    pub(crate) capabilities: &'static [&'static str],
    pub(crate) specimen: &'static str,
    pub(crate) test_family: &'static str,
}

pub(crate) const ELEMENTS: &[&str] = &[
    "VisualElement",
    "Box",
    "Label",
    "TextElement",
    "TextField",
    "Toggle",
    "RadioButton",
    "RadioButtonGroup",
    "ToggleButtonGroup",
    "DropdownField",
    "Button",
    "RepeatButton",
    "GroupBox",
    "PopupWindow",
    "ScrollView",
    "Scroller",
    "Slider",
    "SliderInt",
    "MinMaxSlider",
    "ProgressBar",
    "Tab",
    "TabView",
    "Image",
];

pub(crate) const OUTER_STYLES: &[&str] = &[
    "align_content",
    "align_items",
    "align_self",
    "aspect_ratio",
    "background_color",
    "background_image",
    "background_position_x",
    "background_position_y",
    "background_repeat",
    "background_size",
    "border_bottom_color",
    "border_bottom_left_radius",
    "border_bottom_right_radius",
    "border_bottom_width",
    "border_left_color",
    "border_left_width",
    "border_right_color",
    "border_right_width",
    "border_top_color",
    "border_top_left_radius",
    "border_top_right_radius",
    "border_top_width",
    "bottom",
    "color",
    "cursor",
    "display",
    "filter",
    "flex_basis",
    "flex_direction",
    "flex_grow",
    "flex_shrink",
    "flex_wrap",
    "font_size",
    "height",
    "justify_content",
    "letter_spacing",
    "left",
    "margin_bottom",
    "margin_left",
    "margin_right",
    "margin_top",
    "max_height",
    "max_width",
    "min_height",
    "min_width",
    "opacity",
    "overflow",
    "padding_bottom",
    "padding_left",
    "padding_right",
    "padding_top",
    "position",
    "right",
    "rotate",
    "scale",
    "text_overflow",
    "text_shadow",
    "top",
    "transform_origin",
    "transition_delay",
    "transition_duration",
    "transition_property",
    "transition_timing_function",
    "translate",
    "unity_background_image_tint_color",
    "unity_editor_text_rendering_mode",
    "unity_font_definition",
    "unity_font_style_and_weight",
    "unity_material",
    "unity_overflow_clip_box",
    "unity_paragraph_spacing",
    "unity_slice_bottom",
    "unity_slice_left",
    "unity_slice_right",
    "unity_slice_scale",
    "unity_slice_top",
    "unity_slice_type",
    "unity_text_align",
    "unity_text_auto_size",
    "unity_text_generator",
    "unity_text_outline_color",
    "unity_text_outline_width",
    "unity_text_overflow_position",
    "visibility",
    "white_space",
    "width",
    "word_spacing",
];

pub(crate) const EVENTS: &[&str] = &[
    "PointerDown",
    "PointerMove",
    "PointerUp",
    "PointerCancel",
    "Click",
    "PointerEnter",
    "PointerLeave",
    "PointerOver",
    "PointerOut",
    "Wheel",
    "PointerCapture",
    "PointerCaptureOut",
    "KeyDown",
    "KeyUp",
    "NavigationMove",
    "NavigationCancel",
    "FocusIn",
    "Focus",
    "FocusOut",
    "Blur",
    "GeometryChanged",
    "AttachToPanel",
    "DetachFromPanel",
    "TransitionStart",
    "TransitionEnd",
    "TransitionCancel",
    "ValueChanging",
    "ValueCommitted",
    "Input",
    "SelectionChanged",
    "LinkEnter",
    "LinkLeave",
    "LinkDown",
    "LinkUp",
    "ScrollSettled",
    "ScrollChanged",
    "TabSelectionRequested",
    "TabCloseRequested",
    "TabReorderRequested",
];

pub(crate) const ACTIONS: &[&str] = &[
    "Focus",
    "Blur",
    "CapturePointer",
    "ReleasePointer",
    "ScrollTo",
    "SelectText",
];

pub(crate) const ASSET_SOURCES: &[&str] = &[
    "ImageTexture",
    "ImageSprite",
    "ImageVectorImage",
    "ImageRenderTexture",
    "BackgroundTexture",
    "BackgroundSprite",
    "BackgroundVectorImage",
    "BackgroundRenderTexture",
];

pub(crate) const DOCUMENT_MODES: &[&str] = &["ScreenOverlay", "TargetTexture", "WorldSpace"];

pub(crate) const GROUPS: &[CoverageGroup] = &[
    CoverageGroup {
        title: "ELEMENTS",
        capabilities: ELEMENTS,
        specimen: "01/10-21",
        test_family: "elements",
    },
    CoverageGroup {
        title: "OUTER STYLE",
        capabilities: OUTER_STYLES,
        specimen: "05-09",
        test_family: "styles",
    },
    CoverageGroup {
        title: "PRIVATE PARTS",
        capabilities: coverage_parts::PARTS,
        specimen: "20-21",
        test_family: "parts",
    },
    CoverageGroup {
        title: "EVENTS",
        capabilities: EVENTS,
        specimen: "22-24",
        test_family: "events",
    },
    CoverageGroup {
        title: "ACTIONS",
        capabilities: ACTIONS,
        specimen: "25",
        test_family: "actions",
    },
    CoverageGroup {
        title: "ASSET SOURCES",
        capabilities: ASSET_SOURCES,
        specimen: "04/07/20",
        test_family: "assets",
    },
    CoverageGroup {
        title: "DOCUMENT MODES",
        capabilities: DOCUMENT_MODES,
        specimen: "26-27",
        test_family: "documents",
    },
];

pub(crate) fn validate() -> Result<usize, &'static str> {
    let mut total = 0;
    for group in GROUPS {
        if group.capabilities.is_empty()
            || group.specimen.is_empty()
            || group.test_family.is_empty()
        {
            return Err("coverage groups require capabilities, specimens, and tests");
        }
        let unique = group.capabilities.iter().copied().collect::<HashSet<_>>();
        if unique.len() != group.capabilities.len() {
            return Err("coverage capabilities must be unique within a category");
        }
        total += group.capabilities.len();
    }
    Ok(total)
}
