use std::{
  any::{Any, TypeId},
  sync::atomic::{AtomicU64, Ordering},
};

use crate::{
  hook_storage::{HookKind, HookSlot},
  hooks,
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) fn read() -> String {
  hooks::use_slot(
    HookKind::Id,
    TypeId::of::<String>(),
    |_| IdSlot {
      value: format!(
        "reactant-{}",
        NEXT_ID
          .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
          .expect("Reactant hook ID space exhausted")
      ),
    },
    |slot| slot.value.clone(),
  )
}

#[derive(Clone)]
struct IdSlot {
  value: String,
}

impl HookSlot for IdSlot {
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
    HookKind::Id
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<String>()
  }
}
