//! Deterministic scheduling for fake-host command operations.

use std::collections::VecDeque;

use battlement::{Batch, BatchId, BatchStart, CommandBody, ConflictPolicy, ParallelCommandGroup};

use crate::{client::FakeClient, operation::ScheduledOperation};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperationKey {
  Position(battlement::ObjectId),
  Rotation(battlement::ObjectId),
  Scale(battlement::ObjectId),
  CameraFieldOfView(battlement::ObjectId),
  CameraOrthographicSize(battlement::ObjectId),
  LightColor(battlement::ObjectId),
  LightIntensity(battlement::ObjectId),
  ImageTint(battlement::ObjectId),
  ImageOpacity(battlement::ObjectId),
  TextSize(battlement::ObjectId),
  TextColor(battlement::ObjectId),
  AudioVolume(battlement::CommandId),
}

pub(crate) struct ScheduledBatch {
  pub(crate) batch_id: BatchId,
  pub(crate) start: BatchStart,
  pub(crate) groups: VecDeque<ParallelCommandGroup>,
  pub(crate) next_group_index: usize,
  pub(crate) started: bool,
  pub(crate) prepares_assets: bool,
}

impl ScheduledBatch {
  pub(crate) fn new(batch: Batch) -> Self {
    let prepares_assets = batch.groups.iter().any(|group| {
      group
        .commands
        .iter()
        .any(|command| matches!(command.body, CommandBody::AssetsReplaceSet(_)))
    });
    Self {
      batch_id: batch.batch_id,
      start: batch.start,
      groups: batch.groups.into(),
      next_group_index: 0,
      started: false,
      prepares_assets,
    }
  }
}
pub(crate) fn command_key(body: &CommandBody) -> Option<OperationKey> {
  match body {
    CommandBody::TransformSetLocalPosition(value)
    | CommandBody::TransformSetWorldPosition(value) => {
      Some(OperationKey::Position(value.payload.object_id))
    }
    CommandBody::TransformTweenLocalPosition(value)
    | CommandBody::TransformTweenWorldPosition(value) => {
      Some(OperationKey::Position(value.payload.object_id))
    }
    CommandBody::TransformSetLocalRotation(value)
    | CommandBody::TransformSetWorldRotation(value) => {
      Some(OperationKey::Rotation(value.payload.object_id))
    }
    CommandBody::TransformTweenLocalRotation(value)
    | CommandBody::TransformTweenWorldRotation(value) => {
      Some(OperationKey::Rotation(value.payload.object_id))
    }
    CommandBody::TransformSetLocalScale(value) => {
      Some(OperationKey::Scale(value.payload.object_id))
    }
    CommandBody::TransformTweenLocalScale(value) => {
      Some(OperationKey::Scale(value.payload.object_id))
    }
    CommandBody::CameraSetPerspective(value) => {
      Some(OperationKey::CameraFieldOfView(value.payload.object_id))
    }
    CommandBody::CameraTweenFieldOfView(value) => {
      Some(OperationKey::CameraFieldOfView(value.payload.object_id))
    }
    CommandBody::CameraSetOrthographic(value) => Some(OperationKey::CameraOrthographicSize(
      value.payload.object_id,
    )),
    CommandBody::CameraTweenOrthographicSize(value) => Some(OperationKey::CameraOrthographicSize(
      value.payload.object_id,
    )),
    CommandBody::LightSetColor(value) => Some(OperationKey::LightColor(value.payload.object_id)),
    CommandBody::LightTweenColor(value) => Some(OperationKey::LightColor(value.payload.object_id)),
    CommandBody::LightSetIntensity(value) => {
      Some(OperationKey::LightIntensity(value.payload.object_id))
    }
    CommandBody::LightTweenIntensity(value) => {
      Some(OperationKey::LightIntensity(value.payload.object_id))
    }
    CommandBody::ImageSetTint(value) => Some(OperationKey::ImageTint(value.payload.object_id)),
    CommandBody::ImageTweenTint(value) => Some(OperationKey::ImageTint(value.payload.object_id)),
    CommandBody::ImageSetOpacity(value) => {
      Some(OperationKey::ImageOpacity(value.payload.object_id))
    }
    CommandBody::ImageTweenOpacity(value) => {
      Some(OperationKey::ImageOpacity(value.payload.object_id))
    }
    CommandBody::TextSetSize(value) => Some(OperationKey::TextSize(value.payload.object_id)),
    CommandBody::TextTweenSize(value) => Some(OperationKey::TextSize(value.payload.object_id)),
    CommandBody::TextSetColor(value) => Some(OperationKey::TextColor(value.payload.object_id)),
    CommandBody::TextTweenColor(value) => Some(OperationKey::TextColor(value.payload.object_id)),
    CommandBody::AudioSetVolume(value) => {
      Some(OperationKey::AudioVolume(value.payload.audio_command_id))
    }
    CommandBody::AudioTweenVolume(value) => {
      Some(OperationKey::AudioVolume(value.payload.audio_command_id))
    }
    _ => None,
  }
}

pub(crate) fn conflict_policy(body: &CommandBody) -> ConflictPolicy {
  match body {
    CommandBody::TransformSetLocalPosition(value)
    | CommandBody::TransformSetWorldPosition(value) => value.on_conflict,
    CommandBody::TransformTweenLocalPosition(value)
    | CommandBody::TransformTweenWorldPosition(value) => value.on_conflict,
    CommandBody::TransformSetLocalRotation(value)
    | CommandBody::TransformSetWorldRotation(value) => value.on_conflict,
    CommandBody::TransformTweenLocalRotation(value)
    | CommandBody::TransformTweenWorldRotation(value) => value.on_conflict,
    CommandBody::TransformSetLocalScale(value) => value.on_conflict,
    CommandBody::TransformTweenLocalScale(value) => value.on_conflict,
    CommandBody::CameraSetPerspective(value) => value.on_conflict,
    CommandBody::CameraTweenFieldOfView(value) => value.on_conflict,
    CommandBody::CameraSetOrthographic(value) => value.on_conflict,
    CommandBody::CameraTweenOrthographicSize(value) => value.on_conflict,
    CommandBody::LightSetColor(value) => value.on_conflict,
    CommandBody::LightTweenColor(value) => value.on_conflict,
    CommandBody::LightSetIntensity(value) => value.on_conflict,
    CommandBody::LightTweenIntensity(value) => value.on_conflict,
    CommandBody::ImageSetTint(value) => value.on_conflict,
    CommandBody::ImageTweenTint(value) => value.on_conflict,
    CommandBody::ImageSetOpacity(value) => value.on_conflict,
    CommandBody::ImageTweenOpacity(value) => value.on_conflict,
    CommandBody::TextSetSize(value) => value.on_conflict,
    CommandBody::TextTweenSize(value) => value.on_conflict,
    CommandBody::TextSetColor(value) => value.on_conflict,
    CommandBody::TextTweenColor(value) => value.on_conflict,
    CommandBody::AudioSetVolume(value) => value.on_conflict,
    CommandBody::AudioTweenVolume(value) => value.on_conflict,
    _ => ConflictPolicy::Cancel,
  }
}

impl<E> FakeClient<E>
where
  E: battlement_native::Engine,
{
  pub(crate) fn schedule_batch(&mut self, batch: Batch) {
    self.scheduled_batches.push(ScheduledBatch::new(batch));
    self.pump_presentation();
  }

  pub(crate) fn schedule_operation(&mut self, operation: ScheduledOperation) {
    if operation.advance(&mut self.world, self.presentation_ms) {
      return;
    }
    self.operations.push(operation);
  }

  pub(crate) fn cancel_operation(&mut self, command_id: battlement::CommandId) {
    assert!(
      self.executed_commands.contains(&command_id),
      "unknown operation command: {command_id}"
    );
    self
      .operations
      .retain(|operation| operation.command_id != command_id);
  }

  pub(crate) fn cancel_conflicts(&mut self, key: OperationKey, policy: ConflictPolicy) {
    let conflicts = self
      .operations
      .iter()
      .any(|operation| operation.key == Some(key));
    if !conflicts {
      return;
    }
    assert_eq!(
      policy,
      ConflictPolicy::Cancel,
      "waiting property commands require the conflicting operation to finish first"
    );
    self
      .operations
      .retain(|operation| operation.key != Some(key));
  }

  pub(crate) fn reset_presentation(&mut self) {
    self.scheduled_batches.clear();
    self.operations.clear();
  }

  pub(crate) fn advance_presentation_to(&mut self, target_ms: u64) {
    assert!(
      target_ms >= self.presentation_ms,
      "presentation time cannot move backward"
    );
    self.pump_presentation();
    while let Some(deadline) = self.next_deadline() {
      if deadline > target_ms {
        break;
      }
      self.presentation_ms = deadline;
      self.pump_presentation();
    }
    self.presentation_ms = target_ms;
    self.pump_presentation();
  }

  pub(crate) fn next_deadline(&self) -> Option<u64> {
    self
      .operations
      .iter()
      .filter_map(ScheduledOperation::deadline_ms)
      .filter(|deadline| *deadline > self.presentation_ms)
      .min()
  }

  pub(crate) fn pump_presentation(&mut self) {
    loop {
      let operations = std::mem::take(&mut self.operations);
      self.operations = operations
        .into_iter()
        .filter(|operation| !operation.advance(&mut self.world, self.presentation_ms))
        .collect();

      let mut progressed = false;
      for index in 0..self.scheduled_batches.len() {
        if !self.batch_can_advance(index) {
          continue;
        }
        self.scheduled_batches[index].started = true;
        if let Some(group) = self.scheduled_batches[index].groups.pop_front() {
          let batch_id = self.scheduled_batches[index].batch_id;
          let group_index = self.scheduled_batches[index].next_group_index;
          self.scheduled_batches[index].next_group_index += 1;
          let mut failed = false;
          for (command_index, command) in group.commands.into_iter().enumerate() {
            if !self.execute_command(command, batch_id, group_index, command_index) {
              failed = true;
              break;
            }
          }
          if failed {
            self.scheduled_batches[index].groups.clear();
          }
          progressed = true;
        }
      }
      let before = self.scheduled_batches.len();
      self.scheduled_batches.retain(|batch| {
        !batch.groups.is_empty()
          || self
            .operations
            .iter()
            .any(|operation| operation.batch_id == batch.batch_id && operation.blocking)
      });
      progressed |= before != self.scheduled_batches.len();
      if !progressed {
        break;
      }
    }
  }

  fn batch_can_advance(&self, index: usize) -> bool {
    let batch = &self.scheduled_batches[index];
    if self
      .operations
      .iter()
      .any(|operation| operation.batch_id == batch.batch_id && operation.blocking)
    {
      return false;
    }
    if batch.started {
      return !batch.groups.is_empty();
    }
    match batch.start {
      BatchStart::Now => true,
      BatchStart::AfterEarlierBlockingWork => index == 0,
      BatchStart::AfterEarlierAssetPreparation => !self.scheduled_batches[..index]
        .iter()
        .any(|earlier| earlier.prepares_assets),
    }
  }
}
