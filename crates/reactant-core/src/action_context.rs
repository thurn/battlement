use std::cell::Cell;

use battlement::ActionId;

thread_local! {
  static CURRENT: Cell<Option<ActionId>> = const { Cell::new(None) };
}

pub(crate) struct ActionScope(Option<ActionId>);

pub(crate) fn current() -> Option<ActionId> {
  CURRENT.with(Cell::get)
}

pub(crate) fn enter(action: Option<ActionId>) -> ActionScope {
  ActionScope(CURRENT.with(|current| current.replace(action)))
}

impl Drop for ActionScope {
  fn drop(&mut self) {
    CURRENT.with(|current| current.set(self.0));
  }
}
