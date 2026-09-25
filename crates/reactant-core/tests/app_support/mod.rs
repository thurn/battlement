#![allow(dead_code)]

use battlement::{
  ClickEvent, ClientMessage, Connect, CoreErrorCode, KeyModifiers, ObjectId, PanelPoint,
  PointerButton, Response, ScreenSize, UiEvent, UiEventAction, UiEventDisposition,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

pub fn catalog() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("app/content");
  assets
}

pub fn connect() -> Connect {
  Connect::new("test", "test", ScreenSize::new(800, 600))
}

pub fn click(id: ObjectId) -> UiEvent {
  UiEvent::click(
    id,
    ClickEvent::pointer(
      0,
      PanelPoint::default(),
      PointerButton::Left,
      1,
      KeyModifiers::default(),
    ),
  )
}

pub fn named<E: Engine>(client: &mut FakeClient<E>, root: ObjectId, name: &str) -> ObjectId {
  let ui = client.ui();
  let mut pending = vec![root];
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing element {name}");
}

pub fn text<E: Engine>(client: &mut FakeClient<E>, root: ObjectId, name: &str) -> String {
  let id = self::named(client, root, name);
  client.ui().element(id).text().unwrap().to_owned()
}

pub struct UiEventTestResult {
  pub disposition: UiEventDisposition,
  pub response: Response,
}

pub trait EngineTestExt: Engine {
  fn connect_test(&mut self, value: &Connect) -> Result<Response, EngineError> {
    let bytes = battlement_flatbuffers::write_connect_request(value)
      .map_err(|error| EngineError::new(error.to_string()))?;
    let view = battlement_flatbuffers::ConnectView::read(bytes.as_bytes())
      .map_err(|error| EngineError::new(error.to_string()))?;
    let response = self.connect(view)?;
    battlement_fake::read_response(response.as_bytes()).map_err(EngineError::new)
  }

  fn submit_test(
    &mut self,
    value: ClientMessage<(), CoreErrorCode>,
  ) -> Result<Response, EngineError> {
    let bytes = match value {
      ClientMessage::Action(value) => battlement_flatbuffers::write_core_action(&value),
      ClientMessage::BatchCompleted(value) => {
        battlement_flatbuffers::write_core_batch_completed(&value)
      }
      ClientMessage::BatchFailed(value) => battlement_flatbuffers::write_core_batch_failure(&value),
      ClientMessage::OperationFailed(value) => {
        battlement_flatbuffers::write_core_operation_failure(&value)
      }
      ClientMessage::CustomAction(_) => return Err(EngineError::new("custom action unsupported")),
    }
    .map_err(|error| EngineError::new(error.to_string()))?;
    let response = self
      .submit(bytes.as_bytes())
      .map_err(|error| EngineError::new(error.to_string()))?;
    battlement_fake::read_response(response.as_bytes()).map_err(EngineError::new)
  }

  fn submit_ui_event_test(
    &mut self,
    value: UiEventAction,
  ) -> Result<UiEventTestResult, EngineError> {
    let bytes = battlement_flatbuffers::write_ui_event_action(&value)
      .map_err(|error| EngineError::new(error.to_string()))?;
    let view = battlement_flatbuffers::UiEventActionView::read(bytes.as_bytes())
      .map_err(|error| EngineError::new(error.to_string()))?;
    let result = self.submit_ui_event(view)?;
    Ok(UiEventTestResult {
      disposition: result.disposition,
      response: battlement_fake::read_response(result.response.as_bytes())
        .map_err(EngineError::new)?,
    })
  }

  fn poll_test(&mut self) -> Result<Option<Response>, EngineError> {
    self
      .poll()?
      .map(|response| battlement_fake::read_response(response.as_bytes()).map_err(EngineError::new))
      .transpose()
  }
}

impl<E: Engine> EngineTestExt for E {}
