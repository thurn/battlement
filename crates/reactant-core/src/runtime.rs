//! Runtime lifecycle, roots, sessions, and commits.

use std::{
  any::TypeId,
  cell::RefCell,
  collections::HashMap,
  mem,
  panic::{self, AssertUnwindSafe},
  rc::Rc,
  sync::atomic::{AtomicU64, Ordering},
};

use battlement::{
  self, AccessibilitySnapshot, AccessibilityUpdate, ActionId, Batch, CommandBody,
  GeometryGeneration, GeometryObservationBatch, MotionEventBatch, MotionSequence, ObjectId,
  SessionId, Snapshot, UiDocument, UiEvent, UiEventDisposition,
};
use battlement_flatbuffers::{RetainedUiBudget, RetainedUiSnapshot};
use trox::{Bundle, Localizer, SourceLocale};

use crate::{
  announcement,
  commit::DeliveryReceipt,
  context,
  effect::EffectOperation,
  element_ref::{self, AttachmentSet, ElementRefRuntime},
  error_boundary::ErrorReport,
  event_dispatch,
  executor::Spawner,
  external_portal::{ExternalPortalRegistry, PreparedExternal, SessionExternal},
  geometry,
  geometry_effect::GeometryEffectOperation,
  geometry_runtime::{GeometryPlan, GeometryRuntime},
  lifecycle::{self, EntryCheckpoint, FrozenResources, PlannedSession, RuntimeState},
  localization,
  motion_value_runtime::{self, MotionValueRuntime},
  overlay,
  portal::{self, PortalTarget},
  reconcile,
  render::{Render, RenderTree},
  render_tree::LocalRenderTransaction,
  resource_cache::{FrozenCompletions, PanicPayload},
  resource_runtime::{self, ResourceRuntime},
  root_view::RootRegistration,
  runtime_document, runtime_motion, semantic_projection, ui_host_adapter,
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
  pub(crate) runtime_id: u64,
  pub(crate) index: usize,
}

/// An ordered native mutation commit.
#[must_use]
pub struct ReactantCommit {
  pub(crate) groups: Option<Vec<Vec<CommandBody>>>,
  pub(crate) receipt: Option<DeliveryReceipt>,
  pub(crate) work_owners: HashMap<ObjectId, u64>,
  pub(crate) independent: bool,
}

struct FrozenCommandCounts {
  actions: usize,
  motion: usize,
}

/// The immediate decision and deferred commit produced by one UI event.
#[must_use]
pub struct ReactantEventResult {
  disposition: UiEventDisposition,
  prevented_by_reactant: bool,
  commit: ReactantCommit,
}

impl ReactantEventResult {
  /// Returns the decision Unity must consume before its callback returns.
  #[must_use]
  pub const fn disposition(&self) -> UiEventDisposition {
    self.disposition
  }

  /// Returns whether a Reactant handler added default prevention.
  #[must_use]
  pub const fn prevented_by_reactant(&self) -> bool {
    self.prevented_by_reactant
  }

  /// Returns the ordinary native mutation commit for deferred application.
  pub fn into_commit(self) -> ReactantCommit {
    self.commit
  }

  /// Returns whether deferred reconciliation produced no native mutations.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.commit.is_empty()
  }

  /// Consumes the result and returns its deferred command groups.
  #[must_use]
  pub fn into_groups(self) -> Vec<Vec<CommandBody>> {
    self.commit.into_groups()
  }
}

/// A prospective complete UI state for one session snapshot.
#[must_use]
pub struct SessionUi<'a> {
  pub(crate) objects: Vec<battlement::GameObject>,
  pub(crate) runtime: &'a mut dyn SessionRuntime,
  pub(crate) documents: Vec<UiDocument>,
  pub(crate) retained_ui: Option<Vec<Rc<RetainedUiSnapshot>>>,
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
  pub(crate) track_work_scopes: bool,
  work_owners: HashMap<ObjectId, u64>,
  current_work_owners: HashMap<ObjectId, u64>,
  pending_effects: Vec<EffectOperation>,
  pending_geometry_effects: Vec<GeometryEffectOperation>,
  pending_error_reports: Vec<ErrorReport>,
  element_refs: Rc<RefCell<ElementRefRuntime>>,
  motion_values: Rc<RefCell<MotionValueRuntime>>,
  geometry: Rc<RefCell<GeometryRuntime>>,
  next_portal_target: u64,
  external_portals: ExternalPortalRegistry,
  committed_portals: Option<portal::PortalLayout>,
  retained_ui_budget: RetainedUiBudget,
  retained_ui: Vec<Rc<RetainedUiSnapshot>>,
  last_motion_sequence: Option<MotionSequence>,
  semantic_commit_sequence: u64,
  last_accessibility: Option<AccessibilitySnapshot>,
  localizer: Rc<Localizer>,
  pub(crate) resources: Rc<ResourceRuntime>,
}

impl<G: 'static> Reactant<G> {
  /// Creates a registering runtime with an idle executor.
  #[must_use]
  pub fn new(spawner: impl Spawner) -> Self {
    crate::performance::initialize();
    let runtime_id = NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed);
    Self {
      runtime_id,
      context_defaults: Rc::new(RefCell::new(context::ContextDefaults::default())),
      roots: Vec::new(),
      state: RuntimeState::Registering,
      outstanding: None,
      track_work_scopes: false,
      work_owners: HashMap::new(),
      current_work_owners: HashMap::new(),
      pending_effects: Vec::new(),
      pending_geometry_effects: Vec::new(),
      pending_error_reports: Vec::new(),
      element_refs: ElementRefRuntime::new(),
      motion_values: MotionValueRuntime::new(runtime_id),
      geometry: GeometryRuntime::new(runtime_id),
      next_portal_target: 0,
      external_portals: ExternalPortalRegistry::new(),
      committed_portals: None,
      retained_ui_budget: RetainedUiBudget::default(),
      retained_ui: Vec::new(),
      last_motion_sequence: None,
      semantic_commit_sequence: 0,
      last_accessibility: None,
      localizer: localization::source_localizer(
        SourceLocale::new("en-US").expect("valid default Reactant source locale"),
      ),
      resources: ResourceRuntime::new(spawner),
    }
  }

  /// Returns the allocation capacity of all live immutable UI declaration arenas.
  #[must_use]
  pub fn retained_ui_allocation_bytes(&self) -> usize {
    self.retained_ui_budget.allocated_bytes()
  }

  /// Configures source-language resolution while registration is open.
  pub fn set_source_bundle(&mut self, source: Bundle) {
    self.require_registering();
    self.localizer =
      Rc::new(Localizer::new(source.clone(), source).expect("valid Reactant source bundle"));
  }

  /// Configures bundle-free source-language resolution while registration is open.
  pub fn set_source_locale(&mut self, source: SourceLocale) {
    self.require_registering();
    self.localizer = localization::source_localizer(source);
  }

  /// Configures complete localization while registration is open.
  pub fn set_localizer(&mut self, localizer: Localizer) {
    self.require_registering();
    self.localizer = Rc::new(localizer);
  }

  /// Replaces localization and reconciles every active root and portal.
  pub fn replace_localizer(
    &mut self,
    game: &mut G,
    localizer: Localizer,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    let previous = mem::replace(&mut self.localizer, Rc::new(localizer));
    let _memo_rendering = localization::force_memo_rendering();
    match self.refresh(game) {
      Ok(commit) => Ok(commit),
      Err(error) => {
        self.localizer = previous;
        Err(error)
      }
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
    runtime_document::validate_subscriptions(&document);
    let root = Root {
      runtime_id: self.runtime_id,
      index: self.roots.len(),
    };
    self.roots.push(RootRegistration::new(document, view));
    root
  }

  /// Begins a transactional initial or reconnect render.
  pub fn begin_session<'a>(&'a mut self, game: &mut G) -> Result<SessionUi<'a>, RenderError> {
    self.require_open();
    let _localization = localization::enter(self.localizer.clone());
    let planned = panic::catch_unwind(AssertUnwindSafe(|| self.plan_session(game)));
    let planned = match planned {
      Ok(planned) => planned?,
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    Ok(SessionUi {
      objects: planned.objects,
      runtime: self,
      documents: planned.documents,
      retained_ui: Some(planned.retained_ui),
      committed: planned.committed,
      external: Some(planned.external),
      resource_completions: Some(planned.resource_completions),
      attachments: Some(planned.attachments),
      geometry: Some(planned.geometry),
      frozen_actions: planned.frozen_actions,
      consumed: false,
    })
  }

  fn plan_session(&mut self, game: &mut G) -> Result<PlannedSession, RenderError> {
    let _element_runtime =
      element_ref::enter_runtime(self.runtime_id, &self.element_refs, &self.geometry);
    let _motion_runtime = motion_value_runtime::enter_runtime(self.runtime_id, &self.motion_values);
    let mut session_geometry = self.geometry.borrow().waiting_preview();
    let frozen_actions = self.element_refs.borrow().queued_actions();
    let bindings = self.external_portals.session_bindings();
    let mut resources = self.freeze_resources();
    let resource_overlay = resources.overlay();
    self.freeze_store_wakes();
    let mut retry = 0;
    loop {
      assert!(retry < 25, "Reactant session geometry did not stabilize");
      let _geometry_runtime = geometry::enter_preview(&session_geometry, &self.geometry);
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
        if retry > 0 {
          session_geometry.borrow().stabilize_hosts(&mut rendered);
        }
        let committed = self
          .roots
          .iter()
          .map(|root| &root.committed)
          .collect::<Vec<_>>();
        let previous = portal::layout(self.runtime_id, &committed, &bindings);
        let tentative = rendered.iter().collect::<Vec<_>>();
        let tentative = portal::attachment_hosts(self.runtime_id, &tentative);
        let changed = portal::changed_attachments(&previous, &tentative);
        if !changed.is_empty() {
          for tree in &mut rendered {
            tree.remount_changed_portals(&changed);
          }
        }
        let attachments = AttachmentSet::collect(
          self.runtime_id,
          self
            .roots
            .iter()
            .zip(&rendered)
            .map(|(root, tree)| (root.document.document_id, tree)),
        );
        for tree in &mut rendered {
          tree.resolve_drag_constraints(self.runtime_id, &attachments);
          tree.resolve_overlay_refs(self.runtime_id, &attachments);
        }
        overlay::resolve_order(&mut rendered);
        let desired_trees = rendered.iter().collect::<Vec<_>>();
        let desired = portal::layout(self.runtime_id, &desired_trees, &bindings);
        let documents = self
          .roots
          .iter()
          .zip(&desired.roots)
          .map(|(root, physical)| runtime_document::render(&root.document, physical))
          .collect::<Vec<_>>();
        let retained_ui = self.build_retained_layout(&documents, &desired, &bindings)?;
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
          crate::object_layout::snapshot(&desired.objects),
          retained_ui,
          desired.externals,
          attachments,
          geometry,
        ))
      }));
      let (committed, documents, objects, retained_ui, externals, attachments, geometry) =
        match rendered {
          Ok(Ok(value)) => value,
          Ok(Err(error)) => return Err(error),
          Err(payload) => panic::resume_unwind(payload),
        };
      if geometry.requires_preview(&session_geometry.borrow()) {
        session_geometry = self.geometry.borrow().preview(&geometry);
        retry += 1;
        continue;
      }
      return Ok(PlannedSession {
        objects,
        documents,
        retained_ui,
        committed,
        external: SessionExternal::new(bindings, externals),
        resource_completions: resources.take(),
        attachments,
        geometry,
        frozen_actions,
      });
    }
  }

  /// Dispatches one native UI event while active.
  pub fn dispatch(
    &mut self,
    game: &mut G,
    event: UiEvent,
  ) -> Result<ReactantEventResult, RenderError> {
    self.dispatch_input(game, event_dispatch::LogicalInput::Ui(event))
  }

  pub(crate) fn activate_object(
    &mut self,
    game: &mut G,
    target: ObjectId,
  ) -> Result<ReactantCommit, RenderError> {
    self
      .dispatch_input(game, event_dispatch::LogicalInput::WorldActivation(target))
      .map(ReactantEventResult::into_commit)
  }

  fn dispatch_input(
    &mut self,
    game: &mut G,
    event: event_dispatch::LogicalInput,
  ) -> Result<ReactantEventResult, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      runtime.freeze_store_wakes();
      runtime.flush_effects();
      let reported = runtime.flush_error_reports(game);
      let geometry_effected = runtime.flush_geometry_effects(game);
      let dispatch = event_dispatch::dispatch(
        runtime.runtime_id,
        &runtime
          .roots
          .iter()
          .map(|root| &root.committed)
          .collect::<Vec<_>>(),
        game,
        event,
      );
      let dispatch_dirty = dispatch.invoked || reported || geometry_effected;
      let runtime_dirty = runtime.geometry.borrow().dirty() || runtime.pending_hooks_changed();
      let commit = if dispatch_dirty || runtime_dirty {
        if dispatch.local_invalidation
          && !reported
          && !geometry_effected
          && !runtime.geometry.borrow().dirty()
          && runtime.pending_hooks_changed()
        {
          runtime.render_local_state(game)?
        } else {
          runtime.render(game, None)?
        }
      } else {
        runtime.commit_pending_actions()
      };
      Ok(ReactantEventResult {
        disposition: dispatch.disposition,
        prevented_by_reactant: dispatch.prevented_by_reactant,
        commit,
      })
    })
  }

  /// Dispatches a verified borrowed UI event without reconstructing its callback payload.
  pub fn dispatch_view(
    &mut self,
    game: &mut G,
    action: battlement_native::UiEventActionView<'_>,
  ) -> Result<ReactantEventResult, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      runtime.freeze_store_wakes();
      runtime.flush_effects();
      let reported = runtime.flush_error_reports(game);
      let geometry_effected = runtime.flush_geometry_effects(game);
      let dispatch = event_dispatch::dispatch_view(
        runtime.runtime_id,
        &runtime
          .roots
          .iter()
          .map(|root| &root.committed)
          .collect::<Vec<_>>(),
        game,
        action,
      );
      let dispatch_dirty = dispatch.invoked || reported || geometry_effected;
      let runtime_dirty = runtime.geometry.borrow().dirty() || runtime.pending_hooks_changed();
      let commit = if dispatch_dirty || runtime_dirty {
        if dispatch.local_invalidation
          && !reported
          && !geometry_effected
          && !runtime.geometry.borrow().dirty()
          && runtime.pending_hooks_changed()
        {
          runtime.render_local_state(game)?
        } else {
          runtime.render(game, None)?
        }
      } else {
        runtime.commit_pending_actions()
      };
      Ok(ReactantEventResult {
        disposition: dispatch.disposition,
        prevented_by_reactant: dispatch.prevented_by_reactant,
        commit,
      })
    })
  }

  /// Installs one complete geometry generation while active.
  pub fn observe_geometry(
    &mut self,
    game: &mut G,
    batch: GeometryObservationBatch,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      runtime
        .geometry
        .borrow_mut()
        .accept(&batch)
        .expect("Reactant received an invalid geometry generation");
      let resources = runtime.freeze_resources();
      let resources_changed = resources.changed();
      runtime.freeze_store_wakes();
      runtime.flush_effects();
      let reported = runtime.flush_error_reports(game);
      let geometry_effected = runtime.flush_geometry_effects(game);
      if reported || geometry_effected || resources_changed || runtime.geometry.borrow().dirty() {
        return runtime.render(game, Some(resources));
      }
      if runtime.pending_hooks_changed() {
        runtime.render(game, Some(resources))
      } else {
        let mut resources = resources;
        runtime.apply_resources_transaction(&mut resources);
        Ok(runtime.commit_pending_actions())
      }
    })
  }

  /// Installs a verified native geometry generation, retaining only registry-owned values.
  pub fn observe_geometry_view(
    &mut self,
    game: &mut G,
    batch: battlement_native::GeometryObservationBatchView<'_>,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      runtime
        .geometry
        .borrow_mut()
        .accept_view(batch)
        .expect("Reactant received an invalid geometry generation");
      let resources = runtime.freeze_resources();
      let resources_changed = resources.changed();
      runtime.freeze_store_wakes();
      runtime.flush_effects();
      let reported = runtime.flush_error_reports(game);
      let geometry_effected = runtime.flush_geometry_effects(game);
      if reported || geometry_effected || resources_changed || runtime.geometry.borrow().dirty() {
        return runtime.render(game, Some(resources));
      }
      if runtime.pending_hooks_changed() {
        runtime.render(game, Some(resources))
      } else {
        let mut resources = resources;
        runtime.apply_resources_transaction(&mut resources);
        Ok(runtime.commit_pending_actions())
      }
    })
  }

  /// Applies ordered native Motion lifecycle boundaries.
  pub fn motion_events(
    &mut self,
    game: &mut G,
    batch: MotionEventBatch,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      runtime_motion::validate_batch(runtime.last_motion_sequence, &batch);
      let mut changed = false;
      for event in &batch.events {
        for root in &mut runtime.roots {
          changed |= root.committed.invoke_motion_event(game, event);
          changed |= root.committed.apply_motion_event(event);
        }
      }
      for sample in &batch.samples {
        for root in &mut runtime.roots {
          changed |= root.committed.invoke_motion_sample(game, sample);
        }
      }
      for event in &batch.gesture_events {
        for root in &mut runtime.roots {
          changed |= root.committed.invoke_motion_gesture(game, event);
        }
      }
      changed |= runtime
        .motion_values
        .borrow_mut()
        .apply_samples(&batch.value_samples);
      let playback_invocations = runtime
        .motion_values
        .borrow_mut()
        .take_playback_events(&batch.playback_events);
      for invocation in playback_invocations {
        changed |= invocation.invoke();
      }
      if !batch.events.is_empty() {
        runtime.last_motion_sequence = Some(batch.last_sequence);
      }
      if changed
        || runtime.pending_hooks_changed()
        || runtime_motion::has_ready_presence(&runtime.roots)
      {
        runtime.render(game, None)
      } else {
        Ok(runtime.commit_pending_actions())
      }
    })
  }

  /// Applies a verified native Motion batch without materializing its outer record graph.
  pub fn motion_events_view(
    &mut self,
    game: &mut G,
    batch: battlement_native::MotionEventBatchView<'_>,
  ) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      if let Some(previous) = runtime.last_motion_sequence
        && batch.lifecycle_count() != 0
      {
        assert!(
          batch.first_sequence() > previous.0,
          "Motion event batch sequence is stale"
        );
      }
      let mut changed = false;
      for event in batch.lifecycle_events() {
        for root in &mut runtime.roots {
          changed |= root.committed.invoke_motion_event(game, &event);
          changed |= root.committed.apply_motion_event(&event);
        }
      }
      for sample in batch.presentation_samples() {
        for root in &mut runtime.roots {
          changed |= root.committed.invoke_motion_sample(game, &sample);
        }
      }
      for event in batch.gesture_events() {
        for root in &mut runtime.roots {
          changed |= root.committed.invoke_motion_gesture(game, &event);
        }
      }
      let value_samples = batch.value_samples().collect::<Vec<_>>();
      changed |= runtime
        .motion_values
        .borrow_mut()
        .apply_samples(&value_samples);
      let playback_events = batch.playback_events().collect::<Vec<_>>();
      let playback_invocations = runtime
        .motion_values
        .borrow_mut()
        .take_playback_events(&playback_events);
      for invocation in playback_invocations {
        changed |= invocation.invoke();
      }
      if batch.lifecycle_count() != 0 {
        runtime.last_motion_sequence = Some(battlement::MotionSequence(batch.last_sequence()));
      }
      if changed
        || runtime.pending_hooks_changed()
        || runtime_motion::has_ready_presence(&runtime.roots)
      {
        runtime.render(game, None)
      } else {
        Ok(runtime.commit_pending_actions())
      }
    })
  }

  /// Renders all roots after application state changed.
  pub fn refresh(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      runtime.freeze_store_wakes();
      runtime.flush_effects();
      runtime.flush_error_reports(game);
      runtime.flush_geometry_effects(game);
      runtime.render(game, None)
    })
  }

  fn render(
    &mut self,
    game: &mut G,
    resources: Option<FrozenResources>,
  ) -> Result<ReactantCommit, RenderError> {
    let _motion_runtime = motion_value_runtime::enter_runtime(self.runtime_id, &self.motion_values);
    runtime_motion::invoke_ready_presence(&mut self.roots, game);
    let rendered_generation = self.geometry.borrow().generation;
    self.render_geometry(game, rendered_generation, 0, resources, None, None)
  }

  fn build_retained_layout(
    &self,
    documents: &[UiDocument],
    layout: &portal::PortalLayout,
    bindings: &[(PortalTarget, ObjectId)],
  ) -> Result<Vec<Rc<RetainedUiSnapshot>>, RenderError> {
    let mut snapshots = Vec::with_capacity(documents.len() + layout.externals.len());
    for document in documents {
      snapshots.push(Rc::new(
        RetainedUiSnapshot::build_document_in(&self.retained_ui_budget, document)
          .map_err(|error| RenderError::message(error.to_string()))?,
      ));
    }
    for (target, root) in &layout.externals {
      let container_id = bindings
        .iter()
        .find_map(|(candidate, object_id)| (candidate == target).then_some(*object_id))
        .expect("every retained external portal has a bound container");
      snapshots.push(Rc::new(
        RetainedUiSnapshot::build_in(
          &self.retained_ui_budget,
          container_id,
          container_id,
          &ui_host_adapter::to_ui_nodes(&root.hosts),
        )
        .map_err(|error| RenderError::message(error.to_string()))?,
      ));
    }
    Ok(snapshots)
  }

  fn render_local_state(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    let _motion_runtime = motion_value_runtime::enter_runtime(self.runtime_id, &self.motion_values);
    if runtime_motion::invoke_ready_presence(&mut self.roots, game) {
      let rendered_generation = self.geometry.borrow().generation;
      return self.render_geometry(game, rendered_generation, 0, None, None, None);
    }
    if self.committed_portals.is_none() {
      let committed = self
        .roots
        .iter()
        .map(|root| &root.committed)
        .collect::<Vec<_>>();
      self.committed_portals = Some(portal::layout(
        self.runtime_id,
        &committed,
        &self.external_portals.active_bindings(),
      ));
    }
    let mut transactions: Vec<LocalRenderTransaction> = Vec::with_capacity(self.roots.len());
    for root in &mut self.roots {
      let local = resource_runtime::with_runtime(Rc::clone(&self.resources), None, || {
        context::with_runtime(Rc::clone(&self.context_defaults), || {
          root.committed.rerender_local_state()
        })
      });
      let local = match local {
        Ok(local) => local,
        Err(error) => {
          for (root, transaction) in self.roots.iter_mut().zip(transactions) {
            transaction.rollback(&mut root.committed);
          }
          return Err(error);
        }
      };
      if !local.supported() {
        for (root, transaction) in self.roots.iter_mut().zip(transactions) {
          transaction.rollback(&mut root.committed);
        }
        let rendered_generation = self.geometry.borrow().generation;
        return self.render_geometry(game, rendered_generation, 0, None, None, None);
      }
      transactions.push(local);
    }
    let rendered = self
      .roots
      .iter_mut()
      .map(|root| mem::take(&mut root.committed))
      .collect();
    let rendered_generation = self.geometry.borrow().generation;
    self.render_geometry(
      game,
      rendered_generation,
      0,
      None,
      Some(rendered),
      Some(transactions),
    )
  }

  fn render_geometry(
    &mut self,
    game: &mut G,
    rendered_generation: Option<GeometryGeneration>,
    retry: usize,
    mut resources: Option<FrozenResources>,
    initial_rendered: Option<Vec<RenderTree>>,
    mut local_transactions: Option<Vec<LocalRenderTransaction>>,
  ) -> Result<ReactantCommit, RenderError> {
    assert!(retry < 25, "Reactant geometry render did not stabilize");
    let geometry_revision = self.geometry.borrow().revision();
    let bindings = self.external_portals.active_bindings();
    let previous = self.committed_portals.take().unwrap_or_else(|| {
      let committed = self
        .roots
        .iter()
        .map(|root| &root.committed)
        .collect::<Vec<_>>();
      portal::layout(self.runtime_id, &committed, &bindings)
    });
    let frozen_actions = self.element_refs.borrow().queued_actions();
    let frozen_motion_commands = self.motion_values.borrow().queued_commands();
    let planned = panic::catch_unwind(AssertUnwindSafe(|| {
      let mut rendered = match initial_rendered {
        Some(rendered) => rendered,
        None => self
          .roots
          .iter()
          .map(|root| {
            root.view.render(
              game,
              &root.committed,
              Rc::clone(&self.context_defaults),
              Rc::clone(&self.resources),
              resources.as_ref().map(FrozenResources::overlay),
            )
          })
          .collect::<Result<Vec<_>, _>>()?,
      };
      for tree in &rendered {
        tree.validate_model(TypeId::of::<G>());
      }
      let tentative = rendered.iter().collect::<Vec<_>>();
      let tentative = portal::attachment_hosts(self.runtime_id, &tentative);
      let changed = portal::changed_attachments(&previous, &tentative);
      if !changed.is_empty() {
        for tree in &mut rendered {
          tree.remount_changed_portals(&changed);
        }
      }
      let attachments = AttachmentSet::collect(
        self.runtime_id,
        self
          .roots
          .iter()
          .zip(&rendered)
          .map(|(root, tree)| (root.document.document_id, tree)),
      );
      for tree in &mut rendered {
        tree.resolve_drag_constraints(self.runtime_id, &attachments);
        tree.resolve_overlay_refs(self.runtime_id, &attachments);
      }
      overlay::resolve_order(&mut rendered);
      let desired_trees = rendered.iter().collect::<Vec<_>>();
      let desired = portal::layout(self.runtime_id, &desired_trees, &bindings);
      let documents = self
        .roots
        .iter()
        .zip(&desired.roots)
        .map(|(root, physical)| runtime_document::render(&root.document, physical))
        .collect::<Vec<_>>();
      battlement::validate_documents(&documents)
        .expect("Reactant rendered an invalid UI hierarchy");
      let retained_ui = self.build_retained_layout(&documents, &desired, &bindings)?;
      let unchanged_roots = retained_ui
        .iter()
        .take(documents.len())
        .enumerate()
        .map(|(index, desired)| {
          self
            .retained_ui
            .get(index)
            .is_some_and(|previous| previous.same_declaration(desired))
        })
        .collect::<Vec<_>>();
      let groups = self
        .roots
        .iter()
        .zip(previous.roots.iter().zip(&desired.roots))
        .enumerate()
        .map(|(index, (root, (previous, desired)))| {
          if unchanged_roots[index] {
            return Vec::new();
          }
          let groups =
            reconcile::command_groups(root.document.root_id, &previous.hosts, &desired.hosts);
          runtime_document::with_coverage_barrier(root.document.root_id, previous, desired, groups)
        })
        .fold(Vec::new(), runtime_motion::merge_groups);
      let groups = runtime_motion::merge_groups(
        groups,
        self
          .external_portals
          .active_groups(&previous, &desired, &documents),
      );
      let groups = if previous.objects.is_empty() && desired.objects.is_empty() {
        groups
      } else {
        runtime_motion::merge_groups(
          groups,
          reconcile::command_groups(
            self.roots[0].document.root_id,
            &previous.objects,
            &desired.objects,
          ),
        )
      };
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
      groups.extend(
        self
          .motion_values
          .borrow()
          .command_groups(frozen_motion_commands),
      );
      let semantic_snapshot = semantic_projection::build(
        &rendered,
        self.runtime_id,
        &attachments,
        self.semantic_commit_sequence + 1,
      );
      let accessibility_changed = self
        .last_accessibility
        .as_ref()
        .map_or(!semantic_snapshot.nodes.is_empty(), |previous| {
          !same_accessibility(previous, &semantic_snapshot)
        });
      Ok((
        rendered,
        groups,
        attachments,
        geometry,
        semantic_snapshot,
        accessibility_changed,
        retained_ui,
        unchanged_roots,
      ))
    }));
    let (
      mut rendered,
      mut groups,
      attachments,
      geometry,
      semantic_snapshot,
      accessibility_changed,
      mut retained_ui,
      unchanged_roots,
    ) = match planned {
      Ok(Ok(value)) => value,
      Ok(Err(error)) => {
        self.committed_portals = Some(previous);
        return Err(error);
      }
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    };
    if geometry.generation() != rendered_generation {
      let preview = self.geometry.borrow().preview(&geometry);
      let _geometry_runtime = geometry::enter_preview(&preview, &self.geometry);
      self.committed_portals = Some(previous);
      if let Some(transactions) = local_transactions.take() {
        for ((root, tree), transaction) in
          self.roots.iter_mut().zip(&mut rendered).zip(transactions)
        {
          transaction.rollback(tree);
          root.committed = mem::take(tree);
        }
      }
      return self.render_geometry(
        game,
        geometry.generation(),
        retry + 1,
        resources,
        None,
        None,
      );
    }
    let announcements = announcement::take();
    let sent_accessibility = accessibility_changed || !announcements.is_empty();
    let next_accessibility =
      (!semantic_snapshot.nodes.is_empty()).then(|| semantic_snapshot.clone());
    if sent_accessibility {
      groups.push(vec![CommandBody::AccessibilityUpdate(
        AccessibilityUpdate {
          snapshot: accessibility_changed.then_some(semantic_snapshot),
          announcements,
        },
      )]);
    }
    if let Some(resources) = &mut resources {
      self.apply_resources_transaction(resources);
    }
    self.commit_rendered(
      &mut rendered,
      attachments,
      geometry,
      geometry_revision,
      FrozenCommandCounts {
        actions: frozen_actions,
        motion: frozen_motion_commands,
      },
      local_transactions.as_deref_mut(),
    );
    for (index, unchanged) in unchanged_roots.into_iter().enumerate() {
      if unchanged {
        retained_ui[index] = Rc::clone(&self.retained_ui[index]);
      }
    }
    self.retained_ui = retained_ui;
    // The verified retained arenas own the committed physical declaration.
    // Portal layouts are render-transaction scratch and must not keep a second
    // owned `UiNode` forest alive between entries.
    self.committed_portals = None;
    if accessibility_changed {
      self.semantic_commit_sequence += 1;
      self.last_accessibility = next_accessibility;
    }
    Ok(self.create_commit(groups))
  }

  /// Processes queued runtime work while active.
  pub fn poll(&mut self, game: &mut G) -> Result<ReactantCommit, RenderError> {
    self.require_active();
    self.active_entry(|runtime| {
      let _element_runtime =
        element_ref::enter_runtime(runtime.runtime_id, &runtime.element_refs, &runtime.geometry);
      let _geometry_runtime = geometry::enter_runtime(&runtime.geometry);
      let resources = runtime.freeze_resources();
      let resources_changed = resources.changed();
      runtime.freeze_store_wakes();
      runtime.flush_effects();
      let reported = runtime.flush_error_reports(game);
      let geometry_effected = runtime.flush_geometry_effects(game);
      if reported || geometry_effected || resources_changed || runtime.geometry.borrow().dirty() {
        return runtime.render(game, Some(resources));
      }
      if runtime.pending_hooks_changed() || runtime_motion::has_ready_presence(&runtime.roots) {
        runtime.render(game, Some(resources))
      } else {
        let mut resources = resources;
        runtime.apply_resources_transaction(&mut resources);
        Ok(runtime.commit_pending_actions())
      }
    })
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
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      let _element_runtime =
        element_ref::enter_runtime(self.runtime_id, &self.element_refs, &self.geometry);
      let _geometry_runtime = geometry::enter_runtime(&self.geometry);
      let (groups, geometry) = if self.state == RuntimeState::Active {
        let (groups, geometry) = lifecycle::plan_shutdown(
          self.runtime_id,
          &self.roots,
          &self.external_portals,
          &self.geometry.borrow(),
        );
        self.flush_effects();
        self.flush_error_reports(game);
        self.flush_geometry_effects(game);
        let mut effects = Vec::new();
        let mut geometry_effects = Vec::new();
        for root in &mut self.roots {
          root.committed.unmount_all_effects(&mut effects);
          root
            .committed
            .unmount_all_geometry_effects(&mut geometry_effects);
        }
        self.run_effects(effects);
        self.run_geometry_effects(game, geometry_effects);
        (groups, Some(geometry))
      } else {
        (Vec::new(), None)
      };
      self
        .resources
        .cache
        .borrow_mut()
        .cancel_all()
        .unwrap_or_else(|payload| panic::resume_unwind(payload));
      self.element_refs.borrow_mut().detach_all();
      self.motion_values.borrow_mut().clear();
      for root in &mut self.roots {
        root.committed = RenderTree::default();
      }
      self.retained_ui.clear();
      if let Some(geometry) = geometry {
        self.geometry.borrow_mut().commit(geometry);
      }
      self.state = RuntimeState::Closed;
      self.create_commit(groups)
    }));
    match completed {
      Ok(commit) => commit,
      Err(payload) => {
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    }
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

  fn active_entry<T>(
    &mut self,
    operation: impl FnOnce(&mut Self) -> Result<T, RenderError>,
  ) -> Result<T, RenderError> {
    let _localization = localization::enter(self.localizer.clone());
    let checkpoint = EntryCheckpoint::capture(
      self.roots.iter().map(|root| &root.committed),
      &self.element_refs,
      &self.motion_values,
    );
    match panic::catch_unwind(AssertUnwindSafe(|| operation(self))) {
      Ok(Ok(value)) => Ok(value),
      Ok(Err(error)) => {
        checkpoint.discard_actions(&self.element_refs, &self.motion_values);
        Err(error)
      }
      Err(payload) => {
        checkpoint.rollback(
          self.roots.iter().map(|root| &root.committed),
          &self.element_refs,
          &self.motion_values,
        );
        self.state = RuntimeState::Poisoned;
        panic::resume_unwind(payload);
      }
    }
  }

  fn freeze_resources(&mut self) -> FrozenResources {
    self.resources.flush();
    match FrozenResources::freeze(Rc::clone(&self.resources)) {
      Ok(resources) => resources,
      Err(payload) => panic::resume_unwind(payload),
    }
  }

  fn apply_resources_transaction(&mut self, resources: &mut FrozenResources) {
    if let Err(payload) = resources.apply() {
      panic::resume_unwind(payload);
    }
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
    let motion_owners = self.motion_values.borrow_mut().take_owners();
    if groups.is_empty() {
      return ReactantCommit::empty();
    }
    let receipt = DeliveryReceipt::new();
    self.outstanding = Some(receipt.clone());
    let mut commit = ReactantCommit::new(groups, receipt);
    if self.track_work_scopes {
      commit.work_owners = self.work_owners.clone();
      commit.work_owners.extend(motion_owners);
      commit.independent = true;
    }
    commit
  }

  fn commit_pending_actions(&mut self) -> ReactantCommit {
    let frozen_actions = self.element_refs.borrow().queued_actions();
    let frozen_motion_commands = self.motion_values.borrow().queued_commands();
    if frozen_actions == 0 && frozen_motion_commands == 0 {
      return ReactantCommit::empty();
    }
    let planned = panic::catch_unwind(AssertUnwindSafe(|| {
      let committed = self
        .roots
        .iter()
        .map(|root| &root.committed)
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
          .map(|(root, tree)| (root.document.document_id, *tree)),
      );
      let mut groups =
        attachments.action_groups(&self.element_refs.borrow(), frozen_actions, &layout);
      groups.extend(
        self
          .motion_values
          .borrow()
          .command_groups(frozen_motion_commands),
      );
      groups
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
    self
      .motion_values
      .borrow_mut()
      .consume_commands(frozen_motion_commands);
    self.create_commit(groups)
  }

  fn commit_rendered(
    &mut self,
    committed: &mut [RenderTree],
    attachments: AttachmentSet,
    geometry: GeometryPlan,
    geometry_revision: u64,
    frozen_commands: FrozenCommandCounts,
    local_transactions: Option<&mut [LocalRenderTransaction]>,
  ) {
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      self.install_rendered(committed, attachments, false, local_transactions);
      let mut runtime = self.geometry.borrow_mut();
      runtime.commit(geometry);
      runtime.acknowledge_render(geometry_revision);
      drop(runtime);
      self
        .element_refs
        .borrow_mut()
        .consume_actions(frozen_commands.actions);
      self
        .motion_values
        .borrow_mut()
        .consume_commands(frozen_commands.motion);
      self.state = RuntimeState::Active;
    }));
    if let Err(payload) = completed {
      self.state = RuntimeState::Poisoned;
      panic::resume_unwind(payload);
    }
  }

  pub(crate) fn extract_scoped_snapshot(&self, snapshot: &mut Snapshot) -> Vec<Batch> {
    crate::work_scope::extract_snapshot(snapshot, &self.current_work_owners)
  }

  pub(crate) fn recover_scope(&self, scope: u64, session: SessionId) -> Option<Batch> {
    let trees = self
      .roots
      .iter()
      .map(|root| &root.committed)
      .collect::<Vec<_>>();
    let bindings = self.external_portals.active_bindings();
    let layout = portal::layout(self.runtime_id, &trees, &bindings);
    let mut commands = Vec::new();
    for (root, physical) in self.roots.iter().zip(&layout.roots) {
      crate::work_scope::recovery(
        &physical.hosts,
        root.document.root_id,
        scope,
        &self.current_work_owners,
        &mut commands,
      );
    }
    for (target, physical) in &layout.externals {
      let parent = bindings
        .iter()
        .find(|(binding, _)| binding == target)
        .expect("external binding")
        .1;
      crate::work_scope::recovery(
        &physical.hosts,
        parent,
        scope,
        &self.current_work_owners,
        &mut commands,
      );
    }
    let objects = crate::object_layout::snapshot(&layout.objects)
      .into_iter()
      .filter(|object| self.current_work_owners.get(&object.object_id) == Some(&scope))
      .collect();
    let mut groups = crate::work_scope::object_groups(objects);
    if !commands.is_empty() {
      groups.insert(0, commands);
    }
    (!groups.is_empty()).then(|| {
      Batch::new(
        battlement::BatchId::new_v4(),
        session,
        groups
          .into_iter()
          .map(battlement::ParallelCommandGroup::from_bodies)
          .collect(),
      )
    })
  }

  fn install_rendered(
    &mut self,
    committed: &mut [RenderTree],
    attachments: AttachmentSet,
    reconnect: bool,
    local_transactions: Option<&mut [LocalRenderTransaction]>,
  ) {
    if self.track_work_scopes {
      self.work_owners = mem::take(&mut self.current_work_owners);
      for tree in committed.iter() {
        crate::work_scope::collect(tree, None, &mut self.current_work_owners);
      }
      self.work_owners.extend(&self.current_work_owners);
    }
    let mut next = Vec::new();
    for rendered in committed.iter() {
      rendered.hook_owners(&mut next);
    }
    let mut effects = Vec::new();
    let mut geometry_effects = Vec::new();
    if let Some(transactions) = local_transactions {
      for transaction in transactions {
        transaction.unmount_effects(&next, &mut effects, &mut geometry_effects);
      }
    } else {
      for root in &mut self.roots {
        root.committed.unmount_effects(&next, &mut effects);
        root
          .committed
          .unmount_geometry_effects(&next, &mut geometry_effects);
      }
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
      root.committed = mem::take(rendered);
    }
    attachments.commit(&mut self.element_refs.borrow_mut(), reconnect);
    self.pending_effects.extend(effects);
    self.pending_geometry_effects.extend(geometry_effects);
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

  fn flush_geometry_effects(&mut self, game: &mut G) -> bool {
    let mut effects = mem::take(&mut self.pending_geometry_effects);
    let geometry = Rc::clone(&self.geometry);
    let geometry = geometry.borrow();
    for root in &mut self.roots {
      root
        .committed
        .take_geometry_effect_operations(&geometry, &mut effects);
    }
    drop(geometry);
    let effected = !effects.is_empty();
    self.run_geometry_effects(game, effects);
    effected
  }

  fn run_effects(&mut self, effects: Vec<EffectOperation>) {
    lifecycle::run_or_poison(&mut self.state, || {
      for effect in effects {
        effect.run();
      }
    });
  }

  fn run_geometry_effects(&mut self, game: &mut G, effects: Vec<GeometryEffectOperation>) {
    lifecycle::run_or_poison(&mut self.state, || {
      for effect in effects {
        effect.run(game);
      }
    });
  }
}

impl<G: 'static> Drop for Reactant<G> {
  fn drop(&mut self) {
    lifecycle::drop_runtime(
      &mut self.state,
      &mut self.roots,
      &mut self.pending_effects,
      &self.element_refs,
    );
  }
}

fn same_accessibility(previous: &AccessibilitySnapshot, current: &AccessibilitySnapshot) -> bool {
  previous.roots == current.roots && previous.nodes == current.nodes
}

pub(crate) trait SessionRuntime {
  #[allow(clippy::too_many_arguments)]
  fn commit_session(
    &mut self,
    committed: &mut [RenderTree],
    retained_ui: Vec<Rc<RetainedUiSnapshot>>,
    external: PreparedExternal,
    resource_completions: FrozenCompletions,
    attachments: AttachmentSet,
    geometry: GeometryPlan,
    frozen_actions: usize,
  ) -> ReactantCommit;
  fn discard_session(&mut self, resource_completions: Option<FrozenCompletions>);
}

impl<G: 'static> SessionRuntime for Reactant<G> {
  fn commit_session(
    &mut self,
    committed: &mut [RenderTree],
    retained_ui: Vec<Rc<RetainedUiSnapshot>>,
    external: PreparedExternal,
    resource_completions: FrozenCompletions,
    attachments: AttachmentSet,
    geometry: GeometryPlan,
    frozen_actions: usize,
  ) -> ReactantCommit {
    let _localization = localization::enter(self.localizer.clone());
    let mut external = Some(external);
    let mut resources =
      FrozenResources::from_frozen(Rc::clone(&self.resources), resource_completions);
    let mut geometry = Some(geometry);
    let completed = panic::catch_unwind(AssertUnwindSafe(|| {
      let semantic_snapshot = semantic_projection::build(
        committed,
        self.runtime_id,
        &attachments,
        self.semantic_commit_sequence + 1,
      );
      self.install_rendered(committed, attachments, true, None);
      self.retained_ui = retained_ui;
      self.committed_portals = None;
      self
        .element_refs
        .borrow_mut()
        .consume_actions(frozen_actions);
      let groups = self
        .external_portals
        .commit(external.take().expect("external session is committed once"));
      let geometry = geometry.take().expect("geometry session is committed once");
      let mut groups = geometry.command_groups(groups);
      let has_accessibility = !semantic_snapshot.nodes.is_empty();
      if has_accessibility {
        groups.push(vec![CommandBody::AccessibilityUpdate(
          AccessibilityUpdate {
            snapshot: Some(semantic_snapshot.clone()),
            announcements: Vec::new(),
          },
        )]);
      }
      let _discarded_announcements = announcement::take();
      self.geometry.borrow_mut().commit(geometry);
      self.apply_resources_transaction(&mut resources);
      self.last_motion_sequence = None;
      self.state = RuntimeState::Active;
      if has_accessibility {
        self.semantic_commit_sequence += 1;
      }
      self.last_accessibility = has_accessibility.then_some(semantic_snapshot);
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

  fn discard_session(&mut self, resource_completions: Option<FrozenCompletions>) {
    if let Some(resource_completions) = resource_completions {
      self
        .resources
        .cache
        .borrow_mut()
        .restore(resource_completions);
    }
    self.state = RuntimeState::Poisoned;
  }
}
