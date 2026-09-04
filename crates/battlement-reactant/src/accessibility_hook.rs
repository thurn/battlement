use std::any::{Any, TypeId};

use crate::{
  hook_storage::{HookKind, HookSlot},
  hooks,
};

pub(crate) fn use_pattern(name: &'static str) {
  hooks::use_slot(
    HookKind::Accessibility,
    TypeId::of::<PatternSlot>(),
    |_| PatternSlot { name },
    |slot| {
      assert_eq!(
        slot.name, name,
        "Reactant accessibility hook changed in a stable hook slot"
      );
    },
  );
}

#[derive(Clone)]
struct PatternSlot {
  name: &'static str,
}

impl HookSlot for PatternSlot {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(self.clone())
  }

  fn commit(&mut self) {}

  fn discard_pending(&mut self) {}

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
    HookKind::Accessibility
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<Self>()
  }
}
