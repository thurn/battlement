use super::*;

macro_rules! properties {
  ($patch:ident, $previous:ident, $desired:ident; $($field:ident),* $(,)?) => {
    $(
      $patch.$field = Prop::difference(&$previous.$field, &$desired.$field);
    )*
  };
}

pub(super) fn element(
  previous: &UiElement,
  desired: &UiElement,
  hierarchy_changed: bool,
) -> Option<UiElement> {
  assert_eq!(
    previous.kind(),
    desired.kind(),
    "host kind changed during diff"
  );
  let patch = match (previous, desired) {
    (UiElement::VisualElement(previous), UiElement::VisualElement(desired)) => {
      UiElement::VisualElement(visual(previous, desired))
    }
    (UiElement::Flex(previous), UiElement::Flex(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; direction, wrap, align_items, justify_content, row_gap, column_gap);
      UiElement::Flex(patch)
    }
    (UiElement::Grid(previous), UiElement::Grid(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; columns, rows, auto_columns, auto_rows, auto_flow, row_gap, column_gap, align_items, justify_items);
      UiElement::Grid(patch)
    }
    (UiElement::Stack(previous), UiElement::Stack(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; align_items, justify_items);
      UiElement::Stack(patch)
    }
    (UiElement::Box(previous), UiElement::Box(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      UiElement::Box(patch)
    }
    (UiElement::Label(previous), UiElement::Label(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text, enable_rich_text, emoji_fallback_support, parse_escape_sequences, display_tooltip_when_elided, selectable, double_click_selects_word, triple_click_selects_line, select_all_on_focus, select_all_on_mouse_up);
      UiElement::Label(patch)
    }
    (UiElement::TextElement(previous), UiElement::TextElement(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text, enable_rich_text, emoji_fallback_support, parse_escape_sequences, display_tooltip_when_elided, selectable, double_click_selects_word, triple_click_selects_line, select_all_on_focus, select_all_on_mouse_up);
      UiElement::TextElement(patch)
    }
    (UiElement::TextField(previous), UiElement::TextField(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, value, multiline, vertical_scroller_visibility, password, read_only, placeholder, hide_placeholder_on_focus, cursor_index, select_index, select_all_on_focus, select_all_on_mouse_up);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::TextField(patch)
    }
    (UiElement::Toggle(previous), UiElement::Toggle(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, text, value);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::Toggle(patch)
    }
    (UiElement::RadioButton(previous), UiElement::RadioButton(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, text, value);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::RadioButton(patch)
    }
    (UiElement::RadioButtonGroup(previous), UiElement::RadioButtonGroup(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, choices, selected_index);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::RadioButtonGroup(patch)
    }
    (UiElement::ToggleButtonGroup(previous), UiElement::ToggleButtonGroup(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, multiple_selection, allow_empty_selection, selected_indices);
      if hierarchy_changed && !desired.selected_indices.is_unset() {
        patch.selected_indices.clone_from(&desired.selected_indices);
      }
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::ToggleButtonGroup(patch)
    }
    (UiElement::DropdownField(previous), UiElement::DropdownField(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, show_mixed_value, choices, selection);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::DropdownField(patch)
    }
    (UiElement::Button(previous), UiElement::Button(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text, enable_rich_text, emoji_fallback_support, parse_escape_sequences, display_tooltip_when_elided, icon);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::Button(patch)
    }
    (UiElement::RepeatButton(previous), UiElement::RepeatButton(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text, delay_ms, interval_ms, enable_rich_text, emoji_fallback_support, parse_escape_sequences, display_tooltip_when_elided);
      UiElement::RepeatButton(patch)
    }
    (UiElement::GroupBox(previous), UiElement::GroupBox(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::GroupBox(patch)
    }
    (UiElement::PopupWindow(previous), UiElement::PopupWindow(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text, enable_rich_text, emoji_fallback_support, parse_escape_sequences, display_tooltip_when_elided, selectable, double_click_selects_word, triple_click_selects_line, select_all_on_focus, select_all_on_mouse_up);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::PopupWindow(patch)
    }
    (UiElement::ScrollView(previous), UiElement::ScrollView(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; mode, nested_interaction, horizontal_scroller_visibility, vertical_scroller_visibility, scroll_offset, horizontal_page_size, vertical_page_size, mouse_wheel_scroll_size, touch_scroll_behavior, scroll_deceleration_rate, elasticity, elastic_animation_interval);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::ScrollView(patch)
    }
    (UiElement::Scroller(previous), UiElement::Scroller(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; low_value, high_value, direction, value);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::Scroller(patch)
    }
    (UiElement::Slider(previous), UiElement::Slider(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, low_value, high_value, value, fill, page_size, show_input_field, direction, inverted);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::Slider(patch)
    }
    (UiElement::SliderInt(previous), UiElement::SliderInt(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, low_value, high_value, value, fill, page_size, show_input_field, direction, inverted);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::SliderInt(patch)
    }
    (UiElement::MinMaxSlider(previous), UiElement::MinMaxSlider(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; label, min_value, max_value, low_limit, high_limit);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::MinMaxSlider(patch)
    }
    (UiElement::ProgressBar(previous), UiElement::ProgressBar(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; low_value, high_value, value, title);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::ProgressBar(patch)
    }
    (UiElement::Tab(previous), UiElement::Tab(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; text, icon, closeable);
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::Tab(patch)
    }
    (UiElement::TabView(previous), UiElement::TabView(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; selected_tab_index, reorderable);
      if hierarchy_changed && !desired.selected_tab_index.is_unset() {
        patch.selected_tab_index = desired.selected_tab_index;
      }
      patch.parts = parts::difference(&previous.parts, &desired.parts);
      UiElement::TabView(patch)
    }
    (UiElement::Image(previous), UiElement::Image(desired)) => {
      let mut patch = desired.clone();
      patch.element = visual(&previous.element, &desired.element);
      properties!(patch, previous, desired; source, source_rect, tint_color, scale_mode, uv);
      UiElement::Image(patch)
    }
    _ => unreachable!("validated UI element kinds diverged"),
  };
  (patch != empty(patch.kind())).then_some(patch)
}

fn visual(previous: &UiVisualElement, desired: &UiVisualElement) -> UiVisualElement {
  let mut patch = desired.clone();
  properties!(patch, previous, desired; name, enabled, picking_mode, language_direction, focusable, tab_index, delegates_focus, auto_focus, inert, classes, paint, events, event_subscriptions, motion, grid_item, stack_item, sticky, overlay_placement);
  patch.usage_hints = None;
  patch.style = Style::difference(&previous.style, &desired.style);
  patch
}

fn empty(kind: UiElementKind) -> UiElement {
  match kind {
    UiElementKind::VisualElement => UiElement::VisualElement(UiVisualElement::default()),
    UiElementKind::Flex => UiElement::Flex(UiFlex::default()),
    UiElementKind::Grid => UiElement::Grid(UiGrid::default()),
    UiElementKind::Stack => UiElement::Stack(UiStack::default()),
    UiElementKind::Box => UiElement::Box(UiBox::default()),
    UiElementKind::Label => UiElement::Label(UiLabel::default()),
    UiElementKind::TextElement => UiElement::TextElement(UiTextElement::default()),
    UiElementKind::TextField => UiElement::TextField(UiTextField::default()),
    UiElementKind::Toggle => UiElement::Toggle(UiToggle::default()),
    UiElementKind::RadioButton => UiElement::RadioButton(UiRadioButton::default()),
    UiElementKind::RadioButtonGroup => UiElement::RadioButtonGroup(UiRadioButtonGroup::default()),
    UiElementKind::ToggleButtonGroup => {
      UiElement::ToggleButtonGroup(UiToggleButtonGroup::default())
    }
    UiElementKind::DropdownField => UiElement::DropdownField(UiDropdownField::default()),
    UiElementKind::Button => UiElement::Button(UiButton::default()),
    UiElementKind::RepeatButton => UiElement::RepeatButton(UiRepeatButton::default()),
    UiElementKind::GroupBox => UiElement::GroupBox(UiGroupBox::default()),
    UiElementKind::PopupWindow => UiElement::PopupWindow(UiPopupWindow::default()),
    UiElementKind::ScrollView => UiElement::ScrollView(UiScrollView::default()),
    UiElementKind::Scroller => UiElement::Scroller(UiScroller::default()),
    UiElementKind::Slider => UiElement::Slider(UiSlider::default()),
    UiElementKind::SliderInt => UiElement::SliderInt(UiSliderInt::default()),
    UiElementKind::MinMaxSlider => UiElement::MinMaxSlider(UiMinMaxSlider::default()),
    UiElementKind::ProgressBar => UiElement::ProgressBar(UiProgressBar::default()),
    UiElementKind::Tab => UiElement::Tab(UiTab::default()),
    UiElementKind::TabView => UiElement::TabView(UiTabView::default()),
    UiElementKind::Image => UiElement::Image(UiImage::default()),
  }
}
