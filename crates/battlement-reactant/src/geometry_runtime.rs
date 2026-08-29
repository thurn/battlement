//! Runtime-owned geometry registry state and transactional plans.

use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use battlement::{
  CommandBody, ElementGeometry, GeometryGeneration, GeometryObservation, GeometryObservationBatch,
  GeometryObservationId, GeometryObservationResult, GeometryObservationTarget,
  GeometryObservationUpdate, GeometryRegistry, GeometryValidationError, GeometryValue, ObjectId,
  ViewportGeometry,
};

use crate::{
  element_ref::{AttachmentSet, ElementRef},
  geometry::{
    GeometryTarget, Measurement, MeasurementStatus, ViewportRef, WorldGeometry, WorldRef,
  },
  render::RenderTree,
};

pub(crate) struct GeometryRuntime {
  runtime_id: u64,
  registry: GeometryRegistry,
  entries: HashMap<TargetKey, TargetEntry>,
  cache: HashMap<TargetKey, GeometryValue>,
  retired: HashSet<GeometryObservationId>,
  element_objects: Option<HashMap<u64, ObjectId>>,
  order: Vec<TargetKey>,
  pub(crate) generation: Option<GeometryGeneration>,
  revision: u64,
  dirty: bool,
}

pub(crate) struct GeometryPlan {
  registry: GeometryRegistry,
  entries: HashMap<TargetKey, TargetEntry>,
  order: Vec<TargetKey>,
  generation: Option<GeometryGeneration>,
  retired: HashSet<GeometryObservationId>,
  element_targets: HashSet<u64>,
  removed: Vec<GeometryObservationId>,
  added: Vec<GeometryObservation>,
  revision: u64,
  dirty: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum TargetKey {
  Element { identity: u64, object_id: ObjectId },
  Native(GeometryObservationTarget),
}

#[derive(Clone)]
struct TargetEntry {
  observation_id: GeometryObservationId,
  result: Option<GeometryObservationResult>,
  latest: Option<GeometryValue>,
}

impl GeometryRuntime {
  pub(crate) fn new(runtime_id: u64) -> Rc<RefCell<Self>> {
    Rc::new(RefCell::new(Self {
      runtime_id,
      registry: GeometryRegistry::default(),
      entries: HashMap::new(),
      cache: HashMap::new(),
      retired: HashSet::new(),
      element_objects: None,
      order: Vec::new(),
      generation: None,
      revision: 0,
      dirty: false,
    }))
  }

  pub(crate) const fn revision(&self) -> u64 {
    self.revision
  }

  pub(crate) const fn dirty(&self) -> bool {
    self.dirty
  }

  pub(crate) fn waiting_preview(&self) -> Rc<RefCell<Self>> {
    Rc::new(RefCell::new(Self {
      runtime_id: self.runtime_id,
      registry: GeometryRegistry::default(),
      entries: HashMap::new(),
      cache: self.cache.clone(),
      retired: self.retired.clone(),
      element_objects: Some(
        self
          .entries
          .keys()
          .filter_map(|key| match key {
            TargetKey::Element {
              identity,
              object_id,
            } => Some((*identity, *object_id)),
            TargetKey::Native(_) => None,
          })
          .collect(),
      ),
      order: Vec::new(),
      generation: None,
      revision: self.revision,
      dirty: false,
    }))
  }

  pub(crate) fn preview(&self, plan: &GeometryPlan) -> Rc<RefCell<Self>> {
    Rc::new(RefCell::new(Self {
      runtime_id: self.runtime_id,
      registry: plan.registry.clone(),
      entries: plan.entries.clone(),
      cache: self.cache.clone(),
      retired: plan.retired.clone(),
      element_objects: Some(
        plan
          .entries
          .keys()
          .filter_map(|key| match key {
            TargetKey::Element {
              identity,
              object_id,
            } => Some((*identity, *object_id)),
            TargetKey::Native(_) => None,
          })
          .collect(),
      ),
      order: plan.order.clone(),
      generation: plan.generation,
      revision: plan.revision,
      dirty: plan.dirty,
    }))
  }

  pub(crate) fn acknowledge_render(&mut self, revision: u64) {
    if self.revision == revision {
      self.dirty = false;
    }
  }

  pub(crate) fn stabilize_hosts(&self, trees: &mut [RenderTree]) {
    let Some(object_ids) = &self.element_objects else {
      return;
    };
    for tree in trees {
      tree.stabilize_element_hosts(object_ids);
    }
  }

  pub(crate) fn plan(
    &self,
    targets: &[GeometryTarget],
    attachments: &AttachmentSet,
    reconnect: bool,
  ) -> Result<GeometryPlan, GeometryValidationError> {
    let previous_entries = (!reconnect).then_some(&self.entries);
    let previous_order = (!reconnect).then_some(&self.order);
    let mut registry = if reconnect {
      GeometryRegistry::default()
    } else {
      self.registry.clone()
    };
    let mut retired = self.retired.clone();
    if reconnect {
      retired.extend(self.entries.values().map(|entry| entry.observation_id));
    }
    let mut desired = Vec::new();
    let mut seen = HashSet::new();
    let mut element_targets = HashSet::new();
    for target in targets {
      if let GeometryTarget::Element(element_ref) = target {
        let (runtime_id, identity, _) = element_ref.geometry_identity();
        assert_eq!(
          self.runtime_id, runtime_id,
          "Reactant geometry targets cannot cross runtimes"
        );
        element_targets.insert(identity);
      }
      let Some((key, target)) = self.resolve(target, attachments) else {
        continue;
      };
      if seen.insert(key.clone()) {
        desired.push((key, target));
      }
    }
    let desired_keys = desired
      .iter()
      .map(|(key, _)| key.clone())
      .collect::<HashSet<_>>();
    let removed = previous_order
      .into_iter()
      .flatten()
      .filter(|key| !desired_keys.contains(*key))
      .map(|key| previous_entries.expect("paired previous geometry state")[key].observation_id)
      .collect::<Vec<_>>();
    if !removed.is_empty() {
      retired.extend(removed.iter().copied());
      registry.apply_update(&GeometryObservationUpdate {
        added: Vec::new(),
        removed: removed.clone(),
      })?;
    }
    let mut entries = HashMap::new();
    let mut added = Vec::new();
    let mut order = Vec::with_capacity(desired.len());
    for (key, target) in desired {
      order.push(key.clone());
      if let Some(entry) = previous_entries.and_then(|entries| entries.get(&key)) {
        entries.insert(key, entry.clone());
        continue;
      }
      let observation_id = GeometryObservationId(ObjectId::new_v4());
      added.push(GeometryObservation {
        observation_id,
        target,
      });
      let latest = self.cache.get(&key).copied();
      entries.insert(
        key,
        TargetEntry {
          observation_id,
          result: None,
          latest,
        },
      );
    }
    if !added.is_empty() {
      registry.apply_update(&GeometryObservationUpdate {
        added: added.clone(),
        removed: Vec::new(),
      })?;
    }
    let registry_changed = !removed.is_empty() || !added.is_empty();
    let changed = registry_changed || (reconnect && !self.entries.is_empty());
    let generation = if added.is_empty() && !entries.is_empty() {
      self.generation
    } else {
      None
    };
    Ok(GeometryPlan {
      registry,
      entries,
      order,
      generation,
      retired,
      element_targets,
      removed,
      added,
      revision: self
        .revision
        .checked_add(u64::from(changed))
        .expect("geometry revision overflowed"),
      dirty: self.dirty || changed,
    })
  }

  pub(crate) fn commit(&mut self, plan: GeometryPlan) {
    self.registry = plan.registry;
    self.entries = plan.entries;
    self.retired = plan.retired;
    self.order = plan.order;
    self.generation = plan.generation;
    self.revision = plan.revision;
    self.dirty = plan.dirty;
  }

  pub(crate) fn accept(
    &mut self,
    batch: &GeometryObservationBatch,
  ) -> Result<(), GeometryValidationError> {
    if batch
      .changed
      .iter()
      .any(|value| self.retired.contains(&value.observation_id))
    {
      return Ok(());
    }
    let mut registry = self.registry.clone();
    registry.accept_batch(batch)?;
    let mut entries = self.entries.clone();
    for value in &batch.changed {
      let key = entries
        .iter()
        .find_map(|(key, entry)| {
          (entry.observation_id == value.observation_id).then(|| key.clone())
        })
        .ok_or(GeometryValidationError::UnknownId)?;
      let entry = entries.get_mut(&key).expect("geometry entry key exists");
      entry.result = Some(value.result);
    }
    let generation = (!entries.is_empty() && entries.values().all(|entry| entry.result.is_some()))
      .then_some(batch.generation);
    let mut cache = self.cache.clone();
    if generation.is_some() {
      for (key, entry) in &mut entries {
        if let Some(GeometryObservationResult::Current(current)) = entry.result {
          entry.latest = Some(current);
          cache.insert(key.clone(), current);
        }
      }
    }
    self.registry = registry;
    self.entries = entries;
    self.cache = cache;
    self.generation = generation;
    if generation.is_some() {
      self.revision = self
        .revision
        .checked_add(1)
        .expect("Reactant geometry revision overflow");
      self.dirty = true;
    }
    Ok(())
  }

  pub(crate) fn snapshot_generation(
    &self,
    targets: &[GeometryTarget],
  ) -> Option<GeometryGeneration> {
    let generation = self.generation?;
    for target in targets {
      let key = self.current_key(target)?;
      if self
        .entries
        .get(&key)
        .is_none_or(|entry| entry.result.is_none())
      {
        return None;
      }
    }
    Some(generation)
  }

  pub(crate) fn element(&self, element_ref: &ElementRef) -> Measurement<ElementGeometry> {
    let (runtime_id, identity, object_id) = element_ref.geometry_identity();
    assert_eq!(
      self.runtime_id, runtime_id,
      "Reactant geometry targets cannot cross runtimes"
    );
    let Some(object_id) = self.element_object(identity, object_id) else {
      return Measurement::waiting();
    };
    self.read(
      &TargetKey::Element {
        identity,
        object_id,
      },
      |value| match value {
        GeometryValue::Element(value) => Some(value),
        _ => None,
      },
    )
  }

  pub(crate) fn viewport(&self, viewport: ViewportRef) -> Measurement<ViewportGeometry> {
    self.read(
      &TargetKey::Native(GeometryObservationTarget::Viewport {
        display_id: viewport.display_id,
      }),
      |value| match value {
        GeometryValue::Viewport(value) => Some(value),
        _ => None,
      },
    )
  }

  pub(crate) fn world(&self, world: &WorldRef) -> Measurement<WorldGeometry> {
    self.read(
      &TargetKey::Native(world.target.clone()),
      |value| match value {
        GeometryValue::WorldPoint(value) => Some(WorldGeometry::Point(value)),
        GeometryValue::WorldBounds(value) => Some(WorldGeometry::Bounds(value)),
        _ => None,
      },
    )
  }

  fn resolve(
    &self,
    target: &GeometryTarget,
    attachments: &AttachmentSet,
  ) -> Option<(TargetKey, GeometryObservationTarget)> {
    match target {
      GeometryTarget::Element(element_ref) => {
        let (identity, object_id) = attachments.geometry_target(self.runtime_id, element_ref)?;
        Some((
          TargetKey::Element {
            identity,
            object_id,
          },
          GeometryObservationTarget::UiElement { object_id },
        ))
      }
      GeometryTarget::Viewport(viewport) => {
        let target = GeometryObservationTarget::Viewport {
          display_id: viewport.display_id,
        };
        Some((TargetKey::Native(target.clone()), target))
      }
      GeometryTarget::World(world) => Some((
        TargetKey::Native(world.target.clone()),
        world.target.clone(),
      )),
    }
  }

  fn current_key(&self, target: &GeometryTarget) -> Option<TargetKey> {
    match target {
      GeometryTarget::Element(element_ref) => {
        let (runtime_id, identity, object_id) = element_ref.geometry_identity();
        assert_eq!(
          self.runtime_id, runtime_id,
          "Reactant geometry targets cannot cross runtimes"
        );
        Some(TargetKey::Element {
          identity,
          object_id: self.element_object(identity, object_id)?,
        })
      }
      GeometryTarget::Viewport(viewport) => {
        Some(TargetKey::Native(GeometryObservationTarget::Viewport {
          display_id: viewport.display_id,
        }))
      }
      GeometryTarget::World(world) => Some(TargetKey::Native(world.target.clone())),
    }
  }

  fn read<T: Copy>(
    &self,
    key: &TargetKey,
    convert: impl Fn(GeometryValue) -> Option<T>,
  ) -> Measurement<T> {
    let entry = self.entries.get(key);
    let latest = entry
      .and_then(|entry| entry.latest)
      .or_else(|| self.cache.get(key).copied())
      .map(|value| convert(value).expect("validated Reactant geometry value kind"));
    let status = match self.generation.and(entry.and_then(|entry| entry.result)) {
      Some(GeometryObservationResult::Current(_)) => MeasurementStatus::Current,
      Some(GeometryObservationResult::Unavailable(reason)) => {
        MeasurementStatus::Unavailable(reason)
      }
      None => MeasurementStatus::Waiting,
    };
    Measurement { latest, status }
  }

  fn element_object(&self, identity: u64, committed: Option<ObjectId>) -> Option<ObjectId> {
    self
      .element_objects
      .as_ref()
      .map_or(committed, |objects| objects.get(&identity).copied())
  }

  fn element_latest(&self, identity: u64) -> Option<GeometryValue> {
    let object_id = self.element_object(identity, None).or_else(|| {
      self.entries.keys().find_map(|key| match key {
        TargetKey::Element {
          identity: current_identity,
          object_id,
        } if *current_identity == identity => Some(*object_id),
        TargetKey::Element { .. } | TargetKey::Native(_) => None,
      })
    })?;
    let key = TargetKey::Element {
      identity,
      object_id,
    };
    self
      .entries
      .get(&key)
      .and_then(|entry| entry.latest)
      .or_else(|| self.cache.get(&key).copied())
  }
}

impl GeometryPlan {
  pub(crate) fn requires_preview(&self, runtime: &GeometryRuntime) -> bool {
    self.element_targets.iter().any(|identity| {
      let latest = self.entries.iter().find_map(|(key, entry)| match key {
        TargetKey::Element {
          identity: current_identity,
          ..
        } if current_identity == identity => entry.latest,
        TargetKey::Element { .. } | TargetKey::Native(_) => None,
      });
      runtime.element_latest(*identity) != latest
    })
  }

  pub(crate) const fn generation(&self) -> Option<GeometryGeneration> {
    self.generation
  }

  pub(crate) fn command_groups(&self, mut groups: Vec<Vec<CommandBody>>) -> Vec<Vec<CommandBody>> {
    if !self.removed.is_empty() {
      groups.insert(
        0,
        vec![CommandBody::GeometryObservationUpdate(
          GeometryObservationUpdate {
            added: Vec::new(),
            removed: self.removed.clone(),
          },
        )],
      );
    }
    if !self.added.is_empty() {
      groups.push(vec![CommandBody::GeometryObservationUpdate(
        GeometryObservationUpdate {
          added: self.added.clone(),
          removed: Vec::new(),
        },
      )]);
    }
    groups
  }
}
