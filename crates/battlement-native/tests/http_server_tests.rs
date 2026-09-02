use battlement::{
  ActionId, Batch, BatchId, ClickEvent, ClientMessage, Command, Connect, CoreErrorCode, ObjectId,
  Response, ResponseMessage, SessionId, UiEvent, UiEventAction, UiEventBody, UiEventDisposition,
  UiEventResponse, json,
};
use battlement_native::{Engine, EngineError, http::HttpEngine};

struct Fixture {
  session_id: SessionId,
  fail: bool,
}

impl Engine for Fixture {
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, _message: Connect) -> Result<Response<Command>, EngineError> {
    self.response()
  }

  fn submit(
    &mut self,
    _message: ClientMessage<(), CoreErrorCode>,
  ) -> Result<Response<Command>, EngineError> {
    self.response()
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventAction,
  ) -> Result<UiEventResponse<Command>, EngineError> {
    if self.fail {
      return Err(EngineError::new("fixture failed"));
    }
    Ok(UiEventResponse::new(
      UiEventDisposition::PreventDefault,
      Response {
        session_id: self.session_id,
        messages: vec![ResponseMessage::Batch(Batch {
          batch_id: BatchId::new_v4(),
          session_id: self.session_id,
          caused_by_action_id: Some(action.action_id),
          start: Default::default(),
          groups: Vec::new(),
        })],
      },
    ))
  }

  fn poll(&mut self) -> Result<Option<Response<Command>>, EngineError> {
    self.response().map(Some)
  }
}

impl Fixture {
  fn response(&self) -> Result<Response<Command>, EngineError> {
    if self.fail {
      Err(EngineError::new("fixture failed"))
    } else {
      Ok(Response {
        session_id: self.session_id,
        messages: Vec::new(),
      })
    }
  }
}

#[test]
fn routes_ui_events_with_the_required_header_and_ordinary_body() {
  let session_id = SessionId::new_v4();
  let action = action(session_id);
  let action_id = action.action_id;
  let mut server = HttpEngine::new(Fixture {
    session_id,
    fail: false,
  });
  let response = server.handle("POST", "/ui-events", &json::to_vec(&action).unwrap());

  assert_eq!(response.status, 200);
  assert_eq!(response.ui_event_disposition, Some("1"));
  let decoded: Response<Command> = json::from_slice(&response.body).unwrap();
  assert_eq!(decoded.session_id, session_id);
  let ResponseMessage::Batch(batch) = &decoded.messages[0] else {
    panic!("UI event response should preserve the ordinary response body");
  };
  assert_eq!(batch.caused_by_action_id, Some(action_id));
}

#[test]
fn invalid_and_engine_failures_never_include_a_disposition() {
  let mut invalid_server = HttpEngine::new(Fixture {
    session_id: SessionId::new_v4(),
    fail: false,
  });
  let invalid = invalid_server.handle("POST", "/ui-events", b"not json");
  assert_eq!(invalid.status, 400);
  assert_eq!(invalid.ui_event_disposition, None);

  let session_id = SessionId::new_v4();
  let mut failed_server = HttpEngine::new(Fixture {
    session_id,
    fail: true,
  });
  let failed = failed_server.handle(
    "POST",
    "/ui-events",
    &json::to_vec(&action(session_id)).unwrap(),
  );
  assert_eq!(failed.status, 500);
  assert_eq!(failed.ui_event_disposition, None);
}

fn action(session_id: SessionId) -> UiEventAction {
  UiEventAction {
    action_id: ActionId::new_v4(),
    session_id,
    event: UiEvent::new(
      ObjectId::new_v4(),
      true,
      false,
      UiEventBody::Click(ClickEvent::NavigationSubmit),
    ),
  }
}
