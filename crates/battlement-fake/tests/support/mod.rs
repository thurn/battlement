#![allow(dead_code)]

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use battlement::{ClientMessage, Response, UiEventDisposition};
use battlement_native::{
  ConnectView, Engine, EngineError, EngineResponse, FlatBufferSubmitError, UiEventActionView,
  UiEventResult,
};

pub type SharedProbe = Rc<RefCell<Probe>>;

#[derive(Default)]
#[allow(dead_code)]
pub struct RecordedConnect {
  pub platform: String,
  pub unity_version: String,
  pub screen_width: u32,
  pub screen_height: u32,
  pub modules: Vec<String>,
}

#[derive(Default)]
pub struct Probe {
  pub connects: Vec<RecordedConnect>,
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
  const WIRE_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    self.probe.borrow_mut().connects.push(RecordedConnect {
      platform: message.platform().to_owned(),
      unity_version: message.unity_version().to_owned(),
      screen_width: message.screen_width(),
      screen_height: message.screen_height(),
      modules: message.modules().map(str::to_owned).collect(),
    });
    let response = self
      .connect_responses
      .pop_front()
      .ok_or_else(|| EngineError::new("unexpected connect"))?;
    encoded(response)
  }

  fn submit(&mut self, message: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let (expected, response) = self
      .submit_responses
      .pop_front()
      .ok_or_else(|| FlatBufferSubmitError::engine(EngineError::new("unexpected submit")))?;
    let expected_bytes = match &expected {
      ClientMessage::Action(value) => battlement_flatbuffers::write_core_action(value),
      ClientMessage::CustomAction(_) => panic!("core fake does not submit custom actions"),
      ClientMessage::BatchFailed(_) | ClientMessage::OperationFailed(_) => {
        panic!("this script only records built-in actions")
      }
    }
    .map_err(|error| FlatBufferSubmitError::engine(EngineError::new(error.to_string())))?;
    assert_eq!(message, expected_bytes.as_bytes());
    self.probe.borrow_mut().submits.push(expected);
    encoded(response).map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    let session = battlement::SessionId::from_uuid(uuid::Uuid::from_bytes(action.session_id()))
      .expect("verified session");
    Ok(UiEventResult {
      disposition: if action.default_prevented() {
        UiEventDisposition::PreventDefault
      } else {
        UiEventDisposition::Continue
      },
      response: encoded(Response::empty(session))?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    self.probe.borrow_mut().polls += 1;
    self
      .poll_responses
      .pop_front()
      .ok_or_else(|| EngineError::new("unexpected poll"))?
      .map(encoded)
      .transpose()
  }
}

pub fn encoded(response: Response) -> Result<EngineResponse, EngineError> {
  let session = *response.session_id.as_uuid().as_bytes();
  let message = battlement_flatbuffers::test_support::core_response(&response)
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_core(session, message)
}

pub fn encoded_unchecked(response: Response) -> Result<EngineResponse, EngineError> {
  let session = *response.session_id.as_uuid().as_bytes();
  let message = battlement_flatbuffers::test_support::unchecked_core_response(&response)
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_core(session, message)
}
