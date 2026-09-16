use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::Prop;
use trox::{LocalizedString, Localizer, SourceLocale};

thread_local! {
  static CURRENT: RefCell<Option<Rc<Localizer>>> = const { RefCell::new(None) };
  static ANNOUNCEMENT_CURRENT: RefCell<Option<Rc<Localizer>>> = const { RefCell::new(None) };
  static FORCE_MEMO_RENDERING: Cell<bool> = const { Cell::new(false) };
}

pub(crate) struct Guard {
  previous: Option<Rc<Localizer>>,
  previous_announcement: Option<Rc<Localizer>>,
}

pub(crate) struct MemoRenderingGuard(bool);

impl Drop for Guard {
  fn drop(&mut self) {
    CURRENT.with(|current| current.replace(self.previous.take()));
    ANNOUNCEMENT_CURRENT.with(|current| current.replace(self.previous_announcement.take()));
  }
}

impl Drop for MemoRenderingGuard {
  fn drop(&mut self) {
    FORCE_MEMO_RENDERING.with(|forced| forced.set(self.0));
  }
}

pub(crate) fn source_localizer(source: SourceLocale) -> Rc<Localizer> {
  Rc::new(Localizer::for_source(source).expect("valid Reactant source locale"))
}

pub(crate) fn enter(localizer: Rc<Localizer>) -> Guard {
  Guard {
    previous: CURRENT.with(|current| current.replace(Some(Rc::clone(&localizer)))),
    previous_announcement: ANNOUNCEMENT_CURRENT.with(|current| current.replace(Some(localizer))),
  }
}

pub(crate) fn replace_announcement_localizer(localizer: Rc<Localizer>) {
  ANNOUNCEMENT_CURRENT.with(|current| {
    if current.borrow().is_some() {
      *current.borrow_mut() = Some(localizer);
    }
  });
}

pub(crate) fn force_memo_rendering() -> MemoRenderingGuard {
  MemoRenderingGuard(FORCE_MEMO_RENDERING.with(|forced| forced.replace(true)))
}

pub(crate) fn memo_rendering_is_forced() -> bool {
  FORCE_MEMO_RENDERING.with(Cell::get)
}

pub(crate) fn resolve(value: &LocalizedString) -> String {
  CURRENT.with(|current| {
    current
      .borrow()
      .as_ref()
      .expect("localized content resolved outside Reactant rendering")
      .resolve(value)
  })
}

pub(crate) fn resolve_announcement(value: &LocalizedString) -> String {
  ANNOUNCEMENT_CURRENT.with(|current| {
    current
      .borrow()
      .as_ref()
      .expect("localized announcement resolved outside a Reactant commit")
      .resolve(value)
  })
}

pub(crate) fn resolve_prop(value: &Prop<LocalizedString>) -> Prop<String> {
  match value {
    Prop::Unset => Prop::Unset,
    Prop::Set(value) => Prop::Set(resolve(value)),
    Prop::Reset => Prop::Reset,
  }
}

pub(crate) fn resolve_values(values: &[LocalizedString]) -> Vec<String> {
  values.iter().map(resolve).collect()
}

pub(crate) fn resolve_values_prop(value: &Prop<Vec<LocalizedString>>) -> Prop<Vec<String>> {
  match value {
    Prop::Unset => Prop::Unset,
    Prop::Set(values) => Prop::Set(resolve_values(values)),
    Prop::Reset => Prop::Reset,
  }
}
