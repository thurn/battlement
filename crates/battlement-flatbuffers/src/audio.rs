use battlement::{AudioBus, AudioMix};
use flatbuffers::{Allocator, FlatBufferBuilder, WIPOffset};

use crate::{ProtocolError, command_core_generated as wire, common_generated as common};

pub(crate) fn bus(value: AudioBus) -> common::AudioBus {
  match value {
    AudioBus::Music => common::AudioBus::Music,
    AudioBus::Effects => common::AudioBus::Effects,
  }
}

pub(crate) fn write_mix<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: AudioMix,
) -> Result<WIPOffset<wire::AudioMixPayload<'a>>, ProtocolError> {
  validate_mix(value)?;
  Ok(wire::AudioMixPayload::create(
    builder,
    &wire::AudioMixPayloadArgs {
      master: value.master,
      music: value.music,
      effects: value.effects,
      muted: value.muted,
    },
  ))
}

pub(crate) fn validate_mix(value: AudioMix) -> Result<(), ProtocolError> {
  if [value.master, value.music, value.effects]
    .iter()
    .any(|gain| !(0.0..=1.0).contains(gain))
  {
    return Err(ProtocolError::new(
      "audio mixer gains must be between zero and one",
    ));
  }
  Ok(())
}

pub(crate) fn validate_bus(value: common::AudioBus) -> Result<(), ProtocolError> {
  if value != common::AudioBus::Music && value != common::AudioBus::Effects {
    return Err(ProtocolError::new("audio bus is unknown"));
  }
  Ok(())
}
