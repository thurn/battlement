use crate::{MaterialInstance, PreparedAsset, Snapshot, ValidationError};
use std::collections::{HashMap, HashSet};

pub(crate) fn instances(values: &[MaterialInstance]) -> Result<(), ValidationError> {
  let mut slots = HashSet::new();
  for m in values {
    if m.address.as_str().is_empty() || !slots.insert(m.slot) {
      return Err(ValidationError::InvalidReference);
    }
    let mut names = HashSet::new();
    for p in &m.parameters {
      if p.name.is_empty() || !names.insert(&p.name) {
        return Err(ValidationError::InvalidReference);
      }
      if !p.value.is_finite() {
        return Err(ValidationError::NonFiniteNumber);
      }
    }
  }
  Ok(())
}

pub(crate) fn prepared(snapshot: &Snapshot) -> Result<(), ValidationError> {
  let materials: HashMap<_, _> = snapshot
    .prepared_assets
    .iter()
    .filter_map(|asset| match asset {
      PreparedAsset::Material(address) | PreparedAsset::MaterialParameters { address, .. } => {
        Some((address, asset))
      }
      _ => None,
    })
    .collect();
  for object in &snapshot.objects {
    for instance in &object.material_instances {
      if !materials
        .get(&instance.address)
        .is_some_and(|asset| instance.is_declared_by(asset))
      {
        return Err(ValidationError::InvalidReference);
      }
    }
  }
  Ok(())
}
