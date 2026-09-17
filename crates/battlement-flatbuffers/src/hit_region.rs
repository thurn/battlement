use crate::{
  ProtocolError,
  schema_generated::{common_generated as common, world_generated as wire},
};
use battlement::{BoxHitRegionState, Vector3};
use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub(crate) fn write<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: BoxHitRegionState,
) -> WIPOffset<wire::BoxHitRegionObject<'a>> {
  wire::BoxHitRegionObject::create(
    builder,
    &wire::BoxHitRegionObjectArgs {
      size: Some(&common::Vector3d::new(
        value.size.x,
        value.size.y,
        value.size.z,
      )),
      center: Some(&common::Vector3d::new(
        value.center.x,
        value.center.y,
        value.center.z,
      )),
    },
  )
}
pub(crate) fn validate(value: wire::BoxHitRegionObject<'_>) -> Result<(), ProtocolError> {
  let size = value.size();
  let center = value.center();
  let region = BoxHitRegionState {
    size: Vector3::new(size.x(), size.y(), size.z()),
    center: Vector3::new(center.x(), center.y(), center.z()),
  };
  if !region.is_valid() {
    return Err(ProtocolError::new("invalid box hit-region geometry"));
  }
  Ok(())
}
