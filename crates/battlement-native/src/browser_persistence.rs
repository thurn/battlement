//! Asynchronous IDBFS operations completed on the browser application thread.

use std::{
  ffi::{CString, c_void},
  path::Path,
};

/// Reads and mutations share Unity's serialized IDBFS synchronization boundary.
pub enum BrowserPersistenceOperation<'a> {
  /// Read the last durable bytes or absence.
  Load,
  /// Replace the complete file.
  Store(&'a [u8]),
  /// Delete the file, accepting absence.
  Remove,
}

type Completion = Box<dyn FnOnce(Result<Option<Vec<u8>>, String>)>;

/// Retains the completion until the browser acknowledges the storage transaction.
pub fn start_browser_persistence(
  path: &Path,
  operation: BrowserPersistenceOperation<'_>,
  complete: impl FnOnce(Result<Option<Vec<u8>>, String>) + 'static,
) {
  let Ok(path) = CString::new(path.to_string_lossy().as_bytes()) else {
    complete(Err("persistent path contains a null byte".to_owned()));
    return;
  };
  let (kind, bytes) = match operation {
    BrowserPersistenceOperation::Load => (0, &[][..]),
    BrowserPersistenceOperation::Store(bytes) => (1, bytes),
    BrowserPersistenceOperation::Remove => (2, &[][..]),
  };
  let completion: Box<Completion> = Box::new(Box::new(complete));
  // SAFETY: The bridge copies both borrowed inputs before returning. It invokes
  // finish exactly once on this thread, transferring ownership of completion.
  unsafe {
    battlement_persistence_start(
      path.as_ptr(),
      kind,
      bytes.as_ptr(),
      bytes.len(),
      Box::into_raw(completion).cast(),
      self::finish,
    );
  }
}

unsafe extern "C" fn finish(completion: *mut c_void, status: i32, bytes: *const u8, length: usize) {
  // SAFETY: The bridge returns the unique owner allocated by start and provides
  // a readable buffer of length bytes for the duration of this callback.
  let complete = unsafe { Box::from_raw(completion.cast::<Completion>()) };
  let bytes = if length == 0 {
    Vec::new()
  } else {
    unsafe { std::slice::from_raw_parts(bytes, length) }.to_vec()
  };
  complete(match status {
    0 => Ok(None),
    1 => Ok(Some(bytes)),
    _ => Err(String::from_utf8_lossy(&bytes).into_owned()),
  });
}

unsafe extern "C" {
  fn battlement_persistence_start(
    path: *const std::ffi::c_char,
    kind: i32,
    bytes: *const u8,
    length: usize,
    completion: *mut c_void,
    finish: unsafe extern "C" fn(*mut c_void, i32, *const u8, usize),
  );
}
