//! Timed fake-host command operations.

use battlement::{BatchId, Command, CommandBody, Easing, RepeatMode, Tween, TweenRepeat};

use crate::{
  interpolation::{OperationValue, ease},
  motion_playbacks::RunningMotion,
  presentation::OperationKey,
  world::FakeWorld,
};

pub(crate) struct ScheduledOperation {
  pub(crate) command_id: battlement::CommandId,
  pub(crate) batch_id: BatchId,
  pub(crate) scope: Option<u64>,
  pub(crate) retention: Option<battlement_native::ResponseLease>,
  pub(crate) blocking: bool,
  pub(crate) key: Option<OperationKey>,
  started_ms: u64,
  timing: Timing,
  value: OperationValue,
  motion: Option<RunningMotion>,
  paused_at_ms: Option<u64>,
}

impl ScheduledOperation {
  pub(crate) fn from_motion(
    command: &Command,
    batch_id: BatchId,
    started_ms: u64,
    motion: RunningMotion,
  ) -> Self {
    assert!(
      !command.blocking || !motion.infinite,
      "infinite Motion cannot block a command batch"
    );
    Self {
      command_id: command.command_id,
      batch_id,
      scope: None,
      retention: None,
      blocking: command.blocking,
      key: None,
      started_ms,
      timing: Timing::wait(0),
      value: OperationValue::Wait,
      motion: Some(motion),
      paused_at_ms: None,
    }
  }

  pub(crate) fn from_command(
    command: &Command,
    batch_id: BatchId,
    started_ms: u64,
    world: &FakeWorld,
  ) -> Option<Self> {
    let (timing, key, value) = match &command.body {
      CommandBody::TransformTweenLocalPosition(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::Position(value.payload.object_id)),
        OperationValue::LocalPosition {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .local_transform()
            .position,
          target: value.payload.position,
        },
      ),
      CommandBody::TransformTweenWorldPosition(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::Position(value.payload.object_id)),
        OperationValue::WorldPosition {
          object_id: value.payload.object_id,
          start: world.world_transform(value.payload.object_id).position,
          target: value.payload.position,
        },
      ),
      CommandBody::TransformTweenLocalRotation(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::Rotation(value.payload.object_id)),
        OperationValue::LocalRotation {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .local_transform()
            .rotation,
          target: value.payload.rotation,
        },
      ),
      CommandBody::TransformTweenWorldRotation(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::Rotation(value.payload.object_id)),
        OperationValue::WorldRotation {
          object_id: value.payload.object_id,
          start: world.world_transform(value.payload.object_id).rotation,
          target: value.payload.rotation,
        },
      ),
      CommandBody::TransformTweenLocalScale(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::Scale(value.payload.object_id)),
        OperationValue::LocalScale {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .local_transform()
            .scale,
          target: value.payload.scale,
        },
      ),
      CommandBody::CameraTweenFieldOfView(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::CameraFieldOfView(value.payload.object_id)),
        OperationValue::CameraFieldOfView {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .camera()
            .unwrap()
            .field_of_view,
          target: value.payload.field_of_view,
        },
      ),
      CommandBody::CameraTweenOrthographicSize(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::CameraOrthographicSize(
          value.payload.object_id,
        )),
        OperationValue::CameraOrthographicSize {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .camera()
            .unwrap()
            .orthographic_size,
          target: value.payload.size,
        },
      ),
      CommandBody::LightTweenColor(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::LightColor(value.payload.object_id)),
        OperationValue::LightColor {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .light()
            .unwrap()
            .color,
          target: value.payload.color,
        },
      ),
      CommandBody::LightTweenIntensity(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::LightIntensity(value.payload.object_id)),
        OperationValue::LightIntensity {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .light()
            .unwrap()
            .intensity,
          target: value.payload.intensity,
        },
      ),
      CommandBody::ImageTweenTint(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::ImageTint(value.payload.object_id)),
        OperationValue::ImageTint {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .image()
            .unwrap()
            .tint,
          target: value.payload.tint,
        },
      ),
      CommandBody::ImageTweenOpacity(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::ImageOpacity(value.payload.object_id)),
        OperationValue::ImageOpacity {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .image()
            .unwrap()
            .opacity,
          target: value.payload.opacity,
        },
      ),
      CommandBody::TextTweenSize(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::TextSize(value.payload.object_id)),
        OperationValue::TextSize {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .text()
            .unwrap()
            .size,
          target: value.payload.size,
        },
      ),
      CommandBody::TextTweenColor(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::TextColor(value.payload.object_id)),
        OperationValue::TextColor {
          object_id: value.payload.object_id,
          start: world
            .require_object(value.payload.object_id)
            .text()
            .unwrap()
            .color,
          target: value.payload.color,
        },
      ),
      CommandBody::AudioTweenVolume(value) => (
        Timing::tween(value.payload.tween),
        Some(OperationKey::AudioVolume(value.payload.audio_command_id)),
        OperationValue::AudioVolume {
          command_id: value.payload.audio_command_id,
          start: world
            .audio(value.payload.audio_command_id)
            .unwrap()
            .volume(),
          target: value.payload.volume,
        },
      ),
      CommandBody::TimeWait(value) => (Timing::wait(value.duration_ms), None, OperationValue::Wait),
      _ => return None,
    };
    Some(Self {
      command_id: command.command_id,
      batch_id,
      scope: None,
      retention: None,
      blocking: command.blocking,
      key,
      started_ms,
      timing,
      value,
      motion: None,
      paused_at_ms: None,
    })
  }

  pub(crate) fn advance(&self, world: &mut FakeWorld, now_ms: u64) -> bool {
    if self.paused_at_ms.is_some() {
      return false;
    }
    if let Some(motion) = &self.motion {
      return motion.outcome().is_some();
    }
    let (factor, complete) = self.timing.factor(now_ms.saturating_sub(self.started_ms));
    self.value.apply(world, factor);
    complete
  }

  pub(crate) fn failure(&self) -> Option<String> {
    self.motion.as_ref().and_then(RunningMotion::failure)
  }

  pub(crate) fn deadline_ms(&self) -> Option<u64> {
    if self.paused_at_ms.is_some() {
      return None;
    }
    if self.motion.is_some() {
      return None;
    }
    self.timing.finite_duration_ms().map(|duration| {
      self
        .started_ms
        .checked_add(duration)
        .expect("operation deadline overflowed")
    })
  }

  pub(crate) fn pause(&mut self, now_ms: u64) {
    self.paused_at_ms.get_or_insert(now_ms);
  }

  pub(crate) fn resume(&mut self, now_ms: u64) {
    let Some(paused_at) = self.paused_at_ms.take() else {
      return;
    };
    self.started_ms = self
      .started_ms
      .checked_add(now_ms.saturating_sub(paused_at))
      .expect("operation start overflowed after pause");
  }

  pub(crate) fn motion_identity(&self) -> Option<(battlement::ObjectId, u32)> {
    self.motion.as_ref().map(RunningMotion::identity)
  }
}

impl Drop for ScheduledOperation {
  fn drop(&mut self) {
    if let Some(motion) = &self.motion {
      motion.cancel();
    }
  }
}

#[derive(Clone, Copy)]
struct Timing {
  delay_ms: u64,
  duration_ms: u64,
  easing: Easing,
  repeat: TweenRepeat,
}

impl Timing {
  fn tween(value: Tween) -> Self {
    Self {
      delay_ms: value.delay_ms,
      duration_ms: value.duration_ms,
      easing: value.easing,
      repeat: value.repeat,
    }
  }

  fn wait(duration_ms: u64) -> Self {
    Self {
      delay_ms: duration_ms,
      duration_ms: 0,
      easing: Easing::Linear,
      repeat: TweenRepeat::Once,
    }
  }

  fn finite_duration_ms(self) -> Option<u64> {
    let traversals = match self.repeat {
      TweenRepeat::Once => 1,
      TweenRepeat::Count {
        additional_traversals,
        ..
      } => u64::from(additional_traversals) + 1,
      TweenRepeat::Forever(_) => return None,
    };
    Some(
      self
        .delay_ms
        .checked_add(
          self
            .duration_ms
            .checked_mul(traversals)
            .expect("tween duration overflowed"),
        )
        .expect("tween duration overflowed"),
    )
  }

  fn factor(self, elapsed_ms: u64) -> (f64, bool) {
    if elapsed_ms < self.delay_ms {
      return (0.0, false);
    }
    let active_ms = elapsed_ms - self.delay_ms;
    let Some(total_ms) = self.finite_duration_ms() else {
      return (self.active_factor(active_ms), false);
    };
    if active_ms >= total_ms - self.delay_ms {
      return (self.final_factor(), true);
    }
    (self.active_factor(active_ms), false)
  }

  fn active_factor(self, active_ms: u64) -> f64 {
    if self.duration_ms == 0 {
      return 1.0;
    }
    let traversal = active_ms / self.duration_ms;
    let progress = (active_ms % self.duration_ms) as f64 / self.duration_ms as f64;
    let eased = ease(self.easing, progress);
    if self.is_ping_pong() && traversal % 2 == 1 {
      1.0 - eased
    } else {
      eased
    }
  }

  fn final_factor(self) -> f64 {
    match self.repeat {
      TweenRepeat::Count {
        additional_traversals,
        mode: RepeatMode::PingPong,
      } if additional_traversals % 2 == 1 => 0.0,
      _ => 1.0,
    }
  }

  fn is_ping_pong(self) -> bool {
    matches!(
      self.repeat,
      TweenRepeat::Count {
        mode: RepeatMode::PingPong,
        ..
      } | TweenRepeat::Forever(RepeatMode::PingPong)
    )
  }
}
