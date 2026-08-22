//! Native adapter primitives for typed Masonry rules engines.
//!
//! This crate owns the raw-buffer boundary and MessagePack conversion. A game
//! supplies an [`Engine`] and [`EngineFactory`] and invokes [`export_engine!`]
//! with its constructor to emit the fixed panic-safe C ABI.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod adapter;
mod engine;
#[cfg(feature = "threading")]
pub mod threading;

pub use adapter::*;
pub use engine::*;

/// Exports the fixed Masonry C symbols for one concrete engine factory.
///
/// The factory expression must implement [`EngineFactory`], typically a
/// zero-argument function returning `Result<YourEngine, EngineError>`.
///
/// ```ignore
/// fn create_engine() -> Result<MyEngine, masonry_native::EngineError> {
///     Ok(MyEngine::default())
/// }
///
/// masonry_native::export_engine!(create_engine);
/// ```
#[macro_export]
macro_rules! export_engine {
    ($factory:path $(,)?) => {
        /// Marks this library as implementing the Masonry native ABI version 1.
        #[unsafe(no_mangle)]
        pub extern "C" fn masonry_abi_v1() {}

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn masonry_engine_create(
            out_engine: *mut *mut ::core::ffi::c_void,
            out_error: *mut $crate::MasonryBuffer,
        ) -> i32 {
            // SAFETY: This function is the raw ABI boundary and forwards its contract.
            unsafe { $crate::ffi_create($factory, out_engine, out_error) }
        }

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn masonry_engine_destroy(engine: *mut ::core::ffi::c_void) {
            // SAFETY: This function is the raw ABI boundary and forwards its contract.
            unsafe { $crate::ffi_destroy($factory, engine) }
        }

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn masonry_connect(
            engine: *mut ::core::ffi::c_void,
            messagepack: *const u8,
            length: u64,
            out_buffer: *mut $crate::MasonryBuffer,
        ) -> i32 {
            // SAFETY: This function is the raw ABI boundary and forwards its contract.
            unsafe { $crate::ffi_connect($factory, engine, messagepack, length, out_buffer) }
        }

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn masonry_submit(
            engine: *mut ::core::ffi::c_void,
            messagepack: *const u8,
            length: u64,
            out_buffer: *mut $crate::MasonryBuffer,
        ) -> i32 {
            // SAFETY: This function is the raw ABI boundary and forwards its contract.
            unsafe { $crate::ffi_submit($factory, engine, messagepack, length, out_buffer) }
        }

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn masonry_poll(
            engine: *mut ::core::ffi::c_void,
            out_buffer: *mut $crate::MasonryBuffer,
        ) -> i32 {
            // SAFETY: This function is the raw ABI boundary and forwards its contract.
            unsafe { $crate::ffi_poll($factory, engine, out_buffer) }
        }

        #[doc(hidden)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn masonry_buffer_free(buffer: $crate::MasonryBuffer) {
            // SAFETY: This function is the raw ABI boundary and forwards its contract.
            unsafe { $crate::ffi_buffer_free(buffer) }
        }
    };
}
