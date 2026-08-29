//! Coherent geometry targets, measurements, and observation registration.

#![allow(private_interfaces)]

use std::{
  any::{Any, TypeId},
  array,
  cell::RefCell,
  rc::{Rc, Weak},
};

use battlement::{
  AnchorName, CameraTarget, DisplayId, ElementGeometry, GeometryGeneration,
  GeometryObservationTarget, GeometryUnavailable, ObjectId, ViewportGeometry, WorldBoundsGeometry,
  WorldPointGeometry,
};

use crate::{
  element_ref::ElementRef,
  geometry_runtime::GeometryRuntime,
  hook_storage::{HookKind, HookSlot},
  hooks,
};

thread_local! {
  static CURRENT_RUNTIME: RefCell<Option<RuntimeContext>> = const { RefCell::new(None) };
}

/// Marks supported geometry target shapes.
pub trait GeometryTargets: private::Sealed {
  /// Measurements preserving this target shape.
  type Measurements;

  #[doc(hidden)]
  fn collect_targets(&self, targets: &mut Vec<GeometryTarget>);

  #[doc(hidden)]
  fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements;
}

/// A geometry measurement and the host's current knowledge about it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measurement<T> {
  /// The latest usable value, when one is available.
  pub latest: Option<T>,
  /// Whether the target is waiting, current, or temporarily unavailable.
  pub status: MeasurementStatus,
}

/// The current sampling state for a geometry target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurementStatus {
  /// No complete sample covers the current observation target set.
  Waiting,
  /// The latest value belongs to the snapshot generation.
  Current,
  /// The host could not sample the target in the snapshot generation.
  Unavailable(GeometryUnavailable),
}

/// One coherent generation of geometry measurements.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometrySnapshot<T> {
  /// The native sampling pass shared by every measurement.
  pub generation: Option<GeometryGeneration>,
  /// Measurements preserving the input target shape.
  pub measurements: T,
}

/// An immutable projected-world geometry target.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WorldRef {
  pub(crate) target: GeometryObservationTarget,
}

/// An immutable physical-display geometry target.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ViewportRef {
  pub(crate) display_id: DisplayId,
}

/// Geometry returned by a world target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldGeometry {
  /// A projected object origin or named anchor.
  Point(WorldPointGeometry),
  /// Projected bounds for enabled renderers.
  Bounds(WorldBoundsGeometry),
}

/// Returns a coherent snapshot for one supported target shape.
pub fn use_geometry<T>(targets: T) -> GeometrySnapshot<T::Measurements>
where
  T: GeometryTargets,
{
  let mut flattened = Vec::new();
  targets.collect_targets(&mut flattened);
  hooks::use_slot(
    HookKind::Geometry,
    TypeId::of::<GeometrySlot>(),
    |_| GeometrySlot {
      committed: Vec::new(),
      rendered: flattened.clone(),
    },
    |slot| slot.rendered.clone_from(&flattened),
  );
  self::with_runtime(|runtime| GeometrySnapshot {
    generation: runtime.snapshot_generation(&flattened),
    measurements: targets.read_measurements(runtime),
  })
}

#[derive(Clone)]
pub(crate) enum GeometryTarget {
  Element(ElementRef),
  Viewport(ViewportRef),
  World(WorldRef),
}

pub(crate) struct GeometrySlot {
  committed: Vec<GeometryTarget>,
  rendered: Vec<GeometryTarget>,
}

pub(crate) struct RuntimeGuard(Option<RuntimeContext>);

#[derive(Clone)]
struct RuntimeContext {
  runtime: Weak<RefCell<GeometryRuntime>>,
}

impl<T> Measurement<T> {
  pub(crate) fn waiting() -> Self {
    Self {
      latest: None,
      status: MeasurementStatus::Waiting,
    }
  }
}

impl WorldRef {
  /// Projects an object's root transform origin through a camera.
  #[must_use]
  pub fn origin(object_id: ObjectId, camera: CameraTarget) -> Self {
    Self {
      target: GeometryObservationTarget::WorldOrigin { object_id, camera },
    }
  }

  /// Projects one named authored anchor through a camera.
  #[must_use]
  pub fn named_anchor(
    object_id: ObjectId,
    anchor: impl Into<AnchorName>,
    camera: CameraTarget,
  ) -> Self {
    let anchor = anchor.into();
    assert!(
      !anchor.0.is_empty(),
      "world geometry anchor cannot be empty"
    );
    Self {
      target: GeometryObservationTarget::WorldAnchor {
        object_id,
        anchor,
        camera,
      },
    }
  }

  /// Projects the bounds of enabled renderers through a camera.
  #[must_use]
  pub fn rendered_bounds(object_id: ObjectId, camera: CameraTarget) -> Self {
    Self {
      target: GeometryObservationTarget::WorldRenderedBounds { object_id, camera },
    }
  }
}

impl ViewportRef {
  /// Observes one physical display viewport and safe area.
  #[must_use]
  pub const fn display(display_id: DisplayId) -> Self {
    Self { display_id }
  }
}

impl HookSlot for GeometrySlot {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }

  fn clone_box(&self) -> Box<dyn HookSlot> {
    Box::new(Self {
      committed: self.committed.clone(),
      rendered: self.rendered.clone(),
    })
  }

  fn commit(&mut self) {
    self.committed.clone_from(&self.rendered);
  }

  fn discard_pending(&mut self) {
    self.rendered.clone_from(&self.committed);
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
    HookKind::Geometry
  }

  fn value_type(&self) -> TypeId {
    TypeId::of::<GeometrySlot>()
  }

  fn geometry_targets(&self, targets: &mut Vec<GeometryTarget>) {
    targets.extend(self.rendered.iter().cloned());
  }
}

impl GeometryTargets for ElementRef {
  type Measurements = Measurement<ElementGeometry>;

  fn collect_targets(&self, targets: &mut Vec<GeometryTarget>) {
    targets.push(GeometryTarget::Element(self.clone()));
  }

  fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements {
    runtime.element(self)
  }
}

impl GeometryTargets for ViewportRef {
  type Measurements = Measurement<ViewportGeometry>;

  fn collect_targets(&self, targets: &mut Vec<GeometryTarget>) {
    targets.push(GeometryTarget::Viewport(*self));
  }

  fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements {
    runtime.viewport(*self)
  }
}

impl GeometryTargets for WorldRef {
  type Measurements = Measurement<WorldGeometry>;

  fn collect_targets(&self, targets: &mut Vec<GeometryTarget>) {
    targets.push(GeometryTarget::World(self.clone()));
  }

  fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements {
    runtime.world(self)
  }
}

impl<T, const N: usize> GeometryTargets for [T; N]
where
  T: GeometryTargets,
{
  type Measurements = [T::Measurements; N];

  fn collect_targets(&self, targets: &mut Vec<GeometryTarget>) {
    for target in self {
      target.collect_targets(targets);
    }
  }

  fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements {
    array::from_fn(|index| self[index].read_measurements(runtime))
  }
}

impl<T> GeometryTargets for Vec<T>
where
  T: GeometryTargets,
{
  type Measurements = Vec<T::Measurements>;

  fn collect_targets(&self, targets: &mut Vec<GeometryTarget>) {
    for target in self {
      target.collect_targets(targets);
    }
  }

  fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements {
    self
      .iter()
      .map(|target| target.read_measurements(runtime))
      .collect()
  }
}

macro_rules! geometry_tuple {
  ($($name:ident:$index:tt),+) => {
    impl<$($name),+> GeometryTargets for ($($name,)+)
    where
      $($name: GeometryTargets,)+
    {
      type Measurements = ($($name::Measurements,)+);

      fn collect_targets(&self, targets: &mut Vec<GeometryTarget>) {
        $(self.$index.collect_targets(targets);)+
      }

      fn read_measurements(&self, runtime: &GeometryRuntime) -> Self::Measurements {
        ($(self.$index.read_measurements(runtime),)+)
      }
    }
  };
}

geometry_tuple!(A:0);
geometry_tuple!(A:0, B:1);
geometry_tuple!(A:0, B:1, C:2);
geometry_tuple!(A:0, B:1, C:2, D:3);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10);
geometry_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11);

impl private::Sealed for ElementRef {}
impl private::Sealed for ViewportRef {}
impl private::Sealed for WorldRef {}
impl<T: GeometryTargets, const N: usize> private::Sealed for [T; N] {}
impl<T: GeometryTargets> private::Sealed for Vec<T> {}

macro_rules! sealed_tuple {
  ($($name:ident),+) => {
    impl<$($name: GeometryTargets),+> private::Sealed for ($($name,)+) {}
  };
}

sealed_tuple!(A);
sealed_tuple!(A, B);
sealed_tuple!(A, B, C);
sealed_tuple!(A, B, C, D);
sealed_tuple!(A, B, C, D, E);
sealed_tuple!(A, B, C, D, E, F);
sealed_tuple!(A, B, C, D, E, F, G);
sealed_tuple!(A, B, C, D, E, F, G, H);
sealed_tuple!(A, B, C, D, E, F, G, H, I);
sealed_tuple!(A, B, C, D, E, F, G, H, I, J);
sealed_tuple!(A, B, C, D, E, F, G, H, I, J, K);
sealed_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

impl Drop for RuntimeGuard {
  fn drop(&mut self) {
    CURRENT_RUNTIME.with(|current| current.replace(self.0.take()));
  }
}

pub(crate) fn enter_runtime(runtime: &Rc<RefCell<GeometryRuntime>>) -> RuntimeGuard {
  RuntimeGuard(CURRENT_RUNTIME.with(|current| {
    current.replace(Some(RuntimeContext {
      runtime: Rc::downgrade(runtime),
    }))
  }))
}

fn with_runtime<R>(read: impl FnOnce(&GeometryRuntime) -> R) -> R {
  CURRENT_RUNTIME.with(|current| {
    let current = current.borrow();
    let runtime = current
      .as_ref()
      .expect("Reactant geometry hooks require a runtime render context")
      .runtime
      .upgrade()
      .expect("Reactant geometry runtime is no longer available");
    read(&runtime.borrow())
  })
}

mod private {
  pub trait Sealed {}
}
