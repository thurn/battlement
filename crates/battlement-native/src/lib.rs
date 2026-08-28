//! Native adapter primitives for typed Battlement rules engines.
//!
//! This crate owns the raw-buffer boundary and JSON conversion. A game
//! supplies an [`Engine`] and [`EngineFactory`] and invokes [`export_engine!`]
//! with its constructor to emit the fixed panic-safe C ABI.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod adapter;
mod engine;
mod logging;
mod panic_capture;
#[cfg(feature = "threading")]
pub mod threading;

pub use adapter::*;
pub use engine::*;
pub use logging::*;

/// Exports the fixed Battlement C symbols for one concrete engine factory.
///
/// The factory expression must implement [`EngineFactory`], typically a
/// zero-argument function returning `Result<YourEngine, EngineError>`.
///
/// ```ignore
/// fn create_engine() -> Result<MyEngine, battlement_native::EngineError> {
///     Ok(MyEngine::default())
/// }
///
/// battlement_native::export_engine!(create_engine);
/// ```
#[macro_export]
macro_rules! export_engine {
  ($factory:path $(,)?) => {
    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_logging_drain(
      out_records: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_logging_drain(out_records) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_engine_create(
      out_engine: *mut *mut ::core::ffi::c_void,
      out_error: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_create($factory, out_engine, out_error) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_engine_destroy(
      engine: *mut ::core::ffi::c_void,
      out_error: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_destroy($factory, engine, out_error) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_connect(
      engine: *mut ::core::ffi::c_void,
      json: *const u8,
      length: u64,
      out_buffer: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_connect($factory, engine, json, length, out_buffer) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_submit(
      engine: *mut ::core::ffi::c_void,
      json: *const u8,
      length: u64,
      out_buffer: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_submit($factory, engine, json, length, out_buffer) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_poll(
      engine: *mut ::core::ffi::c_void,
      out_buffer: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_poll($factory, engine, out_buffer) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_buffer_free(buffer: $crate::BattlementBuffer) {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_buffer_free(buffer) }
    }
  };
}
