//! Filters for Battlement-owned decorative paint.

use battlement::{Color, FilterFunction, FilterList, Shadow};

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

/// Ordered filters applied only to Battlement-owned decorative paint.
///
/// Whole-subtree filtering is intentionally unavailable: Unity's native filter
/// path can truncate scaled UI during transitions. Use opacity, scale, clipping,
/// or separate decorative overlays for similar effects. Filters baked into
/// generated artwork and filters on owned paint do not use that Unity path.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintFilterList(Vec<PaintFilter>);

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

impl From<PaintFilterList> for FilterList {
  fn from(value: PaintFilterList) -> Self {
    Self::new(value.0.into_iter().map(FilterFunction::from))
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

impl PaintFilter {
  fn from_protocol(value: FilterFunction) -> Option<Self> {
    match value {
      FilterFunction::Brightness(value) => Some(Self::Brightness(value)),
      FilterFunction::DropShadow(value) => {
        PaintDropShadow::from_protocol(value).map(Self::DropShadow)
      }
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
