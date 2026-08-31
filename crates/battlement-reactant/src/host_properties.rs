use std::num::NonZeroU32;

use battlement::{
  Choice, Color, IconSource, ImageScaleMode, ImageSource, LowerLimit, NestedInteraction, Prop,
  Rect, ScrollViewMode, ScrollerVisibility, SliderDirection, Style, TouchScrollBehavior,
  UpperLimit, Vector,
};

use crate::host::{
  Button, DropdownField, GroupBox, Image, Label, MinMaxSlider, PopupWindow, ProgressBar,
  RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, Slider, SliderInt, Tab,
  TabView, TextElement, TextField, Toggle, ToggleButtonGroup,
};

macro_rules! delegated {
  ($facade:ty => $native:ident { $($name:ident($($arg:ident: $arg_type:ty),*)),* $(,)? }) => {
    impl $facade {
      $(
        #[doc = concat!(
          "Authors [`battlement::", stringify!($native), "::", stringify!($name),
          "`] with the same property semantics, validation, and sparse-update behavior."
        )]
        #[must_use]
        pub fn $name(mut self, $($arg: $arg_type),*) -> Self {
          self.state.host = self.state.host.clone().$name($($arg),*);
          self
        }
      )*
    }
  };
}

macro_rules! text_properties {
  ($facade:ty => $native:ident) => {
    delegated!($facade => $native {
      text(value: impl Into<Prop<String>>),
      rich_text(value: impl Into<Prop<bool>>),
      emoji_fallback(value: impl Into<Prop<bool>>),
      parse_escape_sequences(value: impl Into<Prop<bool>>),
      tooltip_when_elided(value: impl Into<Prop<bool>>),
    });
  };
}

macro_rules! selectable_text_properties {
  ($facade:ty => $native:ident) => {
    text_properties!($facade => $native);
    delegated!($facade => $native {
      selectable(value: impl Into<Prop<bool>>),
      double_click_selects_word(value: impl Into<Prop<bool>>),
      triple_click_selects_line(value: impl Into<Prop<bool>>),
      select_all_on_focus(value: impl Into<Prop<bool>>),
      select_all_on_mouse_up(value: impl Into<Prop<bool>>),
    });
  };
}

selectable_text_properties!(Label => UiLabel);
selectable_text_properties!(TextElement => UiTextElement);
selectable_text_properties!(PopupWindow => UiPopupWindow);
delegated!(PopupWindow => UiPopupWindow {
  content_container_style(value: Style),
});
text_properties!(Button => UiButton);
text_properties!(RepeatButton => UiRepeatButton);

delegated!(Button => UiButton {
  icon_style(value: Style),
  icon(value: impl Into<Prop<IconSource>>),
});
delegated!(RepeatButton => UiRepeatButton {
  timing(delay_ms: impl Into<Prop<u32>>, interval_ms: impl Into<Prop<NonZeroU32>>),
});
delegated!(GroupBox => UiGroupBox {
  title_style(value: Style),
  text(value: impl Into<Prop<String>>),
});

delegated!(TextField => UiTextField {
  label_style(value: Style), input_style(value: Style), text_element_style(value: Style),
  multiline_scroll_view_style(value: Style), vertical_scroller_style(value: Style),
  vertical_slider_style(value: Style), vertical_low_button_style(value: Style),
  vertical_high_button_style(value: Style), vertical_track_style(value: Style),
  vertical_dragger_style(value: Style), vertical_dragger_border_style(value: Style),
  label(value: impl Into<Prop<String>>), value(value: impl Into<Prop<String>>),
  multiline(value: impl Into<Prop<bool>>),
  vertical_scroller_visibility(value: impl Into<Prop<ScrollerVisibility>>),
  password(value: impl Into<Prop<bool>>), read_only(value: impl Into<Prop<bool>>),
  placeholder(value: impl Into<Prop<String>>), hide_placeholder_on_focus(value: impl Into<Prop<bool>>),
  cursor_index(value: impl Into<Prop<u32>>), select_index(value: impl Into<Prop<u32>>),
  select_all_on_focus(value: impl Into<Prop<bool>>), select_all_on_mouse_up(value: impl Into<Prop<bool>>),
});

delegated!(Toggle => UiToggle {
  label_style(value: Style), input_style(value: Style), checkmark_style(value: Style),
  text_style(value: Style), label(value: impl Into<Prop<String>>),
  text(value: impl Into<Prop<String>>), value(value: impl Into<Prop<bool>>),
});
delegated!(RadioButton => UiRadioButton {
  label_style(value: Style), input_style(value: Style), checkmark_background_style(value: Style),
  checkmark_style(value: Style), text_style(value: Style), label(value: impl Into<Prop<String>>),
  text(value: impl Into<Prop<String>>), value(value: impl Into<Prop<bool>>),
});
delegated!(RadioButtonGroup => UiRadioButtonGroup {
  label_style(value: Style), input_style(value: Style), choices_container_style(value: Style),
  content_container_style(value: Style), all_options_style(value: Style),
  option_style(index: u32, value: Style), option_checkmark_background_style(index: u32, value: Style),
  option_checkmark_style(index: u32, value: Style), option_text_style(index: u32, value: Style),
  label(value: impl Into<Prop<String>>),
  choices(values: impl IntoIterator<Item = impl Into<String>>),
  choices_value(value: impl Into<Prop<Vec<String>>>), selected_index(value: impl Into<Prop<u32>>),
});
delegated!(ToggleButtonGroup => UiToggleButtonGroup {
  label_style(value: Style), input_style(value: Style), label(value: impl Into<Prop<String>>),
  multiple_selection(value: impl Into<Prop<bool>>), allow_empty_selection(value: impl Into<Prop<bool>>),
  selected_indices(values: impl IntoIterator<Item = u32>),
  selected_indices_value(value: impl Into<Prop<Vec<u32>>>),
});
delegated!(DropdownField => UiDropdownField {
  label_style(value: Style), input_style(value: Style), text_style(value: Style), arrow_style(value: Style),
  label(value: impl Into<Prop<String>>), show_mixed_value(value: impl Into<Prop<bool>>),
  choices(values: impl IntoIterator<Item = impl Into<String>>),
  choices_value(value: impl Into<Prop<Vec<String>>>), selection(index: u32, value: impl Into<String>),
  selection_value(value: impl Into<Prop<Choice>>), clear_selection(),
});

delegated!(ScrollView => UiScrollView {
  content_and_vertical_scroll_container_style(value: Style), viewport_style(value: Style),
  content_container_style(value: Style), horizontal_scroller_style(value: Style),
  horizontal_slider_style(value: Style), horizontal_low_button_style(value: Style),
  horizontal_high_button_style(value: Style), horizontal_track_style(value: Style),
  horizontal_dragger_style(value: Style), horizontal_dragger_border_style(value: Style),
  vertical_scroller_style(value: Style), vertical_slider_style(value: Style),
  vertical_low_button_style(value: Style), vertical_high_button_style(value: Style),
  vertical_track_style(value: Style), vertical_dragger_style(value: Style),
  vertical_dragger_border_style(value: Style), mode(value: impl Into<Prop<ScrollViewMode>>),
  nested_interaction(value: impl Into<Prop<NestedInteraction>>),
  horizontal_scroller_visibility(value: impl Into<Prop<ScrollerVisibility>>),
  vertical_scroller_visibility(value: impl Into<Prop<ScrollerVisibility>>),
  scroll_offset(value: impl Into<Prop<Vector>>), horizontal_page_size(value: impl Into<Prop<f32>>),
  vertical_page_size(value: impl Into<Prop<f32>>), mouse_wheel_scroll_size(value: impl Into<Prop<f32>>),
  touch_scroll_behavior(value: impl Into<Prop<TouchScrollBehavior>>),
  scroll_deceleration_rate(value: impl Into<Prop<f32>>), elasticity(value: impl Into<Prop<f32>>),
  elastic_animation_interval(value: impl Into<Prop<u32>>),
});
delegated!(Scroller => UiScroller {
  slider_style(value: Style), low_button_style(value: Style), high_button_style(value: Style),
  track_style(value: Style), dragger_style(value: Style), dragger_border_style(value: Style),
  low_value(value: impl Into<Prop<f32>>), high_value(value: impl Into<Prop<f32>>),
  direction(value: impl Into<Prop<SliderDirection>>), value(value: impl Into<Prop<f32>>),
});

macro_rules! slider_properties {
  ($facade:ty => $native:ident, $value:ty) => {
    delegated!($facade => $native {
      label_style(value: Style), input_style(value: Style), track_style(value: Style),
      dragger_style(value: Style), dragger_border_style(value: Style), fill_style(value: Style),
      text_input_style(value: Style), label(value: impl Into<Prop<String>>),
      low_value(value: impl Into<Prop<$value>>), high_value(value: impl Into<Prop<$value>>),
      value(value: impl Into<Prop<$value>>), fill(value: impl Into<Prop<bool>>),
      page_size(value: impl Into<Prop<f32>>), show_input_field(value: impl Into<Prop<bool>>),
      direction(value: impl Into<Prop<SliderDirection>>), inverted(value: impl Into<Prop<bool>>),
    });
  };
}

slider_properties!(Slider => UiSlider, f32);
slider_properties!(SliderInt => UiSliderInt, i32);
delegated!(MinMaxSlider => UiMinMaxSlider {
  label_style(value: Style), input_style(value: Style), track_style(value: Style),
  minimum_thumb_style(value: Style), maximum_thumb_style(value: Style), range_dragger_style(value: Style),
  label(value: impl Into<Prop<String>>), min_value(value: impl Into<Prop<f32>>),
  max_value(value: impl Into<Prop<f32>>), low_limit(value: impl Into<Prop<LowerLimit>>),
  high_limit(value: impl Into<Prop<UpperLimit>>),
});
delegated!(ProgressBar => UiProgressBar {
  container_style(value: Style), background_style(value: Style), progress_style(value: Style),
  title_container_style(value: Style), title_style(value: Style), low_value(value: impl Into<Prop<f32>>),
  high_value(value: impl Into<Prop<f32>>), value(value: impl Into<Prop<f32>>),
  title(value: impl Into<Prop<String>>),
});
delegated!(Tab => UiTab {
  text(value: impl Into<Prop<String>>), header_style(value: Style), label_style(value: Style),
  icon_style(value: Style), underline_style(value: Style), close_button_style(value: Style),
  drag_handle_style(value: Style), drag_handle_leading_bar_style(value: Style),
  drag_handle_trailing_bar_style(value: Style), content_container_style(value: Style),
  icon(value: impl Into<Prop<IconSource>>), closeable(value: impl Into<Prop<bool>>),
});
delegated!(TabView => UiTabView {
  content_viewport_style(value: Style), header_container_style(value: Style),
  content_container_style(value: Style), previous_button_style(value: Style), next_button_style(value: Style),
  selected_tab_index(value: impl Into<Prop<u32>>), reorderable(value: impl Into<Prop<bool>>),
});
delegated!(Image => UiImage {
  source(value: impl Into<Prop<ImageSource>>), source_rect(value: impl Into<Prop<Rect>>),
  tint_color(value: impl Into<Prop<Color>>), scale_mode(value: impl Into<Prop<ImageScaleMode>>),
  uv(value: impl Into<Prop<Rect>>),
});
