use battlement::ObjectId;
use std::{
  cell::RefCell,
  collections::HashMap,
  rc::{Rc, Weak},
};

type Leases = RefCell<HashMap<ObjectId, Weak<NativeIdentityLease>>>;

#[derive(Default)]
pub(crate) struct NativeIdentityLeases(Rc<Leases>);

#[derive(Debug)]
pub(crate) struct NativeIdentityLease {
  object_id: ObjectId,
  owner: Weak<Leases>,
}

impl NativeIdentityLeases {
  pub(crate) fn contains(&self, object_id: ObjectId) -> bool {
    self.0.borrow().contains_key(&object_id)
  }
  pub(crate) fn retain(&self, object_id: ObjectId) -> Rc<NativeIdentityLease> {
    let mut leases = self.0.borrow_mut();
    if let Some(lease) = leases.get(&object_id).and_then(Weak::upgrade) {
      return lease;
    }
    let lease = Rc::new(NativeIdentityLease {
      object_id,
      owner: Rc::downgrade(&self.0),
    });
    leases.insert(object_id, Rc::downgrade(&lease));
    lease
  }
}
impl Drop for NativeIdentityLease {
  fn drop(&mut self) {
    if let Some(owner) = self.owner.upgrade() {
      owner.borrow_mut().remove(&self.object_id);
    }
  }
}
