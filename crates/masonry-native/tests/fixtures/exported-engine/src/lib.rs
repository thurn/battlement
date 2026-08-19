#![deny(missing_docs)]

//! Dynamically loaded engine fixture for the exported Masonry C ABI.

use std::sync::atomic::{AtomicUsize, Ordering};

use masonry::{
    AnyCommand, Batch, BatchId, ClientMessage, CommandId, Connect, CoreErrorCode, CustomCommand,
    ParallelCommandGroup, Response, ResponseMessage, SessionId, messagepack,
};
use masonry_native::{Engine, EngineError};

static SUBMIT_CALLS: AtomicUsize = AtomicUsize::new(0);
static CONNECT_CALLS: AtomicUsize = AtomicUsize::new(0);

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
    type Command = AnyCommand<String>;

    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError> {
        CONNECT_CALLS.fetch_add(1, Ordering::Relaxed);
        self.mode = message.platform;
        if self.mode == "panic-connect" {
            panic!("fixture connect panic");
        }
        if self.mode == "engine-error" {
            return Err(EngineError::new("fixture engine error"));
        }
        if self.mode == "maximum-response" {
            return Ok(sized_response(self.session_id, 16 * 1024 * 1024));
        }
        if self.mode == "oversized-response" {
            return Ok(sized_response(self.session_id, 16 * 1024 * 1024 + 1));
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
        if self.mode == "poll-response" {
            return Ok(Some(Response::new(self.session_id, Vec::new())));
        }
        Ok(None)
    }
}

fn sized_response(session_id: SessionId, target: usize) -> Response<AnyCommand<String>> {
    let mut payload = "x".repeat(target);
    loop {
        let response = Response::new(
            session_id,
            vec![ResponseMessage::Batch(Batch::new(
                BatchId::new_v4(),
                session_id,
                vec![ParallelCommandGroup::new(vec![AnyCommand::Custom(
                    CustomCommand::new(CommandId::new_v4(), "fixture.large", payload),
                )])],
            ))],
        );
        let length = messagepack::to_vec(&response).unwrap().len();
        if length == target {
            return response;
        }

        let mut command = match &response.messages[0] {
            ResponseMessage::Batch(batch) => batch.groups[0].commands[0].clone(),
            ResponseMessage::Snapshot(_) => unreachable!(),
        };
        payload = match &mut command {
            AnyCommand::Custom(custom) => {
                let new_length = custom.payload.len() + target - length;
                "x".repeat(new_length)
            }
            AnyCommand::Core(_) => unreachable!(),
        };
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

#[unsafe(no_mangle)]
/// Returns the number of times the fixture engine received connect.
pub extern "C" fn fixture_connect_calls() -> usize {
    CONNECT_CALLS.load(Ordering::Relaxed)
}
