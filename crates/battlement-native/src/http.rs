use battlement::{ClientMessage, Connect, UiEventAction, UiEventDisposition, json};

use crate::Engine;

/// Required response header carrying an immediate UI-event disposition.
pub const UI_EVENT_DISPOSITION_HEADER: &str = "Battlement-UI-Event-Disposition";

/// One transport-neutral response produced by the localhost HTTP router.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
  /// HTTP status code.
  pub status: u16,
  /// Response media type.
  pub content_type: &'static str,
  /// Optional immediate UI-event disposition header value.
  pub ui_event_disposition: Option<&'static str>,
  /// Response body bytes.
  pub body: Vec<u8>,
}

/// Stateful localhost route handler for one rules engine.
pub struct HttpEngine<E> {
  engine: E,
}

impl<E> HttpEngine<E>
where
  E: Engine,
{
  /// Wraps one engine with the synchronous localhost route contract.
  pub const fn new(engine: E) -> Self {
    Self { engine }
  }

  /// Processes one complete request without retrying or spawning work.
  pub fn handle(&mut self, method: &str, path: &str, body: &[u8]) -> HttpResponse {
    match (method, path) {
      ("POST", "/connect") => self.connect(body),
      ("POST", "/messages") => self.submit(body),
      ("POST", "/ui-events") => self.submit_ui_event(body),
      ("GET", "/poll") if body.is_empty() => self.poll(),
      ("GET", "/poll") => HttpResponse::invalid("poll does not accept a request body"),
      _ => HttpResponse::not_found(),
    }
  }

  /// Returns the hosted engine.
  pub fn into_inner(self) -> E {
    self.engine
  }

  fn connect(&mut self, body: &[u8]) -> HttpResponse {
    let message = match json::from_slice::<Connect>(body) {
      Ok(message) => message,
      Err(error) => return HttpResponse::invalid(error.to_string()),
    };
    match self.engine.connect(message) {
      Ok(response) => HttpResponse::serialized(&response),
      Err(error) => HttpResponse::engine(error.to_string()),
    }
  }

  fn submit(&mut self, body: &[u8]) -> HttpResponse {
    let message = match json::from_slice::<ClientMessage<E::ActionPayload, E::ErrorCode>>(body) {
      Ok(message) => message,
      Err(error) => return HttpResponse::invalid(error.to_string()),
    };
    match self.engine.submit(message) {
      Ok(response) => HttpResponse::serialized(&response),
      Err(error) => HttpResponse::engine(error.to_string()),
    }
  }

  fn submit_ui_event(&mut self, body: &[u8]) -> HttpResponse {
    let action = match json::from_slice::<UiEventAction>(body) {
      Ok(action) => action,
      Err(error) => return HttpResponse::invalid(error.to_string()),
    };
    match self.engine.submit_ui_event(action) {
      Ok(response) => HttpResponse::ui_event(response.disposition, &response.response),
      Err(error) => HttpResponse::engine(error.to_string()),
    }
  }

  fn poll(&mut self) -> HttpResponse {
    match self.engine.poll() {
      Ok(Some(response)) => HttpResponse::serialized(&response),
      Ok(None) => HttpResponse::no_content(),
      Err(error) => HttpResponse::engine(error.to_string()),
    }
  }
}

impl HttpResponse {
  fn serialized(value: &impl serde::Serialize) -> Self {
    match json::to_vec(value) {
      Ok(body) => Self {
        status: 200,
        content_type: "application/json",
        ui_event_disposition: None,
        body,
      },
      Err(error) => Self::engine(error.to_string()),
    }
  }

  fn ui_event(disposition: UiEventDisposition, value: &impl serde::Serialize) -> Self {
    let mut response = Self::serialized(value);
    if response.status == 200 {
      response.ui_event_disposition = Some(match disposition {
        UiEventDisposition::Continue => "0",
        UiEventDisposition::PreventDefault => "1",
      });
    }
    response
  }

  fn invalid(message: impl Into<String>) -> Self {
    Self::diagnostic(400, message)
  }

  fn engine(message: impl Into<String>) -> Self {
    Self::diagnostic(500, message)
  }

  fn diagnostic(status: u16, message: impl Into<String>) -> Self {
    Self {
      status,
      content_type: "text/plain; charset=utf-8",
      ui_event_disposition: None,
      body: message.into().into_bytes(),
    }
  }

  fn no_content() -> Self {
    Self {
      status: 204,
      content_type: "application/json",
      ui_event_disposition: None,
      body: Vec::new(),
    }
  }

  fn not_found() -> Self {
    Self::diagnostic(404, "unknown Battlement route")
  }
}
