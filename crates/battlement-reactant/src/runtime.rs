//! Runtime lifecycle, roots, sessions, and commits.

use std::{
  any::TypeId,
  cell::{Cell, RefCell},
  error::Error,
  fmt,
  hash::Hash,
  marker::PhantomData,
  mem,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
  thread,
};

use battlement::{
  self, ActionId, Batch, BatchId, Command, CommandBody, GeometryObservationBatch, ObjectId,
  ParallelCommandGroup, Prop, Response, ResponseMessage, SessionId, Snapshot, UiDocument, UiEvent,
  VisualElement,
};

use crate::{
  context,
  effect::EffectOperation,
  error_boundary::ErrorReport,
  event_dispatch::{self, CrossingCandidate},
  executor::Spawner,
  external_portal::{ExternalPortalRegistry, PreparedExternal, SessionExternal},
  portal::{self, PortalRoot, PortalTarget},
  reconcile,
  render::{self, Render, RenderTree},
  render_value::{ErrorOwner, SharedRenderError},
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
#[derive(Clone)]
pub struct RenderError {
  owner: ErrorOwner,
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
  external: Option<SessionExternal>,
  consumed: bool,
}

/// Owns the declarative UI state for one game model.
pub struct Reactant<G: 'static> {
  runtime_id: u64,
  context_defaults: Rc<RefCell<context::ContextDefaults>>,
  _spawner: Box<dyn Spawner>,
  roots: Vec<RootRegistration<G>>,
  state: RuntimeState,
  outstanding: Option<DeliveryReceipt>,
  crossing_candidate: Option<CrossingCandidate>,
  pending_effects: Vec<EffectOperation>,
  pending_error_reports: Vec<ErrorReport>,
  next_portal_target: u64,
  external_portals: ExternalPortalRegistry,
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

impl Root {
  pub(crate) const fn new(runtime_id: u64, index: usize) -> Self {
    Self { runtime_id, index }
  }
}

impl Error for RenderError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    self.error().source()
  }
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
      context_defaults: Rc::new(RefCell::new(context::ContextDefaults::default())),
      _spawner: Box::new(spawner),
      roots: Vec::new(),
      state: RuntimeState::Registering,
      outstanding: None,
      crossing_candidate: None,
      pending_effects: Vec::new(),
      pending_error_reports: Vec::new(),
      next_portal_target: 0,
      external_portals: ExternalPortalRegistry::new(),
    }
  }

  /// Creates one internal portal target while registration is open.
  pub fn create_portal_target(&mut self) -> PortalTarget {
    self.require_registering();
    let target = PortalTarget::new(self.runtime_id, self.next_portal_target);
    self.next_portal_target = self
      .next_portal_target
      .checked_add(1)
      .expect("Reactant portal target identity overflow");
    target
  }

  /// Registers one caller-owned portal container while registration is open.
  pub fn register_external_container(&mut self, id: ObjectId) -> PortalTarget {
    self.require_registering();
    let target = PortalTarget::new(self.runtime_id, self.next_portal_target);
    self.external_portals.register(target.clone(), id);
    self.next_portal_target = self
      .next_portal_target
      .checked_add(1)
      .expect("Reactant portal target identity overflow");
    target
  }

  /// Stages a registered external target's container for the next session.
  pub fn stage_external_container_rebind(&mut self, target: &PortalTarget, id: ObjectId) {
    self.require_active();
    self.external_portals.stage(self.runtime_id, target, id);
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
    let bindings = self.external_portals.session_bindings();
    self.freeze_store_wakes();
    self.crossing_candidate = None;
    let rendered = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut rendered = self
        .roots
        .iter()
        .map(|root| {
          root
            .view
            .render(game, &root.committed, Rc::clone(&self.context_defaults))
        })
        .collect::<Result<Vec<_>, _>>()?;
      for tree in &rendered {
        tree.validate_model(TypeId::of::<G>());
      }
      let committed = self
        .roots
        .iter()
        .map(|root| root.committed.clone())
        .collect::<Vec<_>>();
      let previous = portal::layout(self.runtime_id, &committed, &bindings);
      let mut desired = portal::layout(self.runtime_id, &rendered, &bindings);
      let changed = portal::changed_attachments(&previous, &desired);
      if !changed.is_empty() {
        for tree in &mut rendered {
          tree.remount_changed_portals(&changed);
        }
        desired = portal::layout(self.runtime_id, &rendered, &bindings);
      }
      let documents = self
        .roots
        .iter()
        .zip(&desired.roots)
        .map(|(root, physical)| self::render_document(root, physical))
        .collect();
      Ok((rendered, documents, desired.externals))
    }));
    let (committed, documents, externals) = match rendered {
      Ok(Ok(value)) => value,
      Ok(Err(error)) => return Err(error),
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    Ok(SessionUi {
      runtime: self,
      documents,
      committed,
      external: Some(SessionExternal::new(bindings, externals)),
      consumed: false,
    })
  }

  /// Dispatches one native UI event while active.
  pub fn dispatch(&mut self, game: &mut G, event: UiEvent) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.freeze_store_wakes();
    self.flush_effects();
    let reported = self.flush_error_reports(game);
    let invoked = panic::catch_unwind(AssertUnwindSafe(|| {
      event_dispatch::dispatch(
        self.runtime_id,
        &self
          .roots
          .iter()
          .map(|root| &root.committed)
          .collect::<Vec<_>>(),
        &mut self.crossing_candidate,
        game,
        event,
      )
    }));
    match invoked {
      Ok(invoked) if invoked || reported || self.pending_hooks_changed() => self.render(game),
      Ok(_) => Ok(ReactantCommit::empty()),
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    }
  }

  /// Installs one complete geometry generation while active.
  pub fn observe_geometry(
    &mut self,
    game: &mut G,
    _batch: GeometryObservationBatch,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.freeze_store_wakes();
    self.flush_effects();
    let reported = self.flush_error_reports(game);
    if reported || self.pending_hooks_changed() {
      self.render(game)
    } else {
      Ok(ReactantCommit::empty())
    }
  }

  /// Renders all roots after application state changed.
  pub fn refresh(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.freeze_store_wakes();
    self.flush_effects();
    self.flush_error_reports(game);
    self.render(game)
  }

  fn render(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    let bindings = self.external_portals.active_bindings();
    let planned = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut rendered = self
        .roots
        .iter()
        .map(|root| {
          root
            .view
            .render(game, &root.committed, Rc::clone(&self.context_defaults))
        })
        .collect::<Result<Vec<_>, _>>()?;
      for tree in &rendered {
        tree.validate_model(TypeId::of::<G>());
      }
      let committed = self
        .roots
        .iter()
        .map(|root| root.committed.clone())
        .collect::<Vec<_>>();
      let previous = portal::layout(self.runtime_id, &committed, &bindings);
      let mut desired = portal::layout(self.runtime_id, &rendered, &bindings);
      let changed = portal::changed_attachments(&previous, &desired);
      if !changed.is_empty() {
        for tree in &mut rendered {
          tree.remount_changed_portals(&changed);
        }
        desired = portal::layout(self.runtime_id, &rendered, &bindings);
      }
      let documents = self
        .roots
        .iter()
        .zip(&desired.roots)
        .map(|(root, physical)| self::render_document(root, physical))
        .collect::<Vec<_>>();
      battlement::validate_documents(&documents)
        .expect("Reactant rendered an invalid UI hierarchy");
      let groups = self
        .roots
        .iter()
        .zip(previous.roots.iter().zip(&desired.roots))
        .map(|(root, (previous, desired))| {
          let groups =
            reconcile::command_groups(root.document.root_id, &previous.hosts, &desired.hosts);
          self::with_coverage_barrier(root, previous, desired, groups)
        })
        .fold(Vec::new(), self::merge_groups);
      let groups = self::merge_groups(
        groups,
        self
          .external_portals
          .active_groups(&previous, &desired, &documents),
      );
      Ok((rendered, groups))
    }));
    let (mut rendered, groups) = match planned {
      Ok(Ok(value)) => value,
      Ok(Err(error)) => return Err(error),
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    self.commit_rendered(&mut rendered);
    Ok(self.create_commit(groups))
  }

  /// Processes queued runtime work while active.
  pub fn poll(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.freeze_store_wakes();
    self.flush_effects();
    let reported = self.flush_error_reports(game);
    if reported || self.pending_hooks_changed() {
      self.render(game)
    } else {
      Ok(ReactantCommit::empty())
    }
  }

  /// Closes the runtime and returns its final native work.
  pub fn shutdown(&mut self, game: &mut G) -> ReactantCommit {
    self.require_delivery();
    assert!(
      self.state != RuntimeState::Poisoned,
      "Reactant runtime is poisoned"
    );
    if self.state == RuntimeState::Closed {
      return ReactantCommit::empty();
    }
    if self.state == RuntimeState::Active {
      self.flush_effects();
      self.flush_error_reports(game);
      let mut effects = Vec::new();
      let unmounted = panic::catch_unwind(AssertUnwindSafe(|| {
        for root in &mut self.roots {
          root.committed.unmount_all_effects(&mut effects);
        }
      }));
      if let Err(payload) = unmounted {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
      self.run_effects(effects);
    }
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

  fn commit_rendered(&mut self, committed: &mut [RenderTree]) {
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      self.install_rendered(committed);
      self.state = RuntimeState::Active;
    }));
    if let Err(payload) = completed {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
  }

  fn install_rendered(&mut self, committed: &mut [RenderTree]) {
    let mut next = Vec::new();
    for rendered in committed.iter() {
      rendered.hook_owners(&mut next);
    }
    let mut effects = Vec::new();
    for root in &mut self.roots {
      root.committed.unmount_effects(&next, &mut effects);
    }
    for rendered in committed.iter_mut() {
      rendered.take_effect_operations(&mut effects);
      rendered.commit_hooks();
    }
    let mut reports = Vec::new();
    for rendered in committed.iter_mut() {
      rendered.take_error_reports(&mut reports);
    }
    for (root, rendered) in self.roots.iter_mut().zip(committed) {
      root.committed.clone_from(rendered);
    }
    self.pending_effects.extend(effects);
    self.pending_error_reports.extend(reports);
  }

  fn has_pending_hooks(&self) -> bool {
    self
      .roots
      .iter()
      .any(|root| root.committed.has_pending_hooks())
  }

  fn freeze_store_wakes(&mut self) {
    for root in &mut self.roots {
      root.committed.freeze_store_wakes();
    }
  }

  fn pending_hooks_changed(&mut self) -> bool {
    if !self.has_pending_hooks() {
      return false;
    }
    let changed = panic::catch_unwind(AssertUnwindSafe(|| {
      self
        .roots
        .iter()
        .any(|root| root.committed.has_changed_hooks())
    }));
    match changed {
      Ok(true) => true,
      Ok(false) => {
        for root in &mut self.roots {
          root.committed.discard_pending_hooks();
        }
        false
      }
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    }
  }

  fn flush_effects(&mut self) {
    let effects = mem::take(&mut self.pending_effects);
    self.run_effects(effects);
  }

  fn flush_error_reports(&mut self, game: &mut G) -> bool {
    let reports = mem::take(&mut self.pending_error_reports);
    let reported = !reports.is_empty();
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      for report in reports {
        report.run(game);
      }
    }));
    if let Err(payload) = completed {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
    reported
  }

  fn run_effects(&mut self, effects: Vec<EffectOperation>) {
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      for effect in effects {
        effect.run();
      }
    }));
    if let Err(payload) = completed {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
  }
}

impl SessionUi<'_> {
  /// Adds this session UI to a snapshot and returns its complete response.
  pub fn into_response(self, snapshot: Snapshot) -> Response {
    let (snapshot, commit) = self.into_parts(snapshot);
    Response::snapshot(snapshot).append_reactant(commit)
  }

  /// Adds this session UI to a snapshot and returns the minimal commit path.
  pub fn into_parts(mut self, mut snapshot: Snapshot) -> (Snapshot, ReactantCommit) {
    let external = self
      .external
      .take()
      .expect("Reactant session external plan was already consumed")
      .prepare(&mut snapshot, &self.documents);
    let commit = self.runtime.commit_session(&mut self.committed, external);
    self.consumed = true;
    (snapshot, commit)
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

fn render_document<G>(root: &RootRegistration<G>, physical: &PortalRoot) -> UiDocument {
  let mut document = root.document.clone();
  if !physical.subscriptions.is_empty() {
    document.element.event_subscriptions = Prop::Set(physical.subscriptions.clone());
  }
  document.children.clone_from(&physical.hosts);
  document
}

fn coverage_groups<G>(
  root: &RootRegistration<G>,
  previous: &PortalRoot,
  desired: &PortalRoot,
) -> Vec<Vec<CommandBody>> {
  if previous.subscriptions == desired.subscriptions {
    return Vec::new();
  }
  let mut patch = VisualElement::new();
  patch.event_subscriptions = if desired.subscriptions.is_empty() {
    Prop::Reset
  } else {
    Prop::Set(desired.subscriptions.clone())
  };
  vec![vec![
    Command::update_visual_element(root.document.root_id, patch).body,
  ]]
}

fn with_coverage_barrier<G>(
  root: &RootRegistration<G>,
  previous: &PortalRoot,
  desired: &PortalRoot,
  mut groups: Vec<Vec<CommandBody>>,
) -> Vec<Vec<CommandBody>> {
  let mut coverage = self::coverage_groups(root, previous, desired);
  if coverage.is_empty() {
    return groups;
  }
  if desired.subscriptions.is_empty() {
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
  fn render(
    &self,
    game: &G,
    committed: &RenderTree,
    defaults: Rc<RefCell<context::ContextDefaults>>,
  ) -> Result<RenderTree, RenderError>;
}

trait SessionRuntime {
  fn commit_session(
    &mut self,
    committed: &mut [RenderTree],
    external: PreparedExternal,
  ) -> ReactantCommit;
  fn poison(&mut self);
}

impl<G: 'static> SessionRuntime for Reactant<G> {
  fn commit_session(
    &mut self,
    committed: &mut [RenderTree],
    external: PreparedExternal,
  ) -> ReactantCommit {
    let mut external = Some(external);
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      self.install_rendered(committed);
      let groups = self
        .external_portals
        .commit(external.take().expect("external session is committed once"));
      self.state = RuntimeState::Active;
      groups
    }));
    match completed {
      Ok(groups) => self.create_commit(groups),
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    }
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
  fn render(
    &self,
    game: &G,
    committed: &RenderTree,
    defaults: Rc<RefCell<context::ContextDefaults>>,
  ) -> Result<RenderTree, RenderError> {
    context::with_runtime(defaults, || {
      render::lower(
        context::with_hooks_forbidden(|| (self.view)(game)),
        committed,
      )
    })
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RuntimeState {
  Registering,
  Active,
  Closed,
  Poisoned,
}
