use battlement::{
  Color, MaterialInstance, MaterialParameterDeclaration, MaterialParameterKind,
  MaterialParameterValue, MaterialValue, MaterialVector,
};
use battlement_flatbuffers::schema_generated::world_generated as wire;
use flatbuffers::{ForwardsUOffset, Vector};

pub(crate) fn kind(value: wire::MaterialParameterKind) -> Result<MaterialParameterKind, String> {
  Ok(match value {
    wire::MaterialParameterKind::Float => MaterialParameterKind::Float,
    wire::MaterialParameterKind::Color => MaterialParameterKind::Color,
    wire::MaterialParameterKind::Vector => MaterialParameterKind::Vector,
    _ => return Err("unknown material parameter type".into()),
  })
}
pub(crate) fn declarations(
  values: Option<Vector<'_, ForwardsUOffset<wire::MaterialParameterDeclaration<'_>>>>,
) -> Result<Vec<MaterialParameterDeclaration>, String> {
  values
    .into_iter()
    .flatten()
    .map(|p| {
      Ok(MaterialParameterDeclaration {
        name: p.name().into(),
        kind: kind(p.kind())?,
      })
    })
    .collect()
}
pub(crate) fn instances(
  values: Option<Vector<'_, ForwardsUOffset<wire::MaterialInstance<'_>>>>,
) -> Result<Vec<MaterialInstance>, String> {
  values
    .into_iter()
    .flatten()
    .map(|m| {
      Ok(MaterialInstance {
        address: m.address().into(),
        slot: m.slot(),
        parameters: m
          .parameters()
          .iter()
          .map(|p| {
            Ok(MaterialParameterValue {
              name: p.name().into(),
              value: match kind(p.kind())? {
                MaterialParameterKind::Float => MaterialValue::Float(p.x()),
                MaterialParameterKind::Color => MaterialValue::Color(Color {
                  r: p.x(),
                  g: p.y(),
                  b: p.z(),
                  a: p.w(),
                }),
                MaterialParameterKind::Vector => {
                  MaterialValue::Vector(MaterialVector([p.x(), p.y(), p.z(), p.w()]))
                }
              },
            })
          })
          .collect::<Result<_, String>>()?,
      })
    })
    .collect()
}

pub(crate) fn matches(
  asset: &battlement::PreparedAsset,
  expected: &battlement::PreparedAsset,
) -> bool {
  match (asset, expected) {
    (
      battlement::PreparedAsset::MaterialParameters { address, .. },
      battlement::PreparedAsset::Material(other),
    ) => address == other,
    _ => asset == expected,
  }
}

pub(crate) fn validate(
  instances: &[MaterialInstance],
  slots: Option<usize>,
  catalog: &crate::assets::FakeAssetCatalog,
  prepared: &[battlement::PreparedAsset],
) {
  let mut used = std::collections::BTreeSet::new();
  for m in instances {
    assert!(used.insert(m.slot), "duplicate material slot");
    assert!(
      (m.slot as usize) < slots.expect("material overrides require a renderer"),
      "material slot out of range"
    );
    crate::world_validation::assert_prepared(
      prepared,
      battlement::PreparedAsset::Material(m.address.clone()),
    );
    assert!(
      prepared.iter().any(|asset| m.is_declared_by(asset)),
      "material parameter was not declared in the prepared set"
    );
    assert!(
      catalog.validate_material_parameters(&m.address, &m.declarations()),
      "material parameter declaration mismatch"
    );
  }
}
