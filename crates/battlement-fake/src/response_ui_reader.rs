//! Verified UI response decoding for the retained fake world.

use std::collections::HashMap;

use battlement::{Prop, UiElement, UiVisualElementProperties};
use battlement_flatbuffers::schema_generated::ui_generated::battlement::flat_buffers::generated as wire;

use crate::response_reader::object_id;

pub(crate) fn read_document(value: wire::UiDocument<'_>) -> Result<battlement::UiDocument, String> {
  let mut nodes = HashMap::new();
  for node in value.nodes() {
    let id = object_id(node.object_id())?;
    let child_ids = node
      .child_ids()
      .iter()
      .map(object_id)
      .collect::<Result<Vec<_>, _>>()?;
    nodes.insert(id, (read_element(node.element())?, child_ids));
  }
  let children = value
    .root_child_ids()
    .iter()
    .map(|id| build_node(object_id(id)?, &mut nodes))
    .collect::<Result<Vec<_>, _>>()?;
  Ok(battlement::UiDocument {
    document_id: object_id(value.document_id())?,
    root_id: object_id(value.root_id())?,
    element: read_visual_element(value.root_element())?,
    children,
  })
}

pub(crate) fn read_subtree(
  root_id: battlement::ObjectId,
  values: flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<wire::UiNode<'_>>>,
) -> Result<battlement::UiNode, String> {
  let mut nodes = HashMap::new();
  for node in values {
    let id = object_id(node.object_id())?;
    let child_ids = node
      .child_ids()
      .iter()
      .map(object_id)
      .collect::<Result<Vec<_>, _>>()?;
    nodes.insert(id, (read_element(node.element())?, child_ids));
  }
  build_node(root_id, &mut nodes)
}

fn build_node(
  id: battlement::ObjectId,
  nodes: &mut HashMap<battlement::ObjectId, (UiElement, Vec<battlement::ObjectId>)>,
) -> Result<battlement::UiNode, String> {
  let (element, child_ids) = nodes
    .remove(&id)
    .ok_or_else(|| format!("UI node is missing: {id}"))?;
  let children = child_ids
    .into_iter()
    .map(|child| build_node(child, nodes))
    .collect::<Result<Vec<_>, _>>()?;
  Ok(battlement::UiNode {
    object_id: id,
    element,
    children,
  })
}

fn read_visual_element(value: wire::UiElement<'_>) -> Result<battlement::UiVisualElement, String> {
  let mut element = battlement::UiVisualElement::default();
  read_common(value, &mut element)?;
  Ok(element)
}

pub(crate) fn read_element(value: wire::UiElement<'_>) -> Result<UiElement, String> {
  let mut result = match value.kind() {
    wire::UiElementKind::VisualElement => UiElement::VisualElement(Default::default()),
    wire::UiElementKind::Flex => UiElement::Flex(Default::default()),
    wire::UiElementKind::Grid => UiElement::Grid(Default::default()),
    wire::UiElementKind::Stack => UiElement::Stack(Default::default()),
    wire::UiElementKind::Box => UiElement::Box(Default::default()),
    wire::UiElementKind::Label => UiElement::Label(Default::default()),
    wire::UiElementKind::TextElement => UiElement::TextElement(Default::default()),
    wire::UiElementKind::TextField => UiElement::TextField(Default::default()),
    wire::UiElementKind::Toggle => UiElement::Toggle(Default::default()),
    wire::UiElementKind::RadioButton => UiElement::RadioButton(Default::default()),
    wire::UiElementKind::RadioButtonGroup => UiElement::RadioButtonGroup(Default::default()),
    wire::UiElementKind::ToggleButtonGroup => UiElement::ToggleButtonGroup(Default::default()),
    wire::UiElementKind::DropdownField => UiElement::DropdownField(Default::default()),
    wire::UiElementKind::Button => UiElement::Button(Default::default()),
    wire::UiElementKind::RepeatButton => UiElement::RepeatButton(Default::default()),
    wire::UiElementKind::GroupBox => UiElement::GroupBox(Default::default()),
    wire::UiElementKind::PopupWindow => UiElement::PopupWindow(Default::default()),
    wire::UiElementKind::ScrollView => UiElement::ScrollView(Default::default()),
    wire::UiElementKind::Scroller => UiElement::Scroller(Default::default()),
    wire::UiElementKind::Slider => UiElement::Slider(Default::default()),
    wire::UiElementKind::SliderInt => UiElement::SliderInt(Default::default()),
    wire::UiElementKind::MinMaxSlider => UiElement::MinMaxSlider(Default::default()),
    wire::UiElementKind::ProgressBar => UiElement::ProgressBar(Default::default()),
    wire::UiElementKind::Tab => UiElement::Tab(Default::default()),
    wire::UiElementKind::TabView => UiElement::TabView(Default::default()),
    wire::UiElementKind::Image => UiElement::Image(Default::default()),
    _ => return Err("UI element kind is unknown".to_owned()),
  };
  read_common(value, result.visual_element_mut())?;
  for property in value.properties() {
    read_specific(&mut result, property)?;
  }
  for entry in value.part_styles() {
    let mut visual = battlement::UiVisualElement::default();
    for property in entry.properties() {
      read_common_property(property, &mut visual)?;
    }
    apply_part_style(&mut result, entry.part(), entry.index(), visual.style)?;
  }
  Ok(result)
}

fn apply_part_style(
  element: &mut UiElement,
  part: wire::UiPart,
  index: Option<u32>,
  style: battlement::Style,
) -> Result<(), String> {
  macro_rules! apply {
    ($value:ident, $method:ident) => {
      *$value = std::mem::take($value).$method(style)
    };
  }
  macro_rules! indexed {
    ($value:ident, $method:ident) => {
      *$value = std::mem::take($value).$method(
        index.ok_or_else(|| "indexed UI part is missing its index".to_owned())?,
        style,
      )
    };
  }
  match (element, part) {
    (UiElement::Button(value), wire::UiPart::ButtonIcon) => apply!(value, icon_style),
    (UiElement::GroupBox(value), wire::UiPart::GroupBoxTitle) => apply!(value, title_style),
    (UiElement::PopupWindow(value), wire::UiPart::PopupWindowContentContainer) => {
      apply!(value, content_container_style);
    }
    (UiElement::Toggle(value), wire::UiPart::ToggleLabel) => apply!(value, label_style),
    (UiElement::Toggle(value), wire::UiPart::ToggleInput) => apply!(value, input_style),
    (UiElement::Toggle(value), wire::UiPart::ToggleCheckmark) => apply!(value, checkmark_style),
    (UiElement::Toggle(value), wire::UiPart::ToggleText) => apply!(value, text_style),
    (UiElement::RadioButton(value), wire::UiPart::RadioButtonLabel) => {
      apply!(value, label_style);
    }
    (UiElement::RadioButton(value), wire::UiPart::RadioButtonInput) => {
      apply!(value, input_style);
    }
    (UiElement::RadioButton(value), wire::UiPart::RadioButtonCheckmarkBackground) => {
      apply!(value, checkmark_background_style);
    }
    (UiElement::RadioButton(value), wire::UiPart::RadioButtonCheckmark) => {
      apply!(value, checkmark_style);
    }
    (UiElement::RadioButton(value), wire::UiPart::RadioButtonText) => {
      apply!(value, text_style);
    }
    (UiElement::DropdownField(value), wire::UiPart::DropdownFieldLabel) => {
      apply!(value, label_style);
    }
    (UiElement::DropdownField(value), wire::UiPart::DropdownFieldInput) => {
      apply!(value, input_style);
    }
    (UiElement::DropdownField(value), wire::UiPart::DropdownFieldText) => {
      apply!(value, text_style);
    }
    (UiElement::DropdownField(value), wire::UiPart::DropdownFieldArrow) => {
      apply!(value, arrow_style);
    }
    (UiElement::ProgressBar(value), wire::UiPart::ProgressBarContainer) => {
      apply!(value, container_style);
    }
    (UiElement::ProgressBar(value), wire::UiPart::ProgressBarBackground) => {
      apply!(value, background_style);
    }
    (UiElement::ProgressBar(value), wire::UiPart::ProgressBarProgress) => {
      apply!(value, progress_style);
    }
    (UiElement::ProgressBar(value), wire::UiPart::ProgressBarTitleContainer) => {
      apply!(value, title_container_style);
    }
    (UiElement::ProgressBar(value), wire::UiPart::ProgressBarTitle) => {
      apply!(value, title_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewContentAndVerticalScrollContainer) => {
      apply!(value, content_and_vertical_scroll_container_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewViewport) => {
      apply!(value, viewport_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewContentContainer) => {
      apply!(value, content_container_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalScroller) => {
      apply!(value, horizontal_scroller_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalSlider) => {
      apply!(value, horizontal_slider_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalLowButton) => {
      apply!(value, horizontal_low_button_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalHighButton) => {
      apply!(value, horizontal_high_button_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalTrack) => {
      apply!(value, horizontal_track_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalDragger) => {
      apply!(value, horizontal_dragger_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewHorizontalDraggerBorder) => {
      apply!(value, horizontal_dragger_border_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalScroller) => {
      apply!(value, vertical_scroller_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalSlider) => {
      apply!(value, vertical_slider_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalLowButton) => {
      apply!(value, vertical_low_button_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalHighButton) => {
      apply!(value, vertical_high_button_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalTrack) => {
      apply!(value, vertical_track_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalDragger) => {
      apply!(value, vertical_dragger_style);
    }
    (UiElement::ScrollView(value), wire::UiPart::ScrollViewVerticalDraggerBorder) => {
      apply!(value, vertical_dragger_border_style);
    }
    (UiElement::Scroller(value), wire::UiPart::ScrollerSlider) => apply!(value, slider_style),
    (UiElement::Scroller(value), wire::UiPart::ScrollerLowButton) => {
      apply!(value, low_button_style);
    }
    (UiElement::Scroller(value), wire::UiPart::ScrollerHighButton) => {
      apply!(value, high_button_style);
    }
    (UiElement::Scroller(value), wire::UiPart::ScrollerTrack) => apply!(value, track_style),
    (UiElement::Scroller(value), wire::UiPart::ScrollerDragger) => apply!(value, dragger_style),
    (UiElement::Scroller(value), wire::UiPart::ScrollerDraggerBorder) => {
      apply!(value, dragger_border_style);
    }
    (UiElement::Tab(value), wire::UiPart::TabHeader) => apply!(value, header_style),
    (UiElement::Tab(value), wire::UiPart::TabLabel) => apply!(value, label_style),
    (UiElement::Tab(value), wire::UiPart::TabIcon) => apply!(value, icon_style),
    (UiElement::Tab(value), wire::UiPart::TabUnderline) => apply!(value, underline_style),
    (UiElement::Tab(value), wire::UiPart::TabCloseButton) => apply!(value, close_button_style),
    (UiElement::Tab(value), wire::UiPart::TabDragHandle) => apply!(value, drag_handle_style),
    (UiElement::Tab(value), wire::UiPart::TabDragHandleLeadingBar) => {
      apply!(value, drag_handle_leading_bar_style);
    }
    (UiElement::Tab(value), wire::UiPart::TabDragHandleTrailingBar) => {
      apply!(value, drag_handle_trailing_bar_style);
    }
    (UiElement::Tab(value), wire::UiPart::TabContentContainer) => {
      apply!(value, content_container_style);
    }
    (UiElement::TabView(value), wire::UiPart::TabViewContentViewport) => {
      apply!(value, content_viewport_style);
    }
    (UiElement::TabView(value), wire::UiPart::TabViewHeaderContainer) => {
      apply!(value, header_container_style);
    }
    (UiElement::TabView(value), wire::UiPart::TabViewContentContainer) => {
      apply!(value, content_container_style);
    }
    (UiElement::TabView(value), wire::UiPart::TabViewPreviousButton) => {
      apply!(value, previous_button_style);
    }
    (UiElement::TabView(value), wire::UiPart::TabViewNextButton) => {
      apply!(value, next_button_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldLabel) => apply!(value, label_style),
    (UiElement::TextField(value), wire::UiPart::TextFieldInput) => apply!(value, input_style),
    (UiElement::TextField(value), wire::UiPart::TextFieldTextElement) => {
      apply!(value, text_element_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldMultilineScrollView) => {
      apply!(value, multiline_scroll_view_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalScroller) => {
      apply!(value, vertical_scroller_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalSlider) => {
      apply!(value, vertical_slider_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalLowButton) => {
      apply!(value, vertical_low_button_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalHighButton) => {
      apply!(value, vertical_high_button_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalTrack) => {
      apply!(value, vertical_track_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalDragger) => {
      apply!(value, vertical_dragger_style);
    }
    (UiElement::TextField(value), wire::UiPart::TextFieldVerticalDraggerBorder) => {
      apply!(value, vertical_dragger_border_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupLabel) => {
      apply!(value, label_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupInput) => {
      apply!(value, input_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupChoicesContainer) => {
      apply!(value, choices_container_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupContentContainer) => {
      apply!(value, content_container_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupAllOptions) => {
      apply!(value, all_options_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupOption) => {
      indexed!(value, option_style);
    }
    (
      UiElement::RadioButtonGroup(value),
      wire::UiPart::RadioButtonGroupOptionCheckmarkBackground,
    ) => indexed!(value, option_checkmark_background_style),
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupOptionCheckmark) => {
      indexed!(value, option_checkmark_style);
    }
    (UiElement::RadioButtonGroup(value), wire::UiPart::RadioButtonGroupOptionText) => {
      indexed!(value, option_text_style);
    }
    (UiElement::ToggleButtonGroup(value), wire::UiPart::ToggleButtonGroupLabel) => {
      apply!(value, label_style);
    }
    (UiElement::ToggleButtonGroup(value), wire::UiPart::ToggleButtonGroupInput) => {
      apply!(value, input_style);
    }
    (UiElement::Slider(value), wire::UiPart::SliderLabel) => apply!(value, label_style),
    (UiElement::Slider(value), wire::UiPart::SliderInput) => apply!(value, input_style),
    (UiElement::Slider(value), wire::UiPart::SliderTrack) => apply!(value, track_style),
    (UiElement::Slider(value), wire::UiPart::SliderDragger) => apply!(value, dragger_style),
    (UiElement::Slider(value), wire::UiPart::SliderDraggerBorder) => {
      apply!(value, dragger_border_style);
    }
    (UiElement::Slider(value), wire::UiPart::SliderFill) => apply!(value, fill_style),
    (UiElement::Slider(value), wire::UiPart::SliderTextInput) => apply!(value, text_input_style),
    (UiElement::SliderInt(value), wire::UiPart::SliderIntLabel) => apply!(value, label_style),
    (UiElement::SliderInt(value), wire::UiPart::SliderIntInput) => apply!(value, input_style),
    (UiElement::SliderInt(value), wire::UiPart::SliderIntTrack) => apply!(value, track_style),
    (UiElement::SliderInt(value), wire::UiPart::SliderIntDragger) => apply!(value, dragger_style),
    (UiElement::SliderInt(value), wire::UiPart::SliderIntDraggerBorder) => {
      apply!(value, dragger_border_style);
    }
    (UiElement::SliderInt(value), wire::UiPart::SliderIntFill) => apply!(value, fill_style),
    (UiElement::SliderInt(value), wire::UiPart::SliderIntTextInput) => {
      apply!(value, text_input_style);
    }
    (UiElement::MinMaxSlider(value), wire::UiPart::MinMaxSliderLabel) => {
      apply!(value, label_style);
    }
    (UiElement::MinMaxSlider(value), wire::UiPart::MinMaxSliderInput) => {
      apply!(value, input_style);
    }
    (UiElement::MinMaxSlider(value), wire::UiPart::MinMaxSliderTrack) => {
      apply!(value, track_style);
    }
    (UiElement::MinMaxSlider(value), wire::UiPart::MinMaxSliderMinimumThumb) => {
      apply!(value, minimum_thumb_style);
    }
    (UiElement::MinMaxSlider(value), wire::UiPart::MinMaxSliderMaximumThumb) => {
      apply!(value, maximum_thumb_style);
    }
    (UiElement::MinMaxSlider(value), wire::UiPart::MinMaxSliderRangeDragger) => {
      apply!(value, range_dragger_style);
    }
    _ => return Err("UI part does not belong to its element kind".to_owned()),
  }
  if index.is_some()
    && !matches!(
      part,
      wire::UiPart::RadioButtonGroupOption
        | wire::UiPart::RadioButtonGroupOptionCheckmarkBackground
        | wire::UiPart::RadioButtonGroupOptionCheckmark
        | wire::UiPart::RadioButtonGroupOptionText
    )
  {
    return Err("non-indexed UI part carries an index".to_owned());
  }
  Ok(())
}

fn read_common(
  value: wire::UiElement<'_>,
  result: &mut battlement::UiVisualElement,
) -> Result<(), String> {
  result.usage_hints = usage_hints(value.usage_hints());
  if let Some(property) = value
    .properties()
    .iter()
    .find(|property| property.key() == wire::UiPropertyKey::EventSubscriptions)
  {
    result.event_subscriptions = match property.state() {
      wire::PropState::Reset => Prop::Reset,
      wire::PropState::Set => Prop::Set(
        value
          .event_subscriptions()
          .iter()
          .map(|entry| {
            Ok(battlement::UiEventSubscription::new(
              event_kind(entry.kind())?,
              match entry.phases() {
                1 => battlement::UiEventPhase::Trickle,
                2 => battlement::UiEventPhase::Target,
                4 => battlement::UiEventPhase::Bubble,
                _ => return Err("UI event subscription phase is unknown".to_owned()),
              },
            ))
          })
          .collect::<Result<Vec<_>, String>>()?,
      ),
      _ => Prop::Unset,
    };
  }
  for property in value.properties() {
    read_common_property(property, result)?;
  }
  Ok(())
}

fn read_common_property(
  property: wire::UiProperty<'_>,
  result: &mut battlement::UiVisualElement,
) -> Result<(), String> {
  use wire::UiPropertyKey as Key;
  match property.key() {
    Key::Name => result.name = text(property)?,
    Key::Enabled => result.enabled = boolean(property)?,
    Key::PickingMode => result.picking_mode = enumeration(property, picking_mode)?,
    Key::LanguageDirection => {
      result.language_direction = enumeration(property, language_direction)?;
    }
    Key::Focusable => result.focusable = boolean(property)?,
    Key::TabIndex => result.tab_index = integer(property)?,
    Key::DelegatesFocus => result.delegates_focus = boolean(property)?,
    Key::AutoFocus => result.auto_focus = boolean(property)?,
    Key::Inert => result.inert = boolean(property)?,
    Key::Classes => result.classes = text_list(property)?,
    Key::Events => {
      result.events = unsigned_list(property)?.map_values(|values| {
        values
          .into_iter()
          .map(event_kind_number)
          .collect::<Result<Vec<_>, _>>()
      })?;
    }
    Key::EventSubscriptions => {}
    Key::GridItem => result.grid_item = grid_item(property)?,
    Key::StackItem => result.stack_item = stack_item(property)?,
    Key::Sticky => result.sticky = sticky(property)?,
    Key::Paint => result.paint = crate::response_paint_reader::paint(property)?,
    Key::Motion => {
      result.motion = crate::response_motion_descriptor_reader::motion(property)?;
    }
    Key::OverlayPlacement => result.overlay_placement = overlay_placement(property)?,
    Key::StyleUnityFontDefinition => {
      result.style.unity_font_definition = style_ui_font(property)?;
    }
    Key::StyleBorderBottomLeftRadius => {
      result.style.border_bottom_left_radius = style_length(property)?;
    }
    Key::StyleBorderBottomRightRadius => {
      result.style.border_bottom_right_radius = style_length(property)?;
    }
    Key::StyleBorderTopLeftRadius => {
      result.style.border_top_left_radius = style_length(property)?;
    }
    Key::StyleBorderTopRightRadius => {
      result.style.border_top_right_radius = style_length(property)?;
    }
    Key::StyleFontSize => result.style.font_size = style_length(property)?,
    Key::StyleLetterSpacing => result.style.letter_spacing = style_length(property)?,
    Key::StylePaddingBottom => result.style.padding_bottom = style_length(property)?,
    Key::StylePaddingLeft => result.style.padding_left = style_length(property)?,
    Key::StylePaddingRight => result.style.padding_right = style_length(property)?,
    Key::StylePaddingTop => result.style.padding_top = style_length(property)?,
    Key::StyleUnityParagraphSpacing => {
      result.style.unity_paragraph_spacing = style_length(property)?;
    }
    Key::StyleWordSpacing => result.style.word_spacing = style_length(property)?,
    Key::StyleBottom => result.style.bottom = style_length_or_auto(property)?,
    Key::StyleFlexBasis => result.style.flex_basis = style_length_or_auto(property)?,
    Key::StyleHeight => result.style.height = style_length_or_auto(property)?,
    Key::StyleLeft => result.style.left = style_length_or_auto(property)?,
    Key::StyleMarginBottom => result.style.margin_bottom = style_length_or_auto(property)?,
    Key::StyleMarginLeft => result.style.margin_left = style_length_or_auto(property)?,
    Key::StyleMarginRight => result.style.margin_right = style_length_or_auto(property)?,
    Key::StyleMarginTop => result.style.margin_top = style_length_or_auto(property)?,
    Key::StyleMaxHeight => result.style.max_height = style_length_or_auto(property)?,
    Key::StyleMaxWidth => result.style.max_width = style_length_or_auto(property)?,
    Key::StyleMinHeight => result.style.min_height = style_length_or_auto(property)?,
    Key::StyleMinWidth => result.style.min_width = style_length_or_auto(property)?,
    Key::StyleRight => result.style.right = style_length_or_auto(property)?,
    Key::StyleTop => result.style.top = style_length_or_auto(property)?,
    Key::StyleWidth => result.style.width = style_length_or_auto(property)?,
    Key::StyleBorderBottomWidth => {
      result.style.border_bottom_width = style_float(property)?;
    }
    Key::StyleBorderLeftWidth => result.style.border_left_width = style_float(property)?,
    Key::StyleBorderRightWidth => result.style.border_right_width = style_float(property)?,
    Key::StyleBorderTopWidth => result.style.border_top_width = style_float(property)?,
    Key::StyleFlexGrow => result.style.flex_grow = style_float(property)?,
    Key::StyleFlexShrink => result.style.flex_shrink = style_float(property)?,
    Key::StyleOpacity => result.style.opacity = style_float(property)?,
    Key::StyleUnitySliceScale => result.style.unity_slice_scale = style_float(property)?,
    Key::StyleUnityTextOutlineWidth => {
      result.style.unity_text_outline_width = style_float(property)?;
    }
    Key::StyleAlignContent => result.style.align_content = style_enum(property, align)?,
    Key::StyleAlignItems => result.style.align_items = style_enum(property, align)?,
    Key::StyleAlignSelf => result.style.align_self = style_enum(property, align)?,
    Key::StyleAspectRatio => result.style.aspect_ratio = style_aspect_ratio(property)?,
    Key::StyleBackgroundColor => result.style.background_color = style_color(property)?,
    Key::StyleBackgroundImage => {
      result.style.background_image = style_background_source(property)?;
    }
    Key::StyleBackgroundPositionX => {
      result.style.background_position_x = style_background_position(property)?;
    }
    Key::StyleBackgroundPositionY => {
      result.style.background_position_y = style_background_position(property)?;
    }
    Key::StyleBackgroundRepeat => {
      result.style.background_repeat = style_background_repeat(property)?;
    }
    Key::StyleBackgroundSize => {
      result.style.background_size = style_background_size(property)?;
    }
    Key::StyleBorderBottomColor => result.style.border_bottom_color = style_color(property)?,
    Key::StyleBorderLeftColor => result.style.border_left_color = style_color(property)?,
    Key::StyleBorderRightColor => result.style.border_right_color = style_color(property)?,
    Key::StyleBorderTopColor => result.style.border_top_color = style_color(property)?,
    Key::StyleColor => result.style.color = style_color(property)?,
    Key::StyleCursor => result.style.cursor = style_cursor(property)?,
    Key::StyleDisplay => result.style.display = style_enum(property, display)?,
    Key::StyleFlexDirection => {
      result.style.flex_direction = style_enum(property, flex_direction)?;
    }
    Key::StyleFlexWrap => result.style.flex_wrap = style_enum(property, flex_wrap)?,
    Key::StyleJustifyContent => {
      result.style.justify_content = style_enum(property, justify)?;
    }
    Key::StyleOverflow => result.style.overflow = style_enum(property, overflow)?,
    Key::StylePosition => result.style.position = style_enum(property, position)?,
    Key::StyleRotate => result.style.rotate = style_rotate(property)?,
    Key::StyleScale => result.style.scale = style_scale(property)?,
    Key::StyleTextOverflow => {
      result.style.text_overflow = style_enum(property, text_overflow)?;
    }
    Key::StyleTextShadow => result.style.text_shadow = style_text_shadow(property)?,
    Key::StyleTransformOrigin => {
      result.style.transform_origin = style_transform_origin(property)?;
    }
    Key::StyleTranslate => result.style.translate = style_translate(property)?,
    Key::StyleTransitionDelay => {
      result.style.transition_delay = style_time_list(property)?;
    }
    Key::StyleTransitionDuration => {
      result.style.transition_duration = style_time_list(property)?;
    }
    Key::StyleTransitionProperty => {
      result.style.transition_property = style_transition_properties(property)?;
    }
    Key::StyleTransitionTimingFunction => {
      result.style.transition_timing_function = style_transition_easings(property)?;
    }
    Key::StyleUnityBackgroundImageTintColor => {
      result.style.unity_background_image_tint_color = style_color(property)?;
    }
    Key::StyleUnityEditorTextRenderingMode => {
      result.style.unity_editor_text_rendering_mode =
        style_enum(property, editor_text_rendering_mode)?;
    }
    Key::StyleUnityFontStyleAndWeight => {
      result.style.unity_font_style_and_weight = style_enum(property, font_style)?;
    }
    Key::StyleUnityMaterial => result.style.unity_material = style_material(property)?,
    Key::StyleUnityOverflowClipBox => {
      result.style.unity_overflow_clip_box = style_enum(property, overflow_clip_box)?;
    }
    Key::StyleUnitySliceBottom => {
      result.style.unity_slice_bottom = style_integer(property)?;
    }
    Key::StyleUnitySliceLeft => result.style.unity_slice_left = style_integer(property)?,
    Key::StyleUnitySliceRight => result.style.unity_slice_right = style_integer(property)?,
    Key::StyleUnitySliceTop => result.style.unity_slice_top = style_integer(property)?,
    Key::StyleUnitySliceType => {
      result.style.unity_slice_type = style_enum(property, slice_type)?;
    }
    Key::StyleUnityTextAlign => {
      result.style.unity_text_align = style_enum(property, text_anchor)?;
    }
    Key::StyleUnityTextAutoSize => {
      result.style.unity_text_auto_size = style_text_auto_size(property)?;
    }
    Key::StyleUnityTextGenerator => {
      result.style.unity_text_generator = style_enum(property, text_generator)?;
    }
    Key::StyleUnityTextOutlineColor => {
      result.style.unity_text_outline_color = style_color(property)?;
    }
    Key::StyleUnityTextOverflowPosition => {
      result.style.unity_text_overflow_position = style_enum(property, text_overflow_position)?;
    }
    Key::StyleVisibility => result.style.visibility = style_enum(property, visibility)?,
    Key::StyleWhiteSpace => result.style.white_space = style_enum(property, white_space)?,
    _ => {}
  }
  Ok(())
}

pub(crate) fn read_style<'a>(
  properties: impl IntoIterator<Item = wire::UiProperty<'a>>,
) -> Result<battlement::Style, String> {
  let mut visual = battlement::UiVisualElement::default();
  for property in properties {
    read_common_property(property, &mut visual)?;
  }
  Ok(visual.style)
}

fn read_specific(result: &mut UiElement, property: wire::UiProperty<'_>) -> Result<(), String> {
  use wire::UiPropertyKey as Key;
  macro_rules! common_text {
    ($value:expr) => {
      match property.key() {
        Key::Text => $value.text = text(property)?,
        Key::EnableRichText => $value.enable_rich_text = boolean(property)?,
        Key::EmojiFallbackSupport => $value.emoji_fallback_support = boolean(property)?,
        Key::ParseEscapeSequences => $value.parse_escape_sequences = boolean(property)?,
        Key::DisplayTooltipWhenElided => $value.display_tooltip_when_elided = boolean(property)?,
        _ => {}
      }
    };
  }
  macro_rules! selectable_text {
    ($value:expr) => {
      match property.key() {
        Key::Selectable => $value.selectable = boolean(property)?,
        Key::DoubleClickSelectsWord => $value.double_click_selects_word = boolean(property)?,
        Key::TripleClickSelectsLine => $value.triple_click_selects_line = boolean(property)?,
        Key::SelectAllOnFocus => $value.select_all_on_focus = boolean(property)?,
        Key::SelectAllOnMouseUp => $value.select_all_on_mouse_up = boolean(property)?,
        _ => common_text!($value),
      }
    };
  }
  macro_rules! slider {
    ($value:expr, $number:ident) => {
      match property.key() {
        Key::Label => $value.label = text(property)?,
        Key::LowValue => $value.low_value = $number(property)?,
        Key::HighValue => $value.high_value = $number(property)?,
        Key::Value => $value.value = $number(property)?,
        Key::Fill => $value.fill = boolean(property)?,
        Key::PageSize => $value.page_size = float(property)?,
        Key::ShowInputField => $value.show_input_field = boolean(property)?,
        Key::Direction => $value.direction = enumeration(property, slider_direction)?,
        Key::Inverted => $value.inverted = boolean(property)?,
        _ => {}
      }
    };
  }
  match result {
    UiElement::Flex(value) => match property.key() {
      Key::Direction => value.direction = enumeration(property, flex_direction)?,
      Key::Wrap => value.wrap = enumeration(property, flex_wrap)?,
      Key::AlignItems => value.align_items = enumeration(property, align)?,
      Key::JustifyContent => value.justify_content = enumeration(property, justify)?,
      Key::RowGap => value.row_gap = float(property)?,
      Key::ColumnGap => value.column_gap = float(property)?,
      _ => {}
    },
    UiElement::Grid(value) => match property.key() {
      Key::Columns => value.columns = grid_tracks(property)?,
      Key::Rows => value.rows = grid_tracks(property)?,
      Key::AutoColumns => value.auto_columns = grid_track(property)?,
      Key::AutoRows => value.auto_rows = grid_track(property)?,
      Key::AutoFlow => value.auto_flow = enumeration(property, grid_auto_flow)?,
      Key::RowGap => value.row_gap = float(property)?,
      Key::ColumnGap => value.column_gap = float(property)?,
      Key::AlignItems => value.align_items = enumeration(property, align)?,
      Key::JustifyItems => value.justify_items = enumeration(property, align)?,
      _ => {}
    },
    UiElement::Stack(value) => match property.key() {
      Key::AlignItems => value.align_items = enumeration(property, align)?,
      Key::JustifyItems => value.justify_items = enumeration(property, align)?,
      _ => {}
    },
    UiElement::Label(value) => selectable_text!(value),
    UiElement::TextElement(value) => selectable_text!(value),
    UiElement::PopupWindow(value) => selectable_text!(value),
    UiElement::Button(value) => match property.key() {
      Key::Icon => value.icon = icon(property)?,
      _ => common_text!(value),
    },
    UiElement::RepeatButton(value) => match property.key() {
      Key::DelayMs => value.delay_ms = unsigned(property)?,
      Key::IntervalMs => {
        value.interval_ms = unsigned(property)?.map_values(|value| {
          std::num::NonZeroU32::new(value).ok_or_else(|| "repeat interval is zero".to_owned())
        })?;
      }
      _ => common_text!(value),
    },
    UiElement::TextField(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::Value => value.value = text(property)?,
      Key::Multiline => value.multiline = boolean(property)?,
      Key::Password => value.password = boolean(property)?,
      Key::ReadOnly => value.read_only = boolean(property)?,
      Key::Placeholder => value.placeholder = text(property)?,
      Key::HidePlaceholderOnFocus => value.hide_placeholder_on_focus = boolean(property)?,
      Key::CursorIndex => value.cursor_index = unsigned(property)?,
      Key::SelectIndex => value.select_index = unsigned(property)?,
      Key::SelectAllOnFocus => value.select_all_on_focus = boolean(property)?,
      Key::SelectAllOnMouseUp => value.select_all_on_mouse_up = boolean(property)?,
      Key::VerticalScrollerVisibility => {
        value.vertical_scroller_visibility = enumeration(property, scroller_visibility)?;
      }
      _ => {}
    },
    UiElement::Toggle(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::Text => value.text = text(property)?,
      Key::Value => value.value = boolean(property)?,
      _ => {}
    },
    UiElement::RadioButton(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::Text => value.text = text(property)?,
      Key::Value => value.value = boolean(property)?,
      _ => {}
    },
    UiElement::RadioButtonGroup(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::Choices => value.choices = text_list(property)?,
      Key::SelectedIndex => value.selected_index = unsigned(property)?,
      _ => {}
    },
    UiElement::ToggleButtonGroup(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::MultipleSelection => value.multiple_selection = boolean(property)?,
      Key::AllowEmptySelection => value.allow_empty_selection = boolean(property)?,
      Key::SelectedIndices => value.selected_indices = unsigned_list(property)?,
      _ => {}
    },
    UiElement::DropdownField(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::ShowMixedValue => value.show_mixed_value = boolean(property)?,
      Key::Choices => value.choices = text_list(property)?,
      Key::Selection => value.selection = choice(property)?,
      _ => {}
    },
    UiElement::ScrollView(value) => match property.key() {
      Key::ScrollOffset => value.scroll_offset = vector(property)?,
      Key::HorizontalPageSize => value.horizontal_page_size = float(property)?,
      Key::VerticalPageSize => value.vertical_page_size = float(property)?,
      Key::MouseWheelScrollSize => value.mouse_wheel_scroll_size = float(property)?,
      Key::ScrollDecelerationRate => value.scroll_deceleration_rate = float(property)?,
      Key::Elasticity => value.elasticity = float(property)?,
      Key::ElasticAnimationInterval => value.elastic_animation_interval = unsigned(property)?,
      Key::Mode => value.mode = enumeration(property, scroll_view_mode)?,
      Key::NestedInteraction => {
        value.nested_interaction = enumeration(property, nested_interaction)?
      }
      Key::HorizontalScrollerVisibility => {
        value.horizontal_scroller_visibility = enumeration(property, scroller_visibility)?;
      }
      Key::VerticalScrollerVisibility => {
        value.vertical_scroller_visibility = enumeration(property, scroller_visibility)?;
      }
      Key::TouchScrollBehavior => {
        value.touch_scroll_behavior = enumeration(property, touch_scroll_behavior)?;
      }
      _ => {}
    },
    UiElement::Scroller(value) => match property.key() {
      Key::LowValue => value.low_value = float(property)?,
      Key::HighValue => value.high_value = float(property)?,
      Key::Value => value.value = float(property)?,
      Key::Direction => value.direction = enumeration(property, slider_direction)?,
      _ => {}
    },
    UiElement::Slider(value) => slider!(value, float),
    UiElement::SliderInt(value) => slider!(value, integer),
    UiElement::MinMaxSlider(value) => match property.key() {
      Key::Label => value.label = text(property)?,
      Key::MinValue => value.min_value = float(property)?,
      Key::MaxValue => value.max_value = float(property)?,
      Key::LowLimit => value.low_limit = lower_limit(property)?,
      Key::HighLimit => value.high_limit = upper_limit(property)?,
      _ => {}
    },
    UiElement::ProgressBar(value) => match property.key() {
      Key::LowValue => value.low_value = float(property)?,
      Key::HighValue => value.high_value = float(property)?,
      Key::Value => value.value = float(property)?,
      Key::Title => value.title = text(property)?,
      _ => {}
    },
    UiElement::Tab(value) => match property.key() {
      Key::Text => value.text = text(property)?,
      Key::Icon => value.icon = icon(property)?,
      Key::Closeable => value.closeable = boolean(property)?,
      _ => {}
    },
    UiElement::TabView(value) => match property.key() {
      Key::SelectedTabIndex => value.selected_tab_index = unsigned(property)?,
      Key::Reorderable => value.reorderable = boolean(property)?,
      _ => {}
    },
    UiElement::GroupBox(value) => {
      if property.key() == Key::Text {
        value.text = text(property)?;
      }
    }
    UiElement::Image(value) => match property.key() {
      Key::Source => value.source = image_source(property)?,
      Key::SourceRect => value.source_rect = rect(property)?,
      Key::TintColor => value.tint_color = color(property)?,
      Key::ScaleMode => value.scale_mode = enumeration(property, image_scale_mode)?,
      Key::Uv => value.uv = rect(property)?,
      _ => {}
    },
    _ => {}
  }
  Ok(())
}

trait PropResult<T> {
  fn map_values<U>(self, map: impl FnOnce(T) -> Result<U, String>) -> Result<Prop<U>, String>;
}

impl<T> PropResult<T> for Prop<T> {
  fn map_values<U>(self, map: impl FnOnce(T) -> Result<U, String>) -> Result<Prop<U>, String> {
    Ok(match self {
      Prop::Unset => Prop::Unset,
      Prop::Reset => Prop::Reset,
      Prop::Set(value) => Prop::Set(map(value)?),
    })
  }
}

fn property<T>(
  value: wire::UiProperty<'_>,
  read: impl FnOnce(wire::UiProperty<'_>) -> Result<T, String>,
) -> Result<Prop<T>, String> {
  Ok(match value.state() {
    wire::PropState::Unset => Prop::Unset,
    wire::PropState::Reset => Prop::Reset,
    wire::PropState::Set => Prop::Set(read(value)?),
    _ => return Err("UI property state is unknown".to_owned()),
  })
}

fn boolean(value: wire::UiProperty<'_>) -> Result<Prop<bool>, String> {
  property(value, |value| {
    payload(value.value_as_bool_property_value()).map(|v| v.value())
  })
}

fn integer(value: wire::UiProperty<'_>) -> Result<Prop<i32>, String> {
  property(value, |value| {
    payload(value.value_as_int_property_value()).map(|v| v.value())
  })
}

fn unsigned(value: wire::UiProperty<'_>) -> Result<Prop<u32>, String> {
  property(value, |value| {
    payload(value.value_as_uint_property_value()).map(|v| v.value())
  })
}

fn float(value: wire::UiProperty<'_>) -> Result<Prop<f32>, String> {
  property(value, |value| {
    payload(value.value_as_float_property_value()).map(|v| v.value())
  })
}

fn text(value: wire::UiProperty<'_>) -> Result<Prop<String>, String> {
  property(value, |value| {
    payload(value.value_as_text_property_value()).map(|v| v.value().to_owned())
  })
}

fn text_list(value: wire::UiProperty<'_>) -> Result<Prop<Vec<String>>, String> {
  property(value, |value| {
    payload(value.value_as_text_list_property_value())
      .map(|v| v.values().iter().map(str::to_owned).collect())
  })
}

fn unsigned_list(value: wire::UiProperty<'_>) -> Result<Prop<Vec<u32>>, String> {
  property(value, |value| {
    payload(value.value_as_uint_list_property_value()).map(|v| v.values().iter().collect())
  })
}

fn enumeration<T>(
  value: wire::UiProperty<'_>,
  map: impl FnOnce(u32) -> Result<T, String>,
) -> Result<Prop<T>, String> {
  property(value, |value| {
    map(payload(value.value_as_enum_property_value())?.value())
  })
}

fn vector(value: wire::UiProperty<'_>) -> Result<Prop<battlement::Vector>, String> {
  property(value, |value| {
    let vector = payload(value.value_as_vector_2_property_value())?.value();
    Ok(battlement::Vector::new(
      vector.x() as f32,
      vector.y() as f32,
    ))
  })
}

fn choice(value: wire::UiProperty<'_>) -> Result<Prop<battlement::Choice>, String> {
  property(value, |value| {
    let value = payload(value.value_as_choice_property_value())?;
    match value.kind() {
      wire::ChoiceKind::None => Ok(battlement::Choice::none()),
      wire::ChoiceKind::Index => Ok(battlement::Choice {
        index: Some(value.index()),
        value: value.value().map(str::to_owned),
      }),
      _ => Err("choice kind is unknown".to_owned()),
    }
  })
}

fn lower_limit(value: wire::UiProperty<'_>) -> Result<Prop<battlement::LowerLimit>, String> {
  property(value, |value| {
    let value = payload(value.value_as_limit_property_value())?;
    match value.kind() {
      wire::LimitKind::Unbounded => Ok(battlement::LowerLimit::Unbounded),
      wire::LimitKind::Inclusive => Ok(battlement::LowerLimit::Inclusive(value.value())),
      _ => Err("lower limit kind is unknown".to_owned()),
    }
  })
}

fn upper_limit(value: wire::UiProperty<'_>) -> Result<Prop<battlement::UpperLimit>, String> {
  property(value, |value| {
    let value = payload(value.value_as_limit_property_value())?;
    match value.kind() {
      wire::LimitKind::Unbounded => Ok(battlement::UpperLimit::Unbounded),
      wire::LimitKind::Inclusive => Ok(battlement::UpperLimit::Inclusive(value.value())),
      _ => Err("upper limit kind is unknown".to_owned()),
    }
  })
}

fn grid_tracks(value: wire::UiProperty<'_>) -> Result<Prop<Vec<battlement::GridTrack>>, String> {
  property(value, |value| {
    payload(value.value_as_grid_tracks_property_value())?
      .values()
      .iter()
      .map(read_grid_track)
      .collect::<Result<_, _>>()
  })
}

fn grid_track(value: wire::UiProperty<'_>) -> Result<Prop<battlement::GridTrack>, String> {
  grid_tracks(value)?.map_values(|values| {
    values
      .into_iter()
      .next()
      .ok_or_else(|| "UI grid track is missing".to_owned())
  })
}

fn read_grid_track(value: &wire::GridTrackValue) -> Result<battlement::GridTrack, String> {
  match value.kind() {
    wire::GridTrackKind::Pixels => Ok(battlement::GridTrack::Px(value.value())),
    wire::GridTrackKind::Fraction => Ok(battlement::GridTrack::Fraction(value.value())),
    wire::GridTrackKind::Auto => Ok(battlement::GridTrack::Auto),
    _ => Err("UI grid track kind is unknown".to_owned()),
  }
}

fn grid_item(value: wire::UiProperty<'_>) -> Result<Prop<battlement::GridItem>, String> {
  property(value, |value| {
    let value = payload(value.value_as_grid_item_property_value())?;
    Ok(battlement::GridItem {
      row: value.row(),
      column: value.column(),
      row_span: value.row_span(),
      column_span: value.column_span(),
      align_self: align(value.align_self())?,
      justify_self: align(value.justify_self())?,
    })
  })
}

fn stack_item(value: wire::UiProperty<'_>) -> Result<Prop<battlement::StackItem>, String> {
  property(value, |value| {
    let value = payload(value.value_as_stack_item_property_value())?;
    Ok(battlement::StackItem {
      order: value.order(),
      align_self: align(value.align_self())?,
      justify_self: align(value.justify_self())?,
      top: value.top(),
      right: value.right(),
      bottom: value.bottom(),
      left: value.left(),
      contributes_to_size: value.contributes_to_size(),
    })
  })
}

fn sticky(value: wire::UiProperty<'_>) -> Result<Prop<battlement::Sticky>, String> {
  property(value, |value| {
    let value = payload(value.value_as_sticky_property_value())?;
    Ok(battlement::Sticky {
      top: value.top(),
      right: value.right(),
      bottom: value.bottom(),
      left: value.left(),
      order: value.order(),
    })
  })
}

fn overlay_placement(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::OverlayPlacement>, String> {
  property(value, |value| {
    let value = payload(value.value_as_overlay_placement_property_value())?;
    Ok(match value.kind() {
      wire::OverlayPlacementKind::PopoverLayer => {
        battlement::OverlayPlacement::Layer(battlement::OverlayLayer::Popover)
      }
      wire::OverlayPlacementKind::ModalLayer => {
        battlement::OverlayPlacement::Layer(battlement::OverlayLayer::Modal)
      }
      wire::OverlayPlacementKind::Popover => battlement::OverlayPlacement::Popover {
        anchor: object_id(
          value
            .anchor()
            .ok_or_else(|| "UI popover anchor is missing".to_owned())?,
        )?,
        placement: battlement::PopoverPlacement {
          side: match value.side() {
            wire::PlacementSide::Top => battlement::PlacementSide::Top,
            wire::PlacementSide::Right => battlement::PlacementSide::Right,
            wire::PlacementSide::Bottom => battlement::PlacementSide::Bottom,
            wire::PlacementSide::Left => battlement::PlacementSide::Left,
            _ => return Err("UI popover side is unknown".to_owned()),
          },
          align: match value.align() {
            wire::PlacementAlign::Start => battlement::PlacementAlign::Start,
            wire::PlacementAlign::Center => battlement::PlacementAlign::Center,
            wire::PlacementAlign::End => battlement::PlacementAlign::End,
            _ => return Err("UI popover alignment is unknown".to_owned()),
          },
          main_offset: value.main_offset(),
          cross_offset: value.cross_offset(),
          collision_padding: value.collision_padding(),
          flip: value.flip(),
          shift: value.shift(),
        },
      },
      wire::OverlayPlacementKind::Modal => battlement::OverlayPlacement::Modal {
        initial_focus: value.initial_focus().map(object_id).transpose()?,
        restore_focus: value.restore_focus().map(object_id).transpose()?,
      },
      _ => return Err("UI overlay placement kind is unknown".to_owned()),
    })
  })
}

fn rect(value: wire::UiProperty<'_>) -> Result<Prop<battlement::Rect>, String> {
  property(value, |value| {
    let value = payload(value.value_as_rect_property_value())?.value();
    Ok(battlement::Rect {
      x: value.x(),
      y: value.y(),
      width: value.width(),
      height: value.height(),
    })
  })
}

fn color(value: wire::UiProperty<'_>) -> Result<Prop<battlement::Color>, String> {
  property(value, |value| {
    let value = payload(value.value_as_color_property_value())?.value();
    Ok(battlement::Color {
      r: value.r(),
      g: value.g(),
      b: value.b(),
      a: value.a(),
    })
  })
}

fn icon(value: wire::UiProperty<'_>) -> Result<Prop<battlement::IconSource>, String> {
  asset(value, |kind, address| match kind {
    wire::AssetSourceKind::Texture => Ok(battlement::IconSource::Texture(address.into())),
    wire::AssetSourceKind::Sprite => Ok(battlement::IconSource::Sprite(address.into())),
    wire::AssetSourceKind::VectorImage => Ok(battlement::IconSource::VectorImage(address.into())),
    wire::AssetSourceKind::RenderTexture => {
      Ok(battlement::IconSource::RenderTexture(address.into()))
    }
    _ => Err("UI icon source kind is unknown".to_owned()),
  })
}

fn image_source(value: wire::UiProperty<'_>) -> Result<Prop<battlement::ImageSource>, String> {
  asset(value, |kind, address| match kind {
    wire::AssetSourceKind::Texture => Ok(battlement::ImageSource::Texture(address.into())),
    wire::AssetSourceKind::Sprite => Ok(battlement::ImageSource::Sprite(address.into())),
    wire::AssetSourceKind::VectorImage => Ok(battlement::ImageSource::VectorImage(address.into())),
    wire::AssetSourceKind::RenderTexture => {
      Ok(battlement::ImageSource::RenderTexture(address.into()))
    }
    _ => Err("UI image source kind is unknown".to_owned()),
  })
}

fn asset<T>(
  value: wire::UiProperty<'_>,
  map: impl FnOnce(wire::AssetSourceKind, &str) -> Result<T, String>,
) -> Result<Prop<T>, String> {
  property(value, |value| {
    let value = payload(value.value_as_asset_property_value())?;
    map(value.kind(), value.address())
  })
}

fn style_ui_font(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::UiFontAddress>>, String> {
  Ok(match value.state() {
    wire::PropState::Unset => Prop::Unset,
    wire::PropState::Reset => Prop::Reset,
    wire::PropState::Set if value.style_value_kind() == wire::StyleValueKind::Initial => {
      Prop::Set(battlement::StyleValue::Keyword {
        value: battlement::InlineKeyword::Initial,
      })
    }
    wire::PropState::Set => {
      let asset = payload(value.value_as_asset_property_value())?;
      Prop::Set(battlement::StyleValue::Value(
        battlement::UiFontAddress::new(asset.address()),
      ))
    }
    _ => return Err("UI style property state is unknown".to_owned()),
  })
}

fn style_value<T>(
  value: wire::UiProperty<'_>,
  read: impl FnOnce(wire::UiProperty<'_>) -> Result<T, String>,
) -> Result<Prop<battlement::StyleValue<T>>, String> {
  Ok(match value.state() {
    wire::PropState::Unset => Prop::Unset,
    wire::PropState::Reset => Prop::Reset,
    wire::PropState::Set if value.style_value_kind() == wire::StyleValueKind::Initial => {
      Prop::Set(battlement::StyleValue::Keyword {
        value: battlement::InlineKeyword::Initial,
      })
    }
    wire::PropState::Set => Prop::Set(battlement::StyleValue::Value(read(value)?)),
    _ => return Err("UI style property state is unknown".to_owned()),
  })
}

fn style_length(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::Length>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_length_property_value())?;
    Ok(match value.kind() {
      wire::LengthKind::Pixels => battlement::Length::px(value.pixels()),
      wire::LengthKind::Percent => battlement::Length::percent(value.percentage()),
      wire::LengthKind::Calc => battlement::Length::calc(value.pixels(), value.percentage()),
      _ => return Err("UI length kind is unknown".to_owned()),
    })
  })
}

fn style_length_or_auto(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::LengthOrAuto>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_length_property_value())?;
    Ok(match value.kind() {
      wire::LengthKind::Pixels => battlement::LengthOrAuto::Px(value.pixels()),
      wire::LengthKind::Percent => battlement::LengthOrAuto::Percent(value.percentage()),
      wire::LengthKind::Auto => battlement::LengthOrAuto::Auto,
      _ => return Err("UI automatic length kind is unknown".to_owned()),
    })
  })
}

fn style_float(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::FloatValue>>, String> {
  style_value(value, |value| {
    Ok(battlement::FloatValue(
      payload(value.value_as_float_property_value())?.value(),
    ))
  })
}

fn style_integer(value: wire::UiProperty<'_>) -> Result<Prop<battlement::StyleValue<i32>>, String> {
  style_value(value, |value| {
    Ok(payload(value.value_as_int_property_value())?.value())
  })
}

fn style_enum<T>(
  value: wire::UiProperty<'_>,
  map: impl FnOnce(u32) -> Result<T, String>,
) -> Result<Prop<battlement::StyleValue<T>>, String> {
  style_value(value, |value| {
    map(payload(value.value_as_enum_property_value())?.value())
  })
}

fn style_color(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::Color>>, String> {
  style_value(value, |value| {
    Ok(read_color(
      payload(value.value_as_color_property_value())?.value(),
    ))
  })
}

fn style_aspect_ratio(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::AspectRatio>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_aspect_ratio_property_value())?;
    match value.kind() {
      wire::AspectRatioKind::Auto => Ok(battlement::AspectRatio::Auto),
      wire::AspectRatioKind::Ratio => {
        Ok(battlement::AspectRatio::new(value.width(), value.height()))
      }
      _ => Err("UI aspect-ratio kind is unknown".to_owned()),
    }
  })
}

fn style_background_source(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::BackgroundSource>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_asset_property_value())?;
    Ok(match value.kind() {
      wire::AssetSourceKind::Texture => {
        battlement::BackgroundSource::Texture(value.address().into())
      }
      wire::AssetSourceKind::Sprite => battlement::BackgroundSource::Sprite(value.address().into()),
      wire::AssetSourceKind::VectorImage => {
        battlement::BackgroundSource::VectorImage(value.address().into())
      }
      wire::AssetSourceKind::RenderTexture => {
        battlement::BackgroundSource::RenderTexture(value.address().into())
      }
      _ => return Err("UI background source kind is unknown".to_owned()),
    })
  })
}

fn style_material(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::MaterialAddress>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_asset_property_value())?;
    if value.kind() != wire::AssetSourceKind::Material {
      return Err("UI style material has the wrong asset kind".to_owned());
    }
    Ok(battlement::MaterialAddress::new(value.address()))
  })
}

fn style_background_position(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::BackgroundPosition>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_background_position_property_value())?;
    Ok(battlement::BackgroundPosition {
      keyword: background_position_keyword(value.keyword())?,
      offset: read_length(value.offset())?,
    })
  })
}

fn style_background_repeat(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::BackgroundRepeat>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_background_repeat_property_value())?;
    Ok(battlement::BackgroundRepeat::new(
      background_repeat_mode(value.x())?,
      background_repeat_mode(value.y())?,
    ))
  })
}

fn style_background_size(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::BackgroundSize>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_background_size_property_value())?;
    Ok(match value.kind() {
      wire::BackgroundSizeKind::Auto => battlement::BackgroundSize::Auto,
      wire::BackgroundSizeKind::Cover => battlement::BackgroundSize::Cover,
      wire::BackgroundSizeKind::Contain => battlement::BackgroundSize::Contain,
      wire::BackgroundSizeKind::Axes => battlement::BackgroundSize::Axes {
        x: read_length_or_auto(
          value
            .x()
            .ok_or_else(|| "UI background width is missing".to_owned())?,
        )?,
        y: read_length_or_auto(
          value
            .y()
            .ok_or_else(|| "UI background height is missing".to_owned())?,
        )?,
      },
      _ => return Err("UI background-size kind is unknown".to_owned()),
    })
  })
}

fn style_cursor(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::Cursor>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_cursor_property_value())?;
    Ok(match value.kind() {
      wire::CursorKind::Default => battlement::Cursor::Default,
      wire::CursorKind::Texture => {
        let hotspot = value
          .hotspot()
          .ok_or_else(|| "UI cursor hotspot is missing".to_owned())?;
        battlement::Cursor::texture(
          battlement::TextureAddress::new(
            value
              .address()
              .ok_or_else(|| "UI cursor address is missing".to_owned())?,
          ),
          battlement::CursorHotspot::new(hotspot.x() as f32, hotspot.y() as f32),
        )
      }
      _ => return Err("UI cursor kind is unknown".to_owned()),
    })
  })
}

fn style_rotate(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::Rotate>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_rotate_property_value())?;
    Ok(battlement::Rotate::new(
      value.x(),
      value.y(),
      value.z(),
      value.degrees(),
    ))
  })
}

fn style_scale(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::Scale>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_scale_property_value())?;
    Ok(battlement::Scale::new(value.x(), value.y()))
  })
}

fn style_text_shadow(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::TextShadow>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_text_shadow_property_value())?;
    Ok(battlement::TextShadow::new(
      value.x(),
      value.y(),
      value.blur_radius(),
      read_color(value.color()),
    ))
  })
}

fn style_transform_origin(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::TransformOrigin>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_transform_origin_property_value())?;
    Ok(battlement::TransformOrigin::new(
      read_length(value.x())?,
      read_length(value.y())?,
      value.z(),
    ))
  })
}

fn style_translate(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::Translate>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_translate_property_value())?;
    Ok(battlement::Translate::new(
      read_length(value.x())?,
      read_length(value.y())?,
      value.z(),
    ))
  })
}

fn read_length(value: wire::LengthPropertyValue<'_>) -> Result<battlement::Length, String> {
  match value.kind() {
    wire::LengthKind::Pixels => Ok(battlement::Length::px(value.pixels())),
    wire::LengthKind::Percent => Ok(battlement::Length::percent(value.percentage())),
    wire::LengthKind::Calc => Ok(battlement::Length::calc(value.pixels(), value.percentage())),
    _ => Err("UI length kind is unknown".to_owned()),
  }
}

fn read_length_or_auto(
  value: wire::LengthPropertyValue<'_>,
) -> Result<battlement::LengthOrAuto, String> {
  match value.kind() {
    wire::LengthKind::Pixels => Ok(battlement::LengthOrAuto::Px(value.pixels())),
    wire::LengthKind::Percent => Ok(battlement::LengthOrAuto::Percent(value.percentage())),
    wire::LengthKind::Auto => Ok(battlement::LengthOrAuto::Auto),
    _ => Err("UI automatic length kind is unknown".to_owned()),
  }
}

fn read_color(
  value: &battlement_flatbuffers::schema_generated::common_generated::RgbaColor,
) -> battlement::Color {
  battlement::Color {
    r: value.r(),
    g: value.g(),
    b: value.b(),
    a: value.a(),
  }
}

fn style_time_list(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::TransitionList<battlement::TimeValue>>>, String>
{
  style_value(value, |value| {
    Ok(battlement::TransitionList::new(
      payload(value.value_as_float_list_property_value())?
        .values()
        .iter()
        .map(battlement::TimeValue),
    ))
  })
}

fn style_transition_easings(
  value: wire::UiProperty<'_>,
) -> Result<
  Prop<battlement::StyleValue<battlement::TransitionList<battlement::EasingFunction>>>,
  String,
> {
  style_value(value, |value| {
    Ok(battlement::TransitionList::new(
      payload(value.value_as_enum_list_property_value())?
        .values()
        .iter()
        .map(easing_function)
        .collect::<Result<Vec<_>, _>>()?,
    ))
  })
}

fn style_transition_properties(
  value: wire::UiProperty<'_>,
) -> Result<
  Prop<battlement::StyleValue<battlement::TransitionList<battlement::TransitionProperty>>>,
  String,
> {
  style_value(value, |value| {
    Ok(battlement::TransitionList::new(
      payload(value.value_as_enum_list_property_value())?
        .values()
        .iter()
        .map(transition_property)
        .collect::<Result<Vec<_>, _>>()?,
    ))
  })
}

fn style_text_auto_size(
  value: wire::UiProperty<'_>,
) -> Result<Prop<battlement::StyleValue<battlement::TextAutoSize>>, String> {
  style_value(value, |value| {
    let value = payload(value.value_as_text_auto_size_property_value())?;
    Ok(match value.kind() {
      wire::TextAutoSizeKind::None => battlement::TextAutoSize::None,
      wire::TextAutoSizeKind::BestFit => {
        battlement::TextAutoSize::best_fit(value.min_size(), value.max_size())
      }
      _ => return Err("UI text auto-size kind is unknown".to_owned()),
    })
  })
}

fn payload<T>(value: Option<T>) -> Result<T, String> {
  value.ok_or_else(|| "UI property payload is missing".to_owned())
}

fn usage_hints(mask: u32) -> Option<Vec<battlement::UsageHint>> {
  let values = [
    (1 << 0, battlement::UsageHint::DynamicTransform),
    (1 << 1, battlement::UsageHint::GroupTransform),
    (1 << 2, battlement::UsageHint::MaskContainer),
    (1 << 3, battlement::UsageHint::DynamicColor),
    (1 << 4, battlement::UsageHint::DynamicPostProcessing),
    (1 << 5, battlement::UsageHint::LargePixelCoverage),
  ]
  .into_iter()
  .filter_map(|(bit, hint)| (mask & bit != 0).then_some(hint))
  .collect::<Vec<_>>();
  (!values.is_empty()).then_some(values)
}

fn event_kind_number(value: u32) -> Result<battlement::UiEventKind, String> {
  event_kind(wire::UiSubscriptionKind(value as u8))
}

fn event_kind(value: wire::UiSubscriptionKind) -> Result<battlement::UiEventKind, String> {
  use battlement::UiEventKind as Event;
  use wire::UiSubscriptionKind as Wire;
  Ok(match value {
    Wire::AccessibilityAction => Event::AccessibilityAction,
    Wire::PointerDown => Event::PointerDown,
    Wire::PointerMove => Event::PointerMove,
    Wire::PointerUp => Event::PointerUp,
    Wire::PointerCancel => Event::PointerCancel,
    Wire::Click => Event::Click,
    Wire::PointerEnter => Event::PointerEnter,
    Wire::PointerLeave => Event::PointerLeave,
    Wire::PointerOver => Event::PointerOver,
    Wire::PointerOut => Event::PointerOut,
    Wire::Wheel => Event::Wheel,
    Wire::PointerCapture => Event::PointerCapture,
    Wire::PointerCaptureOut => Event::PointerCaptureOut,
    Wire::KeyDown => Event::KeyDown,
    Wire::KeyUp => Event::KeyUp,
    Wire::NavigationMove => Event::NavigationMove,
    Wire::NavigationCancel => Event::NavigationCancel,
    Wire::FocusIn => Event::FocusIn,
    Wire::Focus => Event::Focus,
    Wire::FocusOut => Event::FocusOut,
    Wire::Blur => Event::Blur,
    Wire::GeometryChanged => Event::GeometryChanged,
    Wire::AttachToPanel => Event::AttachToPanel,
    Wire::DetachFromPanel => Event::DetachFromPanel,
    Wire::TransitionStart => Event::TransitionStart,
    Wire::TransitionEnd => Event::TransitionEnd,
    Wire::TransitionCancel => Event::TransitionCancel,
    Wire::ValueChanging => Event::ValueChanging,
    Wire::ValueCommitted => Event::ValueCommitted,
    Wire::Input => Event::Input,
    Wire::SelectionChanged => Event::SelectionChanged,
    Wire::LinkEnter => Event::LinkEnter,
    Wire::LinkLeave => Event::LinkLeave,
    Wire::LinkDown => Event::LinkDown,
    Wire::LinkUp => Event::LinkUp,
    Wire::ScrollSettled => Event::ScrollSettled,
    Wire::ScrollChanged => Event::ScrollChanged,
    Wire::TabSelectionRequested => Event::TabSelectionRequested,
    Wire::TabCloseRequested => Event::TabCloseRequested,
    Wire::TabReorderRequested => Event::TabReorderRequested,
    _ => return Err("UI event kind is unknown".to_owned()),
  })
}

fn picking_mode(value: u32) -> Result<battlement::PickingMode, String> {
  match value {
    0 => Ok(battlement::PickingMode::Position),
    1 => Ok(battlement::PickingMode::Ignore),
    _ => Err("picking mode is unknown".to_owned()),
  }
}

fn language_direction(value: u32) -> Result<battlement::LanguageDirection, String> {
  match value {
    0 => Ok(battlement::LanguageDirection::Inherit),
    1 => Ok(battlement::LanguageDirection::Ltr),
    2 => Ok(battlement::LanguageDirection::Rtl),
    _ => Err("language direction is unknown".to_owned()),
  }
}

fn align(value: u32) -> Result<battlement::Align, String> {
  match value {
    0 => Ok(battlement::Align::Auto),
    1 => Ok(battlement::Align::FlexStart),
    2 => Ok(battlement::Align::Center),
    3 => Ok(battlement::Align::FlexEnd),
    4 => Ok(battlement::Align::Stretch),
    _ => Err("alignment is unknown".to_owned()),
  }
}

fn flex_direction(value: u32) -> Result<battlement::FlexDirection, String> {
  match value {
    0 => Ok(battlement::FlexDirection::Column),
    1 => Ok(battlement::FlexDirection::ColumnReverse),
    2 => Ok(battlement::FlexDirection::Row),
    3 => Ok(battlement::FlexDirection::RowReverse),
    _ => Err("flex direction is unknown".to_owned()),
  }
}

fn flex_wrap(value: u32) -> Result<battlement::FlexWrap, String> {
  match value {
    0 => Ok(battlement::FlexWrap::NoWrap),
    1 => Ok(battlement::FlexWrap::Wrap),
    2 => Ok(battlement::FlexWrap::WrapReverse),
    _ => Err("flex wrap is unknown".to_owned()),
  }
}

fn justify(value: u32) -> Result<battlement::Justify, String> {
  match value {
    0 => Ok(battlement::Justify::FlexStart),
    1 => Ok(battlement::Justify::Center),
    2 => Ok(battlement::Justify::FlexEnd),
    3 => Ok(battlement::Justify::SpaceBetween),
    4 => Ok(battlement::Justify::SpaceAround),
    5 => Ok(battlement::Justify::SpaceEvenly),
    _ => Err("justification is unknown".to_owned()),
  }
}

fn grid_auto_flow(value: u32) -> Result<battlement::GridAutoFlow, String> {
  match value {
    0 => Ok(battlement::GridAutoFlow::Row),
    1 => Ok(battlement::GridAutoFlow::Column),
    _ => Err("grid auto-flow is unknown".to_owned()),
  }
}

fn slider_direction(value: u32) -> Result<battlement::SliderDirection, String> {
  match value {
    0 => Ok(battlement::SliderDirection::Horizontal),
    1 => Ok(battlement::SliderDirection::Vertical),
    _ => Err("slider direction is unknown".to_owned()),
  }
}

fn image_scale_mode(value: u32) -> Result<battlement::ImageScaleMode, String> {
  match value {
    0 => Ok(battlement::ImageScaleMode::ScaleToFit),
    1 => Ok(battlement::ImageScaleMode::ScaleAndCrop),
    2 => Ok(battlement::ImageScaleMode::StretchToFill),
    _ => Err("image scale mode is unknown".to_owned()),
  }
}

fn display(value: u32) -> Result<battlement::Display, String> {
  match value {
    0 => Ok(battlement::Display::Flex),
    1 => Ok(battlement::Display::None),
    _ => Err("display value is unknown".to_owned()),
  }
}

fn visibility(value: u32) -> Result<battlement::Visibility, String> {
  match value {
    0 => Ok(battlement::Visibility::Visible),
    1 => Ok(battlement::Visibility::Hidden),
    _ => Err("visibility value is unknown".to_owned()),
  }
}

fn overflow(value: u32) -> Result<battlement::Overflow, String> {
  match value {
    0 => Ok(battlement::Overflow::Visible),
    1 => Ok(battlement::Overflow::Hidden),
    _ => Err("overflow value is unknown".to_owned()),
  }
}

fn overflow_clip_box(value: u32) -> Result<battlement::OverflowClipBox, String> {
  match value {
    0 => Ok(battlement::OverflowClipBox::PaddingBox),
    1 => Ok(battlement::OverflowClipBox::ContentBox),
    _ => Err("overflow clip box is unknown".to_owned()),
  }
}

fn position(value: u32) -> Result<battlement::Position, String> {
  match value {
    0 => Ok(battlement::Position::Relative),
    1 => Ok(battlement::Position::Absolute),
    _ => Err("position value is unknown".to_owned()),
  }
}

fn text_overflow(value: u32) -> Result<battlement::TextOverflow, String> {
  match value {
    0 => Ok(battlement::TextOverflow::Clip),
    1 => Ok(battlement::TextOverflow::Ellipsis),
    _ => Err("text overflow is unknown".to_owned()),
  }
}

fn text_overflow_position(value: u32) -> Result<battlement::TextOverflowPosition, String> {
  match value {
    0 => Ok(battlement::TextOverflowPosition::Start),
    1 => Ok(battlement::TextOverflowPosition::Middle),
    2 => Ok(battlement::TextOverflowPosition::End),
    _ => Err("text overflow position is unknown".to_owned()),
  }
}

fn white_space(value: u32) -> Result<battlement::WhiteSpace, String> {
  match value {
    0 => Ok(battlement::WhiteSpace::Normal),
    1 => Ok(battlement::WhiteSpace::NoWrap),
    2 => Ok(battlement::WhiteSpace::Pre),
    3 => Ok(battlement::WhiteSpace::PreWrap),
    _ => Err("white-space value is unknown".to_owned()),
  }
}

fn text_generator(value: u32) -> Result<battlement::TextGenerator, String> {
  match value {
    0 => Ok(battlement::TextGenerator::Standard),
    1 => Ok(battlement::TextGenerator::Advanced),
    _ => Err("text generator is unknown".to_owned()),
  }
}

fn editor_text_rendering_mode(value: u32) -> Result<battlement::EditorTextRenderingMode, String> {
  match value {
    0 => Ok(battlement::EditorTextRenderingMode::Sdf),
    1 => Ok(battlement::EditorTextRenderingMode::Bitmap),
    _ => Err("editor text rendering mode is unknown".to_owned()),
  }
}

fn font_style(value: u32) -> Result<battlement::FontStyle, String> {
  match value {
    0 => Ok(battlement::FontStyle::Normal),
    1 => Ok(battlement::FontStyle::Bold),
    2 => Ok(battlement::FontStyle::Italic),
    3 => Ok(battlement::FontStyle::BoldAndItalic),
    _ => Err("font style is unknown".to_owned()),
  }
}

fn text_anchor(value: u32) -> Result<battlement::TextAnchor, String> {
  const VALUES: [battlement::TextAnchor; 9] = [
    battlement::TextAnchor::UpperLeft,
    battlement::TextAnchor::UpperCenter,
    battlement::TextAnchor::UpperRight,
    battlement::TextAnchor::MiddleLeft,
    battlement::TextAnchor::MiddleCenter,
    battlement::TextAnchor::MiddleRight,
    battlement::TextAnchor::LowerLeft,
    battlement::TextAnchor::LowerCenter,
    battlement::TextAnchor::LowerRight,
  ];
  VALUES
    .get(value as usize)
    .copied()
    .ok_or_else(|| "text anchor is unknown".to_owned())
}

fn slice_type(value: u32) -> Result<battlement::SliceType, String> {
  match value {
    0 => Ok(battlement::SliceType::Sliced),
    1 => Ok(battlement::SliceType::Tiled),
    _ => Err("slice type is unknown".to_owned()),
  }
}

fn background_position_keyword(
  value: u32,
) -> Result<battlement::BackgroundPositionKeyword, String> {
  match value {
    0 => Ok(battlement::BackgroundPositionKeyword::Center),
    1 => Ok(battlement::BackgroundPositionKeyword::Top),
    2 => Ok(battlement::BackgroundPositionKeyword::Bottom),
    3 => Ok(battlement::BackgroundPositionKeyword::Left),
    4 => Ok(battlement::BackgroundPositionKeyword::Right),
    _ => Err("background position keyword is unknown".to_owned()),
  }
}

fn background_repeat_mode(value: u32) -> Result<battlement::BackgroundRepeatMode, String> {
  match value {
    0 => Ok(battlement::BackgroundRepeatMode::NoRepeat),
    1 => Ok(battlement::BackgroundRepeatMode::Repeat),
    2 => Ok(battlement::BackgroundRepeatMode::Round),
    3 => Ok(battlement::BackgroundRepeatMode::Space),
    _ => Err("background repeat mode is unknown".to_owned()),
  }
}

fn easing_function(value: u32) -> Result<battlement::EasingFunction, String> {
  use battlement::EasingFunction as E;
  const VALUES: [E; 23] = [
    E::Ease,
    E::EaseIn,
    E::EaseOut,
    E::EaseInOut,
    E::Linear,
    E::EaseInSine,
    E::EaseOutSine,
    E::EaseInOutSine,
    E::EaseInCubic,
    E::EaseOutCubic,
    E::EaseInOutCubic,
    E::EaseInCirc,
    E::EaseOutCirc,
    E::EaseInOutCirc,
    E::EaseInElastic,
    E::EaseOutElastic,
    E::EaseInOutElastic,
    E::EaseInBack,
    E::EaseOutBack,
    E::EaseInOutBack,
    E::EaseInBounce,
    E::EaseOutBounce,
    E::EaseInOutBounce,
  ];
  VALUES
    .get(value as usize)
    .copied()
    .ok_or_else(|| "transition easing is unknown".to_owned())
}

fn transition_property(value: u32) -> Result<battlement::TransitionProperty, String> {
  use battlement::TransitionProperty as P;
  const VALUES: &[P] = &[
    P::All,
    P::AlignContent,
    P::AlignItems,
    P::AlignSelf,
    P::AspectRatio,
    P::BackgroundColor,
    P::BackgroundImage,
    P::BackgroundPositionX,
    P::BackgroundPositionY,
    P::BackgroundRepeat,
    P::BackgroundSize,
    P::BorderBottomColor,
    P::BorderBottomLeftRadius,
    P::BorderBottomRightRadius,
    P::BorderBottomWidth,
    P::BorderLeftColor,
    P::BorderLeftWidth,
    P::BorderRightColor,
    P::BorderRightWidth,
    P::BorderTopColor,
    P::BorderTopLeftRadius,
    P::BorderTopRightRadius,
    P::BorderTopWidth,
    P::Bottom,
    P::Color,
    P::Cursor,
    P::Display,
    P::FlexBasis,
    P::FlexDirection,
    P::FlexGrow,
    P::FlexShrink,
    P::FlexWrap,
    P::FontSize,
    P::Height,
    P::JustifyContent,
    P::Left,
    P::LetterSpacing,
    P::MarginBottom,
    P::MarginLeft,
    P::MarginRight,
    P::MarginTop,
    P::MaxHeight,
    P::MaxWidth,
    P::MinHeight,
    P::MinWidth,
    P::Opacity,
    P::Overflow,
    P::PaddingBottom,
    P::PaddingLeft,
    P::PaddingRight,
    P::PaddingTop,
    P::Position,
    P::Right,
    P::Rotate,
    P::Scale,
    P::TextOverflow,
    P::TextShadow,
    P::Top,
    P::TransformOrigin,
    P::TransitionDelay,
    P::TransitionDuration,
    P::TransitionProperty,
    P::TransitionTimingFunction,
    P::Translate,
    P::UnityBackgroundImageTintColor,
    P::UnityEditorTextRenderingMode,
    P::UnityFontDefinition,
    P::UnityFontStyleAndWeight,
    P::UnityMaterial,
    P::UnityOverflowClipBox,
    P::UnityParagraphSpacing,
    P::UnitySliceBottom,
    P::UnitySliceLeft,
    P::UnitySliceRight,
    P::UnitySliceScale,
    P::UnitySliceTop,
    P::UnitySliceType,
    P::UnityTextAlign,
    P::UnityTextAutoSize,
    P::UnityTextGenerator,
    P::UnityTextOutlineColor,
    P::UnityTextOutlineWidth,
    P::UnityTextOverflowPosition,
    P::Visibility,
    P::WhiteSpace,
    P::Width,
    P::WordSpacing,
  ];
  VALUES
    .get(value as usize)
    .copied()
    .ok_or_else(|| "transition property is unknown".to_owned())
}

fn scroller_visibility(value: u32) -> Result<battlement::ScrollerVisibility, String> {
  match value {
    0 => Ok(battlement::ScrollerVisibility::Auto),
    1 => Ok(battlement::ScrollerVisibility::AlwaysVisible),
    2 => Ok(battlement::ScrollerVisibility::Hidden),
    _ => Err("scroller visibility is unknown".to_owned()),
  }
}

fn scroll_view_mode(value: u32) -> Result<battlement::ScrollViewMode, String> {
  match value {
    0 => Ok(battlement::ScrollViewMode::Vertical),
    1 => Ok(battlement::ScrollViewMode::Horizontal),
    2 => Ok(battlement::ScrollViewMode::VerticalAndHorizontal),
    _ => Err("scroll view mode is unknown".to_owned()),
  }
}

fn nested_interaction(value: u32) -> Result<battlement::NestedInteraction, String> {
  match value {
    0 => Ok(battlement::NestedInteraction::Default),
    1 => Ok(battlement::NestedInteraction::StopScrolling),
    2 => Ok(battlement::NestedInteraction::ForwardScrolling),
    _ => Err("nested interaction is unknown".to_owned()),
  }
}

fn touch_scroll_behavior(value: u32) -> Result<battlement::TouchScrollBehavior, String> {
  match value {
    0 => Ok(battlement::TouchScrollBehavior::Unrestricted),
    1 => Ok(battlement::TouchScrollBehavior::Elastic),
    2 => Ok(battlement::TouchScrollBehavior::Clamped),
    _ => Err("touch scroll behavior is unknown".to_owned()),
  }
}
