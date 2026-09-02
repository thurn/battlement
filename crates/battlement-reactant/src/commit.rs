use std::{cell::Cell, rc::Rc, thread};

use battlement::{
  ActionId, Batch, BatchId, Command, ParallelCommandGroup, Response, ResponseMessage, SessionId,
  Snapshot,
};

use crate::{
  asset_generator,
  runtime::{ReactantCommit, ResponseReactantExt, SessionUi},
};

#[derive(Clone)]
pub(crate) struct DeliveryReceipt {
  state: Rc<Cell<ReceiptState>>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ReceiptState {
  Pending,
  Acknowledged,
  Poisoned,
}

impl ReactantCommit {
  /// Returns whether this commit carries no native work.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self
      .groups
      .as_ref()
      .expect("Reactant commit was already consumed")
      .is_empty()
  }

  /// Consumes this commit into its ordered parallel command-body groups.
  #[must_use]
  pub fn into_groups(mut self) -> Vec<Vec<battlement::CommandBody>> {
    let groups = self.take_groups();
    self.acknowledge();
    groups
  }

  /// Consumes this commit into one Battlement batch, or no batch when empty.
  #[must_use]
  pub fn into_batch(mut self, session_id: SessionId) -> Option<Batch> {
    let groups = self.take_groups();
    let batch = (!groups.is_empty()).then(|| {
      Batch::new(
        BatchId::new_v4(),
        session_id,
        groups
          .into_iter()
          .map(ParallelCommandGroup::from_bodies)
          .collect(),
      )
    });
    self.acknowledge();
    batch
  }

  pub(crate) fn empty() -> Self {
    Self {
      groups: Some(Vec::new()),
      receipt: None,
    }
  }

  pub(crate) fn new(groups: Vec<Vec<battlement::CommandBody>>, receipt: DeliveryReceipt) -> Self {
    Self {
      groups: Some(groups),
      receipt: Some(receipt),
    }
  }

  fn acknowledge(&mut self) {
    if let Some(receipt) = self.receipt.take() {
      receipt.acknowledge();
    }
  }

  fn take_groups(&mut self) -> Vec<Vec<battlement::CommandBody>> {
    self
      .groups
      .take()
      .expect("Reactant commit was already consumed")
  }
}

impl SessionUi<'_> {
  /// Adds this session UI to a snapshot and returns its complete response.
  pub fn into_response(self, snapshot: Snapshot) -> Response {
    let (snapshot, commit) = self.into_parts(snapshot);
    Response::snapshot(snapshot).append_reactant(commit)
  }

  /// Adds this session UI to a snapshot and returns the minimal commit path.
  pub fn into_parts(self, snapshot: Snapshot) -> (Snapshot, ReactantCommit) {
    self.complete(snapshot, false)
  }

  pub(crate) fn into_app_response(self, snapshot: Snapshot) -> Response {
    let (snapshot, commit) = self.complete(snapshot, true);
    Response::snapshot(snapshot).append_reactant(commit)
  }

  fn complete(
    mut self,
    mut snapshot: Snapshot,
    discover_assets: bool,
  ) -> (Snapshot, ReactantCommit) {
    asset_generator::merge_into_snapshot(&mut snapshot);
    let external = self
      .external
      .take()
      .expect("Reactant session external plan was already consumed")
      .prepare(&mut snapshot, &self.documents, discover_assets);
    let commit = self.runtime.commit_session(
      &mut self.committed,
      external,
      self
        .resource_completions
        .take()
        .expect("Reactant session resource transaction was already consumed"),
      self
        .attachments
        .take()
        .expect("Reactant session attachments were already consumed"),
      self
        .geometry
        .take()
        .expect("Reactant session geometry plan was already consumed"),
      self.frozen_actions,
    );
    self.consumed = true;
    (snapshot, commit)
  }
}

impl Drop for SessionUi<'_> {
  fn drop(&mut self) {
    if self.consumed {
      return;
    }
    self
      .runtime
      .discard_session(self.resource_completions.take());
    if !thread::panicking() {
      panic!("a Reactant session must be converted before it is dropped");
    }
  }
}

impl Drop for ReactantCommit {
  fn drop(&mut self) {
    let Some(receipt) = self.receipt.take() else {
      return;
    };
    if receipt.state() != ReceiptState::Pending {
      return;
    }
    receipt.poison();
    if !thread::panicking() {
      panic!("a nonempty Reactant commit was dropped without delivery");
    }
  }
}

impl<C> ResponseReactantExt for Response<C>
where
  C: From<Command>,
{
  fn append_reactant(self, commit: ReactantCommit) -> Self {
    self::append_commit(self, None, commit)
  }

  fn append_reactant_for_action(self, action_id: ActionId, commit: ReactantCommit) -> Self {
    self::append_commit(self, Some(action_id), commit)
  }
}

impl DeliveryReceipt {
  pub(crate) fn new() -> Self {
    Self {
      state: Rc::new(Cell::new(ReceiptState::Pending)),
    }
  }

  pub(crate) fn acknowledge(&self) {
    assert!(
      self.state() == ReceiptState::Pending,
      "Reactant commit delivery receipt is no longer valid"
    );
    self.state.set(ReceiptState::Acknowledged);
  }

  pub(crate) fn poison(&self) {
    self.state.set(ReceiptState::Poisoned);
  }

  pub(crate) fn acknowledged(&self) -> bool {
    self.state() == ReceiptState::Acknowledged
  }

  pub(crate) fn pending(&self) -> bool {
    self.state() == ReceiptState::Pending
  }

  fn state(&self) -> ReceiptState {
    self.state.get()
  }
}

fn append_commit<C>(
  mut response: Response<C>,
  action_id: Option<ActionId>,
  mut commit: ReactantCommit,
) -> Response<C>
where
  C: From<Command>,
{
  let groups = commit.take_groups();
  if groups.is_empty() {
    commit.acknowledge();
    return response;
  }
  let groups = groups
    .into_iter()
    .map(|bodies| {
      ParallelCommandGroup::new(
        bodies
          .into_iter()
          .map(Command::new_v4)
          .map(C::from)
          .collect(),
      )
    })
    .collect();
  let mut batch = Batch::new(BatchId::new_v4(), response.session_id, groups);
  if let Some(action_id) = action_id {
    batch.caused_by_action_id = Some(action_id);
  }
  response.messages.push(ResponseMessage::Batch(batch));
  commit.acknowledge();
  response
}
