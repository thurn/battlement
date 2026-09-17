use crate::{ProtocolError, world_generated as wire};
use battlement::{
  MaterialInstance, MaterialParameterDeclaration, MaterialParameterKind, MaterialValue,
};
use flatbuffers::{FlatBufferBuilder, ForwardsUOffset, Vector, WIPOffset};

pub(crate) fn kind(value: MaterialParameterKind) -> wire::MaterialParameterKind {
  match value {
    MaterialParameterKind::Float => wire::MaterialParameterKind::Float,
    MaterialParameterKind::Color => wire::MaterialParameterKind::Color,
    MaterialParameterKind::Vector => wire::MaterialParameterKind::Vector,
  }
}
pub(crate) fn declarations<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  values: &[MaterialParameterDeclaration],
) -> WIPOffset<Vector<'a, ForwardsUOffset<wire::MaterialParameterDeclaration<'a>>>> {
  let values = values
    .iter()
    .map(|p| {
      let name = builder.create_string(&p.name);
      wire::MaterialParameterDeclaration::create(
        builder,
        &wire::MaterialParameterDeclarationArgs {
          name: Some(name),
          kind: kind(p.kind),
        },
      )
    })
    .collect::<Vec<_>>();
  builder.create_vector(&values)
}
pub(crate) fn instances<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  values: &[MaterialInstance],
) -> WIPOffset<Vector<'a, ForwardsUOffset<wire::MaterialInstance<'a>>>> {
  let values = values
    .iter()
    .map(|m| {
      let address = builder.create_string(m.address.as_str());
      let parameters = m
        .parameters
        .iter()
        .map(|p| {
          let name = builder.create_string(&p.name);
          let [x, y, z, w] = match &p.value {
            MaterialValue::Float(v) => [*v, 0.0, 0.0, 0.0],
            MaterialValue::Color(v) => [v.r, v.g, v.b, v.a],
            MaterialValue::Vector(v) => v.0,
          };
          wire::MaterialParameterValue::create(
            builder,
            &wire::MaterialParameterValueArgs {
              name: Some(name),
              kind: kind(p.value.kind()),
              x,
              y,
              z,
              w,
            },
          )
        })
        .collect::<Vec<_>>();
      let parameters = builder.create_vector(&parameters);
      wire::MaterialInstance::create(
        builder,
        &wire::MaterialInstanceArgs {
          address: Some(address),
          slot: m.slot,
          parameters: Some(parameters),
        },
      )
    })
    .collect::<Vec<_>>();
  builder.create_vector(&values)
}
pub(crate) fn validate_instances(
  values: Option<Vector<'_, ForwardsUOffset<wire::MaterialInstance<'_>>>>,
) -> Result<(), ProtocolError> {
  let mut slots = std::collections::HashSet::new();
  for m in values.into_iter().flatten() {
    if m.address().is_empty() || !slots.insert(m.slot()) {
      return Err(ProtocolError::new(
        "invalid material instance slot or address",
      ));
    }
    let mut names = std::collections::HashSet::new();
    for p in m.parameters() {
      if p.name().is_empty() || !names.insert(p.name()) {
        return Err(ProtocolError::new("invalid material parameter name"));
      }
      if p.kind().variant_name().is_none()
        || [p.x(), p.y(), p.z(), p.w()]
          .iter()
          .any(|x| !x.is_finite() || x.abs() > f64::from(f32::MAX))
      {
        return Err(ProtocolError::new(
          "invalid material parameter type or value",
        ));
      }
    }
  }
  Ok(())
}
pub(crate) fn validate_asset(asset: wire::PreparedAsset<'_>) -> Result<(), ProtocolError> {
  if asset.kind() != wire::PreparedAssetKind::MaterialParameters {
    return Ok(());
  }
  let mut names = std::collections::HashSet::new();
  for p in asset.parameters().into_iter().flatten() {
    if p.name().is_empty() || !names.insert(p.name()) || p.kind().variant_name().is_none() {
      return Err(ProtocolError::new("invalid material parameter declaration"));
    }
  }
  Ok(())
}
