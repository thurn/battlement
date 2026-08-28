//! Runtime lifecycle, roots, sessions, and commits.

use std::{
  error::Error,
  fmt,
  hash::Hash,
  marker::PhantomData,
  panic::{self, AssertUnwindSafe},
  sync::atomic::{AtomicU64, Ordering},
  thread,
};

use battlement::{
  GeometryObservationBatch, Response, Snapshot, UiDocument, UiEvent, UiNode, Validate,
};

use crate::{executor::Spawner, render, render::Render};

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// Identifies one registered document root.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Root {
  runtime_id: u64,
  index: usize,
}

/// A render failure that escaped every boundary.
#[derive(Debug)]
pub struct RenderError {
  message: String,
}

/// An ordered native mutation commit.
#[must_use]
pub struct ReactantCommit {
  _private: (),
}

/// A prospective complete UI state for one session snapshot.
#[must_use]
pub struct SessionUi<'a> {
  runtime: &'a mut dyn SessionRuntime,
  documents: Vec<UiDocument>,
  committed: Vec<Vec<UiNode>>,
  consumed: bool,
}

/// Owns the declarative UI state for one game model.
pub struct Reactant<G: 'static> {
  runtime_id: u64,
  _spawner: Box<dyn Spawner>,
  roots: Vec<RootRegistration<G>>,
  state: RuntimeState,
}

impl fmt::Display for RenderError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl Error for RenderError {}

impl ReactantCommit {
  /// Returns whether this commit carries no native work.
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    true
  }

  const fn empty() -> Self {
    Self { _private: () }
  }
}

impl<G: 'static> Reactant<G> {
  /// Creates a registering runtime with an idle executor.
  #[must_use]
  pub fn new(spawner: impl Spawner) -> Self {
    Self {
      runtime_id: NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed),
      _spawner: Box::new(spawner),
      roots: Vec::new(),
      state: RuntimeState::Registering,
    }
  }

  /// Registers one childless document and its root view factory.
  pub fn register_root<V, R>(&mut self, document: UiDocument, view: V) -> Root
  where
    V: Fn(&G) -> R + 'static,
    R: Render,
  {
    self.require_registering();
    assert!(
      document.children.is_empty(),
      "Reactant roots must be childless"
    );
    assert!(
      document.document_id != document.root_id,
      "a document and its root must have distinct IDs"
    );
    assert!(
      !self.roots.iter().any(|root| root.collides(&document)),
      "Reactant root IDs must be unique"
    );
    let root = Root {
      runtime_id: self.runtime_id,
      index: self.roots.len(),
    };
    self.roots.push(RootRegistration {
      document,
      view: Box::new(ViewAdapter {
        view,
        _types: PhantomData,
      }),
      committed: Vec::new(),
    });
    root
  }

  /// Begins a transactional initial or reconnect render.
  pub fn begin_session<'a>(&'a mut self, game: &mut G) -> Result<SessionUi<'a>, RenderError> {
    self.require_open();
    let rendered = panic::catch_unwind(AssertUnwindSafe(|| {
      self
        .roots
        .iter()
        .map(|root| root.view.render(game, &root.committed))
        .collect::<Vec<_>>()
    }));
    let committed = match rendered {
      Ok(value) => value,
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    let documents = self
      .roots
      .iter()
      .zip(&committed)
      .map(|(root, children)| {
        let mut document = root.document.clone();
        document.children.clone_from(children);
        document
      })
      .collect();
    Ok(SessionUi {
      runtime: self,
      documents,
      committed,
      consumed: false,
    })
  }

  /// Dispatches one native UI event while active.
  pub fn dispatch(
    &mut self,
    _game: &mut G,
    _event: UiEvent,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    Ok(ReactantCommit::empty())
  }

  /// Installs one complete geometry generation while active.
  pub fn observe_geometry(
    &mut self,
    _game: &mut G,
    _batch: GeometryObservationBatch,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    Ok(ReactantCommit::empty())
  }

  /// Renders all roots after application state changed.
  pub fn refresh(&mut self, _game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    Ok(ReactantCommit::empty())
  }

  /// Processes queued runtime work while active.
  pub fn poll(&mut self, _game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    Ok(ReactantCommit::empty())
  }

  /// Closes the runtime and returns its final native work.
  pub fn shutdown(&mut self, _game: &mut G) -> ReactantCommit {
    assert!(
      self.state != RuntimeState::Poisoned,
      "Reactant runtime is poisoned"
    );
    self.state = RuntimeState::Closed;
    ReactantCommit::empty()
  }

  fn require_registering(&self) {
    assert!(
      self.state == RuntimeState::Registering,
      "Reactant registration is closed"
    );
  }

  fn require_open(&self) {
    assert!(
      self.state != RuntimeState::Closed,
      "Reactant runtime is closed"
    );
    assert!(
      self.state != RuntimeState::Poisoned,
      "Reactant runtime is poisoned"
    );
  }

  fn require_active(&self) {
    assert!(
      self.state == RuntimeState::Active,
      "Reactant runtime is not active"
    );
  }
}

impl SessionUi<'_> {
  /// Adds this session UI to a snapshot and returns its complete response.
  pub fn into_response(self, snapshot: Snapshot) -> Response {
    let (snapshot, commit) = self.into_parts(snapshot);
    debug_assert!(commit.is_empty());
    Response::snapshot(snapshot)
  }

  /// Adds this session UI to a snapshot and returns the minimal commit path.
  pub fn into_parts(mut self, mut snapshot: Snapshot) -> (Snapshot, ReactantCommit) {
    snapshot.ui.extend(self.documents.clone());
    if let Err(error) = snapshot.validate() {
      panic!("Reactant session snapshot is invalid: {error}");
    }
    self.runtime.commit_session(&self.committed);
    self.consumed = true;
    (snapshot, ReactantCommit::empty())
  }
}

impl Drop for SessionUi<'_> {
  fn drop(&mut self) {
    if self.consumed {
      return;
    }
    self.runtime.poison();
    if !thread::panicking() {
      panic!("a Reactant session must be converted before it is dropped");
    }
  }
}

struct RootRegistration<G> {
  document: UiDocument,
  view: Box<dyn RootView<G>>,
  committed: Vec<UiNode>,
}

impl<G> RootRegistration<G> {
  fn collides(&self, document: &UiDocument) -> bool {
    let ids = [self.document.document_id, self.document.root_id];
    ids.contains(&document.document_id) || ids.contains(&document.root_id)
  }
}

trait RootView<G> {
  fn render(&self, game: &G, committed: &[UiNode]) -> Vec<UiNode>;
}

trait SessionRuntime {
  fn commit_session(&mut self, committed: &[Vec<UiNode>]);
  fn poison(&mut self);
}

impl<G: 'static> SessionRuntime for Reactant<G> {
  fn commit_session(&mut self, committed: &[Vec<UiNode>]) {
    for (root, rendered) in self.roots.iter_mut().zip(committed) {
      root.committed.clone_from(rendered);
    }
    self.state = RuntimeState::Active;
  }

  fn poison(&mut self) {
    self.state = RuntimeState::Poisoned;
  }
}

struct ViewAdapter<G, V, R> {
  view: V,
  _types: PhantomData<fn(&G) -> R>,
}

impl<G, V, R> RootView<G> for ViewAdapter<G, V, R>
where
  V: Fn(&G) -> R,
  R: Render,
{
  fn render(&self, game: &G, committed: &[UiNode]) -> Vec<UiNode> {
    render::lower((self.view)(game), committed)
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RuntimeState {
  Registering,
  Active,
  Closed,
  Poisoned,
}
