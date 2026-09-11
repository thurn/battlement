use std::num::NonZeroU32;

use battlement_types::{
  Color, MaterialAddress, ObjectId, Rect, ScreenSize, SpriteAddress, TextureAddress,
};
use battlement_ui::{
  Align, AspectRatio, BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat,
  BackgroundRepeatMode, BackgroundSize, BackgroundSource, Choice, Cursor, CursorHotspot, Display,
  DynamicAtlasSettings, FlexDirection, FlexWrap, ImageScaleMode, InlineKeyword,
  InteractionDistance, InteractionLayerMask, Justify, LanguageDirection, Length, LengthOrAuto,
  LengthUnits, LowerLimit, Overflow, OverflowClipBox, PaintLayer, PaintStyle,
  PanelInputConfiguration, PanelInputRedirection, PanelScaleMode, PanelScreenMatchMode,
  PanelSettings, PickingMode, Position, Prop, ScrollViewMode, ScrollerVisibility, SliceType,
  SliderDirection, Style, StyleValue, TouchScrollBehavior, Translate, UiBox, UiButton, UiDocument,
  UiDropdownField, UiElement, UiEventKind, UiGroupBox, UiImage, UiLabel, UiMinMaxSlider, UiNode,
  UiPopupWindow, UiProgressBar, UiRadioButton, UiRadioButtonGroup, UiRepeatButton, UiScrollView,
  UiScroller, UiSlider, UiSliderInt, UiTab, UiTabView, UiTextElement, UiTextField, UiToggle,
  UiToggleButtonGroup, UiValidationError, UiVisualElement, UpperLimit, UsageHint, Vector,
  Visibility, validate_documents, validate_element_update, validate_panel_input_configuration,
  validate_panel_settings,
};

const DOCUMENT_ID: &str = "3b5fe431-f332-4314-a0f6-a7353fa17622";
const ROOT_ID: &str = "471834d0-8abc-4964-a3da-f8bc61de7c16";
const BOX_ID: &str = "fc59ba64-b70c-4a20-83fd-1852b1cb4995";
const LABEL_ID: &str = "a9e0ac34-da16-4d33-8952-b6541ef075e8";

#[path = "ui_api/protocol_encoding.rs"]
mod protocol_encoding;
#[path = "ui_api/validation_contracts.rs"]
mod validation_contracts;

fn id(value: &str) -> ObjectId {
  value.parse().unwrap()
}
