//! Capability-safe filter lists for native host filtering and owned paint.

use battlement::{Color, FilterFunction, FilterList, Shadow};

/// One filter supported by Unity's native host-filter writer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotionFilter {
  /// Multiplies rendered color by the supplied tint.
  Tint(Color),
  /// Multiplies rendered alpha by a unitless factor.
  Opacity(f32),
  /// Blends rendered color toward its inverse by a unitless factor.
  Invert(f32),
  /// Blends rendered color toward grayscale by a unitless factor.
  Grayscale(f32),
  /// Blends rendered color toward sepia by a unitless factor.
  Sepia(f32),
  /// Applies a blur radius in panel pixels.
  Blur(f32),
  /// Adjusts contrast by a unitless factor.
  Contrast(f32),
  /// Rotates rendered hue by degrees.
  HueRotate(f32),
}

/// One filter supported by Battlement's owned decorative paint surface.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PaintFilter {
  /// Multiplies paint RGB channels by a unitless factor.
  Brightness(f32),
  /// Shadows the alpha silhouette of one owned paint surface.
  DropShadow(PaintDropShadow),
}

/// An outer shadow of one owned paint surface's alpha silhouette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintDropShadow {
  x: f32,
  y: f32,
  blur: f32,
  spread: f32,
  color: Color,
}

/// Ordered native host filters accepted by Motion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotionFilterList(Vec<MotionFilter>);

/// Ordered filters applied only to Battlement-owned decorative paint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintFilterList(Vec<PaintFilter>);

impl MotionFilterList {
  /// Creates a native filter list in evaluation order.
  #[must_use]
  pub fn new(values: impl IntoIterator<Item = MotionFilter>) -> Self {
    Self(values.into_iter().collect())
  }

  /// Returns native filters in evaluation order.
  #[must_use]
  pub fn as_slice(&self) -> &[MotionFilter] {
    &self.0
  }

  /// Appends one native filter.
  #[must_use]
  pub fn operation(mut self, value: MotionFilter) -> Self {
    self.0.push(value);
    self
  }

  /// Appends Gaussian blur in pixels.
  #[must_use]
  pub fn blur(self, radius: f32) -> Self {
    self.operation(MotionFilter::Blur(radius))
  }

  /// Appends a contrast multiplier.
  #[must_use]
  pub fn contrast(self, amount: f32) -> Self {
    self.operation(MotionFilter::Contrast(amount))
  }

  /// Appends hue rotation in degrees.
  #[must_use]
  pub fn hue_rotate(self, degrees: f32) -> Self {
    self.operation(MotionFilter::HueRotate(degrees))
  }

  /// Appends an opacity multiplier.
  #[must_use]
  pub fn opacity(self, amount: f32) -> Self {
    self.operation(MotionFilter::Opacity(amount))
  }

  /// Appends another native filter list.
  #[must_use]
  pub fn then(mut self, value: Self) -> Self {
    self.0.extend(value.0);
    self
  }

  pub(crate) fn from_protocol(value: &FilterList) -> Option<Self> {
    value
      .as_slice()
      .iter()
      .copied()
      .map(MotionFilter::from_protocol)
      .collect::<Option<Vec<_>>>()
      .map(Self)
  }

  pub(crate) fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    Self(mix_list(&from.0, &to.0, progress, MotionFilter::mix))
  }
}

impl PaintFilterList {
  /// Appends a brightness multiplier.
  #[must_use]
  pub fn brightness(mut self, amount: f32) -> Self {
    self.0.push(PaintFilter::Brightness(amount));
    self
  }

  /// Sets the one alpha-silhouette shadow in evaluation order.
  #[must_use]
  pub fn drop_shadow(mut self, value: PaintDropShadow) -> Self {
    if let Some(index) = self
      .0
      .iter()
      .position(|filter| matches!(filter, PaintFilter::DropShadow(_)))
    {
      self.0.remove(index);
    }
    self.0.push(PaintFilter::DropShadow(value));
    self
  }

  /// Appends another owned-paint filter list.
  #[must_use]
  pub fn then(mut self, value: Self) -> Self {
    for filter in value.0 {
      match filter {
        PaintFilter::Brightness(value) => self.0.push(PaintFilter::Brightness(value)),
        PaintFilter::DropShadow(value) => self = self.drop_shadow(value),
      }
    }
    self
  }

  pub(crate) fn from_protocol(value: &FilterList) -> Option<Self> {
    let filters = value
      .as_slice()
      .iter()
      .copied()
      .map(PaintFilter::from_protocol)
      .collect::<Option<Vec<_>>>()?;
    (filters
      .iter()
      .filter(|filter| matches!(filter, PaintFilter::DropShadow(_)))
      .count()
      <= 1)
      .then_some(Self(filters))
  }

  pub(crate) fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    Self(mix_list(&from.0, &to.0, progress, PaintFilter::mix))
  }
}

impl PaintDropShadow {
  /// Creates an outer shadow for owned decorative paint.
  #[must_use]
  pub const fn new(x: f32, y: f32, blur: f32, spread: f32, color: Color) -> Self {
    Self {
      x,
      y,
      blur,
      spread,
      color,
    }
  }

  fn from_protocol(value: Shadow) -> Option<Self> {
    (!value.inset).then_some(Self::new(
      value.x,
      value.y,
      value.blur,
      value.spread,
      value.color,
    ))
  }

  fn into_protocol(self) -> Shadow {
    Shadow {
      x: self.x,
      y: self.y,
      blur: self.blur,
      spread: self.spread,
      color: self.color,
      inset: false,
    }
  }

  fn mix(self, to: Self, progress: f64) -> Self {
    Self::new(
      mix(self.x, to.x, progress),
      mix(self.y, to.y, progress),
      mix(self.blur, to.blur, progress),
      mix(self.spread, to.spread, progress),
      mix_color(self.color, to.color, progress),
    )
  }
}

impl From<MotionFilterList> for FilterList {
  fn from(value: MotionFilterList) -> Self {
    Self::new(value.0.into_iter().map(FilterFunction::from))
  }
}

impl From<PaintFilterList> for FilterList {
  fn from(value: PaintFilterList) -> Self {
    Self::new(value.0.into_iter().map(FilterFunction::from))
  }
}

impl From<MotionFilter> for FilterFunction {
  fn from(value: MotionFilter) -> Self {
    match value {
      MotionFilter::Tint(value) => Self::Tint(value),
      MotionFilter::Opacity(value) => Self::Opacity(value),
      MotionFilter::Invert(value) => Self::Invert(value),
      MotionFilter::Grayscale(value) => Self::Grayscale(value),
      MotionFilter::Sepia(value) => Self::Sepia(value),
      MotionFilter::Blur(value) => Self::Blur(value),
      MotionFilter::Contrast(value) => Self::Contrast(value),
      MotionFilter::HueRotate(value) => Self::HueRotate(value),
    }
  }
}

impl From<PaintFilter> for FilterFunction {
  fn from(value: PaintFilter) -> Self {
    match value {
      PaintFilter::Brightness(value) => Self::Brightness(value),
      PaintFilter::DropShadow(value) => Self::DropShadow(value.into_protocol()),
    }
  }
}

impl MotionFilter {
  fn from_protocol(value: FilterFunction) -> Option<Self> {
    match value {
      FilterFunction::Tint(value) => Some(Self::Tint(value)),
      FilterFunction::Opacity(value) => Some(Self::Opacity(value)),
      FilterFunction::Invert(value) => Some(Self::Invert(value)),
      FilterFunction::Grayscale(value) => Some(Self::Grayscale(value)),
      FilterFunction::Sepia(value) => Some(Self::Sepia(value)),
      FilterFunction::Blur(value) => Some(Self::Blur(value)),
      FilterFunction::Contrast(value) => Some(Self::Contrast(value)),
      FilterFunction::HueRotate(value) => Some(Self::HueRotate(value)),
      FilterFunction::Brightness(_)
      | FilterFunction::Saturate(_)
      | FilterFunction::DropShadow(_) => None,
    }
  }

  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    match (from, to) {
      (Self::Tint(from), Self::Tint(to)) => Self::Tint(mix_color(*from, *to, progress)),
      (Self::Opacity(from), Self::Opacity(to)) => Self::Opacity(mix(*from, *to, progress)),
      (Self::Invert(from), Self::Invert(to)) => Self::Invert(mix(*from, *to, progress)),
      (Self::Grayscale(from), Self::Grayscale(to)) => Self::Grayscale(mix(*from, *to, progress)),
      (Self::Sepia(from), Self::Sepia(to)) => Self::Sepia(mix(*from, *to, progress)),
      (Self::Blur(from), Self::Blur(to)) => Self::Blur(mix(*from, *to, progress)),
      (Self::Contrast(from), Self::Contrast(to)) => Self::Contrast(mix(*from, *to, progress)),
      (Self::HueRotate(from), Self::HueRotate(to)) => Self::HueRotate(mix(*from, *to, progress)),
      _ => *if progress < 0.5 { from } else { to },
    }
  }
}

impl PaintFilter {
  fn from_protocol(value: FilterFunction) -> Option<Self> {
    match value {
      FilterFunction::Brightness(value) => Some(Self::Brightness(value)),
      FilterFunction::DropShadow(value) => {
        PaintDropShadow::from_protocol(value).map(Self::DropShadow)
      }
      _ => None,
    }
  }

  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    match (from, to) {
      (Self::Brightness(from), Self::Brightness(to)) => Self::Brightness(mix(*from, *to, progress)),
      (Self::DropShadow(from), Self::DropShadow(to)) => Self::DropShadow(from.mix(*to, progress)),
      _ => *if progress < 0.5 { from } else { to },
    }
  }
}

fn mix_list<T: Clone>(
  from: &[T],
  to: &[T],
  progress: f64,
  mix: impl Fn(&T, &T, f64) -> T,
) -> Vec<T> {
  if from.len() != to.len() {
    return if progress < 0.5 { from } else { to }.to_vec();
  }
  from
    .iter()
    .zip(to)
    .map(|(from, to)| mix(from, to, progress))
    .collect()
}

fn mix(from: f32, to: f32, progress: f64) -> f32 {
  from + (to - from) * progress as f32
}

fn mix_color(from: Color, to: Color, progress: f64) -> Color {
  Color::rgba(
    from.r + (to.r - from.r) * progress,
    from.g + (to.g - from.g) * progress,
    from.b + (to.b - from.b) * progress,
    from.a + (to.a - from.a) * progress,
  )
}
