#![allow(dead_code)]

use reactant_core::{executor::Spawner, runtime::Reactant};

pub fn reactant<G>(spawner: impl Spawner) -> Reactant<G> {
  Reactant::new(spawner)
}

pub fn encoded(
  response: battlement::Response,
) -> Result<battlement_native::EngineResponse, battlement_native::EngineError> {
  let session = *response.session_id.as_uuid().as_bytes();
  let message = battlement_flatbuffers::test_support::core_response(&response)
    .map_err(|error| battlement_native::EngineError::new(error.to_string()))?;
  battlement_native::EngineResponse::from_core(session, message)
}

pub fn empty_ui_result(
  message: battlement_native::UiEventActionView<'_>,
) -> Result<battlement_native::UiEventResult, battlement_native::EngineError> {
  let session_id = battlement::SessionId::from_uuid(uuid::Uuid::from_bytes(message.session_id()))
    .expect("verified session");
  Ok(battlement_native::UiEventResult {
    disposition: if message.default_prevented() {
      battlement::UiEventDisposition::PreventDefault
    } else {
      battlement::UiEventDisposition::Continue
    },
    response: encoded(battlement::Response::empty(session_id))?,
  })
}
