//! Opt-in, payload-free records of scoped output admission.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use battlement::{Batch, BatchId, CommandBody, SessionId};

/// Maximum records captured and traced by one recorder, across all attached applications.
pub const RECORD_LIMIT: usize = 128;

/// Why delivery retained, submitted, or discarded commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryDisposition {
  /// Queued output awaits allocation capacity or a response after independent controls.
  DeferredAdmission,
  /// The owning scope ended before submission.
  CanceledScope,
  /// A newer semantic snapshot replaced this queued observation.
  SupersededObservation,
  /// Output failed encoding or exceeded an individual response limit.
  RejectedOutput,
  /// The host admitted the response, but may not have executed it yet.
  Submitted,
}

/// One batch's affected commands; no player text, command payloads, or saved contents.
#[derive(Clone, Debug)]
pub struct DeliveryRecord {
  /// Protocol session containing this batch.
  pub session_id: SessionId,
  /// Identifies the queued batch across deferral and its terminal disposition.
  pub batch_id: BatchId,
  /// Current application work owner.
  pub current_scope: Option<u64>,
  /// Owner attributed to this output.
  pub batch_scope: Option<u64>,
  /// Delivery decision for the counted commands.
  pub disposition: DeliveryDisposition,
  /// Counts by static command variant name.
  pub commands: BTreeMap<&'static str, usize>,
  /// Inclusive semantic commit sequence bounds among affected snapshot commands.
  pub observation_commits: Option<(u64, u64)>,
}

/// Shared recorder with a fixed lifetime budget; attach explicitly to an application.
#[derive(Clone, Default)]
pub struct DeliveryDiagnostics(Rc<RefCell<Records>>);

#[derive(Default)]
struct Records {
  records: Vec<DeliveryRecord>,
  suppressed: u64,
}

impl DeliveryDiagnostics {
  /// Returns the retained records, bounded by [`RECORD_LIMIT`].
  pub fn records(&self) -> Vec<DeliveryRecord> {
    self.0.borrow().records.clone()
  }

  /// Number of records omitted after the lifetime budget was exhausted.
  pub fn suppressed(&self) -> u64 {
    self.0.borrow().suppressed
  }

  pub(crate) fn record(
    &self,
    batch: &Batch,
    current_scope: Option<u64>,
    disposition: DeliveryDisposition,
    include: impl Fn(&CommandBody) -> bool,
  ) {
    if !batch
      .groups
      .iter()
      .flat_map(|group| &group.commands)
      .any(|command| include(&command.body))
    {
      return;
    }
    let mut records = self.0.borrow_mut();
    if records.records.len() == RECORD_LIMIT {
      if records.suppressed == 0 {
        tracing::event!(
          name: "reactant.delivery.limit", tracing::Level::INFO,
          record_limit = RECORD_LIMIT,
          "Delivery diagnostic record limit reached."
        );
      }
      records.suppressed = records.suppressed.saturating_add(1);
      return;
    }
    let mut record = DeliveryRecord {
      session_id: batch.session_id,
      batch_id: batch.batch_id,
      current_scope,
      batch_scope: batch.work_scope,
      disposition,
      commands: BTreeMap::new(),
      observation_commits: None,
    };
    for command in batch.groups.iter().flat_map(|group| &group.commands) {
      if !include(&command.body) {
        continue;
      }
      *record.commands.entry(command.body.kind_name()).or_default() += 1;
      if let CommandBody::AccessibilityUpdate(update) = &command.body
        && let Some(snapshot) = &update.snapshot
      {
        let sequence = snapshot.commit_sequence;
        record.observation_commits = Some(
          record
            .observation_commits
            .map_or((sequence, sequence), |(first, last)| {
              (first.min(sequence), last.max(sequence))
            }),
        );
      }
    }
    if record.commands.is_empty() {
      return;
    }
    tracing::event!(
      name: "reactant.delivery", tracing::Level::INFO,
      session_id = %record.session_id, batch_id = %record.batch_id,
      current_scope = ?record.current_scope, batch_scope = ?record.batch_scope,
      disposition = ?record.disposition, commands = ?record.commands,
      observation_commits = ?record.observation_commits,
      "Scoped output delivery."
    );
    records.records.push(record);
  }
}
