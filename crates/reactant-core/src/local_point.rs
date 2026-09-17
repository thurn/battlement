//! Typed local points resolve to immutable native identities before dependent work.

use crate::{native_host::ObjectRef, native_identity_lease::NativeIdentityLease};
use battlement::{ObjectId, Vector3};
use std::rc::Rc;

/// A required world-object reference plus a local-space offset.
#[derive(Clone)]
pub struct LocalPoint {
  reference: ObjectRef,
  offset: Vector3,
}

/// How a consumer samples a local point during native playback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointTracking {
  /// Resample the original native object's transformed local point while playing.
  FollowLive,
  /// Sample once when the dependent operation starts, after earlier commands.
  CaptureAtStart,
}

/// A local point with an explicit playback sampling policy.
#[derive(Clone)]
pub struct LocalPointTarget {
  point: LocalPoint,
  tracking: PointTracking,
}

/// A validated target bound to one native handle; it never follows a ref reassignment.
#[derive(Clone, Debug)]
pub struct ResolvedLocalPoint {
  _lease: Rc<NativeIdentityLease>,
  object_id: ObjectId,
  offset: Vector3,
  tracking: PointTracking,
}

impl LocalPoint {
  pub(crate) fn new(reference: ObjectRef, offset: Vector3) -> Self {
    assert!(
      [offset.x, offset.y, offset.z]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= f64::from(f32::MAX)),
      "local point offset must fit native coordinates"
    );
    Self { reference, offset }
  }
  /// Offset in the referenced object's own coordinates, before all parent transforms.
  pub fn offset(&self) -> Vector3 {
    self.offset
  }
  /// Follows the referenced native transform throughout playback.
  pub fn follow(&self) -> LocalPointTarget {
    LocalPointTarget {
      point: self.clone(),
      tracking: PointTracking::FollowLive,
    }
  }
  /// Captures position when playback begins, independently of Rust render time.
  pub fn capture_at_start(&self) -> LocalPointTarget {
    LocalPointTarget {
      point: self.clone(),
      tracking: PointTracking::CaptureAtStart,
    }
  }
}

impl LocalPointTarget {
  /// Validates a required committed reference before submitting dependent commands.
  /// Consumers retain this resolved handle, never the mutable reference, through exits.
  pub fn resolve(&self) -> ResolvedLocalPoint {
    let object_id = self
      .point
      .reference
      .object_id()
      .expect("required local-point reference is not attached");
    ResolvedLocalPoint {
      _lease: self.point.reference.retain_native_identity(object_id),
      object_id,
      offset: self.point.offset,
      tracking: self.tracking,
    }
  }
  /// Explicit native sampling policy; resolving never advances presentation time.
  pub fn tracking(&self) -> PointTracking {
    self.tracking
  }
}

impl ResolvedLocalPoint {
  /// The original native object, including a terminal host awaiting queued removal.
  pub fn object_id(&self) -> ObjectId {
    self.object_id
  }
  /// Local-space position on the original native object.
  pub fn offset(&self) -> Vector3 {
    self.offset
  }
  /// Sampling policy to apply when dependent playback starts.
  pub fn tracking(&self) -> PointTracking {
    self.tracking
  }
}
