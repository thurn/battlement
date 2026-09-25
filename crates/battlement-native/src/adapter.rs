use std::{
  ffi::c_void,
  panic::{AssertUnwindSafe, catch_unwind},
  ptr,
  sync::atomic::{AtomicBool, AtomicUsize, Ordering},
  time::Duration,
};

use battlement::UiEventDisposition;
use battlement_flatbuffers::{ConnectView, UiEventActionView, recycle_message_storage};

use crate::{
  Engine, EngineError, EngineResponse, FlatBufferSubmitError, panic_capture, response_budget,
};

/// Operation completed successfully and returned a FlatBuffer response.
pub const OK: i32 = 0;
/// Operation completed successfully without a response.
pub const NO_MESSAGE: i32 = 1;
/// A pointer, length, or FlatBuffer input was invalid.
pub const INVALID_ARGUMENT: i32 = 2;
/// Engine construction or execution failed.
pub const ENGINE_ERROR: i32 = 3;
/// An exported entry point caught a Rust panic.
pub const PANIC: i32 = 4;
/// Maximum UTF-8 bytes returned for a native diagnostic.
pub const MAXIMUM_DIAGNOSTIC_BYTES: usize = 16 * 1024 * 1024;

const DIAGNOSTIC_TRUNCATED_SUFFIX: &str = "\n[diagnostic truncated]";

static HAS_LIVE_ENGINE: AtomicBool = AtomicBool::new(false);
static OUTSTANDING_BUFFERS: AtomicUsize = AtomicUsize::new(0);

/// One owned byte buffer crossing the C ABI.
#[repr(C)]
#[derive(Debug)]
pub struct BattlementBuffer {
  /// Pointer to the first byte, or null when `length` is zero.
  pub data: *mut u8,
  /// Number of initialized bytes available at `data`.
  pub length: u64,
  /// Bytes retained by the original allocation.
  pub allocation_bytes: u64,
  /// Base pointer of the original allocation. This may precede `data` for a
  /// backwards-built FlatBuffer.
  #[doc(hidden)]
  pub allocation_data: *mut u8,
  /// Initialized length used to reconstruct the original allocation.
  #[doc(hidden)]
  pub allocation_length: u64,
}

impl BattlementBuffer {
  /// The only valid empty-buffer representation.
  pub const EMPTY: Self = Self {
    data: ptr::null_mut(),
    length: 0,
    allocation_bytes: 0,
    allocation_data: ptr::null_mut(),
    allocation_length: 0,
  };

  pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
    Self::from_storage(bytes, 0)
  }

  pub(crate) fn from_storage(bytes: Vec<u8>, start: usize) -> Self {
    assert!(
      start <= bytes.len(),
      "finished buffer start exceeds storage"
    );
    if bytes.is_empty() {
      return Self::EMPTY;
    }
    assert!(start < bytes.len(), "finished buffer cannot be empty");

    let mut bytes = std::mem::ManuallyDrop::new(bytes);
    let allocation_data = bytes.as_mut_ptr();
    let buffer = Self {
      // SAFETY: `start` was checked against the initialized allocation length.
      data: unsafe { allocation_data.add(start) },
      length: (bytes.len() - start) as u64,
      allocation_bytes: bytes.capacity() as u64,
      allocation_data,
      allocation_length: bytes.len() as u64,
    };
    OUTSTANDING_BUFFERS.fetch_add(1, Ordering::Relaxed);
    buffer
  }
}

struct LiveEngineReservation {
  committed: bool,
}

impl Drop for LiveEngineReservation {
  fn drop(&mut self) {
    if !self.committed {
      HAS_LIVE_ENGINE.store(false, Ordering::Release);
    }
  }
}

/// Opaque owner of one concrete engine instance.
#[repr(C)]
pub struct BattlementEngine<E> {
  pub(crate) engine: E,
  pub(crate) poisoned: bool,
}

/// Frees a nonempty adapter-owned buffer. The empty buffer is a no-op.
///
/// # Safety
///
/// A nonempty buffer must have been returned by this adapter and must be freed
/// exactly once.
pub unsafe fn buffer_free(buffer: BattlementBuffer) {
  if buffer.data.is_null() || buffer.length == 0 {
    return;
  }
  let (Ok(length), Ok(capacity)) = (
    usize::try_from(buffer.allocation_length),
    usize::try_from(buffer.allocation_bytes),
  ) else {
    return;
  };
  // SAFETY: The caller returns the exact Vec allocation and dimensions from
  // `from_storage`; `data` may point at a finished suffix and is never used to free.
  let storage = unsafe { Vec::from_raw_parts(buffer.allocation_data, length, capacity) };
  response_budget::release(buffer.allocation_data as usize);
  recycle_message_storage(storage);
  OUTSTANDING_BUFFERS.fetch_sub(1, Ordering::Relaxed);
}

/// Returns the number of nonempty adapter buffers that have not been freed.
///
/// This is intended for native-plugin allocation diagnostics. It is not part
/// of the fixed C ABI unless a game explicitly exports it for tests.
#[doc(hidden)]
pub fn outstanding_buffer_count() -> usize {
  OUTSTANDING_BUFFERS.load(Ordering::Relaxed)
}

/// Reads cumulative direct-response builder and handoff-copy diagnostics.
///
/// # Safety
///
/// Every output must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_transport_diagnostics(
  out_created: *mut u64,
  out_reused: *mut u64,
  out_growths: *mut u64,
  out_copied_bytes: *mut u64,
  out_idle_bytes: *mut u64,
  out_handoff_payload_copies: *mut u64,
) -> i32 {
  for output in [
    out_created,
    out_reused,
    out_growths,
    out_copied_bytes,
    out_idle_bytes,
    out_handoff_payload_copies,
  ] {
    if !output.is_null() {
      // SAFETY: The caller promises each non-null output is writable.
      unsafe { output.write(0) };
    }
  }
  if out_created.is_null()
    || out_reused.is_null()
    || out_growths.is_null()
    || out_copied_bytes.is_null()
    || out_idle_bytes.is_null()
    || out_handoff_payload_copies.is_null()
  {
    return INVALID_ARGUMENT;
  }
  let diagnostics = battlement_flatbuffers::message_writer_diagnostics();
  // SAFETY: All outputs were checked and initialized above.
  unsafe {
    out_created.write(diagnostics.created);
    out_reused.write(diagnostics.reused);
    out_growths.write(diagnostics.growths);
    out_copied_bytes.write(diagnostics.copied_bytes);
    out_idle_bytes.write(diagnostics.idle_bytes as u64);
    out_handoff_payload_copies.write(0);
  }
  OK
}

/// Creates a FlatBuffers engine behind the panic-safe ABI boundary.
///
/// # Safety
///
/// Both output pointers must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_create<F, E>(
  factory: F,
  out_engine: *mut *mut c_void,
  out_error: *mut BattlementBuffer,
) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  if out_engine.is_null() || out_error.is_null() {
    return INVALID_ARGUMENT;
  }
  // SAFETY: The caller promises writable outputs.
  unsafe {
    out_engine.write(ptr::null_mut());
    out_error.write(BattlementBuffer::EMPTY);
  }
  panic_capture::prepare();
  match catch_unwind(AssertUnwindSafe(|| {
    if HAS_LIVE_ENGINE
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .is_err()
    {
      // SAFETY: The output was validated and initialized above.
      unsafe { write_error(out_error, "a Battlement engine instance is already live") };
      return INVALID_ARGUMENT;
    }
    let mut reservation = LiveEngineReservation { committed: false };
    if let Err(error) = crate::logging::log_initialize() {
      // SAFETY: The output was validated and initialized above.
      unsafe {
        write_error(
          out_error,
          format!("could not initialize Rust tracing: {error}"),
        )
      };
      return ENGINE_ERROR;
    }
    match factory() {
      Ok(engine) => {
        let engine = Box::new(BattlementEngine {
          engine,
          poisoned: false,
        });
        // SAFETY: The output was validated above.
        unsafe { out_engine.write(Box::into_raw(engine).cast()) };
        reservation.committed = true;
        OK
      }
      Err(error) => {
        // SAFETY: The output was validated above.
        unsafe { write_error(out_error, error) };
        ENGINE_ERROR
      }
    }
  })) {
    Ok(status) => status,
    Err(payload) => {
      HAS_LIVE_ENGINE.store(false, Ordering::Release);
      // SAFETY: Outputs were validated above.
      unsafe {
        out_engine.write(ptr::null_mut());
        write_panic(out_error, "battlement_engine_create", payload.as_ref());
      }
      PANIC
    }
  }
}

/// Destroys a FlatBuffers engine behind the panic-safe ABI boundary.
///
/// # Safety
///
/// `engine` must be the unique pointer returned by [`ffi_create`].
#[doc(hidden)]
pub unsafe fn ffi_destroy<F, E>(_: F, engine: *mut c_void, out_error: *mut BattlementBuffer) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  if out_error.is_null() {
    return INVALID_ARGUMENT;
  }
  // SAFETY: The caller promises a writable output.
  unsafe { out_error.write(BattlementBuffer::EMPTY) };
  panic_capture::prepare();
  match catch_unwind(AssertUnwindSafe(|| {
    if engine.is_null() {
      return;
    }
    HAS_LIVE_ENGINE.store(false, Ordering::Release);
    // SAFETY: The caller transfers the exact allocation returned at creation.
    drop(unsafe { Box::from_raw(engine.cast::<BattlementEngine<E>>()) });
  })) {
    Ok(()) => OK,
    Err(payload) => {
      // SAFETY: The output was validated above.
      unsafe { write_panic(out_error, "battlement_engine_destroy", payload.as_ref()) };
      PANIC
    }
  }
}

/// Runs  connect and registers no intermediate owned response.
///
/// # Safety
///
/// Pointers must satisfy the exported connect contract.
#[doc(hidden)]
pub unsafe fn ffi_connect<F, E>(
  _: F,
  engine: *mut c_void,
  data: *const u8,
  length: u64,
  out_buffer: *mut BattlementBuffer,
) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  let engine = engine.cast::<BattlementEngine<E>>();
  // SAFETY: The factory marker identifies the concrete engine allocation.
  unsafe {
    ffi_output_call(engine, out_buffer, "battlement_connect", |engine| {
      let bytes =
        input_slice(data, length).map_err(|error| (INVALID_ARGUMENT, error.to_owned()))?;
      let view = ConnectView::read(bytes)
        .map_err(|error| (INVALID_ARGUMENT, format!("invalid connect: {error}")))?;
      engine
        .connect(view)
        .map(Some)
        .map_err(|error| (ENGINE_ERROR, error.to_string()))
    })
  }
}

/// Runs  submission.
///
/// # Safety
///
/// Pointers must satisfy the exported submit contract.
#[doc(hidden)]
pub unsafe fn ffi_submit<F, E>(
  _: F,
  engine: *mut c_void,
  data: *const u8,
  length: u64,
  out_buffer: *mut BattlementBuffer,
) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  let engine = engine.cast::<BattlementEngine<E>>();
  // SAFETY: The factory marker identifies the concrete engine allocation.
  unsafe {
    ffi_output_call(engine, out_buffer, "battlement_submit", |engine| {
      let bytes =
        input_slice(data, length).map_err(|error| (INVALID_ARGUMENT, error.to_owned()))?;
      engine.submit(bytes).map(Some).map_err(|error| match error {
        FlatBufferSubmitError::InvalidArgument(error) => (INVALID_ARGUMENT, error.to_string()),
        FlatBufferSubmitError::Engine(error) => (ENGINE_ERROR, error.to_string()),
      })
    })
  }
}

/// Runs direct synchronous native UI-event submission.
///
/// # Safety
///
/// Pointers must satisfy the exported UI-submit contract.
#[doc(hidden)]
pub unsafe fn ffi_submit_ui_event<F, E>(
  _: F,
  engine: *mut c_void,
  data: *const u8,
  length: u64,
  out_disposition: *mut u32,
  out_buffer: *mut BattlementBuffer,
) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  if out_disposition.is_null() || out_buffer.is_null() {
    return INVALID_ARGUMENT;
  }
  unsafe {
    out_disposition.write(UiEventDisposition::Continue as u32);
    out_buffer.write(BattlementBuffer::EMPTY);
  }
  let engine = engine.cast::<BattlementEngine<E>>();
  if engine.is_null() {
    unsafe { write_error(out_buffer, "engine pointer is null") };
    return INVALID_ARGUMENT;
  }
  if unsafe { (*engine).poisoned } {
    unsafe { write_error(out_buffer, "Rust engine is poisoned after an earlier panic") };
    return PANIC;
  }
  panic_capture::prepare();
  match catch_unwind(AssertUnwindSafe(|| {
    let bytes =
      unsafe { input_slice(data, length) }.map_err(|error| (INVALID_ARGUMENT, error.to_owned()))?;
    let action =
      UiEventActionView::read(bytes).map_err(|error| (INVALID_ARGUMENT, error.to_string()))?;
    let session_id = action.session_id();
    let result = unsafe { &mut (*engine).engine }
      .submit_ui_event(action)
      .map_err(|error| (ENGINE_ERROR, error.to_string()))?;
    if result.response.session_id() != session_id {
      return Err((
        ENGINE_ERROR,
        "UI event response session mismatch".to_owned(),
      ));
    }
    Ok(result)
  })) {
    Ok(Ok(result)) => unsafe {
      out_disposition.write(result.disposition as u32);
      write_finished(out_buffer, result.response)
    },
    Ok(Err((status, error))) => {
      unsafe { write_error(out_buffer, error) };
      status
    }
    Err(payload) => {
      unsafe { (*engine).poisoned = true };
      unsafe { write_panic(out_buffer, "battlement_submit_ui_event", payload.as_ref()) };
      PANIC
    }
  }
}

/// Polls a  engine for an already-finished response.
///
/// # Safety
///
/// Pointers must satisfy the exported poll contract.
#[doc(hidden)]
pub unsafe fn ffi_poll<F, E>(_: F, engine: *mut c_void, out_buffer: *mut BattlementBuffer) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  if out_buffer.is_null() {
    return INVALID_ARGUMENT;
  }
  unsafe { out_buffer.write(BattlementBuffer::EMPTY) };
  let engine = engine.cast::<BattlementEngine<E>>();
  if engine.is_null() {
    unsafe { write_error(out_buffer, "engine pointer is null") };
    return INVALID_ARGUMENT;
  }
  if unsafe { (*engine).poisoned } {
    unsafe { write_error(out_buffer, "Rust engine is poisoned after an earlier panic") };
    return PANIC;
  }
  panic_capture::prepare();
  match catch_unwind(AssertUnwindSafe(|| unsafe { &mut (*engine).engine }.poll())) {
    Ok(Ok(Some(message))) => unsafe { write_finished(out_buffer, message) },
    Ok(Ok(None)) => NO_MESSAGE,
    Ok(Err(error)) => {
      unsafe { write_error(out_buffer, error) };
      ENGINE_ERROR
    }
    Err(payload) => {
      unsafe { (*engine).poisoned = true };
      unsafe { write_panic(out_buffer, "battlement_poll", payload.as_ref()) };
      PANIC
    }
  }
}

/// Samples the host clock without executing application work.
///
/// # Safety
/// The engine and output must satisfy the exported clock contract.
#[doc(hidden)]
pub unsafe fn ffi_set_time<F, E>(
  _: F,
  engine: *mut c_void,
  elapsed_us: u64,
  out_buffer: *mut BattlementBuffer,
) -> i32
where
  F: FnOnce() -> Result<E, EngineError>,
  E: Engine,
{
  // SAFETY: The factory marker identifies the concrete engine allocation.
  unsafe {
    ffi_output_call(
      engine.cast::<BattlementEngine<E>>(),
      out_buffer,
      "battlement_set_time",
      |engine| {
        engine.set_time(Duration::from_micros(elapsed_us));
        Ok(None)
      },
    )
  }
}

unsafe fn ffi_output_call<E: Engine>(
  engine: *mut BattlementEngine<E>,
  out_buffer: *mut BattlementBuffer,
  operation: &'static str,
  call: impl FnOnce(&mut E) -> Result<Option<EngineResponse>, (i32, String)>,
) -> i32 {
  if out_buffer.is_null() {
    return INVALID_ARGUMENT;
  }
  // SAFETY: The caller promises a writable output.
  unsafe { out_buffer.write(BattlementBuffer::EMPTY) };
  if engine.is_null() {
    // SAFETY: The output was validated above.
    unsafe { write_error(out_buffer, "engine pointer is null") };
    return INVALID_ARGUMENT;
  }
  if unsafe { (*engine).poisoned } {
    // SAFETY: The output was validated above.
    unsafe { write_error(out_buffer, "Rust engine is poisoned after an earlier panic") };
    return PANIC;
  }
  panic_capture::prepare();
  match catch_unwind(AssertUnwindSafe(|| call(unsafe { &mut (*engine).engine }))) {
    Ok(Ok(Some(message))) => unsafe { write_finished(out_buffer, message) },
    Ok(Ok(None)) => NO_MESSAGE,
    Ok(Err((status, error))) => {
      unsafe { write_error(out_buffer, error) };
      status
    }
    Err(payload) => {
      unsafe { (*engine).poisoned = true };
      unsafe { write_panic(out_buffer, operation, payload.as_ref()) };
      PANIC
    }
  }
}

/// Frees one output buffer without allowing a panic to cross the ABI.
///
/// # Safety
///
/// The buffer follows the requirements of [`buffer_free`].
#[doc(hidden)]
pub unsafe fn ffi_buffer_free(buffer: BattlementBuffer) {
  let _ = catch_unwind(AssertUnwindSafe(|| {
    // SAFETY: The caller returns an adapter-owned buffer exactly once.
    unsafe { buffer_free(buffer) }
  }));
}

unsafe fn write_panic(
  out_buffer: *mut BattlementBuffer,
  operation: &str,
  payload: &(dyn std::any::Any + Send),
) {
  if out_buffer.is_null() {
    return;
  }
  // SAFETY: The caller promises a writable non-null output pointer.
  let message = panic_capture::describe(operation, payload);
  unsafe { write_error(out_buffer, message) };
}

unsafe fn input_slice<'a>(data: *const u8, length: u64) -> Result<&'a [u8], &'static str> {
  if length > battlement_flatbuffers::MAXIMUM_MESSAGE_BYTES as u64 {
    return Err("input length exceeds 16 MiB");
  }
  if length > isize::MAX as u64 {
    return Err("input length exceeds the maximum Rust slice size");
  }
  let length = usize::try_from(length).map_err(|_| "input length exceeds this platform")?;
  if length == 0 {
    return Ok(&[]);
  }
  if data.is_null() {
    return Err("input pointer is null for a nonempty message");
  }

  // SAFETY: The caller promises `data` is readable for `length` bytes.
  Ok(unsafe { std::slice::from_raw_parts(data, length) })
}

unsafe fn write_finished(out_buffer: *mut BattlementBuffer, response: EngineResponse) -> i32 {
  let (message, lease) = response.into_parts();
  let (storage, start) = message.into_storage();
  response_budget::handoff(storage.as_ptr() as usize, lease);
  // SAFETY: The caller provides a checked, writable output pointer.
  unsafe { out_buffer.write(BattlementBuffer::from_storage(storage, start)) };
  OK
}

unsafe fn write_error(out_buffer: *mut BattlementBuffer, error: impl ToString) {
  // SAFETY: The caller provides a checked, writable output pointer.
  unsafe { out_buffer.write(BattlementBuffer::from_bytes(bounded_diagnostic(error))) };
}

fn bounded_diagnostic(error: impl ToString) -> Vec<u8> {
  let mut diagnostic = error.to_string();
  if diagnostic.len() <= MAXIMUM_DIAGNOSTIC_BYTES {
    return diagnostic.into_bytes();
  }

  let mut retained = MAXIMUM_DIAGNOSTIC_BYTES - DIAGNOSTIC_TRUNCATED_SUFFIX.len();
  while !diagnostic.is_char_boundary(retained) {
    retained -= 1;
  }
  diagnostic.truncate(retained);
  diagnostic.push_str(DIAGNOSTIC_TRUNCATED_SUFFIX);
  diagnostic.into_bytes()
}
