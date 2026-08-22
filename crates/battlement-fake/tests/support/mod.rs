use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use battlement::{ClientMessage, Command, Connect, Response};
use battlement_native::{Engine, EngineError};

pub type SharedProbe = Rc<RefCell<Probe>>;

#[derive(Default)]
pub struct Probe {
    pub connects: Vec<Connect>,
    pub submits: Vec<ClientMessage<(), ()>>,
    pub polls: usize,
}

pub struct ScriptedEngine {
    pub probe: SharedProbe,
    connect_responses: VecDeque<Response>,
    submit_responses: VecDeque<(ClientMessage<(), ()>, Response)>,
    poll_responses: VecDeque<Option<Response>>,
}

impl ScriptedEngine {
    pub fn new(
        connect_responses: impl IntoIterator<Item = Response>,
        submit_responses: impl IntoIterator<Item = (ClientMessage<(), ()>, Response)>,
        poll_responses: impl IntoIterator<Item = Option<Response>>,
    ) -> Self {
        Self {
            probe: Rc::new(RefCell::new(Probe::default())),
            connect_responses: connect_responses.into_iter().collect(),
            submit_responses: submit_responses.into_iter().collect(),
            poll_responses: poll_responses.into_iter().collect(),
        }
    }
}

impl Engine for ScriptedEngine {
    type ActionPayload = ();
    type ErrorCode = ();
    type Command = Command;

    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.probe.borrow_mut().connects.push(message);
        self.connect_responses
            .pop_front()
            .ok_or_else(|| EngineError::new("unexpected connect"))
    }

    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        let (expected, response) = self
            .submit_responses
            .pop_front()
            .ok_or_else(|| EngineError::new("unexpected submit"))?;
        assert_eq!(message, expected);
        self.probe.borrow_mut().submits.push(message);
        Ok(response)
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        self.probe.borrow_mut().polls += 1;
        self.poll_responses
            .pop_front()
            .ok_or_else(|| EngineError::new("unexpected poll"))
    }
}
