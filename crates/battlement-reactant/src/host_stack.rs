use battlement::{Align, Prop};

use crate::host::Stack;

impl Stack {
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
