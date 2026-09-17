//! Admission follows the response allocation until its final host owner releases it.

use std::{
  collections::HashMap,
  sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
};

use crate::EngineResponse;

static NATIVE_LEASES: LazyLock<Mutex<HashMap<usize, ResponseLease>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

/// A cumulative response-allocation budget, independent of playback completion.
#[derive(Clone)]
pub struct ResponseBudget {
  live: Arc<AtomicUsize>,
  limit: usize,
}

/// Shared allocation admission retained by queued work and borrowed host content.
#[derive(Clone)]
pub struct ResponseLease(Arc<Allocation>);

struct Allocation {
  live: Arc<AtomicUsize>,
  bytes: usize,
}

impl ResponseBudget {
  /// Creates a budget in allocated bytes, including unused builder storage.
  pub fn new(limit: usize) -> Self {
    assert!(limit > 0, "response budget must be positive");
    Self {
      live: Arc::new(AtomicUsize::new(0)),
      limit,
    }
  }

  /// Bytes whose final host use has not ended.
  pub fn retained_bytes(&self) -> usize {
    self.live.load(Ordering::Acquire)
  }

  /// Capacity available before consuming another publication.
  pub fn available_bytes(&self) -> usize {
    self.limit.saturating_sub(self.retained_bytes())
  }

  /// Reserves before publication, or leaves the response untouched on backpressure.
  pub fn admit(&self, response: &mut EngineResponse) -> bool {
    assert!(response.lease.is_none(), "response already admitted");
    let bytes = response.allocation_bytes();
    let admitted = self
      .live
      .fetch_update(Ordering::AcqRel, Ordering::Acquire, |live| {
        live.checked_add(bytes).filter(|total| *total <= self.limit)
      })
      .is_ok();
    if admitted {
      response.lease = Some(ResponseLease(Arc::new(Allocation {
        live: Arc::clone(&self.live),
        bytes,
      })));
    }
    admitted
  }
}

impl ResponseLease {
  /// The complete retained allocation, counted once across all clones.
  pub fn allocation_bytes(&self) -> usize {
    self.0.bytes
  }
}

impl Drop for Allocation {
  fn drop(&mut self) {
    self.live.fetch_sub(self.bytes, Ordering::AcqRel);
  }
}

pub(crate) fn handoff(pointer: usize, lease: Option<ResponseLease>) {
  if let Some(lease) = lease {
    assert!(
      NATIVE_LEASES
        .lock()
        .expect("native response leases poisoned")
        .insert(pointer, lease)
        .is_none(),
      "native allocation already retained"
    );
  }
}

pub(crate) fn release(pointer: usize) {
  NATIVE_LEASES
    .lock()
    .expect("native response leases poisoned")
    .remove(&pointer);
}
