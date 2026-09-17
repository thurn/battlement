use std::mem;

use battlement::asset_dependencies::AssetDependencies;
use battlement::{
  ActionId, Batch, BatchId, BatchStart, CommandBody, ParallelCommandGroup, ReplaceAssetSetPayload,
  SessionId,
};

use crate::{app_context::QueuedCommand, asset_generator, runtime::ReactantCommit};

#[derive(Default)]
pub(crate) struct Delivery {
  assets: AssetDependencies,
  session: Option<SessionId>,
}

pub(crate) struct DeliveryResponse {
  pub(crate) session_id: SessionId,
  pub(crate) messages: Vec<DeliveryMessage>,
}

impl DeliveryResponse {
  pub(crate) fn empty(session_id: SessionId) -> Self {
    Self {
      session_id,
      messages: Vec::new(),
    }
  }

  pub(crate) fn snapshot(snapshot: battlement::Snapshot) -> Self {
    Self {
      session_id: snapshot.session_id,
      messages: vec![DeliveryMessage::Snapshot(snapshot)],
    }
  }
}

pub(crate) enum DeliveryMessage {
  Snapshot(battlement::Snapshot),
  Batch(Batch),
}

impl Delivery {
  pub(crate) fn prepare(&mut self, mut response: DeliveryResponse) -> DeliveryResponse {
    if self.session != Some(response.session_id) {
      self.assets = AssetDependencies::default();
      self.session = Some(response.session_id);
    }
    let mut messages = Vec::new();
    for mut message in response.messages {
      match &mut message {
        DeliveryMessage::Snapshot(snapshot) => {
          self.assets.snapshot(snapshot);
          snapshot.prepared_assets = self.assets.assets();
        }
        DeliveryMessage::Batch(batch) => {
          let before = self.assets.assets().len();
          for group in &batch.groups {
            for command in &group.commands {
              assert!(
                !matches!(command.body, CommandBody::AssetsReplaceSet(_)),
                "App owns asset preparation; submit commands that reference assets instead"
              );
              self.assets.command(&command.body);
            }
          }
          if self.assets.assets().len() != before {
            let mut preparation = Batch::parallel(
              response.session_id,
              [CommandBody::AssetsReplaceSet(ReplaceAssetSetPayload {
                assets: self.assets.assets(),
              })],
            );
            preparation.caused_by_action_id = batch.caused_by_action_id;
            preparation.start = BatchStart::AfterEarlierAssetPreparation;
            messages.push(DeliveryMessage::Batch(preparation));
          }
        }
      }
      messages.push(message);
    }
    response.messages = messages;
    for asset in self.assets.assets() {
      asset_generator::validate_discovered_asset(&asset);
    }
    response
  }
}

pub(crate) fn append(
  response: &mut DeliveryResponse,
  action: Option<ActionId>,
  commit: ReactantCommit,
) {
  for mut batch in commit.into_app_batches(response.session_id) {
    batch.caused_by_action_id = action;
    response.messages.push(DeliveryMessage::Batch(batch));
  }
}

pub(crate) fn take_imperative(response: &mut DeliveryResponse) -> Vec<DeliveryMessage> {
  let mut messages = Vec::new();
  for message in mem::take(&mut response.messages) {
    let DeliveryMessage::Batch(mut batch) = message else {
      continue;
    };
    for group in &mut batch.groups {
      group
        .commands
        .retain_mut(|command| match &mut command.body {
          CommandBody::VisualElementPerformAction(_) => true,
          CommandBody::AccessibilityUpdate(update) => {
            update.snapshot = None;
            !update.announcements.is_empty()
          }
          _ => false,
        });
    }
    batch.groups.retain(|group| !group.commands.is_empty());
    if !batch.groups.is_empty() {
      messages.push(DeliveryMessage::Batch(batch));
    }
  }
  messages
}

pub(crate) fn commands(
  response: &mut DeliveryResponse,
  commands: Vec<QueuedCommand>,
  independent: bool,
) {
  let mut pending = commands.into_iter().peekable();
  while let Some(first) = pending.next() {
    let action = first.action;
    let scope = first.scope;
    let cancel = matches!(first.command.body, CommandBody::OperationCancel(_));
    let mut group = vec![first.command];
    while pending.peek().is_some_and(|next| {
      (
        next.action,
        next.scope,
        matches!(next.command.body, CommandBody::OperationCancel(_)),
      ) == (action, scope, cancel)
    }) {
      group.push(pending.next().expect("queued command").command);
    }
    let mut batch = Batch::new(
      BatchId::new_v4(),
      response.session_id,
      vec![ParallelCommandGroup::new(group)],
    );
    batch.caused_by_action_id = action;
    batch.work_scope = if cancel { None } else { scope };
    if !cancel {
      batch.start = if independent && scope.is_none() {
        BatchStart::AfterEarlierAssetPreparation
      } else {
        BatchStart::AfterEarlierBlockingWork
      };
    }
    response.messages.push(DeliveryMessage::Batch(batch));
  }
}
