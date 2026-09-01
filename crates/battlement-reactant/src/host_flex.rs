use battlement::{Align, FlexDirection, FlexWrap, Justify, Prop};

use crate::host::Flex;

impl Flex {
  /// Sets the main-axis direction.
  #[must_use]
  pub fn direction(mut self, value: FlexDirection) -> Self {
    self.state.host.direction = Prop::Set(value);
    self
  }

  /// Sets the line wrapping policy.
  #[must_use]
  pub fn wrap(mut self, value: FlexWrap) -> Self {
    self.state.host.wrap = Prop::Set(value);
    self
  }

  /// Sets the default cross-axis child alignment.
  #[must_use]
  pub fn align_items(mut self, value: Align) -> Self {
    self.state.host.align_items = Prop::Set(value);
    self
  }

  /// Sets the main-axis distribution.
  #[must_use]
  pub fn justify_content(mut self, value: Justify) -> Self {
    self.state.host.justify_content = Prop::Set(value);
    self
  }

  /// Sets the gap between wrapped rows.
  #[must_use]
  pub fn row_gap(mut self, value: f32) -> Self {
    self.state.host.row_gap = Prop::Set(value);
    self
  }

  /// Sets the gap between columns.
  #[must_use]
  pub fn column_gap(mut self, value: f32) -> Self {
    self.state.host.column_gap = Prop::Set(value);
    self
  }

  /// Sets both gaps that have not already been specified.
  #[must_use]
  pub fn gap(mut self, value: f32) -> Self {
    if matches!(self.state.host.row_gap, Prop::Unset) {
      self.state.host.row_gap = Prop::Set(value);
    }
    if matches!(self.state.host.column_gap, Prop::Unset) {
      self.state.host.column_gap = Prop::Set(value);
    }
    self
  }
}
