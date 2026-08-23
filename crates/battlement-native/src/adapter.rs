use std::{
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use battlement::{Response, json};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Engine, EngineError, EngineFactory, panic_capture};

/// Operation completed successfully and returned a JSON response.
pub const OK: i32 = 0;
/// Poll completed successfully without a response.
pub const NO_MESSAGE: i32 = 1;
/// A pointer, length, or JSON input was invalid.
pub const INVALID_ARGUMENT: i32 = 2;
/// Engine construction or execution failed.
pub const ENGINE_ERROR: i32 = 3;
/// An exported entry point caught a Rust panic.
pub const PANIC: i32 = 4;

static HAS_LIVE_ENGINE: AtomicBool = AtomicBool::new(false);
static OUTSTANDING_BUFFERS: AtomicUsize = AtomicUsize::new(0);

/// One owned byte buffer crossing the C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BattlementBuffer {
    /// Pointer to the first byte, or null when `length` is zero.
    pub data: *mut u8,
    /// Number of initialized bytes available at `data`.
    pub length: u64,
}

impl BattlementBuffer {
    /// The only valid empty-buffer representation.
    pub const EMPTY: Self = Self {
        data: ptr::null_mut(),
        length: 0,
    };

    fn from_bytes(bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return Self::EMPTY;
        }

        let mut bytes = bytes.into_boxed_slice();
        let buffer = Self {
            data: bytes.as_mut_ptr(),
            length: bytes.len() as u64,
        };
        std::mem::forget(bytes);
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
pub struct BattlementEngine<E: Engine> {
    engine: E,
    poisoned: bool,
}

/// Creates the process's one live engine instance.
///
/// The output engine is initialized to null and the error buffer to empty
/// before construction begins.
///
/// # Safety
///
/// Non-null output pointers must be writable and correctly aligned. The
/// returned engine must later be passed exactly once to [`destroy`].
pub unsafe fn create<F>(
    factory: F,
    out_engine: *mut *mut BattlementEngine<F::Engine>,
    out_error: *mut BattlementBuffer,
) -> i32
where
    F: EngineFactory,
{
    if !out_engine.is_null() {
        // SAFETY: The caller promises that a non-null output pointer is writable.
        unsafe { out_engine.write(ptr::null_mut()) };
    }
    if !out_error.is_null() {
        // SAFETY: The caller promises that a non-null output pointer is writable.
        unsafe { out_error.write(BattlementBuffer::EMPTY) };
    }
    if out_engine.is_null() || out_error.is_null() {
        return INVALID_ARGUMENT;
    }

    if HAS_LIVE_ENGINE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        // SAFETY: `out_error` was checked and initialized above.
        unsafe { write_error(out_error, "a Battlement engine instance is already live") };
        return INVALID_ARGUMENT;
    }
    let mut reservation = LiveEngineReservation { committed: false };

    match factory.create() {
        Ok(engine) => {
            let engine = Box::new(BattlementEngine {
                engine,
                poisoned: false,
            });
            // SAFETY: `out_engine` was checked and initialized above.
            unsafe { out_engine.write(Box::into_raw(engine)) };
            reservation.committed = true;
            OK
        }
        Err(error) => {
            // SAFETY: `out_error` was checked and initialized above.
            unsafe { write_error(out_error, error.to_string()) };
            ENGINE_ERROR
        }
    }
}

/// Destroys an engine created by [`create`]. A null pointer is a no-op.
///
/// # Safety
///
/// A non-null pointer must be the unique live pointer returned by [`create`]
/// and must not be used again after this call.
pub unsafe fn destroy<E: Engine>(engine: *mut BattlementEngine<E>) {
    if engine.is_null() {
        return;
    }

    HAS_LIVE_ENGINE.store(false, Ordering::Release);
    // SAFETY: The caller transfers the unique allocation returned by `create`.
    unsafe { drop(Box::from_raw(engine)) };
}

/// Decodes a connect request, invokes the engine, and serializes its response.
///
/// # Safety
///
/// `engine` must be the live handle from [`create`]. `json_data` must be
/// readable for `length` bytes for the duration of this call. `out_buffer` must
/// be writable and correctly aligned.
pub unsafe fn connect<E: Engine>(
    engine: *mut BattlementEngine<E>,
    json_data: *const u8,
    length: u64,
    out_buffer: *mut BattlementBuffer,
) -> i32 {
    // SAFETY: All raw pointers are validated before their promised regions are accessed.
    unsafe {
        request(
            engine,
            json_data,
            length,
            out_buffer,
            "connect",
            |engine, value| engine.connect(value),
        )
    }
}

/// Decodes a client message, invokes the engine, and serializes its response.
///
/// # Safety
///
/// `engine` must be the live handle from [`create`]. `json_data` must be
/// readable for `length` bytes for the duration of this call. `out_buffer` must
/// be writable and correctly aligned.
pub unsafe fn submit<E: Engine>(
    engine: *mut BattlementEngine<E>,
    json_data: *const u8,
    length: u64,
    out_buffer: *mut BattlementBuffer,
) -> i32 {
    // SAFETY: All raw pointers are validated before their promised regions are accessed.
    unsafe {
        request(
            engine,
            json_data,
            length,
            out_buffer,
            "client message",
            |engine, value| engine.submit(value),
        )
    }
}

/// Polls one queued response without blocking.
///
/// # Safety
///
/// `engine` must be the live handle from [`create`] and `out_buffer` must be
/// writable and correctly aligned.
pub unsafe fn poll<E: Engine>(
    engine: *mut BattlementEngine<E>,
    out_buffer: *mut BattlementBuffer,
) -> i32 {
    if out_buffer.is_null() {
        return INVALID_ARGUMENT;
    }
    // SAFETY: The caller promises that a non-null output pointer is writable.
    unsafe { out_buffer.write(BattlementBuffer::EMPTY) };
    if engine.is_null() {
        // SAFETY: `out_buffer` was checked and initialized above.
        unsafe { write_error(out_buffer, "engine pointer is null") };
        return INVALID_ARGUMENT;
    }
    // SAFETY: The caller promises a unique live engine pointer for this serial call.
    if unsafe { (*engine).poisoned } {
        // SAFETY: `out_buffer` was checked and initialized above.
        unsafe { write_error(out_buffer, "Rust engine is poisoned after an earlier panic") };
        return PANIC;
    }

    // SAFETY: The caller promises a unique live engine pointer for this serial call.
    let engine = unsafe { &mut (*engine).engine };
    match engine.poll() {
        Ok(Some(response)) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe { write_response(out_buffer, &response) }
        }
        Ok(None) => NO_MESSAGE,
        Err(error) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe { write_error(out_buffer, error.to_string()) };
            ENGINE_ERROR
        }
    }
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
    let Ok(length) = usize::try_from(buffer.length) else {
        return;
    };
    let slice = ptr::slice_from_raw_parts_mut(buffer.data, length);
    // SAFETY: The caller returns the exact boxed slice allocated by `from_bytes`.
    unsafe { drop(Box::from_raw(slice)) };
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

/// Runs engine creation behind the panic-safe C ABI boundary.
///
/// # Safety
///
/// The pointers follow the requirements of [`create`].
#[doc(hidden)]
pub unsafe fn ffi_create<F>(
    factory: F,
    out_engine: *mut *mut c_void,
    out_error: *mut BattlementBuffer,
) -> i32
where
    F: EngineFactory,
{
    panic_capture::prepare();
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The erased handle has the same pointer representation and
        // the caller satisfies the underlying `create` contract.
        unsafe {
            create(
                factory,
                out_engine.cast::<*mut BattlementEngine<F::Engine>>(),
                out_error,
            )
        }
    }));

    match result {
        Ok(status) => status,
        Err(payload) => {
            if !out_engine.is_null() {
                // SAFETY: The caller promises a writable non-null output pointer.
                unsafe { out_engine.write(ptr::null_mut()) };
            }
            // SAFETY: A non-null output pointer is writable by contract.
            unsafe { write_panic(out_error, "battlement_engine_create", payload.as_ref()) };
            PANIC
        }
    }
}

/// Destroys an erased engine handle without allowing a panic to cross the ABI.
///
/// # Safety
///
/// The handle follows the requirements of [`destroy`].
#[doc(hidden)]
pub unsafe fn ffi_destroy<F>(_: F, engine: *mut c_void)
where
    F: EngineFactory,
{
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The constructor marker identifies the concrete handle type.
        unsafe { destroy(engine.cast::<BattlementEngine<F::Engine>>()) }
    }));
}

/// Runs connect behind the panic-safe C ABI boundary.
///
/// # Safety
///
/// The pointers follow the requirements of [`connect`].
#[doc(hidden)]
pub unsafe fn ffi_connect<F>(
    _: F,
    engine: *mut c_void,
    data: *const u8,
    length: u64,
    out_buffer: *mut BattlementBuffer,
) -> i32
where
    F: EngineFactory,
{
    let engine = engine.cast::<BattlementEngine<F::Engine>>();
    // SAFETY: The constructor marker identifies the concrete handle type.
    unsafe {
        ffi_output_call(engine, out_buffer, "battlement_connect", || {
            connect(engine, data, length, out_buffer)
        })
    }
}

/// Runs submit behind the panic-safe C ABI boundary.
///
/// # Safety
///
/// The pointers follow the requirements of [`submit`].
#[doc(hidden)]
pub unsafe fn ffi_submit<F>(
    _: F,
    engine: *mut c_void,
    data: *const u8,
    length: u64,
    out_buffer: *mut BattlementBuffer,
) -> i32
where
    F: EngineFactory,
{
    let engine = engine.cast::<BattlementEngine<F::Engine>>();
    // SAFETY: The constructor marker identifies the concrete handle type.
    unsafe {
        ffi_output_call(engine, out_buffer, "battlement_submit", || {
            submit(engine, data, length, out_buffer)
        })
    }
}

/// Runs poll behind the panic-safe C ABI boundary.
///
/// # Safety
///
/// The pointers follow the requirements of [`poll`].
#[doc(hidden)]
pub unsafe fn ffi_poll<F>(_: F, engine: *mut c_void, out_buffer: *mut BattlementBuffer) -> i32
where
    F: EngineFactory,
{
    let engine = engine.cast::<BattlementEngine<F::Engine>>();
    // SAFETY: The constructor marker identifies the concrete handle type.
    unsafe {
        ffi_output_call(engine, out_buffer, "battlement_poll", || {
            poll(engine, out_buffer)
        })
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

unsafe fn ffi_output_call<E: Engine>(
    engine: *mut BattlementEngine<E>,
    out_buffer: *mut BattlementBuffer,
    operation: &'static str,
    call: impl FnOnce() -> i32,
) -> i32 {
    panic_capture::prepare();
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(status) => status,
        Err(payload) => {
            if !engine.is_null() {
                // SAFETY: The caller supplies the live engine pointer used by the failed call.
                unsafe { (*engine).poisoned = true };
            }
            // SAFETY: A non-null output pointer is writable by contract.
            unsafe { write_panic(out_buffer, operation, payload.as_ref()) };
            PANIC
        }
    }
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
    let message = panic_capture::describe(payload);
    unsafe { write_error(out_buffer, format!("Rust panic in {operation}: {message}")) };
}

unsafe fn request<E, I, F>(
    engine: *mut BattlementEngine<E>,
    data: *const u8,
    length: u64,
    out_buffer: *mut BattlementBuffer,
    input_name: &str,
    operation: F,
) -> i32
where
    E: Engine,
    I: DeserializeOwned,
    F: FnOnce(&mut E, I) -> Result<Response<E::Command>, EngineError>,
{
    if out_buffer.is_null() {
        return INVALID_ARGUMENT;
    }
    // SAFETY: The caller promises that a non-null output pointer is writable.
    unsafe { out_buffer.write(BattlementBuffer::EMPTY) };
    if engine.is_null() {
        // SAFETY: `out_buffer` was checked and initialized above.
        unsafe { write_error(out_buffer, "engine pointer is null") };
        return INVALID_ARGUMENT;
    }
    // SAFETY: The caller promises a unique live engine pointer for this serial call.
    if unsafe { (*engine).poisoned } {
        // SAFETY: `out_buffer` was checked and initialized above.
        unsafe { write_error(out_buffer, "Rust engine is poisoned after an earlier panic") };
        return PANIC;
    }

    // SAFETY: The caller promises the input is readable for the duration of this call.
    let bytes = match unsafe { input_slice(data, length) } {
        Ok(bytes) => bytes,
        Err(error) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe { write_error(out_buffer, error) };
            return INVALID_ARGUMENT;
        }
    };
    let value = match json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe { write_error(out_buffer, format!("invalid {input_name} JSON: {error}")) };
            return INVALID_ARGUMENT;
        }
    };

    // SAFETY: The caller promises a unique live engine pointer for this serial call.
    let engine = unsafe { &mut (*engine).engine };
    match operation(engine, value) {
        Ok(response) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe { write_response(out_buffer, &response) }
        }
        Err(error) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe { write_error(out_buffer, error.to_string()) };
            ENGINE_ERROR
        }
    }
}

unsafe fn input_slice<'a>(data: *const u8, length: u64) -> Result<&'a [u8], &'static str> {
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

unsafe fn write_response<C: Serialize>(
    out_buffer: *mut BattlementBuffer,
    response: &Response<C>,
) -> i32 {
    match json::to_vec(response) {
        Ok(bytes) => {
            // SAFETY: The caller provides a checked, writable output pointer.
            unsafe { out_buffer.write(BattlementBuffer::from_bytes(bytes)) };
            OK
        }
        Err(error) => {
            // SAFETY: The caller provides a checked, writable output pointer.
            unsafe { write_error(out_buffer, format!("could not serialize response: {error}")) };
            ENGINE_ERROR
        }
    }
}

unsafe fn write_error(out_buffer: *mut BattlementBuffer, error: impl ToString) {
    // SAFETY: The caller provides a checked, writable output pointer.
    unsafe { out_buffer.write(BattlementBuffer::from_bytes(error.to_string().into_bytes())) };
}
