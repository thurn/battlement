use battlement::{Overflow, PaintBlendMode, PaintClipPath, PaintStyle, Style};

use crate::{component::Component, host::View, render::Render};

/// A decorative subtree composited as one clipped layer.
///
/// The layout box bounds the offscreen layer; descendants cannot paint outside it.
/// Screen and additive groups use a separate panel: author decorative children
/// with explicit layout and paint instead of inherited typography or stylesheets.
pub struct EffectGroup {
  host: View,
  paint: PaintStyle,
}

impl EffectGroup {
  /// Creates a pointer-transparent, accessibility-hidden effect container.
  pub fn new() -> Self {
    Self {
      host: View::decorative().style(Style::new().overflow(Overflow::Hidden)),
      paint: PaintStyle::new().blend_mode(PaintBlendMode::Normal),
    }
  }

  /// Sets the native query name.
  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Sets the layout and styling, retaining the bounded offscreen surface.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style.overflow(Overflow::Hidden));
    self
  }

  /// Masks the rendered children, including their paint shadows.
  pub fn clip_path(mut self, path: PaintClipPath) -> Self {
    self.paint = self.paint.subtree_clip(path);
    self
  }

  /// Sets how the completed layer combines with its backdrop.
  pub fn blend_mode(mut self, mode: PaintBlendMode) -> Self {
    self.paint = self.paint.blend_mode(mode);
    self
  }

  /// Appends content to the composited layer.
  pub fn child(mut self, child: impl Render) -> Self {
    self.host = self.host.child(child);
    self
  }
}

impl Component for EffectGroup {
  fn render(&self) -> impl Render {
    self.host.clone().paint(self.paint.clone())
  }
}

impl Default for EffectGroup {
  fn default() -> Self {
    Self::new()
  }
}
