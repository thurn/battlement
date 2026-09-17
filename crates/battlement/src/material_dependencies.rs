use std::collections::BTreeMap;

use crate::{MaterialAddress, MaterialParameterDeclaration, PreparedAsset};

pub(crate) fn merge(existing: &PreparedAsset, additional: &PreparedAsset) -> PreparedAsset {
  match (material(existing), material(additional)) {
    (Some((address, old)), Some((other, new))) => {
      assert_eq!(address, other, "material address mismatch");
      let mut declarations = BTreeMap::new();
      for p in old.iter().chain(new) {
        assert!(
          !p.name.is_empty(),
          "material parameter name cannot be empty"
        );
        if let Some(kind) = declarations.insert(p.name.clone(), p.kind) {
          assert_eq!(
            kind, p.kind,
            "conflicting material parameter type: {}",
            p.name
          );
        }
      }
      if declarations.is_empty() {
        return PreparedAsset::Material(address.clone());
      }
      PreparedAsset::MaterialParameters {
        address: address.clone(),
        parameters: declarations
          .into_iter()
          .map(|(name, kind)| MaterialParameterDeclaration { name, kind })
          .collect(),
      }
    }
    _ => {
      assert_eq!(existing, additional, "asset address has conflicting kinds");
      existing.clone()
    }
  }
}

fn material(asset: &PreparedAsset) -> Option<(&MaterialAddress, &[MaterialParameterDeclaration])> {
  match asset {
    PreparedAsset::Material(address) => Some((address, &[])),
    PreparedAsset::MaterialParameters {
      address,
      parameters,
    } => Some((address, parameters)),
    _ => None,
  }
}
