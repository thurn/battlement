//! Runtime helpers for Motion transport and presence boundaries.

use battlement::{CommandBody, MotionEventBatch, MotionSequence};

use crate::root_view::RootRegistration;

pub(crate) fn validate_batch(last: Option<MotionSequence>, batch: &MotionEventBatch) {
  assert!(
    batch.first_sequence <= batch.last_sequence,
    "Motion event batch has a reversed sequence range"
  );
  if let Some(first) = batch.events.first() {
    assert_eq!(
      first.sequence, batch.first_sequence,
      "Motion event batch first sequence does not match its events"
    );
    assert_eq!(
      batch.events.last().map(|event| event.sequence),
      Some(batch.last_sequence),
      "Motion event batch last sequence does not match its events"
    );
    assert!(
      batch
        .events
        .windows(2)
        .all(|pair| pair[1].sequence.0 == pair[0].sequence.0 + 1),
      "Motion lifecycle sequences must be contiguous"
    );
  }
  if let Some(previous) = last
    && !batch.events.is_empty()
  {
    assert!(
      batch.first_sequence.0 > previous.0,
      "Motion event batch sequence is stale"
    );
  }
}

pub(crate) fn has_ready_presence<G>(roots: &[RootRegistration<G>]) -> bool {
  roots.iter().any(|root| root.committed.has_ready_presence())
}

pub(crate) fn invoke_ready_presence<G: 'static>(
  roots: &mut [RootRegistration<G>],
  game: &mut G,
) -> bool {
  roots.iter_mut().fold(false, |invoked, root| {
    root.committed.invoke_ready_presence(game) || invoked
  })
}

pub(crate) fn merge_groups(
  mut merged: Vec<Vec<CommandBody>>,
  groups: Vec<Vec<CommandBody>>,
) -> Vec<Vec<CommandBody>> {
  for (index, group) in groups.into_iter().enumerate() {
    if index == merged.len() {
      merged.push(group);
    } else {
      merged[index].extend(group);
    }
  }
  merged
}
