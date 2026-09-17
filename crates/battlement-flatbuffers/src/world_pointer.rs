use crate::schema_generated::world_generated as wire;
use battlement::WorldPointerSettings;
use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub(crate) fn write<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: WorldPointerSettings,
) -> WIPOffset<wire::WorldPointerSettings<'a>> {
  wire::WorldPointerSettings::create(
    builder,
    &wire::WorldPointerSettingsArgs {
      interaction_layer: value.interaction_layer,
      order: value.order,
      capture_on_press: value.capture_on_press,
      focusable: value.focusable,
    },
  )
}
