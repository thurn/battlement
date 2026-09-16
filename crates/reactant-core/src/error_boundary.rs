//! Recoverable render errors and their nearest fallback boundary.

#![allow(private_interfaces)]

use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  rc::Rc,
};

use crate::{
  context,
  hooks::Dependencies,
  props::Missing,
  render::{Render, RenderSink},
  render_value::Sealed,
  runtime::RenderError,
};

/// Omits dependency-based error-boundary resets.
#[doc(hidden)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NoReset;

/// Omits committed error reporting.
#[doc(hidden)]
pub struct NoErrorHandler;

/// Selects a fallback when its primary subtree returns an explicit error.
pub struct ErrorBoundary<F, C = Missing, D = NoReset, O = NoErrorHandler> {
  fallback: F,
  child: C,
  reset: Option<D>,
  handler: Option<ErrorHandler>,
  _handler: PhantomData<O>,
}

type ErrorCallback = Rc<dyn Fn(&mut dyn Any, &RenderError)>;

#[derive(Clone)]
pub(crate) struct BoundaryState {
  pub(crate) error: Option<RenderError>,
  pub(crate) reset: Option<ErasedDependencies>,
  pub(crate) report: Option<ErrorReport>,
}

#[derive(Clone)]
pub(crate) struct ErrorReport {
  model: TypeId,
  error: RenderError,
  callback: ErrorCallback,
}

#[derive(Clone)]
pub(crate) struct ErrorHandler {
  model: TypeId,
  callback: ErrorCallback,
}

#[derive(Clone)]
pub(crate) struct ErasedDependencies {
  type_id: TypeId,
  value: Rc<dyn DependencyValue>,
}

impl<F> ErrorBoundary<F> {
  /// Creates an incomplete boundary with a fallback factory.
  pub fn new(fallback: F) -> Self {
    Self {
      fallback,
      child: Missing,
      reset: None,
      handler: None,
      _handler: PhantomData,
    }
  }
}

impl<F, C, D, O> ErrorBoundary<F, C, D, O> {
  /// Retries the primary whenever this dependency value changes.
  pub fn reset_on<N: Dependencies>(self, value: N) -> ErrorBoundary<F, C, N, O> {
    ErrorBoundary {
      fallback: self.fallback,
      child: self.child,
      reset: Some(value),
      handler: self.handler,
      _handler: PhantomData,
    }
  }

  /// Reports each newly committed error against the game model.
  pub fn on_error<G: 'static, N>(self, callback: N) -> ErrorBoundary<F, C, D, N>
  where
    N: Fn(&mut G, &RenderError) + 'static,
  {
    ErrorBoundary {
      fallback: self.fallback,
      child: self.child,
      reset: self.reset,
      handler: Some(ErrorHandler::new(callback)),
      _handler: PhantomData,
    }
  }
}

impl<F, D, O> ErrorBoundary<F, Missing, D, O> {
  /// Supplies the primary subtree caught by this boundary.
  pub fn child<R: Render>(self, child: R) -> ErrorBoundary<F, R, D, O> {
    ErrorBoundary {
      fallback: self.fallback,
      child,
      reset: self.reset,
      handler: self.handler,
      _handler: PhantomData,
    }
  }
}

impl<F, C, D, O, R> Render for ErrorBoundary<F, C, D, O>
where
  F: Fn(&RenderError) -> R + 'static,
  C: Render,
  D: Dependencies,
  O: 'static,
  R: Render,
{
}

impl<F, C, D, O, R> Sealed for ErrorBoundary<F, C, D, O>
where
  F: Fn(&RenderError) -> R + 'static,
  C: Render,
  D: Dependencies,
  O: 'static,
  R: Render,
{
  fn descriptor(&self) -> TypeId {
    TypeId::of::<BoundaryMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_error_boundary(
      self.reset.as_ref().map(ErasedDependencies::new),
      self.handler.clone(),
      |children| self.child.render_into(children),
      |error, children| {
        context::with_hooks_forbidden(|| (self.fallback)(error)).render_owned(children);
      },
    );
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    let Self {
      fallback,
      child,
      reset,
      handler,
      _handler: _,
    } = self;
    sink.push_error_boundary(
      reset.as_ref().map(ErasedDependencies::new),
      handler,
      |children| child.render_owned(children),
      |error, children| {
        context::with_hooks_forbidden(|| fallback(error)).render_owned(children);
      },
    );
  }
}

impl ErrorHandler {
  fn new<G: 'static>(callback: impl Fn(&mut G, &RenderError) + 'static) -> Self {
    Self {
      model: TypeId::of::<G>(),
      callback: Rc::new(move |game, error| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("validated Reactant error handler model"),
          error,
        );
      }),
    }
  }

  pub(crate) fn report(&self, error: RenderError) -> ErrorReport {
    ErrorReport {
      model: self.model,
      error,
      callback: Rc::clone(&self.callback),
    }
  }
}

impl ErrorReport {
  pub(crate) fn model(&self) -> TypeId {
    self.model
  }

  pub(crate) fn run<G: 'static>(self, game: &mut G) {
    (self.callback)(game, &self.error);
  }
}

impl ErasedDependencies {
  fn new<D: Dependencies>(value: &D) -> Self {
    Self {
      type_id: TypeId::of::<D>(),
      value: Rc::new(value.clone()),
    }
  }
}

impl PartialEq for ErasedDependencies {
  fn eq(&self, other: &Self) -> bool {
    self.type_id == other.type_id && self.value.equals(other.value.as_ref())
  }
}

impl Eq for ErasedDependencies {}

trait DependencyValue: Any {
  fn as_any(&self) -> &dyn Any;
  fn equals(&self, other: &dyn DependencyValue) -> bool;
}

impl<D: PartialEq + 'static> DependencyValue for D {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn equals(&self, other: &dyn DependencyValue) -> bool {
    other.as_any().downcast_ref::<D>() == Some(self)
  }
}

pub(crate) struct BoundaryMarker;
