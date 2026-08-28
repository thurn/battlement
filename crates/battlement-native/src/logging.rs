use std::{
  collections::VecDeque,
  io,
  sync::{Mutex, OnceLock},
  time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{Map, Value};
use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context, prelude::*};

use crate::{BattlementBuffer, INVALID_ARGUMENT, OK};

const MAXIMUM_QUEUED_RECORDS: usize = 2_048;
const MAXIMUM_QUEUED_BYTES: usize = 4 * 1024 * 1024;
const MAXIMUM_RECORD_BYTES: usize = 64 * 1024;

static INITIALIZATION: OnceLock<Result<(), String>> = OnceLock::new();
static QUEUE: OnceLock<Mutex<QueueState>> = OnceLock::new();

pub(crate) fn log_initialize() -> io::Result<()> {
  INITIALIZATION
    .get_or_init(|| {
      tracing_subscriber::registry()
        .with(QueueLayer)
        .try_init()
        .map_err(|error| error.to_string())
    })
    .as_ref()
    .map_err(|message| io::Error::other(message.clone()))
    .copied()
}

/// Drains all currently queued tracing records as UTF-8 JSON Lines.
pub fn log_drain() -> Vec<u8> {
  let mut queue = queue()
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  let mut records = Vec::new();
  if queue.dropped != 0 {
    append_line(
      &mut records,
      &LogRecord {
        timestamp_unix_us: timestamp_unix_us(),
        severity: "warning",
        event_name: "battlement.logging.records_dropped",
        message: "Rust tracing records were dropped before Unity drained them.",
        fields: serde_json::json!({ "dropped_records": queue.dropped }),
      },
    );
    queue.dropped = 0;
  }
  for record in queue.records.drain(..) {
    records.extend_from_slice(&record);
    records.push(b'\n');
  }
  queue.bytes = 0;
  records
}

/// Drains queued tracing records across the raw C ABI.
///
/// # Safety
///
/// `out_records` must be writable and the returned buffer must be freed once.
pub unsafe fn ffi_logging_drain(out_records: *mut BattlementBuffer) -> i32 {
  if out_records.is_null() {
    return INVALID_ARGUMENT;
  }
  // SAFETY: The caller promises the output is writable.
  unsafe { out_records.write(BattlementBuffer::from_bytes(log_drain())) };
  OK
}

#[derive(Serialize)]
struct LogRecord<'a> {
  timestamp_unix_us: u64,
  severity: &'a str,
  event_name: &'a str,
  message: &'a str,
  fields: Value,
}

#[derive(Default)]
struct EventVisitor {
  message: Option<String>,
  fields: Map<String, Value>,
}

#[derive(Default)]
struct QueueState {
  records: VecDeque<Vec<u8>>,
  bytes: usize,
  dropped: u64,
}

impl QueueState {
  fn push(&mut self, record: Vec<u8>) {
    if record.len() > MAXIMUM_RECORD_BYTES {
      self.dropped += 1;
      return;
    }

    while self.records.len() == MAXIMUM_QUEUED_RECORDS
      || self.bytes + record.len() > MAXIMUM_QUEUED_BYTES
    {
      let Some(discarded) = self.records.pop_front() else {
        break;
      };
      self.bytes -= discarded.len();
      self.dropped += 1;
    }
    self.bytes += record.len();
    self.records.push_back(record);
  }
}

struct QueueLayer;

impl Visit for EventVisitor {
  fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
    self
      .fields
      .insert(field.name().to_owned(), Value::Bool(value));
  }

  fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
    if field.name() == "message" {
      self.message = Some(value.to_owned());
    } else {
      self
        .fields
        .insert(field.name().to_owned(), Value::String(value.to_owned()));
    }
  }

  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    let value = format!("{value:?}");
    if field.name() == "message" {
      self.message = Some(value);
    } else {
      self
        .fields
        .insert(field.name().to_owned(), Value::String(value));
    }
  }
}

impl<S> Layer<S> for QueueLayer
where
  S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
  fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
    let mut visitor = EventVisitor::default();
    event.record(&mut visitor);
    if let Some(scope) = context.event_scope(event) {
      visitor.fields.insert(
        "spans".to_owned(),
        Value::Array(
          scope
            .from_root()
            .map(|span| Value::String(span.metadata().name().to_owned()))
            .collect(),
        ),
      );
    }
    let metadata = event.metadata();
    let message = visitor
      .message
      .unwrap_or_else(|| metadata.name().to_owned());
    enqueue(&LogRecord {
      timestamp_unix_us: timestamp_unix_us(),
      severity: severity_name(*metadata.level()),
      event_name: metadata.name(),
      message: &message,
      fields: Value::Object(visitor.fields),
    });
  }
}

fn append_line(output: &mut Vec<u8>, record: &LogRecord<'_>) {
  if let Ok(line) = serde_json::to_vec(record) {
    output.extend_from_slice(&line);
    output.push(b'\n');
  }
}

fn enqueue(record: &LogRecord<'_>) {
  let Ok(record) = serde_json::to_vec(record) else {
    return;
  };
  let mut queue = queue()
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  queue.push(record);
}

fn queue() -> &'static Mutex<QueueState> {
  QUEUE.get_or_init(|| Mutex::new(QueueState::default()))
}

fn severity_name(level: tracing::Level) -> &'static str {
  match level {
    tracing::Level::ERROR => "error",
    tracing::Level::WARN => "warning",
    tracing::Level::INFO => "information",
    tracing::Level::DEBUG => "debug",
    tracing::Level::TRACE => "trace",
  }
}

fn timestamp_unix_us() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_micros()
    .try_into()
    .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
  use super::{MAXIMUM_QUEUED_BYTES, MAXIMUM_RECORD_BYTES, QueueState};

  #[test]
  fn queue_bounds_records_by_size_and_reports_drops() {
    let mut queue = QueueState::default();
    for _ in 0..=(MAXIMUM_QUEUED_BYTES / MAXIMUM_RECORD_BYTES) {
      queue.push(vec![b'x'; MAXIMUM_RECORD_BYTES]);
    }

    assert!(queue.bytes <= MAXIMUM_QUEUED_BYTES);
    assert_eq!(
      queue.bytes,
      queue.records.iter().map(Vec::len).sum::<usize>()
    );
    assert_eq!(queue.dropped, 1);

    queue.push(vec![b'x'; MAXIMUM_RECORD_BYTES + 1]);
    assert_eq!(queue.dropped, 2);
    assert!(queue.bytes <= MAXIMUM_QUEUED_BYTES);
  }
}
