#![deny(missing_docs)]

//! Dynamically loaded engine fixture for the exported Masonry C ABI.

use std::sync::atomic::{AtomicUsize, Ordering};

use masonry::{ClientMessage, Command, Connect, CoreErrorCode, Response, SessionId};
use masonry_native::{Engine, EngineError};

static SUBMIT_CALLS: AtomicUsize = AtomicUsize::new(0);

struct FixtureEngine {
    mode: String,
    session_id: SessionId,
}

impl Drop for FixtureEngine {
    fn drop(&mut self) {
        if self.mode == "panic-destroy" {
            panic!("fixture destroy panic");
        }
    }
}

impl Engine for FixtureEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.mode = message.platform;
        if self.mode == "panic-connect" {
            panic!("fixture connect panic");
        }
        Ok(Response::new(self.session_id, Vec::new()))
    }

    fn submit(
        &mut self,
        _message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        SUBMIT_CALLS.fetch_add(1, Ordering::Relaxed);
        if self.mode == "panic-submit" {
            panic!("fixture submit panic");
        }
        Ok(Response::new(self.session_id, Vec::new()))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        if self.mode == "panic-poll" {
            panic!("fixture poll panic");
        }
        Ok(None)
    }
}

fn create_engine() -> Result<FixtureEngine, EngineError> {
    match std::env::var("MASONRY_EXPORT_FIXTURE_CREATE").as_deref() {
        Ok("panic") => panic!("fixture create panic"),
        Ok("error") => Err(EngineError::new("fixture create error")),
        _ => Ok(FixtureEngine {
            mode: String::new(),
            session_id: SessionId::new_v4(),
        }),
    }
}

masonry_native::export_engine!(create_engine);

#[unsafe(no_mangle)]
/// Returns the fixture adapter's live output allocation count.
pub extern "C" fn fixture_outstanding_buffers() -> usize {
    masonry_native::outstanding_buffer_count()
}

#[unsafe(no_mangle)]
/// Returns the number of times the fixture engine received submit.
pub extern "C" fn fixture_submit_calls() -> usize {
    SUBMIT_CALLS.load(Ordering::Relaxed)
}
