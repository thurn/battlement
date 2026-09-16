//! Recoverable render error ownership.

use std::{error::Error, fmt, rc::Rc, sync::Arc};

pub(crate) trait SharedRenderError {
  fn error(&self) -> &(dyn Error + 'static);
}

/// A render failure that escaped every boundary.
#[derive(Clone)]
pub struct RenderError {
  owner: ErrorOwner,
}

impl RenderError {
  /// Erases one concrete recoverable render error.
  pub fn new<E: Error + 'static>(error: E) -> Self {
    Self::from_boxed(Box::new(error))
  }

  /// Takes ownership of one already erased recoverable error.
  pub fn from_boxed(error: Box<dyn Error + 'static>) -> Self {
    if error.is::<Self>() {
      return *error
        .downcast::<Self>()
        .expect("checked boxed RenderError type");
    }
    Self {
      owner: ErrorOwner::Local(Rc::from(error)),
    }
  }

  /// Takes ownership of one thread-safe erased recoverable error.
  pub fn from_boxed_send_sync(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
    Self {
      owner: ErrorOwner::Shared(Arc::from(error)),
    }
  }

  /// Creates a recoverable error from owned display text.
  pub fn message(message: impl Into<String>) -> Self {
    Self::new(std::io::Error::other(message.into()))
  }

  /// Borrows the original concrete error when its type matches `E`.
  pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
    self.error().downcast_ref()
  }

  pub(crate) fn from_shared_render(error: Rc<dyn SharedRenderError>) -> Self {
    Self {
      owner: ErrorOwner::Render(error),
    }
  }

  pub(crate) fn from_shared_resource<E>(error: Arc<E>) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    Self {
      owner: ErrorOwner::Shared(error),
    }
  }

  fn error(&self) -> &(dyn Error + 'static) {
    let error = match &self.owner {
      ErrorOwner::Local(error) => error.as_ref(),
      ErrorOwner::Render(error) => error.error(),
      ErrorOwner::Shared(error) => error.as_ref(),
    };
    match error.downcast_ref::<Self>() {
      Some(error) => error.error(),
      None => error,
    }
  }
}

impl fmt::Display for RenderError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self.error(), formatter)
  }
}

impl fmt::Debug for RenderError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.error(), formatter)
  }
}

impl Error for RenderError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    self.error().source()
  }
}

#[derive(Clone)]
enum ErrorOwner {
  Local(Rc<dyn Error>),
  Render(Rc<dyn SharedRenderError>),
  Shared(Arc<dyn Error + Send + Sync>),
}
