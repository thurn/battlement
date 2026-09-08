use serde::{Deserialize, Serialize};

use crate::Length;

/// How a composited element subtree combines with its backdrop.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaintBlendMode {
  #[default]
  /// Premultiplied source-over compositing.
  Normal,
  /// Lightens the backdrop using the inverse product of source and backdrop.
  Screen,
  /// Adds the premultiplied source to the backdrop.
  Additive,
}

/// Interior rule for a path containing closed polygon contours.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaintFillRule {
  #[default]
  /// Includes points whose signed contour crossing count is nonzero.
  NonZero,
  /// Alternates filled regions and holes at each contour crossing.
  EvenOdd,
}

/// A compound polygon mask in border-box coordinates, with at most 64 vertices total.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PaintClipPath {
  /// Closed contours; the last vertex connects to the first.
  pub contours: Vec<Vec<[Length; 2]>>,
  /// Determines which overlapping contours form holes.
  pub fill_rule: PaintFillRule,
}

impl PaintClipPath {
  #[must_use]
  /// Starts a path to which closed contours can be appended.
  pub const fn new(fill_rule: PaintFillRule) -> Self {
    Self {
      contours: Vec::new(),
      fill_rule,
    }
  }

  /// Appends a closed contour without duplicating its first vertex.
  #[must_use]
  pub fn contour(mut self, points: impl IntoIterator<Item = [Length; 2]>) -> Self {
    self.contours.push(points.into_iter().collect());
    self
  }

  pub(crate) fn is_valid(&self) -> bool {
    if self.contours.is_empty() || self.contours.iter().map(Vec::len).sum::<usize>() > 64 {
      return false;
    }
    self.contours.iter().all(|contour| {
      contour.len() >= 3 && contour.iter().flatten().all(|length| length.is_finite())
    })
  }
}
