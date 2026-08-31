//! Type-erased storage for public typed variant maps.

use std::{
  any::{Any, TypeId},
  collections::hash_map::DefaultHasher,
  fmt,
  hash::{Hash, Hasher},
  rc::Rc,
};

use crate::{
  motion::{MotionStyle, MotionTarget},
  motion_variants::VariantOrchestration,
};

type ResolveVariants = dyn Fn(&ErasedVariantSelection, Option<&dyn Any>) -> VariantTarget;

/// A name accepted by a typed variant map or selection.
pub trait VariantKey: Clone + Eq + Hash + fmt::Debug + 'static {}

/// Custom data accepted by a computed variant resolver.
pub trait VariantData: Clone + Hash + 'static {}

/// A validated string variant identity for applications without an enum.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariantName(String);

/// A target plus its parent/child sequencing policy.
#[derive(Clone, Debug, PartialEq)]
pub struct VariantTarget {
  pub(crate) target: MotionTarget,
  pub(crate) orchestration: VariantOrchestration,
}

/// A typed set of static or custom-data-resolved targets.
pub struct Variants<Name = VariantName, Custom = ()> {
  entries: Vec<(Name, VariantDefinition<Custom>)>,
}

#[derive(Clone)]
enum VariantDefinition<Custom> {
  Static(Box<VariantTarget>),
  Computed(Rc<dyn Fn(&Custom) -> VariantTarget>),
}

#[derive(Clone)]
pub(crate) struct ErasedVariants {
  name_type: Option<TypeId>,
  custom_type: Option<TypeId>,
  fingerprint: Vec<VariantFingerprint>,
  resolve: Option<Rc<ResolveVariants>>,
}

#[derive(Clone, Debug, PartialEq)]
struct VariantFingerprint {
  hash: u64,
  label: String,
  static_target: Option<VariantTarget>,
}

#[derive(Clone)]
pub(crate) struct ErasedVariantSelection {
  name_type: TypeId,
  values: Vec<Rc<dyn Any>>,
  hashes: Vec<u64>,
  labels: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct ErasedVariantData {
  data_type: TypeId,
  snapshot: u64,
  value: Rc<dyn Any>,
}

impl<Name, Custom> Clone for Variants<Name, Custom>
where
  Name: Clone,
  Custom: Clone,
{
  fn clone(&self) -> Self {
    Self {
      entries: self.entries.clone(),
    }
  }
}

impl<Name, Custom> fmt::Debug for Variants<Name, Custom>
where
  Name: fmt::Debug,
{
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("Variants")
      .field(
        "names",
        &self
          .entries
          .iter()
          .map(|entry| &entry.0)
          .collect::<Vec<_>>(),
      )
      .finish_non_exhaustive()
  }
}

impl VariantName {
  /// Creates a nonempty stable string name.
  #[must_use]
  pub fn new(value: impl Into<String>) -> Self {
    let value = value.into();
    assert!(!value.trim().is_empty(), "variant name must not be empty");
    assert!(value.len() <= 128, "variant name is too long");
    Self(value)
  }

  /// Returns the stable string identity.
  #[must_use]
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl From<&str> for VariantName {
  fn from(value: &str) -> Self {
    Self::new(value)
  }
}

impl From<String> for VariantName {
  fn from(value: String) -> Self {
    Self::new(value)
  }
}

impl VariantTarget {
  /// Creates a target with concurrent child playback.
  #[must_use]
  pub fn new(target: impl Into<MotionTarget>) -> Self {
    let target = target.into();
    Self {
      orchestration: target.variant_orchestration(),
      target,
    }
  }

  /// Replaces parent/child orchestration.
  #[must_use]
  pub fn orchestration(mut self, value: VariantOrchestration) -> Self {
    self.orchestration = value;
    self
  }
}

impl<Name, Custom> Variants<Name, Custom>
where
  Name: VariantKey,
  Custom: VariantData,
{
  /// Creates an empty typed variant map.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      entries: Vec::new(),
    }
  }

  /// Adds one static target.
  #[must_use]
  pub fn target(mut self, name: impl Into<Name>, target: impl Into<VariantTarget>) -> Self {
    self.insert(
      name.into(),
      VariantDefinition::Static(Box::new(target.into())),
    );
    self
  }

  /// Adds one target resolved from the host's snapshotted custom data.
  #[must_use]
  pub fn resolver(
    mut self,
    name: impl Into<Name>,
    resolve: impl Fn(&Custom) -> VariantTarget + 'static,
  ) -> Self {
    self.insert(name.into(), VariantDefinition::Computed(Rc::new(resolve)));
    self
  }

  pub(crate) fn erase(self) -> ErasedVariants {
    let fingerprint = self
      .entries
      .iter()
      .map(|(name, definition)| VariantFingerprint {
        hash: stable_hash(name),
        label: variant_label(name),
        static_target: match definition {
          VariantDefinition::Static(target) => Some((**target).clone()),
          VariantDefinition::Computed(_) => None,
        },
      })
      .collect();
    let entries = Rc::new(self.entries);
    ErasedVariants {
      name_type: Some(TypeId::of::<Name>()),
      custom_type: Some(TypeId::of::<Custom>()),
      fingerprint,
      resolve: Some(Rc::new(move |selection, custom| {
        let mut result: Option<VariantTarget> = None;
        for value in &selection.values {
          let name = value
            .downcast_ref::<Name>()
            .expect("variant selection type was checked before resolution");
          let definition = entries
            .iter()
            .find(|entry| &entry.0 == name)
            .unwrap_or_else(|| panic!("selected variant is missing: {}", variant_label(name)));
          let target = match &definition.1 {
            VariantDefinition::Static(target) => (**target).clone(),
            VariantDefinition::Computed(resolve) => resolve(
              custom
                .and_then(|value| value.downcast_ref::<Custom>())
                .expect("computed variant requires compatible custom data"),
            ),
          };
          result = Some(merge_targets(result, target));
        }
        result.expect("nonempty variant selection resolves a target")
      })),
    }
  }

  fn insert(&mut self, name: Name, definition: VariantDefinition<Custom>) {
    assert!(
      !self.entries.iter().any(|entry| entry.0 == name),
      "variant map contains duplicate name: {}",
      variant_label(&name),
    );
    self.entries.push((name, definition));
  }
}

impl<Name, Custom> Default for Variants<Name, Custom>
where
  Name: VariantKey,
  Custom: VariantData,
{
  fn default() -> Self {
    Self::new()
  }
}

impl From<MotionTarget> for VariantTarget {
  fn from(value: MotionTarget) -> Self {
    Self::new(value)
  }
}

impl From<MotionStyle> for VariantTarget {
  fn from(value: MotionStyle) -> Self {
    Self::new(value)
  }
}

impl ErasedVariants {
  pub(crate) const fn new() -> Self {
    Self {
      name_type: None,
      custom_type: None,
      fingerprint: Vec::new(),
      resolve: None,
    }
  }

  pub(crate) fn resolve(
    &self,
    selection: &ErasedVariantSelection,
    custom: Option<&ErasedVariantData>,
    explicit: bool,
  ) -> Option<VariantTarget> {
    let Some(resolve) = &self.resolve else {
      return None;
    };
    let compatible_name = self.name_type == Some(selection.name_type);
    let compatible_custom = custom.is_none_or(|value| self.custom_type == Some(value.data_type));
    if !compatible_name || !compatible_custom {
      assert!(
        !explicit,
        "local variant selection is incompatible with its map"
      );
      return None;
    }
    Some(resolve(selection, custom.map(|value| value.value.as_ref())))
  }
}

impl Default for ErasedVariants {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Debug for ErasedVariants {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("ErasedVariants")
      .field("fingerprint", &self.fingerprint)
      .finish_non_exhaustive()
  }
}

impl PartialEq for ErasedVariants {
  fn eq(&self, other: &Self) -> bool {
    self.name_type == other.name_type
      && self.custom_type == other.custom_type
      && self.fingerprint == other.fingerprint
  }
}

impl ErasedVariantSelection {
  pub(crate) fn new<Name: VariantKey>(values: impl IntoIterator<Item = Name>) -> Self {
    let values = values.into_iter().collect::<Vec<_>>();
    for (index, value) in values.iter().enumerate() {
      assert!(
        !values[..index].contains(value),
        "variant selection contains duplicate name: {}",
        variant_label(value),
      );
    }
    Self {
      name_type: TypeId::of::<Name>(),
      hashes: values.iter().map(stable_hash).collect(),
      labels: values.iter().map(variant_label).collect(),
      values: values
        .into_iter()
        .map(|value| Rc::new(value) as Rc<dyn Any>)
        .collect(),
    }
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.values.is_empty()
  }

  pub(crate) fn labels(&self) -> Vec<String> {
    self.labels.clone()
  }
}

impl fmt::Debug for ErasedVariantSelection {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_tuple("VariantSelection")
      .field(&self.labels)
      .finish()
  }
}

impl PartialEq for ErasedVariantSelection {
  fn eq(&self, other: &Self) -> bool {
    self.name_type == other.name_type && self.hashes == other.hashes && self.labels == other.labels
  }
}

impl ErasedVariantData {
  pub(crate) fn new<T: VariantData>(value: T) -> Self {
    Self {
      data_type: TypeId::of::<T>(),
      snapshot: stable_hash(&value),
      value: Rc::new(value),
    }
  }

  pub(crate) const fn snapshot(&self) -> u64 {
    self.snapshot
  }
}

impl fmt::Debug for ErasedVariantData {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("VariantData")
      .field("snapshot", &self.snapshot)
      .finish_non_exhaustive()
  }
}

impl PartialEq for ErasedVariantData {
  fn eq(&self, other: &Self) -> bool {
    self.data_type == other.data_type && self.snapshot == other.snapshot
  }
}

impl<T> VariantKey for T where T: Clone + Eq + Hash + fmt::Debug + 'static {}
impl<T> VariantData for T where T: Clone + Hash + 'static {}

fn merge_targets(current: Option<VariantTarget>, target: VariantTarget) -> VariantTarget {
  current.map_or(target.clone(), |mut current| {
    current.target = current.target.merge(target.target);
    current.orchestration = target.orchestration;
    current
  })
}

fn stable_hash(value: &impl Hash) -> u64 {
  let mut hasher = DefaultHasher::new();
  value.hash(&mut hasher);
  hasher.finish()
}

fn variant_label<Name: VariantKey>(value: &Name) -> String {
  (value as &dyn Any)
    .downcast_ref::<VariantName>()
    .map_or_else(|| format!("{value:?}"), |value| value.0.clone())
}
