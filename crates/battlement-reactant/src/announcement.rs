//! One-shot accessibility announcements.

use std::cell::RefCell;

use crate::semantics::LocalizedText;

thread_local! {
  static PENDING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Imperative handle for the current runtime transaction's announcement queue.
#[derive(Clone, Copy, Debug, Default)]
pub struct Announce;

/// Returns an imperative one-shot announcement handle.
#[must_use]
pub const fn use_announce() -> Announce {
  Announce
}

impl Announce {
  /// Queues nonempty localized text for the current successful commit.
  pub fn send(self, value: LocalizedText) {
    let value = value.resolved();
    if !value.is_empty() {
      PENDING.with(|pending| pending.borrow_mut().push(value));
    }
  }
}

pub(crate) fn take() -> Vec<String> {
  PENDING.with(|pending| pending.take())
}
