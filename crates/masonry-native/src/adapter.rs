use std::{
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use masonry::{Response, messagepack};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Engine, EngineError, EngineFactory};

/// Operation completed successfully and returned a MessagePack response.
pub const OK: i32 = 0;
/// Poll completed successfully without a response.
pub const NO_MESSAGE: i32 = 1;
/// A pointer, length, or MessagePack input was invalid.
pub const INVALID_ARGUMENT: i32 = 2;
/// Engine construction or execution failed.
pub const ENGINE_ERROR: i32 = 3;
/// An exported entry point caught a Rust panic.
pub const PANIC: i32 = 4;

static HAS_LIVE_ENGINE: AtomicBool = AtomicBool::new(false);

/// One owned byte buffer crossing the C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MasonryBuffer {
    /// Pointer to the first byte, or null when `length` is zero.
    pub data: *mut u8,
    /// Number of initialized bytes available at `data`.
    pub length: u64,
}

impl MasonryBuffer {
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
        buffer
    }
}

/// Opaque owner of one concrete engine instance.
#[repr(C)]
pub struct MasonryEngine<E: Engine> {
    engine: E,
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
    out_engine: *mut *mut MasonryEngine<F::Engine>,
    out_error: *mut MasonryBuffer,
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
        unsafe { out_error.write(MasonryBuffer::EMPTY) };
    }
    if out_engine.is_null() || out_error.is_null() {
        return INVALID_ARGUMENT;
    }

    if HAS_LIVE_ENGINE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        // SAFETY: `out_error` was checked and initialized above.
        unsafe { write_error(out_error, "a Masonry engine instance is already live") };
        return INVALID_ARGUMENT;
    }

    match factory.create() {
        Ok(engine) => {
            let engine = Box::new(MasonryEngine { engine });
            // SAFETY: `out_engine` was checked and initialized above.
            unsafe { out_engine.write(Box::into_raw(engine)) };
            OK
        }
        Err(error) => {
            HAS_LIVE_ENGINE.store(false, Ordering::Release);
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
pub unsafe fn destroy<E: Engine>(engine: *mut MasonryEngine<E>) {
    if engine.is_null() {
        return;
    }

    // SAFETY: The caller transfers the unique allocation returned by `create`.
    unsafe { drop(Box::from_raw(engine)) };
    HAS_LIVE_ENGINE.store(false, Ordering::Release);
}

/// Decodes a connect request, invokes the engine, and serializes its response.
///
/// # Safety
///
/// `engine` must be the live handle from [`create`]. `messagepack_data` must be
/// readable for `length` bytes for the duration of this call. `out_buffer` must
/// be writable and correctly aligned.
pub unsafe fn connect<E: Engine>(
    engine: *mut MasonryEngine<E>,
    messagepack_data: *const u8,
    length: u64,
    out_buffer: *mut MasonryBuffer,
) -> i32 {
    // SAFETY: All raw pointers are validated before their promised regions are accessed.
    unsafe {
        request(
            engine,
            messagepack_data,
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
/// `engine` must be the live handle from [`create`]. `messagepack_data` must be
/// readable for `length` bytes for the duration of this call. `out_buffer` must
/// be writable and correctly aligned.
pub unsafe fn submit<E: Engine>(
    engine: *mut MasonryEngine<E>,
    messagepack_data: *const u8,
    length: u64,
    out_buffer: *mut MasonryBuffer,
) -> i32 {
    // SAFETY: All raw pointers are validated before their promised regions are accessed.
    unsafe {
        request(
            engine,
            messagepack_data,
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
    engine: *mut MasonryEngine<E>,
    out_buffer: *mut MasonryBuffer,
) -> i32 {
    if out_buffer.is_null() {
        return INVALID_ARGUMENT;
    }
    // SAFETY: The caller promises that a non-null output pointer is writable.
    unsafe { out_buffer.write(MasonryBuffer::EMPTY) };
    if engine.is_null() {
        // SAFETY: `out_buffer` was checked and initialized above.
        unsafe { write_error(out_buffer, "engine pointer is null") };
        return INVALID_ARGUMENT;
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
pub unsafe fn buffer_free(buffer: MasonryBuffer) {
    if buffer.data.is_null() || buffer.length == 0 {
        return;
    }
    let Ok(length) = usize::try_from(buffer.length) else {
        return;
    };
    let slice = ptr::slice_from_raw_parts_mut(buffer.data, length);
    // SAFETY: The caller returns the exact boxed slice allocated by `from_bytes`.
    unsafe { drop(Box::from_raw(slice)) };
}

unsafe fn request<E, I, F>(
    engine: *mut MasonryEngine<E>,
    data: *const u8,
    length: u64,
    out_buffer: *mut MasonryBuffer,
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
    unsafe { out_buffer.write(MasonryBuffer::EMPTY) };
    if engine.is_null() {
        // SAFETY: `out_buffer` was checked and initialized above.
        unsafe { write_error(out_buffer, "engine pointer is null") };
        return INVALID_ARGUMENT;
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
    let value = match messagepack::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => {
            // SAFETY: `out_buffer` was checked and initialized above.
            unsafe {
                write_error(
                    out_buffer,
                    format!("invalid {input_name} MessagePack: {error}"),
                )
            };
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
    out_buffer: *mut MasonryBuffer,
    response: &Response<C>,
) -> i32 {
    match messagepack::to_vec(response) {
        Ok(bytes) => {
            // SAFETY: The caller provides a checked, writable output pointer.
            unsafe { out_buffer.write(MasonryBuffer::from_bytes(bytes)) };
            OK
        }
        Err(error) => {
            // SAFETY: The caller provides a checked, writable output pointer.
            unsafe { write_error(out_buffer, format!("could not serialize response: {error}")) };
            ENGINE_ERROR
        }
    }
}

unsafe fn write_error(out_buffer: *mut MasonryBuffer, error: impl ToString) {
    // SAFETY: The caller provides a checked, writable output pointer.
    unsafe { out_buffer.write(MasonryBuffer::from_bytes(error.to_string().into_bytes())) };
}
