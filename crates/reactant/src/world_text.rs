//! Rich world text through the existing TextMesh Pro host.

use battlement::RenderOrder;

use battlement::{
  Color, ColorPayload, CommandBody, GameObjectKind, HorizontalAlignment, ObjectEnabledPayload,
  ObjectId, PropertyCommand, RgbColor, SetFontPayload, TextAlignmentPayload, TextContentPayload,
  TextMeshProFontAddress, TextSizePayload, TextState, TextWrappingPayload, VerticalAlignment,
};

use crate::world_object::WorldObject;

/// Prepared-font world text with explicit placement, wrapping, and alignment.
pub type Text = WorldObject<TextState>;

impl Text {
  /// Sets renderer order relative to the nearest enclosing sorting group.
  pub fn layer(mut self, order: i16) -> Self {
    self.group.render_order = Some(RenderOrder::Layer(order));
    self
  }

  /// Creates centered, unwrapped text; set its required font before rendering.
  pub fn new() -> Self {
    Self::with_properties(TextState::new("", ""), |text| {
      assert!(!text.font.as_str().is_empty(), "world text requires a font");
      GameObjectKind::Text { text: text.clone() }
    })
  }

  /// Sets content, including rich-text tags when enabled.
  pub fn text(mut self, text: impl Into<String>) -> Self {
    self.properties.text = text.into();
    self
  }

  /// Selects a prepared TextMesh Pro font.
  pub fn font(mut self, font: impl Into<TextMeshProFontAddress>) -> Self {
    self.properties.font = font.into();
    self
  }

  /// Sets the existing text host's positive font size.
  pub fn size(mut self, size: f64) -> Self {
    self.properties.size = size;
    self
  }

  /// Sets linear RGBA text color.
  pub fn color(mut self, color: Color) -> Self {
    self.properties.color = color;
    self
  }

  /// Sets linear RGB while preserving opacity.
  pub fn tint(mut self, tint: RgbColor) -> Self {
    self.properties.color.r = tint.r;
    self.properties.color.g = tint.g;
    self.properties.color.b = tint.b;
    self
  }

  /// Sets opacity in the inclusive range zero to one.
  pub fn opacity(mut self, opacity: f64) -> Self {
    self.properties.color.a = opacity;
    self
  }

  /// Sets horizontal and vertical alignment in the local text rectangle.
  pub fn alignment(mut self, horizontal: HorizontalAlignment, vertical: VerticalAlignment) -> Self {
    self.properties.horizontal = horizontal;
    self.properties.vertical = vertical;
    self
  }

  /// Sets a positive local wrapping width; none disables wrapping.
  pub fn wrapping(mut self, width: Option<f64>) -> Self {
    self.properties.wrap_width = width;
    self
  }

  /// Enables or disables the existing host's rich-text interpretation.
  pub fn rich_text(mut self, enabled: bool) -> Self {
    self.properties.rich_text = enabled;
    self
  }

  /// Rotates the text toward the existing input camera when enabled.
  pub fn face_camera(mut self, enabled: bool) -> Self {
    self.properties.face_camera = enabled;
    self
  }
}

impl Default for Text {
  fn default() -> Self {
    Self::new()
  }
}

pub(crate) fn commands(
  object_id: ObjectId,
  old: &TextState,
  new: &TextState,
  out: &mut Vec<CommandBody>,
) {
  if old.text != new.text {
    out.push(CommandBody::TextSetContent(TextContentPayload {
      object_id,
      content: new.text.clone(),
    }));
  }
  if old.font != new.font {
    out.push(CommandBody::TextSetFont(SetFontPayload {
      object_id,
      address: new.font.clone(),
    }));
  }
  if old.size != new.size {
    out.push(CommandBody::TextSetSize(PropertyCommand::canceling(
      TextSizePayload {
        object_id,
        size: new.size,
      },
    )));
  }
  if old.color != new.color {
    out.push(CommandBody::TextSetColor(PropertyCommand::canceling(
      ColorPayload {
        object_id,
        color: new.color,
      },
    )));
  }
  if old.horizontal != new.horizontal || old.vertical != new.vertical {
    out.push(CommandBody::TextSetAlignment(TextAlignmentPayload {
      object_id,
      horizontal: new.horizontal,
      vertical: new.vertical,
    }));
  }
  if old.wrap_width != new.wrap_width {
    out.push(CommandBody::TextSetWrapping(TextWrappingPayload {
      object_id,
      wrap_width: new.wrap_width,
    }));
  }
  if old.rich_text != new.rich_text {
    out.push(CommandBody::TextSetRichText(ObjectEnabledPayload {
      object_id,
      enabled: new.rich_text,
    }));
  }
  if old.face_camera != new.face_camera {
    out.push(CommandBody::TextSetFaceCamera(ObjectEnabledPayload {
      object_id,
      enabled: new.face_camera,
    }));
  }
}
