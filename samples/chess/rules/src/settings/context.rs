//! Application-lifetime preferences with ordered persistence and session fallback.

use std::rc::Rc;
use std::sync::Arc;

use reactant::{
  PersistenceBackend,
  hooks::{self, Callback},
  prelude::*,
};

use crate::settings::{ChessSettings, FILE_NAME, SettingsChange};

/// Current preferences and stable actions, independent of menu lifetime.
#[derive(Clone, PartialEq)]
pub struct SettingsContext {
  pub desired: ChessSettings,
  pub durable: Option<ChessSettings>,
  pub pending: bool,
  pub failed: bool,
  dispatch: Callback<Rc<dyn Fn(SettingsChange)>>,
  retry: Callback<Rc<dyn Fn()>>,
}

/// Hydrates preferences before mounting consumers or startup effects.
pub struct SettingsRoot {
  pub backend: Option<Arc<dyn PersistenceBackend>>,
  pub children: Children,
}

pub fn use_settings() -> SettingsContext {
  hooks::use_required_context::<SettingsContext>()
}

struct PersistentSettings {
  backend: Arc<dyn PersistenceBackend>,
  children: Children,
}

struct SessionSettings {
  children: Children,
}

impl SettingsContext {
  pub fn change(&self, change: SettingsChange) {
    (self.dispatch)(change);
  }

  pub fn retry(&self) {
    (self.retry)();
  }

  /// Converts a controlled input into a root-owned preference update.
  pub fn callback<T: 'static>(&self, change: fn(T) -> SettingsChange) -> EventCallback<T> {
    let dispatch = self.dispatch.clone();
    EventCallback::new(move |value| dispatch(change(value)))
  }
}

impl Component for SettingsRoot {
  fn render(&self) -> impl Render {
    match &self.backend {
      Some(backend) => Either::left(PersistentSettings {
        backend: backend.clone(),
        children: self.children.clone(),
      }),
      None => Either::right(SessionSettings {
        children: self.children.clone(),
      }),
    }
  }
}

impl Component for PersistentSettings {
  fn render(&self) -> impl Render {
    let saved =
      reactant::use_persistent_state_with::<ChessSettings>(FILE_NAME, self.backend.clone());
    let store = saved.clone();
    let dispatch: Rc<dyn Fn(SettingsChange)> = Rc::new(move |change| {
      store.update_with(|current| change.apply(current.unwrap_or_default()));
    });
    let dispatch = hooks::use_callback(dispatch, ());
    let store = saved.clone();
    let retry: Rc<dyn Fn()> = Rc::new(move || store.retry());
    let retry = hooks::use_callback(retry, ());
    if !saved.hydrated() && saved.error().is_none() {
      return Either::left(());
    }
    Either::right(
      ContextProvider::new()
        .context(SettingsContext {
          desired: saved.desired().copied().unwrap_or_default(),
          durable: saved.value().copied(),
          pending: saved.pending().is_some(),
          failed: saved.error().is_some(),
          dispatch,
          retry,
        })
        .child(self.children.render()),
    )
  }
}

impl Component for SessionSettings {
  fn render(&self) -> impl Render {
    let (desired, setter) = hooks::use_state(ChessSettings::default());
    let dispatch: Rc<dyn Fn(SettingsChange)> =
      Rc::new(move |change| setter.update(move |value| change.apply(value)));
    let retry: Rc<dyn Fn()> = Rc::new(|| {});
    ContextProvider::new()
      .context(SettingsContext {
        desired,
        durable: None,
        pending: false,
        failed: false,
        dispatch: hooks::use_callback(dispatch, ()),
        retry: hooks::use_callback(retry, ()),
      })
      .child(self.children.render())
  }
}
