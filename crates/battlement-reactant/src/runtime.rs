//! Runtime lifecycle, roots, sessions, and commits.

use std::{
  any::TypeId,
  cell::Cell,
  error::Error,
  fmt,
  hash::Hash,
  marker::PhantomData,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  sync::atomic::{AtomicU64, Ordering},
  thread,
};

use battlement::{
  self, ActionId, Batch, BatchId, Command, CommandBody, GeometryObservationBatch,
  ParallelCommandGroup, Prop, Response, ResponseMessage, SessionId, Snapshot, UiDocument, UiEvent,
  Validate, VisualElement,
};

use crate::{
  context,
  event::{ElementTarget, EventPhase},
  event_handler::HandlerPhase,
  executor::Spawner,
  reconcile,
  render::{self, Render, RenderTree},
};

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// Adds an ordered Reactant commit to a Battlement response.
pub trait ResponseReactantExt: Sized {
  /// Appends the commit as one batch when it is nonempty.
  fn append_reactant(self, commit: ReactantCommit) -> Self;

  /// Appends the commit as one action-caused batch when it is nonempty.
  fn append_reactant_for_action(self, action_id: ActionId, commit: ReactantCommit) -> Self;
}

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
  groups: Option<Vec<Vec<CommandBody>>>,
  receipt: Option<DeliveryReceipt>,
}

/// A prospective complete UI state for one session snapshot.
#[must_use]
pub struct SessionUi<'a> {
  runtime: &'a mut dyn SessionRuntime,
  documents: Vec<UiDocument>,
  committed: Vec<RenderTree>,
  consumed: bool,
}

/// Owns the declarative UI state for one game model.
pub struct Reactant<G: 'static> {
  runtime_id: u64,
  _spawner: Box<dyn Spawner>,
  roots: Vec<RootRegistration<G>>,
  state: RuntimeState,
  outstanding: Option<DeliveryReceipt>,
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
  pub fn is_empty(&self) -> bool {
    self
      .groups
      .as_ref()
      .expect("Reactant commit was already consumed")
      .is_empty()
  }

  /// Consumes this commit into its ordered parallel command-body groups.
  #[must_use]
  pub fn into_groups(mut self) -> Vec<Vec<CommandBody>> {
    let groups = self.take_groups();
    self.acknowledge();
    groups
  }

  /// Consumes this commit into one Battlement batch, or no batch when empty.
  #[must_use]
  pub fn into_batch(mut self, session_id: SessionId) -> Option<Batch> {
    let groups = self.take_groups();
    let batch = (!groups.is_empty()).then(|| {
      Batch::new(
        BatchId::new_v4(),
        session_id,
        groups
          .into_iter()
          .map(ParallelCommandGroup::from_bodies)
          .collect(),
      )
    });
    self.acknowledge();
    batch
  }

  fn empty() -> Self {
    Self {
      groups: Some(Vec::new()),
      receipt: None,
    }
  }

  fn new(groups: Vec<Vec<CommandBody>>, receipt: DeliveryReceipt) -> Self {
    Self {
      groups: Some(groups),
      receipt: Some(receipt),
    }
  }

  fn acknowledge(&mut self) {
    if let Some(receipt) = self.receipt.take() {
      receipt.acknowledge();
    }
  }

  fn take_groups(&mut self) -> Vec<Vec<CommandBody>> {
    self
      .groups
      .take()
      .expect("Reactant commit was already consumed")
  }
}

impl Drop for ReactantCommit {
  fn drop(&mut self) {
    let Some(receipt) = self.receipt.take() else {
      return;
    };
    if receipt.state() != ReceiptState::Pending {
      return;
    }
    receipt.poison();
    if !thread::panicking() {
      panic!("a nonempty Reactant commit was dropped without delivery");
    }
  }
}

impl<C> ResponseReactantExt for Response<C>
where
  C: From<Command>,
{
  fn append_reactant(self, commit: ReactantCommit) -> Self {
    self::append_commit(self, None, commit)
  }

  fn append_reactant_for_action(self, action_id: ActionId, commit: ReactantCommit) -> Self {
    self::append_commit(self, Some(action_id), commit)
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
      outstanding: None,
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
    self::validate_document_subscriptions(&document);
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
      committed: RenderTree::default(),
    });
    root
  }

  /// Begins a transactional initial or reconnect render.
  pub fn begin_session<'a>(&'a mut self, game: &mut G) -> Result<SessionUi<'a>, RenderError> {
    self.require_open();
    let rendered = panic::catch_unwind(AssertUnwindSafe(|| {
      let rendered = self
        .roots
        .iter()
        .map(|root| root.view.render(game, &root.committed))
        .collect::<Vec<_>>();
      for tree in &rendered {
        tree.validate_model(TypeId::of::<G>());
      }
      rendered
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
      .map(|(root, rendered)| self::render_document(root, rendered))
      .collect();
    Ok(SessionUi {
      runtime: self,
      documents,
      committed,
      consumed: false,
    })
  }

  /// Dispatches one native UI event while active.
  pub fn dispatch(&mut self, game: &mut G, event: UiEvent) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let handlers = self.roots.iter().enumerate().find_map(|(index, root)| {
      let mut handlers = root.committed.handlers(event.target_id, event.kind());
      (!handlers.is_empty()).then(|| {
        handlers.sort_by_key(|handler| match handler.phase() {
          HandlerPhase::Capture => 0,
          HandlerPhase::Default => 1,
        });
        (
          handlers,
          ElementTarget::new(
            Root {
              runtime_id: self.runtime_id,
              index,
            },
            event.target_id,
          ),
        )
      })
    });
    let Some((handlers, target)) = handlers else {
      return Ok(ReactantCommit::empty());
    };
    let invoked = panic::catch_unwind(AssertUnwindSafe(|| {
      for handler in handlers {
        handler.invoke(game, target, EventPhase::Target, event.body.clone());
      }
    }));
    if let Err(payload) = invoked {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
    self.refresh(game)
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
  pub fn refresh(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let planned = panic::catch_unwind(AssertUnwindSafe(|| {
      let rendered = self
        .roots
        .iter()
        .map(|root| root.view.render(game, &root.committed))
        .collect::<Vec<_>>();
      for tree in &rendered {
        tree.validate_model(TypeId::of::<G>());
      }
      let documents = self
        .roots
        .iter()
        .zip(&rendered)
        .map(|(root, tree)| self::render_document(root, tree))
        .collect::<Vec<_>>();
      battlement::validate_documents(&documents)
        .expect("Reactant rendered an invalid UI hierarchy");
      let groups = self
        .roots
        .iter()
        .zip(&rendered)
        .map(|(root, tree)| {
          let groups = reconcile::command_groups(
            root.document.root_id,
            &root.committed.hosts(),
            &tree.hosts(),
          );
          self::with_coverage_barrier(root, tree, groups)
        })
        .fold(Vec::new(), self::merge_groups);
      (rendered, groups)
    }));
    let (rendered, groups) = match planned {
      Ok(value) => value,
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    self.commit_session(&rendered);
    Ok(self.create_commit(groups))
  }

  /// Processes queued runtime work while active.
  pub fn poll(&mut self, _game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    Ok(ReactantCommit::empty())
  }

  /// Closes the runtime and returns its final native work.
  pub fn shutdown(&mut self, _game: &mut G) -> ReactantCommit {
    self.require_delivery();
    assert!(
      self.state != RuntimeState::Poisoned,
      "Reactant runtime is poisoned"
    );
    self.state = RuntimeState::Closed;
    ReactantCommit::empty()
  }

  fn require_registering(&mut self) {
    self.require_delivery();
    assert!(
      self.state == RuntimeState::Registering,
      "Reactant registration is closed"
    );
  }

  fn require_open(&mut self) {
    self.require_delivery();
    assert!(
      self.state != RuntimeState::Closed,
      "Reactant runtime is closed"
    );
    assert!(
      self.state != RuntimeState::Poisoned,
      "Reactant runtime is poisoned"
    );
  }

  fn require_active(&mut self) {
    self.require_delivery();
    assert!(
      self.state == RuntimeState::Active,
      "Reactant runtime is not active"
    );
  }

  fn require_delivery(&mut self) {
    let Some(state) = self.outstanding.as_ref().map(DeliveryReceipt::state) else {
      return;
    };
    match state {
      ReceiptState::Acknowledged => self.outstanding = None,
      ReceiptState::Pending => {
        self
          .outstanding
          .as_ref()
          .expect("outstanding receipt exists")
          .poison();
        self.state = RuntimeState::Poisoned;
        panic!("Reactant cannot reenter while a commit delivery receipt is outstanding");
      }
      ReceiptState::Poisoned => {
        self.state = RuntimeState::Poisoned;
        panic!("Reactant runtime is poisoned by an undelivered commit");
      }
    }
  }

  fn create_commit(&mut self, groups: Vec<Vec<CommandBody>>) -> ReactantCommit {
    if groups.is_empty() {
      return ReactantCommit::empty();
    }
    let receipt = DeliveryReceipt::new();
    self.outstanding = Some(receipt.clone());
    ReactantCommit::new(groups, receipt)
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

#[derive(Clone)]
struct DeliveryReceipt {
  state: Rc<Cell<ReceiptState>>,
}

impl DeliveryReceipt {
  fn new() -> Self {
    Self {
      state: Rc::new(Cell::new(ReceiptState::Pending)),
    }
  }

  fn acknowledge(&self) {
    assert!(
      self.state() == ReceiptState::Pending,
      "Reactant commit delivery receipt is no longer valid"
    );
    self.state.set(ReceiptState::Acknowledged);
  }

  fn poison(&self) {
    self.state.set(ReceiptState::Poisoned);
  }

  fn state(&self) -> ReceiptState {
    self.state.get()
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ReceiptState {
  Pending,
  Acknowledged,
  Poisoned,
}

fn append_commit<C>(
  mut response: Response<C>,
  action_id: Option<ActionId>,
  mut commit: ReactantCommit,
) -> Response<C>
where
  C: From<Command>,
{
  let groups = commit.take_groups();
  if groups.is_empty() {
    commit.acknowledge();
    return response;
  }
  let groups = groups
    .into_iter()
    .map(|bodies| {
      ParallelCommandGroup::new(
        bodies
          .into_iter()
          .map(Command::new_v4)
          .map(C::from)
          .collect(),
      )
    })
    .collect();
  let mut batch = Batch::new(BatchId::new_v4(), response.session_id, groups);
  if let Some(action_id) = action_id {
    batch.caused_by_action_id = Some(action_id);
  }
  response.messages.push(ResponseMessage::Batch(batch));
  commit.acknowledge();
  response
}

fn merge_groups(
  mut merged: Vec<Vec<CommandBody>>,
  groups: Vec<Vec<CommandBody>>,
) -> Vec<Vec<CommandBody>> {
  for (index, group) in groups.into_iter().enumerate() {
    if index == merged.len() {
      merged.push(group);
    } else {
      merged[index].extend(group);
    }
  }
  merged
}

fn validate_document_subscriptions(document: &UiDocument) {
  let authored_events = matches!(&document.element.events, Prop::Set(values) if !values.is_empty());
  let authored_routes =
    matches!(&document.element.event_subscriptions, Prop::Set(values) if !values.is_empty());
  assert!(
    !authored_events && !authored_routes,
    "Reactant owns native event subscriptions"
  );
}

fn render_document<G>(root: &RootRegistration<G>, tree: &RenderTree) -> UiDocument {
  let mut document = root.document.clone();
  let subscriptions = tree.coverage_subscriptions();
  if !subscriptions.is_empty() {
    document.element.event_subscriptions = Prop::Set(subscriptions);
  }
  document.children = tree.hosts();
  document
}

fn coverage_groups<G>(root: &RootRegistration<G>, desired: &RenderTree) -> Vec<Vec<CommandBody>> {
  let previous = root.committed.coverage_subscriptions();
  let desired = desired.coverage_subscriptions();
  if previous == desired {
    return Vec::new();
  }
  let mut patch = VisualElement::new();
  patch.event_subscriptions = if desired.is_empty() {
    Prop::Reset
  } else {
    Prop::Set(desired)
  };
  vec![vec![
    Command::update_visual_element(root.document.root_id, patch).body,
  ]]
}

fn with_coverage_barrier<G>(
  root: &RootRegistration<G>,
  desired: &RenderTree,
  mut groups: Vec<Vec<CommandBody>>,
) -> Vec<Vec<CommandBody>> {
  let mut coverage = self::coverage_groups(root, desired);
  if coverage.is_empty() {
    return groups;
  }
  if desired.coverage_subscriptions().is_empty() {
    groups.append(&mut coverage);
    groups
  } else {
    coverage.append(&mut groups);
    coverage
  }
}

struct RootRegistration<G> {
  document: UiDocument,
  view: Box<dyn RootView<G>>,
  committed: RenderTree,
}

impl<G> RootRegistration<G> {
  fn collides(&self, document: &UiDocument) -> bool {
    let ids = [self.document.document_id, self.document.root_id];
    ids.contains(&document.document_id) || ids.contains(&document.root_id)
  }
}

trait RootView<G> {
  fn render(&self, game: &G, committed: &RenderTree) -> RenderTree;
}

trait SessionRuntime {
  fn commit_session(&mut self, committed: &[RenderTree]);
  fn poison(&mut self);
}

impl<G: 'static> SessionRuntime for Reactant<G> {
  fn commit_session(&mut self, committed: &[RenderTree]) {
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
  fn render(&self, game: &G, committed: &RenderTree) -> RenderTree {
    render::lower(
      context::with_hooks_forbidden(|| (self.view)(game)),
      committed,
    )
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RuntimeState {
  Registering,
  Active,
  Closed,
  Poisoned,
}
