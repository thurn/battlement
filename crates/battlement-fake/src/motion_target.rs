//! Property adapters for the fake Motion registry.

use std::collections::HashMap;

use battlement::{
  Color, FilterList, Gradient, Length, MaterialValue, MotionDescriptor, MotionDiscreteValue,
  MotionProperty, MotionPropertyTarget, MotionPropertyTrack, MotionValue, MotionValueKind,
  ObjectId,
};
use battlement_ui_fake::UiWorld;

use crate::{motion_transform::TransformWriter, world::FakeWorld};

#[derive(Clone)]
pub(crate) struct Target {
  pub(crate) host: ObjectId,
  world: Option<TransformWriter>,
  presentation: HashMap<MotionProperty, MotionValue>,
  pub(crate) failure: Option<String>,
  scale_contribution: Option<[f32; 2]>,
  material_scalar: Option<(u32, String)>,
  audio_volume: Option<ObjectId>,
  light_intensity: bool,
  particle_emission: bool,
  world_text: bool,
}

impl Target {
  pub(crate) fn new(host: ObjectId, world: &FakeWorld, ui: &UiWorld) -> Self {
    assert!(
      world.object(host).is_some() || ui.element(host).is_some(),
      "Motion host is absent"
    );
    Self {
      host,
      world: world
        .object(host)
        .map(|_| TransformWriter::new(host, world)),
      presentation: HashMap::new(),
      failure: None,
      scale_contribution: None,
      material_scalar: None,
      audio_volume: None,
      light_intensity: false,
      particle_emission: false,
      world_text: world
        .object(host)
        .and_then(|object| object.text())
        .is_some(),
    }
  }

  pub(crate) fn reconnect(
    &mut self,
    definition: &MotionDescriptor,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
  ) {
    let presentation = self.presentation.clone();
    let scale_contribution = self.scale_contribution;
    *self = Self::new(self.host, world, ui);
    self.configure(definition, world);
    self.scale_contribution = scale_contribution;
    for (property, value) in presentation {
      self.write(property, value, world, ui);
    }
  }

  pub(crate) fn configure(&mut self, definition: &MotionDescriptor, world: &FakeWorld) {
    let mut material = None;
    let mut audio = None;
    for track in tracks(definition) {
      match &track.target {
        MotionPropertyTarget::Host => {}
        MotionPropertyTarget::MaterialScalar { slot, parameter } => {
          let next = (*slot, parameter.clone());
          assert!(
            material.as_ref().is_none_or(|value| value == &next),
            "one Motion host cannot target multiple material scalars"
          );
          material = Some(next);
        }
        MotionPropertyTarget::AudioVolume { playback_id } => {
          assert!(
            audio.is_none_or(|value| value == *playback_id),
            "one Motion host cannot target multiple audio playbacks"
          );
          audio = Some(*playback_id);
        }
      }
    }
    if let Some((slot, parameter)) = &material {
      assert!(
        matches!(
          world
            .object(self.host)
            .and_then(|object| object.material_parameter(*slot, parameter)),
          Some(MaterialValue::Float(_))
        ),
        "material scalar Motion requires a prepared float parameter"
      );
    }
    let light_intensity =
      tracks(definition).any(|track| track.property == MotionProperty::LightIntensity);
    if light_intensity {
      assert!(
        world
          .object(self.host)
          .and_then(|object| object.light())
          .is_some(),
        "light intensity Motion requires a light host"
      );
    }
    let particle_emission =
      tracks(definition).any(|track| track.property == MotionProperty::ParticleEmission);
    if particle_emission {
      assert!(
        world
          .object(self.host)
          .and_then(|object| object.particle_emission())
          .is_some(),
        "particle emission Motion requires a particle host"
      );
    }
    if let Some(playback_id) = audio {
      assert!(
        world.audio(command_id(playback_id)).is_some(),
        "audio volume Motion requires a live audio playback"
      );
    }
    self.material_scalar = material;
    self.audio_volume = audio;
    self.light_intensity = light_intensity;
    self.particle_emission = particle_emission;
  }

  pub(crate) fn capture(&mut self, property: MotionProperty, world: &FakeWorld, ui: &UiWorld) {
    let value = self.read(property, world, ui);
    self.presentation.insert(property, value);
  }

  pub(crate) fn exists(&self, world: &FakeWorld, ui: &UiWorld) -> bool {
    if self.world.is_some() {
      world.object(self.host).is_some()
    } else {
      ui.element(self.host).is_some()
    }
  }

  pub(crate) fn is_world(&self) -> bool {
    self.world.is_some()
  }

  pub(crate) fn supports(&self, property: MotionProperty) -> bool {
    if self.is_world() {
      return property.is_world_transform()
        || (property == MotionProperty::Opacity && self.world_text)
        || (property == MotionProperty::MaterialScalar && self.material_scalar.is_some())
        || (property == MotionProperty::AudioVolume && self.audio_volume.is_some())
        || (property == MotionProperty::LightIntensity && self.light_intensity)
        || (property == MotionProperty::ParticleEmission && self.particle_emission);
    }
    !property.is_world_transform() && !property.is_world_effect()
  }

  pub(crate) fn read(
    &mut self,
    property: MotionProperty,
    world: &FakeWorld,
    ui: &UiWorld,
  ) -> MotionValue {
    match property {
      MotionProperty::MaterialScalar => {
        let (slot, parameter) = self
          .material_scalar
          .as_ref()
          .expect("material scalar Motion target is unconfigured");
        let MaterialValue::Float(value) = world
          .object(self.host)
          .and_then(|object| object.material_parameter(*slot, parameter))
          .expect("material scalar Motion target is absent")
        else {
          panic!("material scalar Motion target has a non-float prepared type")
        };
        return MotionValue::Scalar(*value as f32);
      }
      MotionProperty::Opacity if self.world_text => {
        return MotionValue::Scalar(
          world
            .object(self.host)
            .and_then(|object| object.text())
            .expect("world text Motion target is absent")
            .color
            .a as f32,
        );
      }
      MotionProperty::LightIntensity => {
        return MotionValue::Scalar(
          world
            .object(self.host)
            .and_then(|object| object.light())
            .expect("light intensity Motion target is absent")
            .intensity as f32,
        );
      }
      MotionProperty::ParticleEmission => {
        return MotionValue::Scalar(
          world
            .object(self.host)
            .and_then(|object| object.particle_emission())
            .expect("particle emission Motion target is absent") as f32,
        );
      }
      MotionProperty::AudioVolume => {
        let playback = self
          .audio_volume
          .expect("audio volume Motion target is unconfigured");
        return MotionValue::Scalar(
          world
            .audio(command_id(playback))
            .expect("audio volume Motion playback is absent")
            .volume() as f32,
        );
      }
      _ => {}
    }
    if let Some(writer) = &mut self.world {
      return writer.read(property, world);
    }
    if self.scale_contribution.is_some()
      && let Some(MotionValue::Vector2(base)) = self.presentation.get(&MotionProperty::Scale)
    {
      match property {
        MotionProperty::Scale => return MotionValue::Vector2(*base),
        MotionProperty::ScaleX => return MotionValue::Scalar(base[0]),
        MotionProperty::ScaleY => return MotionValue::Scalar(base[1]),
        _ => {}
      }
    }
    ui.motion_value(self.host, property)
      .or_else(|| self.presentation.get(&property).cloned())
      .unwrap_or_else(|| initial(property))
  }

  pub(crate) fn clear_contribution(&mut self, world: &mut FakeWorld, ui: &mut UiWorld) {
    if self.scale_contribution.take().is_some()
      && let Some(base) = self.presentation.get(&MotionProperty::Scale).cloned()
    {
      self.write(MotionProperty::Scale, base, world, ui);
    }
  }

  pub(crate) fn set_contribution(
    &mut self,
    value: MotionValue,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
  ) {
    assert!(!self.is_world(), "scale composition is a UI property");
    let MotionValue::Vector2(factor) = value else {
      panic!("scale contribution requires two factors");
    };
    let base = self
      .presentation
      .get(&MotionProperty::Scale)
      .cloned()
      .unwrap_or_else(|| self.read(MotionProperty::Scale, world, ui));
    self.scale_contribution = Some(factor);
    self.write(MotionProperty::Scale, base, world, ui);
  }

  pub(crate) fn write(
    &mut self,
    property: MotionProperty,
    value: MotionValue,
    world: &mut FakeWorld,
    ui: &mut UiWorld,
  ) {
    if self.failure.is_some() {
      return;
    }
    if let Err(error) = validate_presentation(&value) {
      self.failure = Some(format!(
        "Motion produced a non-finite or invalid presentation value for {property:?}: {error}; {value:?}"
      ));
      return;
    }
    if let Some(writer) = &mut self.world {
      let MotionValue::Scalar(number) = &value else {
        writer.write(property, &value, world);
        self.presentation.insert(property, value);
        return;
      };
      let number = *number;
      match property {
        MotionProperty::MaterialScalar => {
          let (slot, parameter) = self
            .material_scalar
            .as_ref()
            .expect("material scalar Motion target is unconfigured");
          world.set_material_scalar(self.host, *slot, parameter, f64::from(number));
        }
        MotionProperty::Opacity if self.world_text => {
          let object = world.object_mut(self.host);
          let battlement::GameObjectKind::Text { text } = &mut object.kind else {
            panic!("world text Motion target is absent")
          };
          text.color.a = f64::from(number.clamp(0.0, 1.0));
        }
        MotionProperty::LightIntensity => {
          world.light_mut(self.host).intensity = f64::from(number);
        }
        MotionProperty::ParticleEmission => {
          *world.particle_emission_mut(self.host) = f64::from(number);
        }
        MotionProperty::AudioVolume => {
          let playback = self
            .audio_volume
            .expect("audio volume Motion target is unconfigured");
          world.audio_mut(command_id(playback)).volume = f64::from(number);
        }
        _ => writer.write(property, &MotionValue::Scalar(number), world),
      }
    } else {
      let presented = match (&value, self.scale_contribution) {
        (MotionValue::Vector2(base), Some(factor)) if property == MotionProperty::Scale => {
          MotionValue::Vector2([base[0] * factor[0], base[1] * factor[1]])
        }
        (MotionValue::Scalar(base), Some(factor)) if property == MotionProperty::ScaleX => {
          MotionValue::Scalar(base * factor[0])
        }
        (MotionValue::Scalar(base), Some(factor)) if property == MotionProperty::ScaleY => {
          MotionValue::Scalar(base * factor[1])
        }
        _ => value.clone(),
      };
      ui.apply_motion_value(self.host, property, &presented);
      if matches!(property, MotionProperty::ScaleX | MotionProperty::ScaleY) {
        let mut base = match self.presentation.get(&MotionProperty::Scale) {
          Some(MotionValue::Vector2(base)) => *base,
          _ => [1.0; 2],
        };
        if let MotionValue::Scalar(value) = value {
          base[usize::from(property == MotionProperty::ScaleY)] = value;
          self
            .presentation
            .insert(MotionProperty::Scale, MotionValue::Vector2(base));
        }
      }
    }
    self.presentation.insert(property, value);
  }
}

fn command_id(value: ObjectId) -> battlement::CommandId {
  battlement::CommandId::from_uuid(*value.as_uuid()).expect("Motion playback identity is nonzero")
}

fn tracks(definition: &MotionDescriptor) -> impl Iterator<Item = &MotionPropertyTrack> {
  definition
    .initial
    .iter()
    .flat_map(|target| &target.tracks)
    .chain(definition.slots.iter().flat_map(|slot| &slot.target.tracks))
    .chain(
      definition
        .named_targets
        .iter()
        .flat_map(|target| &target.target.tracks),
    )
}

// Native host reads use empty shapes to represent absent gradient or polygon paint.
fn validate_presentation(value: &MotionValue) -> Result<(), &'static str> {
  match value {
    MotionValue::Gradient(Gradient::Linear { angle, stops })
      if stops.is_empty() && angle.is_finite() =>
    {
      Ok(())
    }
    MotionValue::ClipPolygon(vertices) if vertices.is_empty() => Ok(()),
    _ => value.validate(),
  }
}

pub(crate) fn spatial(property: MotionProperty) -> bool {
  property.is_world_transform()
    || matches!(
      property,
      MotionProperty::X
        | MotionProperty::Y
        | MotionProperty::Z
        | MotionProperty::Translate
        | MotionProperty::Rotate
        | MotionProperty::RotateX
        | MotionProperty::RotateY
        | MotionProperty::Scale
        | MotionProperty::ScaleX
        | MotionProperty::ScaleY
        | MotionProperty::SkewX
        | MotionProperty::SkewY
        | MotionProperty::TransformList
        | MotionProperty::Layout
    )
}

fn initial(property: MotionProperty) -> MotionValue {
  match property.metadata().value_kind {
    MotionValueKind::Scalar => {
      MotionValue::Scalar(property.metadata().initial_value.parse().unwrap_or(0.0))
    }
    MotionValueKind::Length => MotionValue::Length(Length::Px(0.0)),
    MotionValueKind::Angle => MotionValue::Angle(0.0),
    MotionValueKind::Vector2 => MotionValue::Vector2(if property == MotionProperty::Scale {
      [1.0; 2]
    } else {
      [0.0; 2]
    }),
    MotionValueKind::Vector3 => MotionValue::Vector3([0.0; 3]),
    MotionValueKind::Color => MotionValue::Color(if property == MotionProperty::Color {
      Color::WHITE
    } else {
      Color::rgba(0.0, 0.0, 0.0, 0.0)
    }),
    MotionValueKind::TransformList => MotionValue::TransformList(Vec::new()),
    MotionValueKind::FilterList => MotionValue::FilterList(FilterList::default()),
    MotionValueKind::ShadowList => MotionValue::ShadowList(Vec::new()),
    MotionValueKind::Gradient => MotionValue::Gradient(Gradient::Linear {
      angle: 0.0,
      stops: Vec::new(),
    }),
    MotionValueKind::ClipInset => MotionValue::ClipInset([Length::Px(0.0); 4]),
    MotionValueKind::ClipPolygon => MotionValue::ClipPolygon(Vec::new()),
    MotionValueKind::Discrete => MotionValue::Discrete(MotionDiscreteValue::String(
      property.metadata().initial_value.to_owned(),
    )),
  }
}
