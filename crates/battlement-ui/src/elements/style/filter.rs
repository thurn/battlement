use serde::{Deserialize, Serialize};

use crate::Shadow;

/// One filter evaluated only on Battlement-owned decorative paint.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum FilterFunction {
  /// Adjusts brightness by a unitless factor.
  Brightness(f32),
  /// Applies one painted drop shadow.
  DropShadow(crate::Shadow),
}

/// Ordered filters applied only to Battlement-owned decorative paint.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FilterList(Vec<FilterFunction>);

impl FilterList {
  /// Creates a filter list in evaluation order.
  #[must_use]
  pub fn new(values: impl IntoIterator<Item = FilterFunction>) -> Self {
    Self(values.into_iter().collect())
  }

  /// Returns filter functions in evaluation order.
  #[must_use]
  pub fn as_slice(&self) -> &[FilterFunction] {
    &self.0
  }

  /// Appends one operation, preserving duplicates and authored order.
  #[must_use]
  pub fn operation(mut self, value: FilterFunction) -> Self {
    self.0.push(value);
    self
  }

  /// Appends a brightness multiplier.
  #[must_use]
  pub fn brightness(self, amount: f32) -> Self {
    self.operation(FilterFunction::Brightness(amount))
  }

  /// Appends one painted drop shadow.
  #[must_use]
  pub fn drop_shadow(self, shadow: Shadow) -> Self {
    self.operation(FilterFunction::DropShadow(shadow))
  }

  /// Appends another ordered filter list.
  #[must_use]
  pub fn then(mut self, value: Self) -> Self {
    self.0.extend(value.0);
    self
  }
}

impl IntoIterator for FilterList {
  type Item = FilterFunction;
  type IntoIter = std::vec::IntoIter<FilterFunction>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl FromIterator<FilterFunction> for FilterList {
  fn from_iter<T: IntoIterator<Item = FilterFunction>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}
