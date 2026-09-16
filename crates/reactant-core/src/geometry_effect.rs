//! Deferred model-aware operations selected by coherent geometry snapshots.

use std::{
  any::{Any, TypeId},
  cell::RefCell,
  rc::Rc,
};

use crate::{
  geometry::{GeometrySnapshot, GeometryTarget, GeometryTargets, IntoGeometryEffectCleanup},
  geometry_runtime::GeometryRuntime,
  hook_storage::{HookKind, HookSlot},
  hooks::Dependencies,
};

type GeometryCleanup = Box<dyn FnOnce(&mut dyn Any)>;
type GeometryRun = Box<dyn FnOnce(&mut dyn Any) -> Option<GeometryCleanup>>;
type GeometryReader<M> = Rc<dyn Fn(&GeometryRuntime) -> GeometrySnapshot<M>>;
type GeometrySetup<M> =
  Box<dyn FnOnce(&mut dyn Any, GeometrySnapshot<M>) -> Option<GeometryCleanup>>;

pub(crate) struct GeometryEffectOperation {
  cleanup: Rc<RefCell<Option<GeometryCleanup>>>,
  setup: Option<GeometryRun>,
}

pub(crate) struct GeometryEffectSlot<D, M>
where
  D: Dependencies,
  M: Clone + PartialEq + 'static,
{
  committed_dependencies: D,
  rendered_dependencies: D,
  committed_targets: Vec<GeometryTarget>,
  rendered_targets: Vec<GeometryTarget>,
  committed_reader: GeometryReader<M>,
  rendered_reader: Option<GeometryReader<M>>,
  rendered_setup: Option<GeometrySetup<M>>,
  setup: Rc<RefCell<Option<GeometrySetup<M>>>>,
  cleanup: Rc<RefCell<Option<GeometryCleanup>>>,
  last_dependencies: Option<D>,
  last_measurements: Option<M>,
  last_targets: Option<Vec<GeometryTarget>>,
  value_type: TypeId,
  model_type: TypeId,
}

impl GeometryEffectOperation {
  fn new(cleanup: Rc<RefCell<Option<GeometryCleanup>>>, setup: Option<GeometryRun>) -> Self {
    Self { cleanup, setup }
  }

  pub(crate) fn run<G: 'static>(self, game: &mut G) {
    if let Some(cleanup) = self.cleanup.borrow_mut().take() {
      cleanup(game);
    }
    if let Some(setup) = self.setup {
      self.cleanup.replace(setup(game));
    }
  }
}

impl<D, M> GeometryEffectSlot<D, M>
where
  D: Dependencies,
  M: Clone + PartialEq + 'static,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    dependencies: D,
    targets: Vec<GeometryTarget>,
    reader: GeometryReader<M>,
    setup: GeometrySetup<M>,
    value_type: TypeId,
    model_type: TypeId,
  ) -> Self {
    Self {
      committed_dependencies: dependencies.clone(),
      rendered_dependencies: dependencies,
      committed_targets: targets.clone(),
      rendered_targets: targets,
      committed_reader: Rc::clone(&reader),
      rendered_reader: Some(reader),
      rendered_setup: Some(setup),
      setup: Rc::new(RefCell::new(None)),
      cleanup: Rc::new(RefCell::new(None)),
      last_dependencies: None,
      last_measurements: None,
      last_targets: None,
      value_type,
      model_type,
    }
  }

  pub(crate) fn prepare(
    &mut self,
    dependencies: D,
    targets: Vec<GeometryTarget>,
    reader: GeometryReader<M>,
    setup: GeometrySetup<M>,
  ) {
    self.rendered_dependencies = dependencies;
    self.rendered_targets = targets;
    self.rendered_reader = Some(reader);
    self.rendered_setup = Some(setup);
  }
}

impl<D, M> HookSlot for GeometryEffectSlot<D, M>
where
  D: Dependencies,
  M: Clone + PartialEq + 'static,
{
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    assert!(
      self.rendered_reader.is_none() && self.rendered_setup.is_none(),
      "Reactant cannot clone an uncommitted geometry effect"
    );
    Box::new(Self {
      committed_dependencies: self.committed_dependencies.clone(),
      rendered_dependencies: self.rendered_dependencies.clone(),
      committed_targets: self.committed_targets.clone(),
      rendered_targets: self.rendered_targets.clone(),
      committed_reader: Rc::clone(&self.committed_reader),
      rendered_reader: None,
      rendered_setup: None,
      setup: Rc::clone(&self.setup),
      cleanup: Rc::clone(&self.cleanup),
      last_dependencies: self.last_dependencies.clone(),
      last_measurements: self.last_measurements.clone(),
      last_targets: self.last_targets.clone(),
      value_type: self.value_type,
      model_type: self.model_type,
    })
  }

  fn commit(&mut self) {
    self
      .committed_dependencies
      .clone_from(&self.rendered_dependencies);
    self.committed_targets.clone_from(&self.rendered_targets);
    self.committed_reader = self
      .rendered_reader
      .take()
      .expect("Reactant geometry effect render has a reader");
    self.setup.replace(Some(
      self
        .rendered_setup
        .take()
        .expect("Reactant geometry effect render has a setup"),
    ));
  }

  fn discard_pending(&mut self) {
    self
      .rendered_dependencies
      .clone_from(&self.committed_dependencies);
    self.rendered_targets.clone_from(&self.committed_targets);
    self.rendered_reader = None;
    self.rendered_setup = None;
  }

  fn has_pending(&self) -> bool {
    false
  }

  fn has_pending_change(&self) -> bool {
    false
  }

  fn context_changed(&self) -> bool {
    false
  }

  fn kind(&self) -> HookKind {
    HookKind::GeometryEffect
  }

  fn value_type(&self) -> TypeId {
    self.value_type
  }

  fn geometry_effect_model(&self) -> Option<TypeId> {
    Some(self.model_type)
  }

  fn take_geometry_effect_operation(
    &mut self,
    runtime: &GeometryRuntime,
  ) -> Option<GeometryEffectOperation> {
    let snapshot = (self.committed_reader)(runtime);
    snapshot.generation?;
    let dependencies_changed = self
      .last_dependencies
      .as_ref()
      .is_none_or(|value| value != &self.committed_dependencies);
    let measurements_changed = self
      .last_measurements
      .as_ref()
      .is_none_or(|value| value != &snapshot.measurements);
    let targets_changed = self
      .last_targets
      .as_ref()
      .is_none_or(|value| value != &self.committed_targets);
    if !dependencies_changed && !measurements_changed && !targets_changed {
      return None;
    }
    let setup = self
      .setup
      .borrow_mut()
      .take()
      .expect("selected Reactant geometry effect has a committed setup");
    self.last_dependencies = Some(self.committed_dependencies.clone());
    self.last_measurements = Some(snapshot.measurements.clone());
    self.last_targets = Some(self.committed_targets.clone());
    Some(GeometryEffectOperation::new(
      Rc::clone(&self.cleanup),
      Some(Box::new(move |game| setup(game, snapshot))),
    ))
  }

  fn take_geometry_unmount_operation(&mut self) -> Option<GeometryEffectOperation> {
    self
      .cleanup
      .borrow()
      .is_some()
      .then(|| GeometryEffectOperation::new(Rc::clone(&self.cleanup), None))
  }

  fn geometry_targets(&self, targets: &mut Vec<GeometryTarget>) {
    targets.extend(self.rendered_targets.iter().cloned());
  }
}

pub(crate) fn reader<T>(targets: Rc<T>) -> GeometryReader<T::Measurements>
where
  T: GeometryTargets + 'static,
{
  Rc::new(move |runtime| {
    let mut flattened = Vec::new();
    targets.collect_targets(&mut flattened);
    GeometrySnapshot {
      generation: runtime.snapshot_generation(&flattened),
      measurements: targets.read_measurements(runtime),
    }
  })
}

pub(crate) fn setup<G, M, S, C>(setup: S) -> GeometrySetup<M>
where
  G: 'static,
  M: 'static,
  S: FnOnce(&mut G, GeometrySnapshot<M>) -> C + 'static,
  C: IntoGeometryEffectCleanup<G>,
{
  Box::new(move |game, snapshot| {
    let cleanup = setup(
      game
        .downcast_mut::<G>()
        .expect("validated Reactant geometry effect model"),
      snapshot,
    )
    .into_cleanup();
    cleanup.map(|cleanup| {
      Box::new(move |game: &mut dyn Any| {
        cleanup(
          game
            .downcast_mut::<G>()
            .expect("validated Reactant geometry cleanup model"),
        );
      }) as GeometryCleanup
    })
  })
}
