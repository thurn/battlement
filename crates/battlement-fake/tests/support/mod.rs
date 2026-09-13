use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use battlement::{ClientMessage, Command, Response, UiEventAction, UiEventResponse};
use battlement_native::{ConnectView, Engine, EngineError};

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
  type ActionPayload = ();
  type ErrorCode = ();
  type Command = Command;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<Response<Self::Command>, EngineError> {
    self.probe.borrow_mut().connects.push(RecordedConnect {
      platform: message.platform().to_owned(),
      unity_version: message.unity_version().to_owned(),
      screen_width: message.screen_width(),
      screen_height: message.screen_height(),
      modules: message.modules().map(str::to_owned).collect(),
    });
    self
      .connect_responses
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

  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Self::Command>, EngineError> {
    Ok(UiEventResponse::from_event(
      &action.event,
      Response::empty(action.session_id),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
    self.probe.borrow_mut().polls += 1;
    self
      .poll_responses
      .pop_front()
      .ok_or_else(|| EngineError::new("unexpected poll"))
  }
}
