//! Runtime resource administration.

use std::{
  error::Error,
  hash::Hash,
  panic::{self, AssertUnwindSafe},
};

use crate::{resource::Resource, runtime::Reactant};

impl<G: 'static> Reactant<G> {
  /// Starts loading a missing resource entry without mounting a consumer.
  pub fn preload<K, T, E>(&mut self, resource: &Resource<K, T, E>, key: K)
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + Sync + 'static,
    E: Error + Send + Sync + 'static,
  {
    self.require_open();
    let started = panic::catch_unwind(AssertUnwindSafe(|| {
      self.resources.request(resource, key, self.spawner.as_ref())
    }));
    if let Err(payload) = started {
      self.resume_resource_panic(payload);
    }
  }

  /// Removes one cached entry and requests cancellation of pending work.
  pub fn invalidate<K, T, E>(&mut self, resource: &Resource<K, T, E>, key: &K)
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + Sync + 'static,
    E: Error + Send + Sync + 'static,
  {
    self.require_open();
    let invalidated = panic::catch_unwind(AssertUnwindSafe(|| {
      self.resources.invalidate(resource, key)
    }));
    match invalidated {
      Ok(Ok(())) => {}
      Ok(Err(payload)) | Err(payload) => self.resume_resource_panic(payload),
    }
  }

  /// Removes every cached entry for one resource and cancels pending work.
  pub fn clear<K, T, E>(&mut self, resource: &Resource<K, T, E>)
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + Sync + 'static,
    E: Error + Send + Sync + 'static,
  {
    self.require_open();
    let cleared = panic::catch_unwind(AssertUnwindSafe(|| self.resources.clear(resource)));
    match cleared {
      Ok(Ok(())) => {}
      Ok(Err(payload)) | Err(payload) => self.resume_resource_panic(payload),
    }
  }

  #[allow(dead_code)]
  pub(crate) fn request_resource<K, T, E>(&mut self, resource: &Resource<K, T, E>, key: K) -> u64
  where
    K: Clone + Eq + Hash + Send + 'static,
    T: Send + 'static,
    E: Error + Send + Sync + 'static,
  {
    self.resources.request(resource, key, self.spawner.as_ref())
  }

  #[cfg(test)]
  pub(crate) fn resource_is_pending<K, T, E>(&self, resource: &Resource<K, T, E>, key: &K) -> bool
  where
    K: Eq + Hash + 'static,
    T: 'static,
    E: 'static,
  {
    self.resources.is_pending(resource, key)
  }
}
