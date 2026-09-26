use std::sync::Mutex;

use crate::wire::result::ErrorOccurrence;

/// Shared allocation order for scenario, supervision and cleanup failures in one run.
#[derive(Default)]
pub struct RunErrors {
  entries: Mutex<Vec<ErrorOccurrence>>,
}

impl RunErrors {
  pub fn snapshot(&self) -> Vec<ErrorOccurrence> {
    self.entries.lock().unwrap().clone()
  }

  pub(crate) fn record(&self, error: ErrorOccurrence) -> String {
    append(&mut self.entries.lock().unwrap(), error)
  }

  pub(crate) fn observe(&self, error: ErrorOccurrence) -> String {
    let mut entries = self.entries.lock().unwrap();
    if let Some(existing) = entries.iter().find(|stored| {
      (
        &stored.job_id,
        &stored.scenario_id,
        stored.step_index,
        stored.log_sequence,
      ) == (
        &error.job_id,
        &error.scenario_id,
        error.step_index,
        error.log_sequence,
      )
    }) {
      return existing.id.clone();
    }
    append(&mut entries, error)
  }

  pub(crate) fn update(&self, error: ErrorOccurrence) {
    let mut entries = self.entries.lock().unwrap();
    let stored = entries
      .iter_mut()
      .find(|stored| stored.id == error.id)
      .expect("error was not allocated");
    *stored = error;
  }
}

fn append(entries: &mut Vec<ErrorOccurrence>, mut error: ErrorOccurrence) -> String {
  error.id = format!("E{:04}", entries.len() + 1);
  let id = error.id.clone();
  entries.push(error);
  id
}
