//! Registered root factories and their render adapters.

use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use battlement::UiDocument;

use crate::{
  context,
  render::{Render, RenderTree},
  render_value,
  resource_cache::ResourceOverlay,
  resource_runtime::{self, ResourceRuntime},
  runtime::{RenderError, Root},
};

pub(crate) struct RootRegistration<G> {
  pub(crate) document: UiDocument,
  pub(crate) view: Box<dyn RootView<G>>,
  pub(crate) committed: RenderTree,
}

pub(crate) trait RootView<G> {
  fn render(
    &self,
    game: &G,
    committed: &RenderTree,
    defaults: Rc<RefCell<context::ContextDefaults>>,
    resources: Rc<ResourceRuntime>,
    resource_overlay: Option<Rc<ResourceOverlay>>,
  ) -> Result<RenderTree, RenderError>;
}

pub(crate) struct ViewAdapter<G, V, R> {
  view: V,
  _types: PhantomData<fn(&G) -> R>,
}

impl Root {
  pub(crate) const fn new(runtime_id: u64, index: usize) -> Self {
    Self { runtime_id, index }
  }
}

impl<G: 'static> RootRegistration<G> {
  pub(crate) fn new<V, R>(document: UiDocument, view: V) -> Self
  where
    V: Fn(&G) -> R + 'static,
    R: Render,
  {
    Self {
      document,
      view: Box::new(ViewAdapter {
        view,
        _types: PhantomData,
      }),
      committed: RenderTree::default(),
    }
  }

  pub(crate) fn collides(&self, document: &UiDocument) -> bool {
    let ids = [self.document.document_id, self.document.root_id];
    ids.contains(&document.document_id) || ids.contains(&document.root_id)
  }
}

impl<G, V, R> RootView<G> for ViewAdapter<G, V, R>
where
  V: Fn(&G) -> R,
  R: Render,
{
  fn render(
    &self,
    game: &G,
    committed: &RenderTree,
    defaults: Rc<RefCell<context::ContextDefaults>>,
    resources: Rc<ResourceRuntime>,
    resource_overlay: Option<Rc<ResourceOverlay>>,
  ) -> Result<RenderTree, RenderError> {
    resource_runtime::with_runtime(resources, resource_overlay, || {
      context::with_runtime(defaults, || {
        render_value::lower(
          context::with_hooks_forbidden(|| (self.view)(game)),
          committed,
        )
      })
    })
  }
}
