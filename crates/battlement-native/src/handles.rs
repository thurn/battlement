use std::{
  any::TypeId,
  cell::RefCell,
  collections::HashMap,
  ffi::c_void,
  panic::{AssertUnwindSafe, catch_unwind},
  ptr,
  sync::atomic::{AtomicU64, Ordering},
};

use crate::{
  BattlementBuffer, EngineFactory, INVALID_ARGUMENT, NativeEngineFactory, PANIC, buffer_free,
  ffi_connect, ffi_create, ffi_destroy, ffi_native_connect, ffi_native_create, ffi_native_destroy,
  ffi_native_poll, ffi_native_submit, ffi_native_submit_ui_event, ffi_poll, ffi_submit,
  ffi_submit_ui_event,
};

/// Opaque identity for one live native engine.
pub type EngineHandle = u64;
/// Opaque identity for one immutable native output buffer.
pub type BufferHandle = u64;

const EMPTY_HANDLE: u64 = 0;

static NEXT_ENGINE_HANDLE: AtomicU64 = AtomicU64::new(1);
static NEXT_BUFFER_HANDLE: AtomicU64 = AtomicU64::new(1);

thread_local! {
  static ENGINES: RefCell<HashMap<EngineHandle, EngineEntry>> = RefCell::new(HashMap::new());
  static BUFFERS: RefCell<BufferRegistry> = RefCell::new(BufferRegistry::default());
}

const MAXIMUM_LIVE_BUFFER_BYTES: u64 = 64 * 1024 * 1024;

struct EngineEntry {
  pointer: *mut c_void,
  type_id: TypeId,
  in_call: bool,
}

struct RegisteredBuffer(Option<BattlementBuffer>);

#[derive(Default)]
struct BufferRegistry {
  entries: HashMap<BufferHandle, RegisteredBuffer>,
  allocation_bytes: u64,
}

impl Drop for RegisteredBuffer {
  fn drop(&mut self) {
    // SAFETY: A registered buffer exclusively owns the adapter allocation until removal.
    unsafe {
      buffer_free(
        self
          .0
          .take()
          .expect("registered buffer owns an adapter allocation"),
      )
    };
  }
}

struct CallGuard {
  handle: EngineHandle,
}

impl Drop for CallGuard {
  fn drop(&mut self) {
    ENGINES.with_borrow_mut(|engines| {
      if let Some(entry) = engines.get_mut(&self.handle) {
        entry.in_call = false;
      }
    });
  }
}

/// Creates one engine and publishes its non-pointer identity.
///
/// # Safety
///
/// Non-null outputs must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_create_handle<F>(
  factory: F,
  out_engine: *mut EngineHandle,
  out_error: *mut BufferHandle,
) -> i32
where
  F: EngineFactory,
  F::Engine: 'static,
{
  if !out_engine.is_null() {
    // SAFETY: The caller promises a writable non-null output.
    unsafe { out_engine.write(EMPTY_HANDLE) };
  }
  if !out_error.is_null() {
    // SAFETY: The caller promises a writable non-null output.
    unsafe { out_error.write(EMPTY_HANDLE) };
  }
  if out_engine.is_null() || out_error.is_null() {
    return INVALID_ARGUMENT;
  }

  let mut pointer = ptr::null_mut();
  let mut error = BattlementBuffer::EMPTY;
  // SAFETY: Local outputs are valid for the delegated call.
  let status = unsafe { ffi_create(factory, &mut pointer, &mut error) };
  // SAFETY: The caller's output was validated above.
  unsafe { out_error.write(register_buffer(error)) };
  if pointer.is_null() {
    return status;
  }

  let handle = next_handle(&NEXT_ENGINE_HANDLE);
  ENGINES.with_borrow_mut(|engines| {
    engines.insert(
      handle,
      EngineEntry {
        pointer,
        type_id: TypeId::of::<F::Engine>(),
        in_call: false,
      },
    );
  });
  // SAFETY: The caller's output was validated above.
  unsafe { out_engine.write(handle) };
  status
}

/// Destroys one engine identity without accepting allocation pointers from the caller.
///
/// # Safety
///
/// `out_error` must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_destroy_handle<F>(
  factory: F,
  engine: EngineHandle,
  out_error: *mut BufferHandle,
) -> i32
where
  F: EngineFactory,
  F::Engine: 'static,
{
  if out_error.is_null() {
    return INVALID_ARGUMENT;
  }
  // SAFETY: The caller promises a writable non-null output.
  unsafe { out_error.write(EMPTY_HANDLE) };
  let entry = ENGINES.with_borrow_mut(|engines| {
    let valid = engines
      .get(&engine)
      .is_some_and(|entry| entry.type_id == TypeId::of::<F::Engine>() && !entry.in_call);
    valid.then(|| {
      engines
        .remove(&engine)
        .expect("validated engine entry exists")
    })
  });
  let Some(entry) = entry else {
    // SAFETY: The output was validated and initialized above.
    unsafe {
      out_error.write(diagnostic_handle(
        "engine handle is invalid, stale, or busy",
      ))
    };
    return INVALID_ARGUMENT;
  };

  let mut error = BattlementBuffer::EMPTY;
  // SAFETY: The registry returns the exact typed pointer created for this factory.
  let status = unsafe { ffi_destroy(factory, entry.pointer, &mut error) };
  // SAFETY: The output was validated above.
  unsafe { out_error.write(register_buffer(error)) };
  status
}

/// Invokes connect through a validated engine identity.
///
/// # Safety
///
/// Input and output pointers must satisfy the delegated adapter contract.
#[doc(hidden)]
pub unsafe fn ffi_connect_handle<F>(
  factory: F,
  engine: EngineHandle,
  data: *const u8,
  length: u64,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: EngineFactory,
  F::Engine: 'static,
{
  // SAFETY: This function forwards the caller's pointer contract after resolving the handle.
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_connect(factory, pointer, data, length, output)
    })
  }
}

/// Invokes submit through a validated engine identity.
///
/// # Safety
///
/// Input and output pointers must satisfy the delegated adapter contract.
#[doc(hidden)]
pub unsafe fn ffi_submit_handle<F>(
  factory: F,
  engine: EngineHandle,
  data: *const u8,
  length: u64,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: EngineFactory,
  F::Engine: 'static,
{
  // SAFETY: This function forwards the caller's pointer contract after resolving the handle.
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_submit(factory, pointer, data, length, output)
    })
  }
}

/// Invokes synchronous UI submission through a validated engine identity.
///
/// # Safety
///
/// Input and output pointers must satisfy the delegated adapter contract.
#[doc(hidden)]
pub unsafe fn ffi_submit_ui_event_handle<F>(
  factory: F,
  engine: EngineHandle,
  data: *const u8,
  length: u64,
  out_disposition: *mut u32,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: EngineFactory,
  F::Engine: 'static,
{
  if !out_disposition.is_null() {
    // SAFETY: The caller promises a writable non-null output.
    unsafe { out_disposition.write(0) };
  }
  // SAFETY: This function forwards the caller's pointer contract after resolving the handle.
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_submit_ui_event(factory, pointer, data, length, out_disposition, output)
    })
  }
}

/// Polls through a validated engine identity.
///
/// # Safety
///
/// `out_buffer` must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_poll_handle<F>(
  factory: F,
  engine: EngineHandle,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: EngineFactory,
  F::Engine: 'static,
{
  // SAFETY: This function forwards the caller's pointer contract after resolving the handle.
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_poll(factory, pointer, output)
    })
  }
}

/// Creates one direct native engine and publishes its non-pointer identity.
///
/// # Safety
///
/// Non-null outputs must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_native_create_handle<F>(
  factory: F,
  out_engine: *mut EngineHandle,
  out_error: *mut BufferHandle,
) -> i32
where
  F: NativeEngineFactory,
  F::Engine: 'static,
{
  if out_engine.is_null() || out_error.is_null() {
    return INVALID_ARGUMENT;
  }
  unsafe {
    out_engine.write(EMPTY_HANDLE);
    out_error.write(EMPTY_HANDLE);
  }
  let mut pointer = ptr::null_mut();
  let mut error = BattlementBuffer::EMPTY;
  let status = unsafe { ffi_native_create(factory, &mut pointer, &mut error) };
  unsafe { out_error.write(register_buffer(error)) };
  if pointer.is_null() {
    return status;
  }
  let handle = next_handle(&NEXT_ENGINE_HANDLE);
  ENGINES.with_borrow_mut(|engines| {
    engines.insert(
      handle,
      EngineEntry {
        pointer,
        type_id: TypeId::of::<F::Engine>(),
        in_call: false,
      },
    );
  });
  unsafe { out_engine.write(handle) };
  status
}

/// Destroys one direct native engine identity.
///
/// # Safety
///
/// `out_error` must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_native_destroy_handle<F>(
  factory: F,
  engine: EngineHandle,
  out_error: *mut BufferHandle,
) -> i32
where
  F: NativeEngineFactory,
  F::Engine: 'static,
{
  if out_error.is_null() {
    return INVALID_ARGUMENT;
  }
  unsafe { out_error.write(EMPTY_HANDLE) };
  let entry = ENGINES.with_borrow_mut(|engines| {
    let valid = engines
      .get(&engine)
      .is_some_and(|entry| entry.type_id == TypeId::of::<F::Engine>() && !entry.in_call);
    valid.then(|| {
      engines
        .remove(&engine)
        .expect("validated engine entry exists")
    })
  });
  let Some(entry) = entry else {
    unsafe {
      out_error.write(diagnostic_handle(
        "engine handle is invalid, stale, or busy",
      ))
    };
    return INVALID_ARGUMENT;
  };
  let mut error = BattlementBuffer::EMPTY;
  let status = unsafe { ffi_native_destroy(factory, entry.pointer, &mut error) };
  unsafe { out_error.write(register_buffer(error)) };
  status
}

/// Invokes direct native connect through a validated engine identity.
///
/// # Safety
///
/// Pointers must satisfy the delegated adapter contract.
#[doc(hidden)]
pub unsafe fn ffi_native_connect_handle<F>(
  factory: F,
  engine: EngineHandle,
  data: *const u8,
  length: u64,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: NativeEngineFactory,
  F::Engine: 'static,
{
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_native_connect(factory, pointer, data, length, output)
    })
  }
}

/// Invokes direct native submit through a validated engine identity.
///
/// # Safety
///
/// Pointers must satisfy the delegated adapter contract.
#[doc(hidden)]
pub unsafe fn ffi_native_submit_handle<F>(
  factory: F,
  engine: EngineHandle,
  data: *const u8,
  length: u64,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: NativeEngineFactory,
  F::Engine: 'static,
{
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_native_submit(factory, pointer, data, length, output)
    })
  }
}

/// Invokes direct native UI submission through a validated engine identity.
///
/// # Safety
///
/// Pointers must satisfy the delegated adapter contract.
#[doc(hidden)]
pub unsafe fn ffi_native_submit_ui_event_handle<F>(
  factory: F,
  engine: EngineHandle,
  data: *const u8,
  length: u64,
  out_disposition: *mut u32,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: NativeEngineFactory,
  F::Engine: 'static,
{
  if !out_disposition.is_null() {
    unsafe { out_disposition.write(0) };
  }
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_native_submit_ui_event(factory, pointer, data, length, out_disposition, output)
    })
  }
}

/// Polls a direct native engine through a validated identity.
///
/// # Safety
///
/// `out_buffer` must be writable and correctly aligned.
#[doc(hidden)]
pub unsafe fn ffi_native_poll_handle<F>(
  factory: F,
  engine: EngineHandle,
  out_buffer: *mut BufferHandle,
) -> i32
where
  F: NativeEngineFactory,
  F::Engine: 'static,
{
  unsafe {
    output_call::<F::Engine, _>(engine, out_buffer, |pointer, output| {
      ffi_native_poll(factory, pointer, output)
    })
  }
}

/// Publishes one immutable buffer's finished range and allocation size.
///
/// # Safety
///
/// Non-null outputs must be writable and correctly aligned. The returned pointer
/// remains valid until the matching buffer handle is released on this thread.
#[doc(hidden)]
pub unsafe fn ffi_buffer_info(
  handle: BufferHandle,
  out_data: *mut *const u8,
  out_length: *mut u64,
  out_allocation_bytes: *mut u64,
) -> i32 {
  if !out_data.is_null() {
    // SAFETY: The caller promises a writable non-null output.
    unsafe { out_data.write(ptr::null()) };
  }
  if !out_length.is_null() {
    // SAFETY: The caller promises a writable non-null output.
    unsafe { out_length.write(0) };
  }
  if !out_allocation_bytes.is_null() {
    // SAFETY: The caller promises a writable non-null output.
    unsafe { out_allocation_bytes.write(0) };
  }
  if out_data.is_null() || out_length.is_null() || out_allocation_bytes.is_null() {
    return INVALID_ARGUMENT;
  }

  BUFFERS.with_borrow(|buffers| {
    let Some(buffer) = buffers.entries.get(&handle) else {
      return INVALID_ARGUMENT;
    };
    let buffer = buffer
      .0
      .as_ref()
      .expect("registered buffer owns an adapter allocation");
    // SAFETY: All outputs were validated above and the registry owns immutable bytes.
    unsafe {
      out_data.write(buffer.data.cast_const());
      out_length.write(buffer.length);
      out_allocation_bytes.write(buffer.allocation_bytes);
    }
    crate::OK
  })
}

/// Releases one immutable output identity exactly once.
#[doc(hidden)]
pub fn ffi_release_buffer(handle: BufferHandle) -> i32 {
  if handle == EMPTY_HANDLE {
    return INVALID_ARGUMENT;
  }
  BUFFERS.with_borrow_mut(|buffers| {
    if let Some(buffer) = buffers.entries.remove(&handle) {
      let allocation_bytes = buffer
        .0
        .as_ref()
        .expect("registered buffer owns an adapter allocation")
        .allocation_bytes;
      buffers.allocation_bytes = buffers
        .allocation_bytes
        .checked_sub(allocation_bytes)
        .expect("registered allocation accounting underflow");
      drop(buffer);
      crate::OK
    } else {
      INVALID_ARGUMENT
    }
  })
}

/// Registers an internally produced output buffer for ABI inspection.
#[doc(hidden)]
pub fn register_buffer(buffer: BattlementBuffer) -> BufferHandle {
  if buffer.data.is_null() || buffer.length == 0 {
    return EMPTY_HANDLE;
  }
  let allocation_bytes = buffer.allocation_bytes;
  let handle = next_handle(&NEXT_BUFFER_HANDLE);
  let mut pending = Some(buffer);
  let registered = BUFFERS.with_borrow_mut(|buffers| {
    let Some(total) = buffers.allocation_bytes.checked_add(allocation_bytes) else {
      return false;
    };
    if total > MAXIMUM_LIVE_BUFFER_BYTES {
      return false;
    }
    buffers.allocation_bytes = total;
    buffers.entries.insert(
      handle,
      RegisteredBuffer(Some(
        pending
          .take()
          .expect("unregistered buffer remains available"),
      )),
    );
    true
  });
  if registered {
    handle
  } else {
    // SAFETY: Registration did not take ownership, so the adapter allocation remains unique.
    unsafe { buffer_free(pending.expect("rejected buffer remains available")) };
    EMPTY_HANDLE
  }
}

unsafe fn output_call<E, C>(engine: EngineHandle, out_buffer: *mut BufferHandle, call: C) -> i32
where
  E: 'static,
  C: FnOnce(*mut c_void, *mut BattlementBuffer) -> i32,
{
  if out_buffer.is_null() {
    return INVALID_ARGUMENT;
  }
  // SAFETY: The caller promises a writable non-null output.
  unsafe { out_buffer.write(EMPTY_HANDLE) };
  let Some((pointer, _guard)) = begin_call::<E>(engine) else {
    // SAFETY: The output was validated and initialized above.
    unsafe {
      out_buffer.write(diagnostic_handle(
        "engine handle is invalid, stale, or busy",
      ))
    };
    return INVALID_ARGUMENT;
  };
  let mut buffer = BattlementBuffer::EMPTY;
  let result = catch_unwind(AssertUnwindSafe(|| call(pointer, &mut buffer)));
  let had_buffer = !buffer.data.is_null() && buffer.length != 0;
  let handle = register_buffer(buffer);
  if had_buffer && handle == EMPTY_HANDLE {
    // SAFETY: The oversized allocation was released before this bounded diagnostic is registered.
    unsafe {
      out_buffer.write(diagnostic_handle(
        "native output exceeds the 64 MiB live buffer budget",
      ))
    };
    return crate::ENGINE_ERROR;
  }
  // SAFETY: The output was validated above and ownership transfers to the registry.
  unsafe { out_buffer.write(handle) };
  match result {
    Ok(status) => status,
    Err(_) => PANIC,
  }
}

fn begin_call<E: 'static>(handle: EngineHandle) -> Option<(*mut c_void, CallGuard)> {
  ENGINES.with_borrow_mut(|engines| {
    let entry = engines.get_mut(&handle)?;
    if entry.type_id != TypeId::of::<E>() || entry.in_call {
      return None;
    }
    entry.in_call = true;
    Some((entry.pointer, CallGuard { handle }))
  })
}

fn diagnostic_handle(message: &str) -> BufferHandle {
  register_buffer(BattlementBuffer::from_bytes(message.as_bytes().to_vec()))
}

fn next_handle(counter: &AtomicU64) -> u64 {
  let handle = counter.fetch_add(1, Ordering::Relaxed);
  assert_ne!(handle, EMPTY_HANDLE, "native handle space exhausted");
  handle
}

#[cfg(test)]
mod tests {
  use battlement_flatbuffers::{MessageWriter, message_writer_diagnostics};

  use super::*;

  #[test]
  fn registry_rejects_live_allocations_beyond_sixty_four_mebibytes() {
    let mut handles = Vec::new();
    for _ in 0..4 {
      let handle = register_buffer(BattlementBuffer::from_bytes(vec![0; 16 * 1024 * 1024]));
      assert_ne!(handle, EMPTY_HANDLE);
      handles.push(handle);
    }
    assert_eq!(
      register_buffer(BattlementBuffer::from_bytes(vec![0; 1])),
      EMPTY_HANDLE
    );
    for handle in handles {
      assert_eq!(ffi_release_buffer(handle), crate::OK);
    }
  }

  #[test]
  fn registry_exposes_finished_suffix_and_frees_original_allocation() {
    let storage = vec![0xaa, 0xbb, 1, 2, 3, 4];
    let allocation_bytes = storage.capacity() as u64;
    let handle = register_buffer(BattlementBuffer::from_storage(storage, 2));
    assert_ne!(handle, EMPTY_HANDLE);

    let mut data = ptr::null();
    let mut length = 0;
    let mut retained = 0;
    // SAFETY: The local outputs are writable and `handle` is live.
    assert_eq!(
      unsafe { ffi_buffer_info(handle, &mut data, &mut length, &mut retained) },
      crate::OK
    );
    assert_eq!(length, 4);
    assert_eq!(retained, allocation_bytes);
    // SAFETY: The registry owns at least `length` initialized bytes at `data`.
    assert_eq!(unsafe { std::slice::from_raw_parts(data, 4) }, [1, 2, 3, 4]);
    assert_eq!(ffi_release_buffer(handle), crate::OK);
    assert_eq!(ffi_release_buffer(handle), INVALID_ARGUMENT);
  }

  #[test]
  fn registry_release_returns_eligible_storage_to_the_message_writer_pool() {
    let handle = register_buffer(BattlementBuffer::from_bytes(vec![0x5a; 4 * 1024]));
    assert_ne!(handle, EMPTY_HANDLE);
    let idle_before = message_writer_diagnostics().idle_bytes;
    assert_eq!(ffi_release_buffer(handle), crate::OK);
    assert_eq!(
      message_writer_diagnostics().idle_bytes,
      idle_before + 4 * 1024
    );

    let reused_before = message_writer_diagnostics().reused;
    let response = MessageWriter::default().finish([0x71; 16], &[]).unwrap();
    assert!(message_writer_diagnostics().reused > reused_before);
    assert_eq!(response.allocation_bytes(), 4 * 1024);
  }
}
