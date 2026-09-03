use crate::{
  CommandBody, CssAnimationDescriptor, MotionControlCommand, MotionControlTarget, MotionDescriptor,
  MotionProperty, MotionPropertyValue, MotionScopeCommand, MotionTargetDescriptor, MotionValue,
  PreparedAsset, asset_dependencies::AssetDependencies,
};

impl AssetDependencies {
  pub(crate) fn motion(&mut self, descriptor: &MotionDescriptor) {
    if let Some(initial) = &descriptor.initial {
      self.motion_target(initial);
    }
    for slot in &descriptor.slots {
      self.motion_target(&slot.target);
    }
    for named in &descriptor.named_targets {
      self.motion_target(&named.target);
    }
    for pseudo in &descriptor.pseudo_styles {
      self.motion_properties(&pseudo.values);
    }
    self.css_animations(&descriptor.animations);
    for decoration in &descriptor.decorations {
      self.style(&decoration.style);
      self.css_animations(&decoration.animations);
    }
  }

  pub(crate) fn motion_command(&mut self, command: &CommandBody) {
    match command {
      CommandBody::MotionControl(operation) => match &operation.command {
        MotionControlCommand::Start { target, .. } | MotionControlCommand::Set(target) => {
          if let MotionControlTarget::Target(target) = target {
            self.motion_target(target);
          }
        }
        _ => {}
      },
      CommandBody::MotionScope(operation) => match &operation.command {
        MotionScopeCommand::Start { steps, .. } => {
          for step in steps {
            self.motion_target(&step.target);
          }
        }
        MotionScopeCommand::Set { target, .. } => self.motion_target(target),
        _ => {}
      },
      _ => {}
    }
  }

  fn motion_target(&mut self, target: &MotionTargetDescriptor) {
    for track in &target.tracks {
      for value in &track.values {
        self.motion_value(track.property, value);
      }
    }
    self.motion_properties(&target.transition_end);
  }

  fn css_animations(&mut self, animations: &[CssAnimationDescriptor]) {
    for animation in animations {
      for track in &animation.tracks {
        for value in &track.values {
          self.motion_value(track.property, value);
        }
      }
    }
  }

  fn motion_properties(&mut self, values: &[MotionPropertyValue]) {
    for value in values {
      self.motion_value(value.property, &value.value);
    }
  }

  fn motion_value(&mut self, property: MotionProperty, value: &MotionValue) {
    let MotionValue::Discrete(value) = value else {
      return;
    };
    let Some(address) = value.as_str().filter(|address| !address.is_empty()) else {
      return;
    };
    if address == "none" {
      return;
    }
    match property {
      MotionProperty::BackgroundImage | MotionProperty::Mask => {
        self.insert(PreparedAsset::texture(address))
      }
      MotionProperty::UnityMaterial => self.insert(PreparedAsset::material(address)),
      MotionProperty::UnityFontDefinition => self.insert(PreparedAsset::ui_font(address)),
      _ => {}
    }
  }
}
