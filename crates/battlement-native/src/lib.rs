//! Native adapter primitives for typed Battlement rules engines.
//!
//! This crate owns the verified FlatBuffers boundary. A game
//! supplies a [`NativeEngine`] and invokes [`export_native_engine!`] with its
//! constructor to emit the fixed panic-safe C ABI.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod adapter;
mod engine;
mod handles;
mod logging;
mod native_engine;
mod panic_capture;
#[cfg(feature = "threading")]
pub mod threading;

pub use adapter::*;
pub use battlement_flatbuffers::ReducedMotionPreference as NativeReducedMotionPreference;
pub use battlement_flatbuffers::{
  ConnectInput, ConnectView, CoreActionBodyView, CoreClientMessageView, CoreCommandOffset,
  GameObjectOffset, GeometryObservationBatchView, MessageWriter, MotionEventBatchView,
  NativeBatchStart, NativeControllerButton, NativeControllerInput, NativeDragMode, NativeEasing,
  NativeImageFit, NativeObjectPlacement, NativeParentScene, NativePhysicalKey, NativePointerEvent,
  NativePreparedAssetKind, NativeSnapshotInput, NativeTransform, PreparedAssetOffset,
  ProtocolError, ResponseView, SceneOffset, UiChoiceView, UiDocumentOffset, UiElementBuilder,
  UiElementOffset, UiEventActionView, UiFocusView, UiGeometryView, UiIndicesView, UiKeyView,
  UiLinkView, UiNavigationView, UiNodeOffset, UiPointerButtonView, UiPointerMoveView,
  UiSelectionView, UiTabCloseView, UiTabReorderView, UiTabSelectionView, UiTransitionView,
  UiValueCommitView, UiValueView, UiWheelView, write_connect,
};
pub use engine::*;
pub use handles::*;
pub use logging::*;
pub use native_engine::*;

/// Wire version implemented by engines that opt into Ditto's deterministic runtime contract.
pub const DITTO_DETERMINISM_CONTRACT_V3: u32 = 3;
/// Required v3 declarations: controlled clock and randomness, isolated external state,
/// reset persistent state, semantic input delivery, and protocol-owned visible output.
pub const DITTO_DETERMINISM_CAPABILITIES_V3: u64 = 0b11_1111;
/// SHA-256 of the canonical native ABI manifest.
pub const NATIVE_ABI_DIGEST: &str =
  "5cb6150a485693a6a744f64a7ef64af1b2fc9d63a84ab279dde266a2dc3a7b14";
/// SHA-256 of the canonical wire-contract manifest.
pub const WIRE_CONTRACT_DIGEST: &str =
  "94063de87b8aa3df3d8492dd61c6fae2b6e1fd962c0efb6750c7299e3ecbc9cb";

#[doc(hidden)]
pub static NATIVE_ABI_DIGEST_C: &[u8; 65] =
  b"5cb6150a485693a6a744f64a7ef64af1b2fc9d63a84ab279dde266a2dc3a7b14\0";
#[doc(hidden)]
pub static WIRE_CONTRACT_DIGEST_C: &[u8; 65] =
  b"94063de87b8aa3df3d8492dd61c6fae2b6e1fd962c0efb6750c7299e3ecbc9cb\0";

#[doc(hidden)]
pub fn wire_contract_digest_for_native_factory<F, I>(_: F) -> *const core::ffi::c_char
where
  F: FnOnce() -> I,
  I: IntoNativeEngine,
{
  <I::Engine as NativeEngine>::WIRE_CONTRACT_DIGEST_C
    .as_ptr()
    .cast()
}

/// Exports the fixed Battlement C symbols for a direct FlatBuffers engine.
///
/// The factory result must implement [`NativeEngine`]. The generated native
/// entrypoints accept borrowed verified inputs and can return only finished
/// FlatBuffer allocations; the owned [`Engine`] response bridge is not part of
/// this export path.
#[macro_export]
macro_rules! export_native_engine {
  ($factory:path $(,)?) => {
    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_native_abi_digest() -> *const ::core::ffi::c_char {
      $crate::NATIVE_ABI_DIGEST_C.as_ptr().cast()
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_wire_contract_digest() -> *const ::core::ffi::c_char {
      $crate::wire_contract_digest_for_native_factory($factory)
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_logging_drain(
      out_records: *mut $crate::BufferHandle,
    ) -> i32 {
      if out_records.is_null() {
        return $crate::INVALID_ARGUMENT;
      }
      let mut buffer = $crate::BattlementBuffer::EMPTY;
      // SAFETY: The caller promises a writable output and the delegated call initializes it.
      let status = unsafe { $crate::ffi_logging_drain(&mut buffer) };
      // SAFETY: The caller promises a writable non-null output.
      unsafe { out_records.write($crate::register_buffer(buffer)) };
      status
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_engine_create(
      out_engine: *mut $crate::EngineHandle,
      out_error: *mut $crate::BufferHandle,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_native_create_handle($factory, out_engine, out_error) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_engine_destroy(
      engine: $crate::EngineHandle,
      out_error: *mut $crate::BufferHandle,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_native_destroy_handle($factory, engine, out_error) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_connect(
      engine: $crate::EngineHandle,
      input: *const u8,
      length: u64,
      out_buffer: *mut $crate::BufferHandle,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_native_connect_handle($factory, engine, input, length, out_buffer) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_submit(
      engine: $crate::EngineHandle,
      input: *const u8,
      length: u64,
      out_buffer: *mut $crate::BufferHandle,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_native_submit_handle($factory, engine, input, length, out_buffer) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_submit_ui_event(
      engine: $crate::EngineHandle,
      input: *const u8,
      length: u64,
      out_disposition: *mut u32,
      out_buffer: *mut $crate::BufferHandle,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe {
        $crate::ffi_native_submit_ui_event_handle(
          $factory,
          engine,
          input,
          length,
          out_disposition,
          out_buffer,
        )
      }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_poll(
      engine: $crate::EngineHandle,
      out_buffer: *mut $crate::BufferHandle,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_native_poll_handle($factory, engine, out_buffer) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_buffer_info(
      buffer: $crate::BufferHandle,
      out_data: *mut *const u8,
      out_length: *mut u64,
      out_allocation_bytes: *mut u64,
    ) -> i32 {
      // SAFETY: This function is the raw ABI boundary and forwards its contract.
      unsafe { $crate::ffi_buffer_info(buffer, out_data, out_length, out_allocation_bytes) }
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub extern "C" fn battlement_release_buffer(buffer: $crate::BufferHandle) -> i32 {
      $crate::ffi_release_buffer(buffer)
    }

    #[doc(hidden)]
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn battlement_transport_diagnostics(
      out_created: *mut u64,
      out_reused: *mut u64,
      out_growths: *mut u64,
      out_copied_bytes: *mut u64,
      out_idle_bytes: *mut u64,
      out_handoff_payload_copies: *mut u64,
    ) -> i32 {
      unsafe {
        $crate::ffi_transport_diagnostics(
          out_created,
          out_reused,
          out_growths,
          out_copied_bytes,
          out_idle_bytes,
          out_handoff_payload_copies,
        )
      }
    }
  };
}

/// Exports a direct FlatBuffers engine with the deterministic Ditto handshake.
#[macro_export]
macro_rules! export_deterministic_native_engine {
  (
    $factory:path,
    clock = virtualized,
    randomness = seeded,
    external_state = isolated,
    persistent_state = reset,
    input = semantic,
    visible_output = protocol_owned $(,)?
  ) => {
    $crate::export_native_engine!($factory);

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
