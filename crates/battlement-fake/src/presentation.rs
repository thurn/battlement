//! Deterministic scheduling for fake-host command operations.

use std::collections::VecDeque;

use battlement::{
  Batch, BatchId, BatchStart, CommandBody, ConflictPolicy, ParallelCommandGroup, UiNode,
};

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
  pub(crate) scope: Option<u64>,
  pub(crate) cancellation: bool,
  pub(crate) failed: bool,
  pub(crate) retention: Option<battlement_native::ResponseLease>,
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
      scope: batch.work_scope,
      cancellation: batch.cancel_scope.is_some(),
      failed: false,
      retention: None,
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
    if let Some(control) = batch.presentation_control {
      self.apply_presentation_control(control);
      self.pump_presentation();
      return;
    }
    if let Some(scope) = batch.cancel_scope {
      self.canceled_scopes.insert(scope);
      self.paused_scopes.remove(&scope);
      let owned = self
        .work_objects
        .iter()
        .filter(|(_, (owner, _))| *owner == scope)
        .map(|(id, (_, ui))| (*id, *ui))
        .collect::<Vec<_>>();
      for (id, ui) in owned {
        if ui {
          if self.ui_world.element(id).is_some() {
            self.ui_world.destroy(id).expect("owned UI cleanup");
          }
        } else if self.world.object(id).is_some() {
          self.world.destroy_object(id);
        }
        self.work_objects.remove(&id);
      }
      let ids = self
        .scheduled_batches
        .iter()
        .filter(|batch| batch.scope == Some(scope))
        .map(|batch| batch.batch_id)
        .collect::<std::collections::HashSet<_>>();
      for batch in &mut self.scheduled_batches {
        if batch.scope == Some(scope) {
          batch.groups.clear();
          batch.failed = true;
          batch.retention = None;
        }
      }
      let paused_motion = self
        .operations
        .iter()
        .filter(|operation| operation.scope == Some(scope))
        .filter_map(ScheduledOperation::motion_identity)
        .collect::<Vec<_>>();
      self
        .operations
        .retain(|operation| !ids.contains(&operation.batch_id) && operation.scope != Some(scope));
      for (id, _) in paused_motion {
        self.motion.discard_scope_pause(id);
      }
    }
    if batch
      .work_scope
      .is_some_and(|scope| self.canceled_scopes.contains(&scope))
    {
      return;
    }
    let mut scheduled = ScheduledBatch::new(batch);
    scheduled.retention = self.response_retention.clone();
    crate::batch_ordering::assert_ordered(&self.scheduled_batches, &scheduled, &self.ui_world);
    self.scheduled_batches.push(scheduled);
    self.pump_presentation();
  }

  fn apply_presentation_control(&mut self, control: battlement::PresentationControl) {
    if self.canceled_scopes.contains(&control.work_scope) {
      return;
    }
    if control.paused {
      let owners = self.paused_scopes.entry(control.work_scope).or_default();
      if !owners.insert(control.owner_id) || owners.len() != 1 {
        return;
      }
      let identities = self
        .operations
        .iter_mut()
        .filter(|operation| operation.scope == Some(control.work_scope))
        .filter_map(|operation| {
          operation.pause(self.presentation_ms);
          operation.motion_identity()
        })
        .collect::<Vec<_>>();
      for (id, generation) in identities {
        self
          .motion
          .pause_for_scope(id, generation, self.presentation_ms * 1000);
      }
      return;
    }
    let Some(owners) = self.paused_scopes.get_mut(&control.work_scope) else {
      return;
    };
    if !owners.remove(&control.owner_id) || !owners.is_empty() {
      return;
    }
    self.paused_scopes.remove(&control.work_scope);
    let identities = self
      .operations
      .iter_mut()
      .filter(|operation| operation.scope == Some(control.work_scope))
      .filter_map(|operation| {
        operation.resume(self.presentation_ms);
        operation.motion_identity()
      })
      .collect::<Vec<_>>();
    for (id, generation) in identities {
      self
        .motion
        .resume_for_scope(id, generation, self.presentation_ms * 1000);
    }
  }

  pub(crate) fn schedule_operation(&mut self, mut operation: ScheduledOperation) {
    if let Some(batch) = self
      .scheduled_batches
      .iter()
      .find(|batch| batch.batch_id == operation.batch_id)
    {
      operation.retention = batch.retention.clone();
      operation.scope = batch.scope;
    }
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
    self.work_objects.clear();
    self.motion.clear_scope_pauses();
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
      .chain(
        self
          .motion
          .deadline(self.presentation_ms * 1000)
          .map(|value| value.div_ceil(1000)),
      )
      .filter(|deadline| *deadline > self.presentation_ms)
      .min()
  }

  pub(crate) fn pump_presentation(&mut self) {
    if self.presentation_advancing {
      return;
    }
    self.presentation_advancing = true;
    self.advance_batches();
    self.repick_geometric_pointers();
    self.advance_batches();
    while let Some(events) = self.motion.drain() {
      self.submit_motion(events);
      self.advance_batches();
    }
    self.presentation_advancing = false;
  }

  fn advance_batches(&mut self) {
    loop {
      let frame = self.frame();
      self.motion.sample(
        &mut self.world,
        &mut self.ui_world,
        self.presentation_ms * 1000,
        frame,
      );
      self.record_motion_occurrences();
      if let Some(events) = self.motion.drain() {
        self.submit_motion(events);
      }
      let operations = std::mem::take(&mut self.operations);
      let mut failures = Vec::new();
      for operation in operations {
        if !operation.advance(&mut self.world, self.presentation_ms) {
          self.operations.push(operation);
        } else if let Some(message) = operation.failure() {
          failures.push((
            operation.batch_id,
            operation.command_id,
            operation.blocking,
            message,
          ));
        }
      }
      for (batch, command, blocking, message) in failures {
        if blocking {
          for pending in &mut self.scheduled_batches {
            if pending.batch_id == batch {
              pending.groups.clear();
              pending.failed = true;
            }
          }
          self
            .operations
            .retain(|operation| operation.batch_id != batch);
        }
        self.submit_presentation_failure(
          batch,
          command,
          battlement::CoreErrorCode::InvalidProperty,
          &message,
          blocking,
        );
      }

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
          let cancellation = self.scheduled_batches[index].cancellation;
          for (command_index, command) in group.commands.into_iter().enumerate() {
            if cancellation && self.missing_cleanup_target(&command.body) {
              continue;
            }
            self.record_work_creation(&command.body, self.scheduled_batches[index].scope);
            let destroyed = matches!(
              command.body,
              CommandBody::VisualElementDestroy(_) | CommandBody::ObjectDestroy(_)
            );
            if !self.execute_command(command, batch_id, group_index, command_index) {
              failed = true;
              break;
            }
            if destroyed {
              self.work_objects.retain(|id, (_, ui)| {
                if *ui {
                  self.ui_world.element(*id).is_some()
                } else {
                  self.world.object(*id).is_some()
                }
              });
            }
          }
          if failed {
            self.scheduled_batches[index].failed = true;
            self.scheduled_batches[index].groups.clear();
          }
          progressed = true;
        }
      }
      let before = self.scheduled_batches.len();
      let mut completed = Vec::new();
      self.scheduled_batches.retain(|batch| {
        let work_remains = !batch.groups.is_empty()
          || self
            .operations
            .iter()
            .any(|operation| operation.batch_id == batch.batch_id && operation.blocking);
        let pending = !batch.failed && (!batch.started || work_remains);
        if !pending && !batch.failed && batch.scope.is_some() {
          completed.push(batch.batch_id);
        }
        pending
      });
      progressed |= before != self.scheduled_batches.len();
      for batch in completed {
        self.submit_presentation_completed(batch);
      }
      if !progressed {
        break;
      }
    }
  }

  fn record_work_creation(&mut self, command: &CommandBody, scope: Option<u64>) {
    let Some(scope) = scope else {
      return;
    };
    match command {
      CommandBody::VisualElementCreate(value) => self.record_work_ui(&value.node, scope),
      CommandBody::ObjectCreate(value) => {
        self
          .work_objects
          .insert(value.object.object_id, (scope, false));
      }
      _ => (),
    }
  }

  fn record_work_ui(&mut self, node: &UiNode, scope: u64) {
    self.work_objects.insert(node.object_id, (scope, true));
    for child in &node.children {
      self.record_work_ui(child, scope);
    }
  }

  fn missing_cleanup_target(&self, command: &CommandBody) -> bool {
    match command {
      CommandBody::VisualElementDestroy(value) => self.ui_world.element(value.object_id).is_none(),
      CommandBody::ObjectDestroy(value) => self.world.object(value.object_id).is_none(),
      _ => false,
    }
  }

  fn batch_can_advance(&self, index: usize) -> bool {
    let batch = &self.scheduled_batches[index];
    if batch
      .scope
      .is_some_and(|scope| self.paused_scopes.contains_key(&scope))
    {
      return false;
    }
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
    !self.scheduled_batches[..index]
      .iter()
      .any(|earlier| crate::batch_ordering::depends_on(batch, earlier))
  }
}
