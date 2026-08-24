use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    ptr,
    sync::{
        Mutex, Once, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{Map, Value};
use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context, prelude::*};
use uuid::Uuid;

use crate::{BattlementBuffer, INVALID_ARGUMENT, OK};

const ACTIVE_FILE_NAME: &str = "battlement.jsonl";
const MAXIMUM_FILE_BYTES: u64 = 8 * 1024 * 1024;
const RETAINED_FILE_COUNT: usize = 4;
const MAXIMUM_READ_BYTES: u64 = 4 * 1024 * 1024;

static LOGGER: OnceLock<Mutex<LoggerState>> = OnceLock::new();
static PANIC_LOGGER: OnceLock<Mutex<Option<PanicLogger>>> = OnceLock::new();
static INSTALL_TRACING: Once = Once::new();
static INITIALIZATION_ATTEMPTED: AtomicBool = AtomicBool::new(false);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct LoggerState {
    path: PathBuf,
    session_id: String,
    writer: Option<File>,
}

struct PanicLogger {
    session_id: String,
    writer: File,
}

#[derive(Serialize)]
struct LogRecord<'a> {
    schema: u32,
    session_id: &'a str,
    sequence: u64,
    timestamp_unix_us: u64,
    source: &'a str,
    severity: &'a str,
    event_name: &'a str,
    message: &'a str,
    fields: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    exception: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stack_trace: Option<&'a str>,
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    fields: Map<String, Value>,
}

struct FileLayer;

impl Visit for EventVisitor {
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_owned(), Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_owned(), Value::from(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_owned(), Value::from(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_owned(), Value::from(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        } else {
            self.fields
                .insert(field.name().to_owned(), Value::String(value.to_owned()));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields
                .insert(field.name().to_owned(), Value::String(value));
        }
    }
}

impl<S> Layer<S> for FileLayer
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, context: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        if let Some(scope) = context.event_scope(event) {
            let spans = scope
                .from_root()
                .map(|span| Value::String(span.metadata().name().to_owned()))
                .collect();
            visitor
                .fields
                .insert("spans".to_owned(), Value::Array(spans));
        }
        let metadata = event.metadata();
        let message = visitor
            .message
            .unwrap_or_else(|| metadata.name().to_owned());
        let severity = match *metadata.level() {
            tracing::Level::ERROR => "error",
            tracing::Level::WARN => "warning",
            tracing::Level::INFO => "information",
            tracing::Level::DEBUG => "debug",
            tracing::Level::TRACE => "trace",
        };
        if let Err(error) = append_record(
            "rust",
            severity,
            metadata.name(),
            &message,
            Value::Object(visitor.fields),
            None,
            None,
        ) {
            eprintln!("Battlement file logging failed: {error}");
        }
    }
}

/// Initializes the process-wide append-only log at `directory`.
pub fn log_initialize(directory: &Path) -> io::Result<()> {
    INITIALIZATION_ATTEMPTED.store(true, Ordering::Release);
    let path = directory.join(ACTIVE_FILE_NAME);
    if let Some(logger) = LOGGER.get() {
        let mut state = logger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.path != path {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Battlement logging was initialized at {} and cannot move to {}",
                    state.path.display(),
                    path.display()
                ),
            ));
        }
        if state.writer.is_none() {
            let writer = open_append(&path)?;
            set_panic_logger(&state.session_id, writer.try_clone()?);
            state.writer = Some(writer);
        }
        return Ok(());
    }

    fs::create_dir_all(directory)?;
    rotate_if_needed(&path)?;
    let writer = open_append(&path)?;
    let session_id = Uuid::new_v4().to_string();
    set_panic_logger(&session_id, writer.try_clone()?);
    let state = LoggerState {
        path,
        session_id,
        writer: Some(writer),
    };
    LOGGER
        .set(Mutex::new(state))
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "logger already initialized"))?;
    INSTALL_TRACING.call_once(|| {
        if let Err(error) = tracing_subscriber::registry().with(FileLayer).try_init() {
            eprintln!("Battlement could not install its tracing subscriber: {error}");
        }
    });
    Ok(())
}

/// Appends one Unity-authored record to the native stream.
pub fn log_write(record: &[u8]) -> io::Result<()> {
    let value: Value = serde_json::from_slice(record).map_err(io::Error::other)?;
    let object = value
        .as_object()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "record must be an object"))?;
    append_record(
        "unity",
        string_field(object, "severity")?,
        string_field(object, "event_name")?,
        string_field(object, "message")?,
        object.get("fields").cloned().unwrap_or_else(empty_fields),
        optional_string(object, "exception"),
        optional_string(object, "stack_trace"),
    )
}

/// Reads complete JSON Lines records starting at `offset`.
pub fn log_read(offset: u64, maximum_bytes: u64) -> io::Result<(Vec<u8>, u64)> {
    let logger = LOGGER
        .get()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "logger is not initialized"))?;
    let path = logger
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .path
        .clone();
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if offset > length {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "log offset is past the end of the file",
        ));
    }
    let capacity = maximum_bytes.min(MAXIMUM_READ_BYTES);
    let capacity = usize::try_from(capacity)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "read size is too large"))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(capacity as u64).read_to_end(&mut bytes)?;
    let Some(last_newline) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        return Ok((Vec::new(), offset));
    };
    bytes.truncate(last_newline + 1);
    Ok((bytes, offset + last_newline as u64 + 1))
}

/// Flushes the active file and requests durable storage.
pub fn log_sync() -> io::Result<()> {
    with_writer(|writer| writer.sync_data())
}

/// Flushes and closes the ordinary append handle.
pub fn log_close() -> io::Result<()> {
    let logger = LOGGER
        .get()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "logger is not initialized"))?;
    let mut state = logger
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(writer) = state.writer.take() {
        writer.sync_data()?;
    }
    if let Some(panic_logger) = PANIC_LOGGER.get() {
        *panic_logger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
    Ok(())
}

/// Initializes logging across the raw C ABI.
///
/// # Safety
///
/// `path` must be readable for `length` bytes and `out_error` must be writable.
pub unsafe fn ffi_log_initialize(
    path: *const u8,
    length: u64,
    out_error: *mut BattlementBuffer,
) -> i32 {
    // SAFETY: The caller supplies the readable input and writable output described above.
    unsafe {
        ffi_input_call(path, length, out_error, |bytes| {
            let path = std::str::from_utf8(bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
            log_initialize(Path::new(path))
        })
    }
}

/// Appends a Unity record across the raw C ABI.
///
/// # Safety
///
/// `record` must be readable for `length` bytes and `out_error` must be writable.
pub unsafe fn ffi_log_write(
    record: *const u8,
    length: u64,
    out_error: *mut BattlementBuffer,
) -> i32 {
    // SAFETY: The caller supplies the readable input and writable output described above.
    unsafe { ffi_input_call(record, length, out_error, log_write) }
}

/// Reads complete records across the raw C ABI.
///
/// # Safety
///
/// Both output pointers must be writable and the returned buffer must be freed once.
pub unsafe fn ffi_log_read(
    offset: u64,
    maximum_bytes: u64,
    out_records: *mut BattlementBuffer,
    out_next_offset: *mut u64,
) -> i32 {
    if out_records.is_null() || out_next_offset.is_null() {
        return INVALID_ARGUMENT;
    }
    // SAFETY: Both pointers were checked and are writable by contract.
    unsafe {
        out_records.write(BattlementBuffer::EMPTY);
        out_next_offset.write(offset);
    }
    match log_read(offset, maximum_bytes) {
        Ok((records, next_offset)) => {
            // SAFETY: Both pointers were checked and are writable by contract.
            unsafe {
                out_records.write(BattlementBuffer::from_bytes(records));
                out_next_offset.write(next_offset);
            }
            OK
        }
        Err(error) => {
            // SAFETY: `out_records` was checked and initialized above.
            unsafe { out_records.write(BattlementBuffer::from_bytes(error.to_string().into())) };
            INVALID_ARGUMENT
        }
    }
}

/// Synchronizes logging across the raw C ABI.
///
/// # Safety
///
/// `out_error` must be writable.
pub unsafe fn ffi_log_sync(out_error: *mut BattlementBuffer) -> i32 {
    // SAFETY: The caller supplies the writable output described above.
    unsafe { ffi_action(out_error, log_sync) }
}

/// Closes logging across the raw C ABI.
///
/// # Safety
///
/// `out_error` must be writable.
pub unsafe fn ffi_log_close(out_error: *mut BattlementBuffer) -> i32 {
    // SAFETY: The caller supplies the writable output described above.
    unsafe { ffi_action(out_error, log_close) }
}

pub(crate) fn append_panic(message: &str, location: Option<&str>, thread: &str, backtrace: &str) {
    let Some(logger) = PANIC_LOGGER.get() else {
        return;
    };
    let Ok(mut guard) = logger.try_lock() else {
        return;
    };
    let Some(logger) = guard.as_mut() else {
        return;
    };
    let mut fields = BTreeMap::new();
    fields.insert("thread", thread);
    if let Some(location) = location {
        fields.insert("location", location);
    }
    let record = LogRecord {
        schema: 1,
        session_id: &logger.session_id,
        sequence: next_sequence(),
        timestamp_unix_us: timestamp_unix_us(),
        source: "rust",
        severity: "error",
        event_name: "battlement.rust.panic",
        message,
        fields: serde_json::to_value(fields).unwrap_or_else(|_| empty_fields()),
        exception: Some("panic"),
        stack_trace: Some(backtrace),
    };
    let Ok(mut line) = serde_json::to_vec(&record) else {
        return;
    };
    line.push(b'\n');
    let _ = logger.writer.write_all(&line);
    let _ = logger.writer.sync_data();
}

pub(crate) fn initialization_was_attempted() -> bool {
    INITIALIZATION_ATTEMPTED.load(Ordering::Acquire)
}

fn append_record(
    source: &str,
    severity: &str,
    event_name: &str,
    message: &str,
    fields: Value,
    exception: Option<&str>,
    stack_trace: Option<&str>,
) -> io::Result<()> {
    let logger = LOGGER
        .get()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "logger is not initialized"))?;
    let mut state = logger
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let record = LogRecord {
        schema: 1,
        session_id: &state.session_id,
        sequence: next_sequence(),
        timestamp_unix_us: timestamp_unix_us(),
        source,
        severity,
        event_name,
        message,
        fields,
        exception,
        stack_trace,
    };
    let mut line = serde_json::to_vec(&record).map_err(io::Error::other)?;
    line.push(b'\n');
    state
        .writer
        .as_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "logger is closed"))?
        .write_all(&line)
}

fn with_writer(action: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let logger = LOGGER
        .get()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "logger is not initialized"))?;
    let mut state = logger
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    action(
        state
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "logger is closed"))?,
    )
}

unsafe fn ffi_input_call(
    data: *const u8,
    length: u64,
    out_error: *mut BattlementBuffer,
    action: impl FnOnce(&[u8]) -> io::Result<()>,
) -> i32 {
    if out_error.is_null() {
        return INVALID_ARGUMENT;
    }
    // SAFETY: The caller promises the output is writable.
    unsafe { out_error.write(BattlementBuffer::EMPTY) };
    let Ok(length) = usize::try_from(length) else {
        return INVALID_ARGUMENT;
    };
    if data.is_null() && length != 0 {
        return INVALID_ARGUMENT;
    }
    let bytes = if length == 0 {
        &[]
    } else {
        // SAFETY: The caller promises this region is readable for the call.
        unsafe { std::slice::from_raw_parts(data, length) }
    };
    match action(bytes) {
        Ok(()) => OK,
        Err(error) => {
            // SAFETY: The output was checked and initialized above.
            unsafe { out_error.write(BattlementBuffer::from_bytes(error.to_string().into())) };
            INVALID_ARGUMENT
        }
    }
}

unsafe fn ffi_action(
    out_error: *mut BattlementBuffer,
    action: impl FnOnce() -> io::Result<()>,
) -> i32 {
    // SAFETY: A zero-length input avoids dereferencing the null pointer.
    unsafe { ffi_input_call(ptr::null(), 0, out_error, |_| action()) }
}

fn string_field<'a>(object: &'a Map<String, Value>, name: &str) -> io::Result<&'a str> {
    object.get(name).and_then(Value::as_str).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{name} must be a string"),
        )
    })
}

fn optional_string<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    object.get(name).and_then(Value::as_str)
}

fn empty_fields() -> Value {
    Value::Object(Map::new())
}

fn next_sequence() -> u64 {
    SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1
}

fn timestamp_unix_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(path)
}

fn set_panic_logger(session_id: &str, writer: File) {
    let logger = PANIC_LOGGER.get_or_init(|| Mutex::new(None));
    *logger
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(PanicLogger {
        session_id: session_id.to_owned(),
        writer,
    });
}

fn rotate_if_needed(path: &Path) -> io::Result<()> {
    if path.metadata().map(|metadata| metadata.len()).unwrap_or(0) < MAXIMUM_FILE_BYTES {
        return Ok(());
    }
    for index in (1..RETAINED_FILE_COUNT).rev() {
        let source = retained_path(path, index);
        if source.exists() {
            fs::rename(source, retained_path(path, index + 1))?;
        }
    }
    fs::rename(path, retained_path(path, 1))
}

fn retained_path(path: &Path, index: usize) -> PathBuf {
    let mut name = OsString::from(ACTIVE_FILE_NAME);
    name.push(format!(".{index}"));
    path.with_file_name(name)
}
