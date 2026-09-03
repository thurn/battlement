//! Center a fixed design canvas inside the actual space assigned by layout.

use crate::{component::Component, element_ref, geometry, host::View, render::Render};
use battlement::{
  Align, Justify, Length, LengthUnits, Overflow, Position, Scale, Style, TransformOrigin,
};
use std::rc::Rc;

/// A measured viewport containing a uniformly scaled, fixed-size canvas.
///
/// Flex/grid layout determines the available space. Children retain their design
/// coordinates, while the bounds reserve exactly the scaled size. Until the host
/// reports geometry the canvas has zero scale, avoiding an oversized first frame.
/// Viewport padding and borders are excluded from the measured content area.
/// Scaling never enlarges the design unless `max_scale` explicitly allows it.
///
/// ```
/// use battlement_reactant::{scale_to_fit::ScaleToFit, host::Label};
/// let canvas = ScaleToFit::new(1024.0, 1536.0).child(Label::new("Portrait"));
/// ```
pub struct ScaleToFit<R = ()> {
  width: f32,
  height: f32,
  max_scale: f32,
  roomy: Option<(f32, f32)>,
  viewport: View,
  canvas: View,
  viewport_style: Rc<dyn Fn(Style) -> Style>,
  canvas_style: Rc<dyn Fn(Style) -> Style>,
  bounds_name: String,
  children: Rc<R>,
}

impl ScaleToFit {
  /// Creates a fit viewport for positive finite design dimensions.
  pub fn new(width: f32, height: f32) -> Self {
    assert!(
      width.is_finite() && width > 0.0,
      "design width must be positive and finite"
    );
    assert!(
      height.is_finite() && height > 0.0,
      "design height must be positive and finite"
    );
    Self {
      width,
      height,
      max_scale: 1.0,
      roomy: None,
      viewport: View::new(),
      canvas: View::new(),
      viewport_style: Rc::new(|style| style),
      canvas_style: Rc::new(|style| style),
      bounds_name: "fit-bounds".into(),
      children: Rc::new(()),
    }
  }
}

impl<R: Render> ScaleToFit<R> {
  /// Sets content authored in design coordinates.
  pub fn child<C: Render>(self, child: C) -> ScaleToFit<C> {
    ScaleToFit {
      width: self.width,
      height: self.height,
      max_scale: self.max_scale,
      roomy: self.roomy,
      viewport: self.viewport,
      canvas: self.canvas,
      viewport_style: self.viewport_style,
      canvas_style: self.canvas_style,
      bounds_name: self.bounds_name,
      children: Rc::new(child),
    }
  }

  /// Caps magnification; one preserves the authored maximum size.
  pub fn max_scale(mut self, scale: f32) -> Self {
    assert!(
      scale.is_finite() && scale >= 0.0,
      "maximum scale must be finite and nonnegative"
    );
    self.max_scale = scale;
    self
  }

  /// Leaves extra breathing room when the available width reaches `min_width`.
  pub fn roomy_scale(mut self, min_width: f32, factor: f32) -> Self {
    assert!(
      min_width.is_finite() && min_width >= 0.0,
      "breakpoint must be finite and nonnegative"
    );
    assert!(
      (0.0..=1.0).contains(&factor),
      "roomy scale must be between zero and one"
    );
    self.roomy = Some((min_width, factor));
    self
  }

  /// Configures viewport metadata and semantics; use `viewport_style` for styling.
  pub fn viewport(mut self, configure: impl FnOnce(View) -> View) -> Self {
    self.viewport = configure(self.viewport);
    self
  }

  /// Configures canvas metadata and semantics; use `canvas_style` for styling.
  pub fn canvas(mut self, configure: impl FnOnce(View) -> View) -> Self {
    self.canvas = configure(self.canvas);
    self
  }

  /// Adds viewport styling while retaining the supplied sizing and alignment.
  pub fn viewport_style(mut self, style: impl Fn(Style) -> Style + 'static) -> Self {
    self.viewport_style = Rc::new(style);
    self
  }

  /// Adds canvas styling while retaining the supplied design transform.
  pub fn canvas_style(mut self, style: impl Fn(Style) -> Style + 'static) -> Self {
    self.canvas_style = Rc::new(style);
    self
  }

  /// Names the host that reserves the scaled canvas dimensions.
  pub fn bounds_name(mut self, name: impl Into<String>) -> Self {
    self.bounds_name = name.into();
    self
  }
}

impl<R: Render> Component for ScaleToFit<R> {
  fn render(&self) -> impl Render {
    let reference = element_ref::use_element_ref();
    let scale = geometry::use_geometry(reference.clone())
      .measurements
      .latest
      .map_or(0.0, |value| {
        let width = value.layout.width as f32;
        let fit = (width / self.width)
          .min(value.layout.height as f32 / self.height)
          .clamp(0.0, self.max_scale);
        fit
          * self
            .roomy
            .filter(|(threshold, _)| width >= *threshold)
            .map_or(1.0, |(_, factor)| factor)
      });
    self
      .viewport
      .clone()
      .style((self.viewport_style)(
        Style::new()
          .min_width(0)
          .min_height(0)
          .height(100.pct())
          .flex_grow(1)
          .overflow(Overflow::Hidden),
      ))
      .child(
        View::new()
          .element_ref(reference)
          .style(
            Style::new()
              .min_width(0)
              .min_height(0)
              .flex_basis(0)
              .flex_grow(1)
              .align_self(Align::Stretch)
              .align_items(Align::Center)
              .justify_content(Justify::Center),
          )
          .child(
            View::new()
              .name(self.bounds_name.clone())
              .style(
                Style::new()
                  .position(Position::Relative)
                  .width(self.width * scale)
                  .height(self.height * scale)
                  .flex_shrink(0),
              )
              .child(
                self
                  .canvas
                  .clone()
                  .style((self.canvas_style)(
                    Style::new()
                      .position(Position::Absolute)
                      .left(0)
                      .top(0)
                      .width(self.width)
                      .height(self.height)
                      .scale(Scale::uniform(scale))
                      .transform_origin(TransformOrigin::two_dimensional(
                        Length::Px(0.0),
                        Length::Px(0.0),
                      )),
                  ))
                  .child(Rc::clone(&self.children)),
              ),
          ),
      )
  }
}
