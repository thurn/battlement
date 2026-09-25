//! Bounded native delivery and publication acceptance.

use std::{collections::HashMap, mem, rc::Rc};

use battlement::{Batch, BatchId, CommandBody};
use battlement_flatbuffers::{MessageWriter, NativeBatchStart};
use battlement_native::{EngineError, EngineResponse, ResponseBudget};

use crate::{
  app_delivery::{DeliveryMessage, DeliveryResponse},
  app_runtime::{AppOutput, AppRuntime},
};

/// Bounds gameplay allocations while reserving independent app/control delivery.
#[derive(Clone, Copy)]
pub struct DeliveryLimits {
  /// Cumulative gameplay allocations, including queued and still-borrowed content.
  pub gameplay_bytes: usize,
  /// Largest individual gameplay response; the wire limit remains authoritative.
  pub message_bytes: usize,
}

impl Default for DeliveryLimits {
  fn default() -> Self {
    Self {
      gameplay_bytes: 32 * 1024 * 1024,
      message_bytes: 16 * 1024 * 1024,
    }
  }
}

pub(crate) struct OutputDelivery {
  limits: DeliveryLimits,
  gameplay: ResponseBudget,
  control: ResponseBudget,
  scope: Option<u64>,
  pending: Vec<DeliveryMessage>,
  encoded: Option<EngineResponse>,
  pub(crate) publication: Option<Box<dyn AppOutput>>,
  batches: HashMap<[u8; 16], (u64, bool)>,
}

impl Default for OutputDelivery {
  fn default() -> Self {
    Self::new(DeliveryLimits::default())
  }
}

impl OutputDelivery {
  pub(crate) fn new(limits: DeliveryLimits) -> Self {
    assert!(
      limits.gameplay_bytes <= 32 * 1024 * 1024,
      "gameplay must reserve half the native budget for controls"
    );
    assert!(
      limits.message_bytes > 0 && limits.message_bytes <= 16 * 1024 * 1024,
      "invalid response message limit"
    );
    Self {
      limits,
      gameplay: ResponseBudget::new(limits.gameplay_bytes),
      control: ResponseBudget::new(32 * 1024 * 1024),
      scope: None,
      pending: Vec::new(),
      encoded: None,
      publication: None,
      batches: HashMap::new(),
    }
  }

  pub(crate) fn ending_scope(&self, active: Option<u64>) -> Option<u64> {
    self.scope.filter(|scope| Some(*scope) != active)
  }

  pub(crate) fn can_consume(&self) -> bool {
    self.publication.is_none() && self.pending.is_empty() && self.gameplay.available_bytes() > 0
  }

  pub(crate) fn failed_batch(&self, id: [u8; 16]) -> Option<u64> {
    self.batches.get(&id).map(|(scope, _)| *scope)
  }

  pub(crate) fn completed_batch(&mut self, id: [u8; 16], runtime: Option<&dyn AppRuntime>) {
    if let Some((scope, true)) = self.batches.remove(&id)
      && let Some(runtime) = runtime
    {
      runtime.presentation_pending(scope, false);
    }
  }

  pub(crate) fn retained_bytes(&self) -> usize {
    self.gameplay.retained_bytes()
  }

  pub(crate) fn finish(
    &mut self,
    mut response: DeliveryResponse,
    runtime: Option<Rc<dyn AppRuntime>>,
  ) -> Result<Option<EngineResponse>, EngineError> {
    if runtime.is_none() {
      return if response.messages.is_empty() {
        Ok(None)
      } else {
        self::encode(&response).map(Some)
      };
    }
    let active = runtime.as_ref().and_then(|runtime| runtime.work_scope());
    let mut controls = Vec::new();
    if self.scope != active {
      if let Some(old) = self.scope {
        let mut cancel = Batch::new(BatchId::new_v4(), response.session_id, Vec::new());
        cancel.cancel_scope = Some(old);
        controls.push(DeliveryMessage::Batch(cancel));
      }
      self.scope = active;
      self.pending.clear();
      self.encoded = None;
      self.batches.clear();
      if self
        .publication
        .as_ref()
        .is_some_and(|output| Some(output.scope()) != active)
      {
        self.publication = None;
      }
    }
    for message in mem::take(&mut response.messages) {
      match message {
        DeliveryMessage::Batch(mut batch) if batch.work_scope.is_some() => {
          if batch.work_scope == active {
            self.replace_pending_observation(&batch);
            self.pending.push(DeliveryMessage::Batch(batch));
            self.encoded = None;
          } else {
            batch.cancel_scope = batch.work_scope.take();
            batch.start = battlement::BatchStart::Now;
            for group in &mut batch.groups {
              group.commands.retain(|command| {
                matches!(
                  command.body,
                  CommandBody::VisualElementDestroy(_) | CommandBody::ObjectDestroy(_)
                )
              });
            }
            batch.groups.retain(|group| !group.commands.is_empty());
            controls.push(DeliveryMessage::Batch(batch));
          }
        }
        message => controls.push(message),
      }
    }
    // Validate the one unsubmitted output even while independent controls are busy.
    if !self.pending.is_empty() && self.encoded.is_none() {
      let pending = DeliveryResponse {
        session_id: response.session_id,
        messages: mem::take(&mut self.pending),
      };
      let result = self::encode(&pending);
      self.pending = pending.messages;
      match result {
        Ok(encoded) if encoded.as_bytes().len() <= self.limits.message_bytes => {
          if encoded.allocation_bytes() > self.limits.gameplay_bytes {
            self.fail(
              runtime.as_deref(),
              "single gameplay output exceeds its allocation budget".to_owned(),
            );
          } else {
            self.encoded = Some(encoded);
          }
        }
        Ok(_) => self.fail(
          runtime.as_deref(),
          "single gameplay output exceeds its response limit".to_owned(),
        ),
        Err(error) => self.fail(runtime.as_deref(), error.to_string()),
      }
    }
    if !controls.is_empty() && self.encoded.is_some() {
      let count = controls.len();
      controls.append(&mut self.pending);
      response.messages = controls;
      let combined = self::encode(&response);
      self.pending = response.messages.split_off(count);
      controls = mem::take(&mut response.messages);
      if let Ok(mut combined) = combined
        && self.gameplay.admit(&mut combined)
      {
        self.record_submission(runtime.as_deref());
        self.encoded = None;
        self.accept();
        return Ok(Some(combined));
      }
    }
    if !controls.is_empty() {
      response.messages = controls;
      let mut encoded = self::encode(&response)?;
      if !self.control.admit(&mut encoded) {
        return Err(EngineError::new(
          "independent control allocation budget exhausted",
        ));
      }
      if self.pending.is_empty() {
        self.accept();
      }
      return Ok(Some(encoded));
    }
    if let Some(encoded) = &mut self.encoded {
      if !self.gameplay.admit(encoded) {
        return Ok(None);
      }
      self.record_submission(runtime.as_deref());
      let encoded = self.encoded.take();
      self.accept();
      return Ok(encoded);
    }
    self.accept();
    Ok(None)
  }

  fn replace_pending_observation(&mut self, incoming: &Batch) {
    let replaces_snapshot = incoming.groups.iter().flat_map(|group| &group.commands).any(|command| {
      matches!(&command.body, CommandBody::AccessibilityUpdate(update) if update.snapshot.is_some())
    });
    if !replaces_snapshot {
      return;
    }
    // Only the current unsubmitted publication can be pending. Menu rerenders replace
    // its complete document observation, while every announcement remains ordered.
    self.pending.retain_mut(|message| {
      let DeliveryMessage::Batch(batch) = message else {
        return true;
      };
      for group in &mut batch.groups {
        group.commands.retain_mut(|command| {
          if let CommandBody::AccessibilityUpdate(update) = &mut command.body {
            update.snapshot = None;
            return !update.announcements.is_empty();
          }
          true
        });
      }
      batch.groups.retain(|group| !group.commands.is_empty());
      !batch.groups.is_empty()
    });
  }

  fn record_submission(&mut self, runtime: Option<&dyn AppRuntime>) {
    let publication = self.publication.is_some();
    for message in self.pending.drain(..) {
      if let DeliveryMessage::Batch(batch) = message {
        let scope = batch.work_scope.expect("owned output");
        self
          .batches
          .insert(*batch.batch_id.as_uuid().as_bytes(), (scope, publication));
        if publication && let Some(runtime) = runtime {
          runtime.presentation_pending(scope, true);
        }
      }
    }
  }

  fn accept(&mut self) {
    if let Some(output) = self.publication.take() {
      output.submitted();
    }
  }

  fn fail(&mut self, runtime: Option<&dyn AppRuntime>, message: String) {
    if let (Some(runtime), Some(scope)) = (runtime, self.scope) {
      runtime.fail_work(scope, message);
    }
    self.publication = None;
    self.pending.clear();
    self.encoded = None;
  }
}
pub(crate) fn encode(response: &DeliveryResponse) -> Result<EngineResponse, EngineError> {
  let session_id = *response.session_id.as_uuid().as_bytes();
  let mut writer = MessageWriter::default();
  let mut messages = Vec::with_capacity(response.messages.len());
  for message in &response.messages {
    let offset = match message {
      DeliveryMessage::Snapshot(snapshot) => writer.snapshot_message(snapshot),
      DeliveryMessage::Batch(batch) => {
        let mut groups = Vec::with_capacity(batch.groups.len());
        for group in &batch.groups {
          let commands = group
            .commands
            .iter()
            .map(|command| writer.command(command))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| EngineError::new(error.to_string()))?;
          groups.push(
            writer
              .parallel_group(&commands)
              .map_err(|error| EngineError::new(error.to_string()))?,
          );
        }
        writer.scoped_batch(
          *batch.batch_id.as_uuid().as_bytes(),
          *batch.session_id.as_uuid().as_bytes(),
          batch
            .caused_by_action_id
            .map(|value| *value.as_uuid().as_bytes()),
          match batch.start {
            battlement::BatchStart::Now => NativeBatchStart::Now,
            battlement::BatchStart::AfterEarlierBlockingWork => {
              NativeBatchStart::AfterEarlierBlockingWork
            }
            battlement::BatchStart::AfterEarlierAssetPreparation => {
              NativeBatchStart::AfterEarlierAssetPreparation
            }
          },
          batch.work_scope,
          batch.cancel_scope,
          batch.presentation_control,
          &groups,
        )
      }
    }
    .map_err(|error| EngineError::new(error.to_string()))?;
    messages.push(offset);
  }
  let message = writer
    .finish(session_id, &messages)
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_core(session_id, message)
}
