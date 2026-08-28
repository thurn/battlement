//! Core command validation and instant execution.

use std::collections::HashSet;

use battlement::{
  AnimatorState, Command, CommandBody, GameObjectKind, IconSource, ImageSource, MaterialAssignment,
  PreparedAsset, Prop, PropertyCommand, Style, StyleValue, UiElement, UiNode, Validate,
  VisualElementProperties,
};

use crate::{assets, client::FakeClient, journal::ExecutedCommand, tween, world};

impl<E> FakeClient<E>
where
  E: battlement_native::Engine<Command = Command>,
{
  pub(crate) fn execute_command(
    &mut self,
    command: Command,
    batch_id: battlement::BatchId,
    group_index: usize,
    command_index: usize,
  ) -> bool {
    assert!(
      !self.executed_commands.contains(&command.command_id),
      "duplicate command ID: {}",
      command.command_id
    );
    if let CommandBody::Diagnostics(diagnostics) = &command.body {
      assert!(command.blocking, "Diagnostics commands must be blocking");
      let result = self.diagnostics.execute(command.command_id, diagnostics);
      match result {
        Ok(()) => {
          self.record_executed(command, batch_id, group_index, command_index);
          return true;
        }
        Err(code) => {
          let command_id = command.command_id;
          self.record_executed(command, batch_id, group_index, command_index);
          self.submit_batch_failure(batch_id, command_id, code);
          return false;
        }
      }
    }
    command
      .validate()
      .unwrap_or_else(|error| panic!("command {} validation failed: {error}", command.command_id));
    self.execute_body(&command.body, command.command_id);
    self.reconcile_ui_interactions(&command.body);
    self.record_executed(command, batch_id, group_index, command_index);
    self.reconcile_device_state();
    true
  }

  fn record_executed(
    &mut self,
    command: Command,
    batch_id: battlement::BatchId,
    group_index: usize,
    command_index: usize,
  ) {
    self.executed_commands.insert(command.command_id);
    self.journal.push(ExecutedCommand {
      session_id: self.session_id,
      batch_id,
      group_index,
      command_index,
      command,
    });
  }

  fn execute_body(&mut self, body: &CommandBody, command_id: battlement::CommandId) {
    match body {
      CommandBody::Diagnostics(_) => unreachable!("Diagnostics commands use the Diagnostics fake"),
      CommandBody::AssetsReplaceSet(value) => {
        for (source, _) in self.ui_world.asset_usage() {
          assert!(
            value
              .assets
              .iter()
              .any(|asset| *asset == prepared_for_source(source)),
            "prepared UI asset is still in use: {}",
            source.address()
          );
        }
        self
          .world
          .replace_prepared_assets(value.assets.clone(), &self.assets);
      }
      CommandBody::SceneLoad(value) => self.world.load_scene(
        value.scene_id,
        value.address.clone(),
        value.make_primary,
        &self.assets,
      ),
      CommandBody::SceneUnload(value) => self.world.unload_scene(value.scene_id),
      CommandBody::SceneSetPrimary(value) => self.world.set_primary_scene(value.scene_id),
      CommandBody::ObjectCreate(value) => {
        assert!(
          self.ui_world.element(value.object.object_id).is_none(),
          "object create used a live UI identity: {}",
          value.object.object_id
        );
        self.world.create_object(value.object.clone(), &self.assets)
      }
      CommandBody::ObjectDestroy(value) => self.world.destroy_object(value.object_id),
      CommandBody::ObjectSetActive(value) => self.world.set_active(value.object_id, value.active),
      CommandBody::ObjectReparent(value) => {
        self
          .world
          .reparent(value.object_id, value.parent_id, value.world_position_stays)
      }
      CommandBody::TransformSetLocalPosition(value) => self
        .world
        .set_local_position(value.payload.object_id, value.payload.position),
      CommandBody::TransformSetWorldPosition(value) => self
        .world
        .set_world_position(value.payload.object_id, value.payload.position),
      CommandBody::TransformTweenLocalPosition(value) => {
        let current = self
          .world
          .require_object(value.payload.object_id)
          .local_transform()
          .position;
        self.world.set_local_position(
          value.payload.object_id,
          tween::vector(current, value.payload.position, value.payload.tween),
        );
      }
      CommandBody::TransformTweenWorldPosition(value) => {
        let current = self.world.world_transform(value.payload.object_id).position;
        self.world.set_world_position(
          value.payload.object_id,
          tween::vector(current, value.payload.position, value.payload.tween),
        );
      }
      CommandBody::TransformSetLocalRotation(value) => self
        .world
        .set_local_rotation(value.payload.object_id, value.payload.rotation),
      CommandBody::TransformSetWorldRotation(value) => self
        .world
        .set_world_rotation(value.payload.object_id, value.payload.rotation),
      CommandBody::TransformTweenLocalRotation(value) => {
        let current = self
          .world
          .require_object(value.payload.object_id)
          .local_transform()
          .rotation;
        self.world.set_local_rotation(
          value.payload.object_id,
          if tween::final_factor(value.payload.tween) == 1.0 {
            value.payload.rotation
          } else {
            current
          },
        );
      }
      CommandBody::TransformTweenWorldRotation(value) => {
        let current = self.world.world_transform(value.payload.object_id).rotation;
        self.world.set_world_rotation(
          value.payload.object_id,
          if tween::final_factor(value.payload.tween) == 1.0 {
            value.payload.rotation
          } else {
            current
          },
        );
      }
      CommandBody::TransformSetLocalScale(value) => self
        .world
        .set_local_scale(value.payload.object_id, value.payload.scale),
      CommandBody::TransformTweenLocalScale(value) => {
        let current = self
          .world
          .require_object(value.payload.object_id)
          .local_transform()
          .scale;
        self.world.set_local_scale(
          value.payload.object_id,
          tween::vector(current, value.payload.scale, value.payload.tween),
        );
      }
      CommandBody::RendererSetMaterial(value) => self.set_material(value),
      CommandBody::CameraSetEnabled(value) => self
        .world
        .set_camera_enabled(value.object_id, value.enabled),
      CommandBody::CameraSetPerspective(value) => {
        let camera = self.world.camera_mut(value.payload.object_id);
        camera.projection = battlement::CameraProjection::Perspective;
        camera.field_of_view = value.payload.field_of_view;
      }
      CommandBody::CameraTweenFieldOfView(value) => {
        let camera = self.world.camera_mut(value.payload.object_id);
        camera.projection = battlement::CameraProjection::Perspective;
        camera.field_of_view = tween::scalar(
          camera.field_of_view,
          value.payload.field_of_view,
          value.payload.tween,
        );
      }
      CommandBody::CameraSetOrthographic(value) => {
        let camera = self.world.camera_mut(value.payload.object_id);
        camera.projection = battlement::CameraProjection::Orthographic;
        camera.orthographic_size = value.payload.size;
      }
      CommandBody::CameraTweenOrthographicSize(value) => {
        let camera = self.world.camera_mut(value.payload.object_id);
        camera.projection = battlement::CameraProjection::Orthographic;
        camera.orthographic_size = tween::scalar(
          camera.orthographic_size,
          value.payload.size,
          value.payload.tween,
        );
      }
      CommandBody::CameraSetClipping(value) => {
        let camera = self.world.camera_mut(value.object_id);
        camera.near = value.near;
        camera.far = value.far;
      }
      CommandBody::CameraSetClear(value) => {
        let camera = self.world.camera_mut(value.object_id);
        camera.clear_mode = value.clear_mode;
        if let Some(color) = value.clear_color {
          camera.clear_color = color;
        }
      }
      CommandBody::LightSetEnabled(value) => {
        self.world.light_mut(value.object_id).enabled = value.enabled
      }
      CommandBody::LightSetType(value) => {
        self.world.light_mut(value.object_id).light_type = value.light_type
      }
      CommandBody::LightSetColor(value) => {
        self.world.light_mut(value.payload.object_id).color = value.payload.color
      }
      CommandBody::LightTweenColor(value) => {
        let light = self.world.light_mut(value.payload.object_id);
        light.color = tween::color(light.color, value.payload.color, value.payload.tween);
      }
      CommandBody::LightSetIntensity(value) => {
        self.world.light_mut(value.payload.object_id).intensity = value.payload.intensity
      }
      CommandBody::LightTweenIntensity(value) => {
        let light = self.world.light_mut(value.payload.object_id);
        light.intensity = tween::scalar(
          light.intensity,
          value.payload.intensity,
          value.payload.tween,
        );
      }
      CommandBody::LightSetRange(value) => {
        self.world.light_mut(value.object_id).range = value.range
      }
      CommandBody::LightSetSpotAngle(value) => {
        let light = self.world.light_mut(value.object_id);
        light.inner_spot_angle = value.inner_spot_angle;
        light.outer_spot_angle = value.outer_spot_angle;
      }
      CommandBody::LightSetShadows(value) => {
        self.world.light_mut(value.object_id).shadows = value.shadows
      }
      CommandBody::ImageSetTexture(value) => {
        self.require_prepared(
          PreparedAsset::Texture(value.address.clone()),
          value.address.as_str(),
        );
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Image { image } => image.texture = value.address.clone(),
          _ => panic!("object is not an image: {}", value.object_id),
        }
      }
      CommandBody::ImageSetSize(value) => match &mut self.world.object_mut(value.object_id).kind {
        GameObjectKind::Image { image } => {
          image.width = value.width;
          image.height = value.height;
        }
        _ => panic!("object is not an image: {}", value.object_id),
      },
      CommandBody::ImageSetFit(value) => match &mut self.world.object_mut(value.object_id).kind {
        GameObjectKind::Image { image } => image.fit = value.fit,
        _ => panic!("object is not an image: {}", value.object_id),
      },
      CommandBody::ImageSetTint(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Image { image } => image.tint = value.payload.tint,
          _ => panic!("object is not an image: {}", value.payload.object_id),
        }
      }
      CommandBody::ImageTweenTint(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Image { image } => {
            image.tint = tween::rgb(image.tint, value.payload.tint, value.payload.tween)
          }
          _ => panic!("object is not an image: {}", value.payload.object_id),
        }
      }
      CommandBody::ImageSetOpacity(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Image { image } => image.opacity = value.payload.opacity,
          _ => panic!("object is not an image: {}", value.payload.object_id),
        }
      }
      CommandBody::ImageTweenOpacity(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Image { image } => {
            image.opacity = tween::scalar(image.opacity, value.payload.opacity, value.payload.tween)
          }
          _ => panic!("object is not an image: {}", value.payload.object_id),
        }
      }
      CommandBody::ImageSetFaceCamera(value) => {
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Image { image } => image.face_camera = value.enabled,
          _ => panic!("object is not an image: {}", value.object_id),
        }
      }
      CommandBody::TextSetContent(value) => {
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Text { text } => text.text = value.text.clone(),
          _ => panic!("object is not text: {}", value.object_id),
        }
      }
      CommandBody::TextSetFont(value) => {
        self.require_prepared(
          PreparedAsset::TextMeshProFont(value.address.clone()),
          value.address.as_str(),
        );
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Text { text } => text.font = value.address.clone(),
          _ => panic!("object is not text: {}", value.object_id),
        }
      }
      CommandBody::TextSetSize(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Text { text } => text.size = value.payload.size,
          _ => panic!("object is not text: {}", value.payload.object_id),
        }
      }
      CommandBody::TextTweenSize(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Text { text } => {
            text.size = tween::scalar(text.size, value.payload.size, value.payload.tween)
          }
          _ => panic!("object is not text: {}", value.payload.object_id),
        }
      }
      CommandBody::TextSetColor(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Text { text } => text.color = value.payload.color,
          _ => panic!("object is not text: {}", value.payload.object_id),
        }
      }
      CommandBody::TextTweenColor(value) => {
        match &mut self.world.object_mut(value.payload.object_id).kind {
          GameObjectKind::Text { text } => {
            text.color = tween::color(text.color, value.payload.color, value.payload.tween)
          }
          _ => panic!("object is not text: {}", value.payload.object_id),
        }
      }
      CommandBody::TextSetAlignment(value) => {
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Text { text } => {
            text.horizontal = value.horizontal;
            text.vertical = value.vertical;
          }
          _ => panic!("object is not text: {}", value.object_id),
        }
      }
      CommandBody::TextSetWrapping(value) => {
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Text { text } => text.wrap_width = value.wrap_width,
          _ => panic!("object is not text: {}", value.object_id),
        }
      }
      CommandBody::TextSetRichText(value) => {
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Text { text } => text.rich_text = value.enabled,
          _ => panic!("object is not text: {}", value.object_id),
        }
      }
      CommandBody::TextSetFaceCamera(value) => {
        match &mut self.world.object_mut(value.object_id).kind {
          GameObjectKind::Text { text } => text.face_camera = value.enabled,
          _ => panic!("object is not text: {}", value.object_id),
        }
      }
      CommandBody::AnimatorPlay(value) => self.play_animator(
        value.object_id,
        &value.state,
        value.layer,
        value.normalized_start_time,
      ),
      CommandBody::AnimatorCrossFade(value) => self.play_animator(
        value.object_id,
        &value.state,
        value.layer,
        value.normalized_start_time,
      ),
      CommandBody::AnimatorSetBool(value) => {
        self.require_animator_parameter(
          value.object_id,
          &value.parameter,
          assets::ParameterKind::Bool,
        );
        self
          .world
          .ensure_animator_mut(value.object_id)
          .bool_parameters
          .insert(value.parameter.clone(), value.value);
      }
      CommandBody::AnimatorSetInt(value) => {
        self.require_animator_parameter(
          value.object_id,
          &value.parameter,
          assets::ParameterKind::Int,
        );
        self
          .world
          .ensure_animator_mut(value.object_id)
          .int_parameters
          .insert(value.parameter.clone(), value.value);
      }
      CommandBody::AnimatorSetFloat(value) => {
        self.require_animator_parameter(
          value.object_id,
          &value.parameter,
          assets::ParameterKind::Float,
        );
        self
          .world
          .ensure_animator_mut(value.object_id)
          .float_parameters
          .insert(value.parameter.clone(), value.value);
      }
      CommandBody::AnimatorSetTrigger(value) => self.require_animator_parameter(
        value.object_id,
        &value.parameter,
        assets::ParameterKind::Trigger,
      ),
      CommandBody::AnimatorSetSpeed(value) => {
        self.world.ensure_animator_mut(value.object_id).speed = value.speed
      }
      CommandBody::ParticlePlay(value) => self.set_particles(value.object_id, true),
      CommandBody::ParticleStop(value) => self.set_particles(value.object_id, false),
      CommandBody::ParticleSpawn(value) => {
        self.require_prepared(
          PreparedAsset::ParticleEffect(value.address.clone()),
          value.address.as_str(),
        );
        if let battlement::ParticleSpawnLocation::GameObject(object_id) = value.location {
          self.world.require_object(object_id);
        }
      }
      CommandBody::AudioPlay(value) => {
        self.require_prepared(
          PreparedAsset::AudioClip(value.address.clone()),
          value.address.as_str(),
        );
        self.world.audio_play(
          command_id,
          world::FakeAudio::new(
            value.address.clone(),
            value.volume,
            value.pitch,
            value.r#loop,
          ),
        );
      }
      CommandBody::AudioStop(value) => self.world.audio_remove(value.audio_command_id),
      CommandBody::AudioSetVolume(value) => {
        self.world.audio_mut(value.payload.audio_command_id).volume = value.payload.volume
      }
      CommandBody::AudioTweenVolume(value) => {
        let audio = self.world.audio_mut(value.payload.audio_command_id);
        audio.volume = tween::scalar(audio.volume, value.payload.volume, value.payload.tween);
      }
      CommandBody::TimeWait(_) => {}
      CommandBody::OperationCancel(value) => assert!(
        self.executed_commands.contains(&value.command_id),
        "unknown operation command: {}",
        value.command_id
      ),
      CommandBody::InputSetEnabled(value) => {
        self.world.set_input_enabled(value.enabled);
        if !value.enabled {
          self.ui_world.clear_interaction_state();
        }
      }
      CommandBody::InputSetCamera(value) => self.world.set_input_camera(value.object_id),
      CommandBody::InputSetPointerEvents(value) => self
        .world
        .set_pointer_events(value.object_id, dedupe(value.events.clone())),
      CommandBody::InputSetGlobalKeys(value) => {
        self.world.set_global_keys(dedupe(value.keys.clone()))
      }
      CommandBody::InputSetController(value) => {
        let mut settings = value.clone();
        settings.buttons = dedupe(settings.buttons);
        self.world.set_controller_input(settings);
      }
      CommandBody::ControllerVibrate(_) => {}
      CommandBody::DebugUi(value) => self
        .world
        .set_debug_ui_visible(value.surface, value.visible),
      CommandBody::VisualElementCreate(value) => {
        let identities = battlement::validate_create_subtree(&value.node)
          .expect("validated UI create subtree became invalid");
        assert!(
          identities.iter().all(|id| self.world.object(*id).is_none()),
          "UI create used a live GameObject identity"
        );
        self.require_ui_node_assets(&value.node);
        self
          .ui_world
          .create(value.as_ref().clone())
          .unwrap_or_else(|error| panic!("UI create failed: {error:?}"));
      }
      CommandBody::VisualElementUpdate(value) => {
        assert!(
          self.world.object(value.object_id()).is_none(),
          "UI update targeted a GameObject identity"
        );
        if let battlement::VisualElementUpdate::Properties { element, .. } = value.as_ref() {
          self.require_ui_element_assets(element);
        }
        self
          .ui_world
          .update(value.as_ref().clone())
          .unwrap_or_else(|error| panic!("UI update failed: {error:?}"));
      }
      CommandBody::VisualElementDestroy(value) => {
        assert!(
          self.world.object(value.object_id).is_none(),
          "UI destroy targeted a GameObject identity"
        );
        self
          .ui_world
          .destroy(value.object_id)
          .unwrap_or_else(|error| panic!("UI destroy failed: {error:?}"));
      }
      CommandBody::VisualElementPerformAction(value) => self
        .ui_world
        .perform_action(value.object_id, &value.action)
        .unwrap_or_else(|error| panic!("UI action failed: {error:?}")),
      CommandBody::GeometryObservationUpdate(value) => self
        .geometry_registry
        .apply_update(value)
        .unwrap_or_else(|error| panic!("geometry registry update failed: {error:?}")),
    }
  }

  fn require_prepared(&self, expected: PreparedAsset, address: &str) {
    assert!(
      self.world.prepared(&expected),
      "asset is not prepared: {address}"
    );
    let valid = match &expected {
      PreparedAsset::Scene(value) => self.assets.has_scene(value),
      PreparedAsset::Prefab(value) => self.assets.prefab(value).is_some(),
      PreparedAsset::ParticleEffect(value) => self.assets.has_particle_effect(value),
      PreparedAsset::Material(value) => self.assets.has_material(value),
      PreparedAsset::Texture(value) => self.assets.has_texture(value),
      PreparedAsset::Sprite(value) => self.assets.has_sprite(value),
      PreparedAsset::VectorImage(value) => self.assets.has_vector_image(value),
      PreparedAsset::RenderTexture(value) => self.assets.has_render_texture(value),
      PreparedAsset::AudioClip(value) => self.assets.has_audio_clip(value),
      PreparedAsset::TextMeshProFont(value) => self.assets.has_text_mesh_pro_font(value),
      PreparedAsset::UiFont(value) => self.assets.has_ui_font(value),
    };
    assert!(valid, "unknown asset: {address}");
  }

  fn require_ui_node_assets(&self, node: &UiNode) {
    self.require_ui_element_assets(&node.element);
    for child in &node.children {
      self.require_ui_node_assets(child);
    }
  }

  fn require_ui_element_assets(&self, element: &UiElement) {
    if let UiElement::Image(image) = element
      && let Prop::Set(source) = &image.source
    {
      self.require_ui_source(source);
    }
    if let UiElement::Button(button) = element
      && let Prop::Set(source) = &button.icon
    {
      self.require_prepared(prepared_for_icon(source), source.address());
    }
    self.require_ui_style_assets(&element.visual_element().style);
  }

  fn require_ui_style_assets(&self, style: &Style) {
    if let Prop::Set(StyleValue::Value(address)) = &style.unity_font_definition {
      self.require_prepared(PreparedAsset::UiFont(address.clone()), address.as_str());
    }
  }

  fn require_ui_source(&self, source: &ImageSource) {
    self.require_prepared(prepared_for_source(source), source.address());
  }

  fn set_material(&mut self, value: &PropertyCommand<battlement::SetMaterialPayload>) {
    self.require_prepared(
      PreparedAsset::Material(value.payload.address.clone()),
      value.payload.address.as_str(),
    );
    let object = self.world.require_object(value.payload.object_id);
    let slots = object.renderer_slot_count().unwrap_or_else(|| {
      panic!(
        "object has no supported renderer: {}",
        value.payload.object_id
      )
    });
    let materials = world::materials_mut(&mut self.world.object_mut(value.payload.object_id).kind)
      .unwrap_or_else(|| {
        panic!(
          "object has no supported renderer: {}",
          value.payload.object_id
        )
      });
    if let Some(slot) = value.payload.slot {
      assert!(
        usize::try_from(slot).is_ok_and(|slot| slot < slots),
        "material slot out of range: {slot}"
      );
      if let Some(existing) = materials
        .iter_mut()
        .find(|assignment| assignment.slot == slot)
      {
        existing.address = value.payload.address.clone();
      } else {
        materials.push(MaterialAssignment::new(slot, value.payload.address.clone()));
      }
    } else {
      materials.clear();
      materials.extend(
        (0..slots).map(|slot| MaterialAssignment::new(slot as u32, value.payload.address.clone())),
      );
    }
  }

  fn play_animator(
    &mut self,
    object_id: battlement::ObjectId,
    state: &str,
    layer: u32,
    normalized_start_time: f64,
  ) {
    assert!(
      self
        .world
        .animator_descriptor(object_id)
        .has_state(layer, state),
      "unknown animator state: {state}"
    );
    let object = self.world.object_mut(object_id);
    match &mut object.kind {
      GameObjectKind::Prefab { animator, .. } => {
        let animator = animator.get_or_insert_with(|| AnimatorState::new(state));
        animator.state = state.to_owned();
        animator.layer = layer;
        animator.normalized_start_time = normalized_start_time;
      }
      _ => panic!("object has no Animator component: {object_id}"),
    }
  }

  fn require_animator_parameter(
    &self,
    object_id: battlement::ObjectId,
    name: &str,
    kind: assets::ParameterKind,
  ) {
    assert!(
      self
        .world
        .animator_descriptor(object_id)
        .has_parameter(name, kind),
      "unknown Animator parameter: {name}"
    );
  }

  fn set_particles(&mut self, object_id: battlement::ObjectId, playing: bool) {
    let ids = std::iter::once(object_id).chain(self.world.descendant_ids(object_id));
    let mut changed = false;
    for id in ids {
      if self
        .world
        .object(id)
        .is_some_and(|object| object.particles_playing().is_some())
      {
        *self.world.particles_mut(id) = playing;
        changed = true;
      }
    }
    assert!(
      changed,
      "particle command found no particle systems: {object_id}"
    );
  }
}

fn prepared_for_source(source: &ImageSource) -> PreparedAsset {
  match source {
    ImageSource::Texture(value) => PreparedAsset::Texture(value.clone()),
    ImageSource::Sprite(value) => PreparedAsset::Sprite(value.clone()),
    ImageSource::VectorImage(value) => PreparedAsset::VectorImage(value.clone()),
    ImageSource::RenderTexture(value) => PreparedAsset::RenderTexture(value.clone()),
  }
}

fn prepared_for_icon(source: &IconSource) -> PreparedAsset {
  match source {
    IconSource::Texture(value) => PreparedAsset::Texture(value.clone()),
    IconSource::Sprite(value) => PreparedAsset::Sprite(value.clone()),
    IconSource::VectorImage(value) => PreparedAsset::VectorImage(value.clone()),
    IconSource::RenderTexture(value) => PreparedAsset::RenderTexture(value.clone()),
  }
}

fn dedupe<T>(values: Vec<T>) -> Vec<T>
where
  T: Copy + Eq + std::hash::Hash,
{
  let mut seen = HashSet::with_capacity(values.len());
  values
    .into_iter()
    .filter(|value| seen.insert(*value))
    .collect()
}
