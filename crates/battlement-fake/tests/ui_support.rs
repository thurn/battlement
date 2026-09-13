#![allow(dead_code)]

use battlement::{
  ActionId, ObjectId, Response, SessionId, UiEvent, UiEventAction, UiEventDisposition,
  UiEventResponse,
};
use battlement_native::{EngineError, EngineResponse, UiEventActionView, UiEventResult};
use uuid::Uuid;

pub fn encoded(response: Response) -> Result<EngineResponse, EngineError> {
  let session_id = *response.session_id.as_uuid().as_bytes();
  let message = battlement_flatbuffers::test_support::core_response(&response)
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_core(session_id, message)
}

pub fn action_id(value: UiEventActionView<'_>) -> ActionId {
  ActionId::from_uuid(Uuid::from_bytes(value.action_id())).expect("verified action UUID")
}

pub fn event(value: UiEventActionView<'_>) -> UiEvent {
  UiEvent::new(
    ObjectId::from_uuid(Uuid::from_bytes(value.target_id())).expect("verified object UUID"),
    value.cancelable(),
    value.default_prevented(),
    value.copy_body().expect("verified UI event body"),
  )
}

pub fn action(value: UiEventActionView<'_>) -> UiEventAction {
  let session_id =
    SessionId::from_uuid(Uuid::from_bytes(value.session_id())).expect("verified session UUID");
  UiEventAction::new(action_id(value), session_id, event(value))
}

pub fn from_owned(
  _value: UiEventActionView<'_>,
  response: UiEventResponse,
) -> Result<UiEventResult, EngineError> {
  Ok(UiEventResult {
    disposition: response.disposition,
    response: encoded(response.response)?,
  })
}

pub fn result(
  value: UiEventActionView<'_>,
  response: Response,
) -> Result<UiEventResult, EngineError> {
  Ok(UiEventResult {
    disposition: disposition(value),
    response: encoded(response)?,
  })
}

pub fn empty_result(
  value: UiEventActionView<'_>,
  session_id: SessionId,
) -> Result<UiEventResult, EngineError> {
  result(value, Response::empty(session_id))
}

pub fn disposition(value: UiEventActionView<'_>) -> UiEventDisposition {
  if value.default_prevented() {
    UiEventDisposition::PreventDefault
  } else {
    UiEventDisposition::Continue
  }
}
