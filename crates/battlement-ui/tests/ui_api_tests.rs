use battlement_types::{Color, ObjectId, Rect, SpriteAddress, TextureAddress};
use battlement_ui::{
  AspectRatio, BackgroundPosition, BackgroundPositionKeyword, BackgroundSize, Cursor,
  CursorHotspot, DynamicAtlasSettings, InteractionDistance, LanguageDirection, Length,
  LengthOrAuto, PaintLayer, PaintStyle, PanelInputConfiguration, PanelScaleMode, PanelSettings,
  PickingMode, Style, UiBox, UiDocument, UiEventKind, UiImage, UiLabel, UiNode, UiScrollView,
  UiScroller, UiTab, UiTabView, UiTextElement, UiTextField, UiValidationError, UiVisualElement,
  UsageHint, Vector, validate_documents, validate_element_update,
  validate_panel_input_configuration, validate_panel_settings,
};

const DOCUMENT_ID: &str = "3b5fe431-f332-4314-a0f6-a7353fa17622";
const ROOT_ID: &str = "471834d0-8abc-4964-a3da-f8bc61de7c16";
const BOX_ID: &str = "fc59ba64-b70c-4a20-83fd-1852b1cb4995";
const LABEL_ID: &str = "a9e0ac34-da16-4d33-8952-b6541ef075e8";

#[path = "ui_api/validation_contracts.rs"]
mod validation_contracts;

fn id(value: &str) -> ObjectId {
  value.parse().unwrap()
}
