//! Resource invalidation requested from component callbacks.

use std::{
  error::Error,
  hash::Hash,
  rc::{Rc, Weak},
};

use crate::{
  action_context, hooks,
  resource::Resource,
  resource_runtime::{self, ResourceOperation, ResourceRuntime},
};

/// A runtime-bound handle for invalidating one resource's cached entries.
pub struct ResourceControl<K, T, E> {
  runtime: Weak<ResourceRuntime>,
  resource: Resource<K, T, E>,
  generation: u64,
}

/// Returns a stable invalidation handle for a resource in the current runtime.
pub fn use_resource_control<K, T, E>(resource: &Resource<K, T, E>) -> ResourceControl<K, T, E>
where
  K: Clone + Eq + Hash + Send + 'static,
  T: Send + Sync + 'static,
  E: Error + Send + Sync + 'static,
{
  let runtime = resource_runtime::current();
  let resource = resource.clone();
  let generation = runtime.generation.get();
  let identity = Rc::as_ptr(&runtime) as usize;
  let dependencies = (generation, identity, resource.clone());
  hooks::use_memo(
    move || ResourceControl {
      runtime: Rc::downgrade(&runtime),
      resource,
      generation,
    },
    dependencies,
  )
}

impl<K, T, E> Clone for ResourceControl<K, T, E> {
  fn clone(&self) -> Self {
    Self {
      runtime: self.runtime.clone(),
      resource: self.resource.clone(),
      generation: self.generation,
    }
  }
}

impl<K, T, E> ResourceControl<K, T, E>
where
  K: Clone + Eq + Hash + Send + 'static,
  T: Send + Sync + 'static,
  E: Error + Send + Sync + 'static,
{
  /// Cancels a pending load, forgets its value, and rerenders subscribed consumers.
  pub fn invalidate(&self, key: K) {
    if let Some(runtime) = self.runtime.upgrade() {
      if runtime.generation.get() != self.generation {
        return;
      }
      let resource = self.resource.clone();
      runtime.operations.borrow_mut().push(ResourceOperation {
        action: action_context::current(),
        run: Box::new(move |cache| cache.invalidate(&resource, &key)),
      });
    }
  }
}
