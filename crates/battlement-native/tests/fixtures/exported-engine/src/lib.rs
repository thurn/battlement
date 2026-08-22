#![deny(missing_docs)]

//! Dynamically loaded engine fixture for the exported Battlement C ABI.

mod release_scenarios;

use std::sync::atomic::{AtomicUsize, Ordering};

use battlement::{
    AnyCommand, Batch, BatchId, ClientMessage, Command, CommandBody, CommandId, Connect,
    CoreErrorCode, ParallelCommandGroup, Response, ResponseMessage, SessionId, TextContentPayload,
    messagepack,
};
use battlement_native::{Engine, EngineError};

pub use release_scenarios::FlashPayload;
use release_scenarios::ReleaseScenario;

static SUBMIT_CALLS: AtomicUsize = AtomicUsize::new(0);
static CONNECT_CALLS: AtomicUsize = AtomicUsize::new(0);

/// Stateful fixture used by both the exported ABI and loopback HTTP server.
pub struct FixtureEngine {
    mode: String,
    session_id: SessionId,
    release_scenario: Option<ReleaseScenario>,
    connect_count: usize,
    poll_count: usize,
}

impl Drop for FixtureEngine {
    fn drop(&mut self) {
        if self.mode == "panic-destroy" {
            panic!("fixture destroy panic");
        }
    }
}

impl Engine for FixtureEngine {
    type ActionPayload = FlashPayload;
    type ErrorCode = CoreErrorCode;
    type Command = AnyCommand<FlashPayload>;

    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError> {
        CONNECT_CALLS.fetch_add(1, Ordering::Relaxed);
        self.mode = message.platform.clone();
        self.connect_count += 1;
        self.poll_count = 0;
        self.release_scenario = ReleaseScenario::from_connect(&message);
        if let Some(scenario) = self.release_scenario {
            self.session_id = SessionId::new_v4();
            return Ok(scenario.connect_response(self.session_id));
        }
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
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        SUBMIT_CALLS.fetch_add(1, Ordering::Relaxed);
        if let Some(scenario) = self.release_scenario {
            return Ok(scenario.submit_response(self.session_id, message));
        }
        if self.mode == "panic-submit" {
            panic!("fixture submit panic");
        }
        Ok(Response::new(self.session_id, Vec::new()))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        if let Some(scenario) = self.release_scenario {
            let result =
                scenario.poll_response(self.session_id, self.connect_count, self.poll_count);
            self.poll_count += 1;
            return result;
        }
        if self.mode == "panic-poll" {
            panic!("fixture poll panic");
        }
        if self.mode == "poll-response" {
            return Ok(Some(Response::new(self.session_id, Vec::new())));
        }
        Ok(None)
    }
}

fn sized_response(session_id: SessionId, target: usize) -> Response<AnyCommand<FlashPayload>> {
    let mut payload = "x".repeat(target);
    loop {
        let response = Response::new(
            session_id,
            vec![ResponseMessage::Batch(Batch::new(
                BatchId::new_v4(),
                session_id,
                vec![ParallelCommandGroup::new(vec![AnyCommand::Core(
                    Command::new(
                        CommandId::new_v4(),
                        CommandBody::TextSetContent(TextContentPayload {
                            object_id: release_scenarios::object_id(999),
                            text: payload,
                        }),
                    ),
                )])],
            ))],
        );
        let length = messagepack::to_vec(&response).unwrap().len();
        if length == target {
            return response;
        }

        let command = match &response.messages[0] {
            ResponseMessage::Batch(batch) => batch.groups[0].commands[0].clone(),
            ResponseMessage::Snapshot(_) => unreachable!(),
        };
        payload = match command {
            AnyCommand::Core(Command {
                body: CommandBody::TextSetContent(text),
                ..
            }) => {
                let new_length = text.text.len() + target - length;
                "x".repeat(new_length)
            }
            _ => unreachable!(),
        };
    }
}

/// Creates a fresh engine with no active session.
pub fn create_engine() -> Result<FixtureEngine, EngineError> {
    match std::env::var("BATTLEMENT_EXPORT_FIXTURE_CREATE").as_deref() {
        Ok("panic") => panic!("fixture create panic"),
        Ok("error") => Err(EngineError::new("fixture create error")),
        _ => Ok(FixtureEngine {
            mode: String::new(),
            session_id: SessionId::new_v4(),
            release_scenario: None,
            connect_count: 0,
            poll_count: 0,
        }),
    }
}

battlement_native::export_engine!(create_engine);

#[unsafe(no_mangle)]
/// Returns the fixture adapter's live output allocation count.
pub extern "C" fn fixture_outstanding_buffers() -> usize {
    battlement_native::outstanding_buffer_count()
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
