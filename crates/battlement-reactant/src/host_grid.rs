use battlement::{Align, GridAutoFlow, GridTrack, Prop};

use crate::host::Grid;

impl Grid {
  /// Replaces the explicit column tracks.
  #[must_use]
  pub fn columns(mut self, value: impl IntoIterator<Item = GridTrack>) -> Self {
    self.state.host.columns = Prop::Set(value.into_iter().collect());
    self
  }

  /// Replaces the explicit row tracks.
  #[must_use]
  pub fn rows(mut self, value: impl IntoIterator<Item = GridTrack>) -> Self {
    self.state.host.rows = Prop::Set(value.into_iter().collect());
    self
  }

  /// Sets the size of implicit columns.
  #[must_use]
  pub fn auto_columns(mut self, value: GridTrack) -> Self {
    self.state.host.auto_columns = Prop::Set(value);
    self
  }

  /// Sets the size of implicit rows.
  #[must_use]
  pub fn auto_rows(mut self, value: GridTrack) -> Self {
    self.state.host.auto_rows = Prop::Set(value);
    self
  }

  /// Selects the major-axis auto-placement scan direction.
  #[must_use]
  pub fn auto_flow(mut self, value: GridAutoFlow) -> Self {
    self.state.host.auto_flow = Prop::Set(value);
    self
  }

  /// Sets the gap between rows.
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

  /// Sets the default vertical item alignment.
  #[must_use]
  pub fn align_items(mut self, value: Align) -> Self {
    self.state.host.align_items = Prop::Set(value);
    self
  }

  /// Sets the default horizontal item alignment.
  #[must_use]
  pub fn justify_items(mut self, value: Align) -> Self {
    self.state.host.justify_items = Prop::Set(value);
    self
  }
}
