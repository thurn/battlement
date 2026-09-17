//! Property adapters for the fake Motion registry.

use std::collections::HashMap;

use battlement::{
  Color, FilterList, Gradient, Length, MotionDiscreteValue, MotionProperty, MotionValue,
  MotionValueKind, ObjectId,
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
    }
  }

  pub(crate) fn reconnect(&mut self, world: &mut FakeWorld, ui: &mut UiWorld) {
    let presentation = self.presentation.clone();
    let scale_contribution = self.scale_contribution;
    *self = Self::new(self.host, world, ui);
    self.scale_contribution = scale_contribution;
    for (property, value) in presentation {
      self.write(property, value, world, ui);
    }
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

  pub(crate) fn read(
    &mut self,
    property: MotionProperty,
    world: &FakeWorld,
    ui: &UiWorld,
  ) -> MotionValue {
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
      writer.write(property, &value, world);
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
