use std::num::NonZeroU32;

use battlement::{
  Choice, Color, IconSource, ImageScaleMode, ImageSource, LowerLimit, NestedInteraction, Prop,
  Rect, ScrollViewMode, ScrollerVisibility, SliderDirection, Style, TouchScrollBehavior,
  UpperLimit, Vector,
};
use trox::LocalizedString;

use crate::{
  host::{
    ButtonHost, DropdownField, GroupBox, ImageHost, Label, LocalizedChoice, MinMaxSlider,
    PopupWindow, ProgressBar, RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller,
    SliderHost, SliderInt, TabHost, TabView, TextElement, TextField, ToggleButtonGroup, ToggleHost,
  },
  localization,
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
      rich_text(value: impl Into<Prop<bool>>),
      emoji_fallback(value: impl Into<Prop<bool>>),
      parse_escape_sequences(value: impl Into<Prop<bool>>),
      tooltip_when_elided(value: impl Into<Prop<bool>>),
    });
    localized_properties!($facade => $native { text });
  };
}

macro_rules! localized_properties {
  ($facade:ty => $native:ident { $($name:ident),+ $(,)? }) => {
    impl $facade {
      $(
        #[doc = concat!("Authors localized `", stringify!($name), "` text.")]
        #[must_use]
        pub fn $name(mut self, value: impl Into<Prop<LocalizedString>>) -> Self {
          let value = value.into();
          self.state.localizers.push(std::rc::Rc::new(move |host| {
            host.$name(localization::resolve_prop(&value))
          }));
          self
        }
      )+
    }
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
text_properties!(ButtonHost => UiButton);
text_properties!(RepeatButton => UiRepeatButton);

delegated!(ButtonHost => UiButton {
  icon_style(value: Style),
  icon(value: impl Into<Prop<IconSource>>),
});
delegated!(RepeatButton => UiRepeatButton {
  timing(delay_ms: impl Into<Prop<u32>>, interval_ms: impl Into<Prop<NonZeroU32>>),
});
delegated!(GroupBox => UiGroupBox {
  title_style(value: Style),
});
localized_properties!(GroupBox => UiGroupBox { text });

delegated!(TextField => UiTextField {
  label_style(value: Style), input_style(value: Style), text_element_style(value: Style),
  multiline_scroll_view_style(value: Style), vertical_scroller_style(value: Style),
  vertical_slider_style(value: Style), vertical_low_button_style(value: Style),
  vertical_high_button_style(value: Style), vertical_track_style(value: Style),
  vertical_dragger_style(value: Style), vertical_dragger_border_style(value: Style),
  value(value: impl Into<Prop<String>>),
  multiline(value: impl Into<Prop<bool>>),
  vertical_scroller_visibility(value: impl Into<Prop<ScrollerVisibility>>),
  password(value: impl Into<Prop<bool>>), read_only(value: impl Into<Prop<bool>>),
  hide_placeholder_on_focus(value: impl Into<Prop<bool>>),
  cursor_index(value: impl Into<Prop<u32>>), select_index(value: impl Into<Prop<u32>>),
  select_all_on_focus(value: impl Into<Prop<bool>>), select_all_on_mouse_up(value: impl Into<Prop<bool>>),
});
localized_properties!(TextField => UiTextField { label, placeholder });

delegated!(ToggleHost => UiToggle {
  label_style(value: Style), input_style(value: Style), checkmark_style(value: Style),
  text_style(value: Style), value(value: impl Into<Prop<bool>>),
});
localized_properties!(ToggleHost => UiToggle { label, text });
delegated!(RadioButton => UiRadioButton {
  label_style(value: Style), input_style(value: Style), checkmark_background_style(value: Style),
  checkmark_style(value: Style), text_style(value: Style), value(value: impl Into<Prop<bool>>),
});
localized_properties!(RadioButton => UiRadioButton { label, text });
delegated!(RadioButtonGroup => UiRadioButtonGroup {
  label_style(value: Style), input_style(value: Style), choices_container_style(value: Style),
  content_container_style(value: Style), all_options_style(value: Style),
  option_style(index: u32, value: Style), option_checkmark_background_style(index: u32, value: Style),
  option_checkmark_style(index: u32, value: Style), option_text_style(index: u32, value: Style),
  selected_index(value: impl Into<Prop<u32>>),
});
localized_properties!(RadioButtonGroup => UiRadioButtonGroup { label });
delegated!(ToggleButtonGroup => UiToggleButtonGroup {
  label_style(value: Style), input_style(value: Style),
  multiple_selection(value: impl Into<Prop<bool>>), allow_empty_selection(value: impl Into<Prop<bool>>),
  selected_indices(values: impl IntoIterator<Item = u32>),
  selected_indices_value(value: impl Into<Prop<Vec<u32>>>),
});
localized_properties!(ToggleButtonGroup => UiToggleButtonGroup { label });
delegated!(DropdownField => UiDropdownField {
  label_style(value: Style), input_style(value: Style), text_style(value: Style), arrow_style(value: Style),
  show_mixed_value(value: impl Into<Prop<bool>>), clear_selection(),
});
localized_properties!(DropdownField => UiDropdownField { label });

impl RadioButtonGroup {
  /// Replaces the ordered localized option labels.
  #[must_use]
  pub fn choices(mut self, values: impl IntoIterator<Item = LocalizedString>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    self.state.localizers.push(std::rc::Rc::new(move |host| {
      host.choices(localization::resolve_values(&values))
    }));
    self
  }

  /// Replaces or resets the ordered localized option labels.
  #[must_use]
  pub fn choices_value(mut self, value: impl Into<Prop<Vec<LocalizedString>>>) -> Self {
    let value = value.into();
    self.state.localizers.push(std::rc::Rc::new(move |host| {
      host.choices_value(localization::resolve_values_prop(&value))
    }));
    self
  }
}

impl DropdownField {
  /// Replaces the ordered localized option labels.
  #[must_use]
  pub fn choices(mut self, values: impl IntoIterator<Item = LocalizedString>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    self.state.localizers.push(std::rc::Rc::new(move |host| {
      host.choices(localization::resolve_values(&values))
    }));
    self
  }

  /// Replaces or resets the ordered localized option labels.
  #[must_use]
  pub fn choices_value(mut self, value: impl Into<Prop<Vec<LocalizedString>>>) -> Self {
    let value = value.into();
    self.state.localizers.push(std::rc::Rc::new(move |host| {
      host.choices_value(localization::resolve_values_prop(&value))
    }));
    self
  }

  /// Selects one localized display value by index.
  #[must_use]
  pub fn selection(mut self, index: u32, value: LocalizedString) -> Self {
    self.state.localizers.push(std::rc::Rc::new(move |host| {
      host.selection(index, localization::resolve(&value))
    }));
    self
  }

  /// Replaces or resets the localized selection.
  #[must_use]
  pub fn selection_value(mut self, value: impl Into<Prop<LocalizedChoice>>) -> Self {
    let value = value.into();
    self.state.localizers.push(std::rc::Rc::new(move |host| {
      host.selection_value(match &value {
        Prop::Unset => Prop::Unset,
        Prop::Set(value) => Prop::Set(Choice {
          index: value.index,
          value: value.value.as_ref().map(localization::resolve),
        }),
        Prop::Reset => Prop::Reset,
      })
    }));
    self
  }
}

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
      text_input_style(value: Style),
      low_value(value: impl Into<Prop<$value>>), high_value(value: impl Into<Prop<$value>>),
      value(value: impl Into<Prop<$value>>), fill(value: impl Into<Prop<bool>>),
      page_size(value: impl Into<Prop<f32>>), show_input_field(value: impl Into<Prop<bool>>),
      direction(value: impl Into<Prop<SliderDirection>>), inverted(value: impl Into<Prop<bool>>),
    });
    localized_properties!($facade => $native { label });
  };
}

slider_properties!(SliderHost => UiSlider, f32);
slider_properties!(SliderInt => UiSliderInt, i32);
delegated!(MinMaxSlider => UiMinMaxSlider {
  label_style(value: Style), input_style(value: Style), track_style(value: Style),
  minimum_thumb_style(value: Style), maximum_thumb_style(value: Style), range_dragger_style(value: Style),
  min_value(value: impl Into<Prop<f32>>),
  max_value(value: impl Into<Prop<f32>>), low_limit(value: impl Into<Prop<LowerLimit>>),
  high_limit(value: impl Into<Prop<UpperLimit>>),
});
localized_properties!(MinMaxSlider => UiMinMaxSlider { label });
delegated!(ProgressBar => UiProgressBar {
  container_style(value: Style), background_style(value: Style), progress_style(value: Style),
  title_container_style(value: Style), title_style(value: Style), low_value(value: impl Into<Prop<f32>>),
  high_value(value: impl Into<Prop<f32>>), value(value: impl Into<Prop<f32>>),
});
localized_properties!(ProgressBar => UiProgressBar { title });
delegated!(TabHost => UiTab {
  header_style(value: Style), label_style(value: Style),
  icon_style(value: Style), underline_style(value: Style), close_button_style(value: Style),
  drag_handle_style(value: Style), drag_handle_leading_bar_style(value: Style),
  drag_handle_trailing_bar_style(value: Style), content_container_style(value: Style),
  icon(value: impl Into<Prop<IconSource>>), closeable(value: impl Into<Prop<bool>>),
});
localized_properties!(TabHost => UiTab { text });
delegated!(TabView => UiTabView {
  content_viewport_style(value: Style), header_container_style(value: Style),
  content_container_style(value: Style), previous_button_style(value: Style), next_button_style(value: Style),
  selected_tab_index(value: impl Into<Prop<u32>>), reorderable(value: impl Into<Prop<bool>>),
});
delegated!(ImageHost => UiImage {
  source(value: impl Into<Prop<ImageSource>>), source_rect(value: impl Into<Prop<Rect>>),
  tint_color(value: impl Into<Prop<Color>>), scale_mode(value: impl Into<Prop<ImageScaleMode>>),
  uv(value: impl Into<Prop<Rect>>),
});
