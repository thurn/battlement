use std::{
  any::{Any, TypeId},
  cell::RefCell,
  rc::Rc,
};

use crate::hook_storage::{HookKind, HookSlot};

pub(crate) type EffectCleanup = Box<dyn FnOnce()>;
pub(crate) type EffectSetup = Box<dyn FnOnce() -> Option<EffectCleanup>>;

pub(crate) struct EffectOperation {
  cleanup: Rc<RefCell<Option<EffectCleanup>>>,
  setup: Option<EffectSetup>,
}

pub(crate) struct EffectSlot<D> {
  pub(crate) committed_dependencies: D,
  pub(crate) rendered_dependencies: D,
  pub(crate) rendered_setup: Option<EffectSetup>,
  pub(crate) cleanup: Rc<RefCell<Option<EffectCleanup>>>,
  pub(crate) value_type: TypeId,
  pub(crate) always: bool,
  pub(crate) committed: bool,
  pub(crate) replace: bool,
}

impl EffectOperation {
  fn new(cleanup: Rc<RefCell<Option<EffectCleanup>>>, setup: Option<EffectSetup>) -> Self {
    Self { cleanup, setup }
  }

  pub(crate) fn run(self) {
    let cleanup = self.cleanup.borrow_mut().take();
    if let Some(cleanup) = cleanup {
      cleanup();
    }
    if let Some(setup) = self.setup {
      self.cleanup.replace(setup());
    }
  }

  pub(crate) fn run_cleanup(self) {
    let cleanup = self.cleanup.borrow_mut().take();
    if let Some(cleanup) = cleanup {
      cleanup();
    }
  }
}

impl<D> EffectSlot<D>
where
  D: Clone + PartialEq + 'static,
{
  pub(crate) fn new(dependencies: D, setup: EffectSetup, value_type: TypeId, always: bool) -> Self {
    Self {
      committed_dependencies: dependencies.clone(),
      rendered_dependencies: dependencies,
      rendered_setup: Some(setup),
      cleanup: Rc::new(RefCell::new(None)),
      value_type,
      always,
      committed: false,
      replace: true,
    }
  }

  pub(crate) fn prepare(&mut self, dependencies: D, setup: EffectSetup) {
    self.replace = !self.committed || self.always || self.committed_dependencies != dependencies;
    self.rendered_dependencies = dependencies;
    self.rendered_setup = self.replace.then_some(setup);
  }
}

impl<D> HookSlot for EffectSlot<D>
where
  D: Clone + PartialEq + 'static,
{
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    assert!(
      self.rendered_setup.is_none() && !self.replace,
      "Reactant cannot clone an uncommitted effect"
    );
    Box::new(Self {
      committed_dependencies: self.committed_dependencies.clone(),
      rendered_dependencies: self.rendered_dependencies.clone(),
      rendered_setup: None,
      cleanup: Rc::clone(&self.cleanup),
      value_type: self.value_type,
      always: self.always,
      committed: true,
      replace: false,
    })
  }

  fn commit(&mut self) {
    assert!(
      self.rendered_setup.is_none(),
      "Reactant effect setup was not queued"
    );
    self
      .committed_dependencies
      .clone_from(&self.rendered_dependencies);
    self.committed = true;
    self.replace = false;
  }

  fn discard_pending(&mut self) {
    self
      .rendered_dependencies
      .clone_from(&self.committed_dependencies);
    self.rendered_setup = None;
    self.replace = false;
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
    HookKind::Effect
  }

  fn value_type(&self) -> TypeId {
    self.value_type
  }

  fn take_effect_operation(&mut self) -> Option<EffectOperation> {
    self.replace.then(|| {
      EffectOperation::new(
        Rc::clone(&self.cleanup),
        Some(
          self
            .rendered_setup
            .take()
            .expect("changed Reactant effect has a setup"),
        ),
      )
    })
  }

  fn take_unmount_operation(&mut self) -> Option<EffectOperation> {
    Some(EffectOperation::new(Rc::clone(&self.cleanup), None))
  }
}
