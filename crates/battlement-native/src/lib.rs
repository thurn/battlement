//! Native adapter primitives for typed Battlement rules engines.
//!
//! This crate owns the raw-buffer boundary and JSON conversion. A game
//! supplies an [`Engine`] and [`EngineFactory`] and invokes [`export_engine!`]
//! with its constructor to emit the fixed panic-safe C ABI.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod adapter;
mod engine;
/// Transport-neutral localhost HTTP routing.
pub mod http;
mod logging;
mod panic_capture;
#[cfg(feature = "threading")]
pub mod threading;

pub use adapter::*;
pub use engine::*;
pub use logging::*;

/// Wire version implemented by engines that opt into Ditto's deterministic runtime contract.
pub const DITTO_DETERMINISM_CONTRACT_V3: u32 = 3;
/// Required v3 declarations: controlled clock and randomness, isolated external state,
/// reset persistent state, semantic input delivery, and protocol-owned visible output.
pub const DITTO_DETERMINISM_CAPABILITIES_V3: u64 = 0b11_1111;
/// SHA-256 of the canonical native ABI manifest.
pub const NATIVE_ABI_DIGEST: &str =
  "f27c823eda0c569cd86feda6c9d24b7a1d1d5c0eedc2206ee92ec2e26aa9cc79";
/// SHA-256 of the canonical JSON wire-contract manifest.
pub const WIRE_CONTRACT_DIGEST: &str =
  "3c7f1f5a672808b12f91b111f91973cdac13b37ddaedce812b2044caa1da10cd";

#[doc(hidden)]
pub static NATIVE_ABI_DIGEST_C: &[u8; 65] =
  b"f27c823eda0c569cd86feda6c9d24b7a1d1d5c0eedc2206ee92ec2e26aa9cc79\0";
#[doc(hidden)]
pub static WIRE_CONTRACT_DIGEST_C: &[u8; 65] =
  b"3c7f1f5a672808b12f91b111f91973cdac13b37ddaedce812b2044caa1da10cd\0";

/// Exports the fixed Battlement C symbols for one concrete engine factory.
///
/// The factory expression must implement [`EngineFactory`], typically a
/// zero-argument function returning an engine directly. Construction that can
/// fail may return `Result<YourEngine, EngineError>`.
///
/// ```ignore
/// fn create_engine() -> MyEngine {
///     MyEngine::default()
/// }
///
/// battlement_native::export_engine!(create_engine);
/// ```
#[macro_export]
macro_rules! export_engine {
  ($factory:path $(,)?) => {
    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_native_abi_digest() -> *const ::core::ffi::c_char {
      $crate::NATIVE_ABI_DIGEST_C.as_ptr().cast()
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_wire_contract_digest() -> *const ::core::ffi::c_char {
      $crate::WIRE_CONTRACT_DIGEST_C.as_ptr().cast()
    }

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
    pub unsafe extern "C" fn battlement_submit_ui_event(
      engine: *mut ::core::ffi::c_void,
      json: *const u8,
      length: u64,
      out_disposition: *mut u32,
      out_buffer: *mut $crate::BattlementBuffer,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe {
        $crate::ffi_submit_ui_event($factory, engine, json, length, out_disposition, out_buffer)
      }
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

/// Exports an engine together with the deterministic Ditto capability handshake.
#[macro_export]
macro_rules! export_deterministic_engine {
  (
    $factory:path,
    clock = virtualized,
    randomness = seeded,
    external_state = isolated,
    persistent_state = reset,
    input = semantic,
    visible_output = protocol_owned $(,)?
  ) => {
    $crate::export_engine!($factory);

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_ditto_determinism_contract() -> u32 {
      $crate::DITTO_DETERMINISM_CONTRACT_V3
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_ditto_determinism_capabilities() -> u64 {
      $crate::DITTO_DETERMINISM_CAPABILITIES_V3
    }
  };
}
