//! Runtime lifecycle, roots, sessions, and commits.

use std::{
  any::TypeId,
  cell::RefCell,
  marker::PhantomData,
  mem,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  sync::atomic::{AtomicU64, Ordering},
};

use battlement::{
  self, ActionId, Command, CommandBody, GeometryGeneration, GeometryObservationBatch, ObjectId,
  Prop, UiDocument, UiEvent, VisualElement,
};

use crate::{
  commit::DeliveryReceipt,
  context,
  effect::EffectOperation,
  element_ref::{self, AttachmentSet, ElementRefRuntime},
  error_boundary::ErrorReport,
  event_dispatch::{self, CrossingCandidate},
  executor::Spawner,
  external_portal::{ExternalPortalRegistry, PreparedExternal, SessionExternal},
  geometry,
  geometry_runtime::{GeometryPlan, GeometryRuntime},
  portal::{self, PortalRoot, PortalTarget},
  reconcile,
  render::{self, Render, RenderTree},
  resource_cache::{FrozenCompletions, PanicPayload, ResourceOverlay},
  resource_runtime::{self, ResourceRuntime},
};

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// A render failure that escaped every boundary.
pub type RenderError = crate::render_error::RenderError;

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

/// An ordered native mutation commit.
#[must_use]
pub struct ReactantCommit {
  pub(crate) groups: Option<Vec<Vec<CommandBody>>>,
  pub(crate) receipt: Option<DeliveryReceipt>,
}

/// A prospective complete UI state for one session snapshot.
#[must_use]
pub struct SessionUi<'a> {
  pub(crate) runtime: &'a mut dyn SessionRuntime,
  pub(crate) documents: Vec<UiDocument>,
  pub(crate) committed: Vec<RenderTree>,
  pub(crate) external: Option<SessionExternal>,
  pub(crate) resource_completions: Option<FrozenCompletions>,
  pub(crate) attachments: Option<AttachmentSet>,
  pub(crate) geometry: Option<GeometryPlan>,
  pub(crate) frozen_actions: usize,
  pub(crate) consumed: bool,
}

/// Owns the declarative UI state for one game model.
pub struct Reactant<G: 'static> {
  runtime_id: u64,
  context_defaults: Rc<RefCell<context::ContextDefaults>>,
  roots: Vec<RootRegistration<G>>,
  state: RuntimeState,
  outstanding: Option<DeliveryReceipt>,
  crossing_candidate: Option<CrossingCandidate>,
  pending_effects: Vec<EffectOperation>,
  pending_error_reports: Vec<ErrorReport>,
  element_refs: Rc<RefCell<ElementRefRuntime>>,
  geometry: Rc<RefCell<GeometryRuntime>>,
  next_portal_target: u64,
  external_portals: ExternalPortalRegistry,
  pub(crate) resources: Rc<ResourceRuntime>,
}

impl Root {
  pub(crate) const fn new(runtime_id: u64, index: usize) -> Self {
    Self { runtime_id, index }
  }
}

impl<G: 'static> Reactant<G> {
  /// Creates a registering runtime with an idle executor.
  #[must_use]
  pub fn new(spawner: impl Spawner) -> Self {
    let runtime_id = NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed);
    Self {
      runtime_id,
      context_defaults: Rc::new(RefCell::new(context::ContextDefaults::default())),
      roots: Vec::new(),
      state: RuntimeState::Registering,
      outstanding: None,
      crossing_candidate: None,
      pending_effects: Vec::new(),
      pending_error_reports: Vec::new(),
      element_refs: ElementRefRuntime::new(),
      geometry: GeometryRuntime::new(runtime_id),
      next_portal_target: 0,
      external_portals: ExternalPortalRegistry::new(),
      resources: ResourceRuntime::new(spawner),
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
    let _element_runtime = element_ref::enter_runtime(self.runtime_id, &self.element_refs);
    let session_geometry = self.geometry.borrow().waiting_preview();
    let _geometry_runtime = geometry::enter_runtime(&session_geometry);
    let frozen_actions = self.element_refs.borrow().queued_actions();
    let bindings = self.external_portals.session_bindings();
    let (resource_completions, resource_overlay) = {
      let mut resources = self.resources.cache.borrow_mut();
      let mut completions = resources.freeze();
      if let Some(payload) = resources.current_panic(&mut completions) {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
      let overlay = Rc::new(resources.overlay(&completions));
      (completions, overlay)
    };
    self.freeze_store_wakes();
    self.crossing_candidate = None;
    let rendered = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut rendered = self
        .roots
        .iter()
        .map(|root| {
          root.view.render(
            game,
            &root.committed,
            Rc::clone(&self.context_defaults),
            Rc::clone(&self.resources),
            Some(Rc::clone(&resource_overlay)),
          )
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
      let attachments = AttachmentSet::collect(
        self.runtime_id,
        self
          .roots
          .iter()
          .zip(&rendered)
          .map(|(root, tree)| (root.document.document_id, tree)),
      );
      let mut geometry_targets = Vec::new();
      for tree in &rendered {
        tree.geometry_targets(&mut geometry_targets);
      }
      let geometry = self
        .geometry
        .borrow()
        .plan(&geometry_targets, &attachments, true)
        .expect("Reactant planned an invalid geometry registry");
      Ok((
        rendered,
        documents,
        desired.externals,
        attachments,
        geometry,
      ))
    }));
    let (committed, documents, externals, attachments, geometry) = match rendered {
      Ok(Ok(value)) => value,
      Ok(Err(error)) => {
        self
          .resources
          .cache
          .borrow_mut()
          .restore(resource_completions);
        return Err(error);
      }
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
      resource_completions: Some(resource_completions),
      attachments: Some(attachments),
      geometry: Some(geometry),
      frozen_actions,
      consumed: false,
    })
  }

  /// Dispatches one native UI event while active.
  pub fn dispatch(&mut self, game: &mut G, event: UiEvent) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let _element_runtime = element_ref::enter_runtime(self.runtime_id, &self.element_refs);
    let _geometry_runtime = geometry::enter_runtime(&self.geometry);
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
      Ok(invoked) => {
        if invoked || reported {
          return self.render(game);
        }
        if self.geometry.borrow().dirty() || self.pending_hooks_changed() {
          self.render(game)
        } else {
          Ok(self.commit_pending_actions())
        }
      }
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
    batch: GeometryObservationBatch,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let _element_runtime = element_ref::enter_runtime(self.runtime_id, &self.element_refs);
    let _geometry_runtime = geometry::enter_runtime(&self.geometry);
    let accepted = panic::catch_unwind(AssertUnwindSafe(|| {
      self
        .geometry
        .borrow_mut()
        .accept(&batch)
        .expect("Reactant received an invalid geometry generation");
    }));
    if let Err(payload) = accepted {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
    let resource_completions = self.resources.cache.borrow_mut().freeze();
    self.apply_resource_completions(resource_completions);
    self.freeze_store_wakes();
    self.flush_effects();
    let reported = self.flush_error_reports(game);
    if reported || self.geometry.borrow().dirty() {
      return self.render(game);
    }
    if self.pending_hooks_changed() {
      self.render(game)
    } else {
      Ok(self.commit_pending_actions())
    }
  }

  /// Renders all roots after application state changed.
  pub fn refresh(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let _element_runtime = element_ref::enter_runtime(self.runtime_id, &self.element_refs);
    let _geometry_runtime = geometry::enter_runtime(&self.geometry);
    self.freeze_store_wakes();
    self.flush_effects();
    self.flush_error_reports(game);
    self.render(game)
  }

  fn render(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    let rendered_generation = self.geometry.borrow().generation;
    self.render_geometry(game, rendered_generation, 0)
  }

  fn render_geometry(
    &mut self,
    game: &mut G,
    rendered_generation: Option<GeometryGeneration>,
    retry: usize,
  ) -> Result<ReactantCommit, RenderError> {
    assert!(retry < 25, "Reactant geometry render did not stabilize");
    let geometry_revision = self.geometry.borrow().revision();
    let bindings = self.external_portals.active_bindings();
    let frozen_actions = self.element_refs.borrow().queued_actions();
    let planned = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut rendered = self
        .roots
        .iter()
        .map(|root| {
          root.view.render(
            game,
            &root.committed,
            Rc::clone(&self.context_defaults),
            Rc::clone(&self.resources),
            None,
          )
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
      let attachments = AttachmentSet::collect(
        self.runtime_id,
        self
          .roots
          .iter()
          .zip(&rendered)
          .map(|(root, tree)| (root.document.document_id, tree)),
      );
      let mut geometry_targets = Vec::new();
      for tree in &rendered {
        tree.geometry_targets(&mut geometry_targets);
      }
      let geometry = self
        .geometry
        .borrow()
        .plan(&geometry_targets, &attachments, false)
        .expect("Reactant planned an invalid geometry registry");
      let mut groups = geometry.command_groups(groups);
      groups.extend(attachments.action_groups(
        &self.element_refs.borrow(),
        frozen_actions,
        &desired,
      ));
      Ok((rendered, groups, attachments, geometry))
    }));
    let (mut rendered, groups, attachments, geometry) = match planned {
      Ok(Ok(value)) => value,
      Ok(Err(error)) => return Err(error),
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    if geometry.generation() != rendered_generation {
      let preview = self.geometry.borrow().preview(&geometry);
      let _geometry_runtime = geometry::enter_runtime(&preview);
      return self.render_geometry(game, geometry.generation(), retry + 1);
    }
    self.commit_rendered(
      &mut rendered,
      attachments,
      geometry,
      geometry_revision,
      frozen_actions,
    );
    Ok(self.create_commit(groups))
  }

  /// Processes queued runtime work while active.
  pub fn poll(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let _element_runtime = element_ref::enter_runtime(self.runtime_id, &self.element_refs);
    let _geometry_runtime = geometry::enter_runtime(&self.geometry);
    let resource_completions = self.resources.cache.borrow_mut().freeze();
    self.apply_resource_completions(resource_completions);
    self.freeze_store_wakes();
    self.flush_effects();
    let reported = self.flush_error_reports(game);
    if reported || self.geometry.borrow().dirty() {
      return self.render(game);
    }
    if self.pending_hooks_changed() {
      self.render(game)
    } else {
      Ok(self.commit_pending_actions())
    }
  }

  /// Closes the runtime and returns its final native work.
  pub fn shutdown(&mut self, game: &mut G) -> ReactantCommit {
    self.require_delivery();
    let _element_runtime = element_ref::enter_runtime(self.runtime_id, &self.element_refs);
    let _geometry_runtime = geometry::enter_runtime(&self.geometry);
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
    if let Err(payload) = self.resources.cache.borrow_mut().cancel_all() {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
    self.element_refs.borrow_mut().detach_all();
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

  pub(crate) fn require_open(&mut self) {
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

  pub(crate) fn resume_resource_panic(&mut self, payload: PanicPayload) -> ! {
    self.state = RuntimeState::Poisoned;
    panic::resume_unwind(payload);
  }

  fn require_active(&mut self) {
    self.require_delivery();
    assert!(
      self.state == RuntimeState::Active,
      "Reactant runtime is not active"
    );
  }

  fn require_delivery(&mut self) {
    let Some(receipt) = self.outstanding.as_ref() else {
      return;
    };
    if receipt.acknowledged() {
      self.outstanding = None;
      return;
    }
    if receipt.pending() {
      receipt.poison();
      self.state = RuntimeState::Poisoned;
      panic!("Reactant cannot reenter while a commit delivery receipt is outstanding");
    }
    self.state = RuntimeState::Poisoned;
    panic!("Reactant runtime is poisoned by an undelivered commit");
  }

  fn create_commit(&mut self, groups: Vec<Vec<CommandBody>>) -> ReactantCommit {
    if groups.is_empty() {
      return ReactantCommit::empty();
    }
    let receipt = DeliveryReceipt::new();
    self.outstanding = Some(receipt.clone());
    ReactantCommit::new(groups, receipt)
  }

  fn commit_pending_actions(&mut self) -> ReactantCommit {
    let frozen_actions = self.element_refs.borrow().queued_actions();
    if frozen_actions == 0 {
      return ReactantCommit::empty();
    }
    let planned = panic::catch_unwind(AssertUnwindSafe(|| {
      let committed = self
        .roots
        .iter()
        .map(|root| root.committed.clone())
        .collect::<Vec<_>>();
      let layout = portal::layout(
        self.runtime_id,
        &committed,
        &self.external_portals.active_bindings(),
      );
      let attachments = AttachmentSet::collect(
        self.runtime_id,
        self
          .roots
          .iter()
          .zip(&committed)
          .map(|(root, tree)| (root.document.document_id, tree)),
      );
      attachments.action_groups(&self.element_refs.borrow(), frozen_actions, &layout)
    }));
    let groups = match planned {
      Ok(groups) => groups,
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    self
      .element_refs
      .borrow_mut()
      .consume_actions(frozen_actions);
    self.create_commit(groups)
  }

  fn commit_rendered(
    &mut self,
    committed: &mut [RenderTree],
    attachments: AttachmentSet,
    geometry: GeometryPlan,
    geometry_revision: u64,
    frozen_actions: usize,
  ) {
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      self.install_rendered(committed, attachments, false);
      let mut runtime = self.geometry.borrow_mut();
      runtime.commit(geometry);
      runtime.acknowledge_render(geometry_revision);
      drop(runtime);
      self
        .element_refs
        .borrow_mut()
        .consume_actions(frozen_actions);
      self.state = RuntimeState::Active;
    }));
    if let Err(payload) = completed {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
  }

  fn install_rendered(
    &mut self,
    committed: &mut [RenderTree],
    attachments: AttachmentSet,
    reconnect: bool,
  ) {
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
    attachments.commit(&mut self.element_refs.borrow_mut(), reconnect);
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

  fn apply_resource_completions(&mut self, frozen: FrozenCompletions) {
    if let Err(payload) = self.resources.cache.borrow_mut().apply(frozen) {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
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
    resources: Rc<ResourceRuntime>,
    resource_overlay: Option<Rc<ResourceOverlay>>,
  ) -> Result<RenderTree, RenderError>;
}

pub(crate) trait SessionRuntime {
  fn commit_session(
    &mut self,
    committed: &mut [RenderTree],
    external: PreparedExternal,
    resource_completions: FrozenCompletions,
    attachments: AttachmentSet,
    geometry: GeometryPlan,
    frozen_actions: usize,
  ) -> ReactantCommit;
  fn poison(&mut self);
}

impl<G: 'static> SessionRuntime for Reactant<G> {
  fn commit_session(
    &mut self,
    committed: &mut [RenderTree],
    external: PreparedExternal,
    resource_completions: FrozenCompletions,
    attachments: AttachmentSet,
    geometry: GeometryPlan,
    frozen_actions: usize,
  ) -> ReactantCommit {
    let mut external = Some(external);
    let mut resource_completions = Some(resource_completions);
    let mut geometry = Some(geometry);
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      self
        .resources
        .cache
        .borrow_mut()
        .apply(
          resource_completions
            .take()
            .expect("resource session is committed once"),
        )
        .unwrap_or_else(|payload| panic::resume_unwind(payload));
      self.install_rendered(committed, attachments, true);
      self
        .element_refs
        .borrow_mut()
        .consume_actions(frozen_actions);
      let groups = self
        .external_portals
        .commit(external.take().expect("external session is committed once"));
      let geometry = geometry.take().expect("geometry session is committed once");
      let groups = geometry.command_groups(groups);
      self.geometry.borrow_mut().commit(geometry);
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
    resources: Rc<ResourceRuntime>,
    resource_overlay: Option<Rc<ResourceOverlay>>,
  ) -> Result<RenderTree, RenderError> {
    resource_runtime::with_runtime(resources, resource_overlay, || {
      context::with_runtime(defaults, || {
        render::lower(
          context::with_hooks_forbidden(|| (self.view)(game)),
          committed,
        )
      })
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
