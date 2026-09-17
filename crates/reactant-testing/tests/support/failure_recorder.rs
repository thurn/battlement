use std::ops::{Deref, DerefMut};

use battlement_native::{
  ConnectView, CoreClientMessageView, Engine, EngineError, EngineResponse, FlatBufferSubmitError,
  UiEventActionView, UiEventResult,
};

pub struct FailureRecorder<E> {
  engine: E,
  failure: Option<Vec<u8>>,
  replay: Option<Vec<u8>>,
}

impl<E> FailureRecorder<E> {
  pub fn new(engine: E) -> Self {
    Self {
      engine,
      failure: None,
      replay: None,
    }
  }
  pub fn replay_failure(&mut self) {
    self.replay = Some(
      self
        .failure
        .clone()
        .expect("a real host failure was recorded"),
    );
  }
}
impl<E> Deref for FailureRecorder<E> {
  type Target = E;
  fn deref(&self) -> &E {
    &self.engine
  }
}
impl<E> DerefMut for FailureRecorder<E> {
  fn deref_mut(&mut self) -> &mut E {
    &mut self.engine
  }
}
impl<E: Engine> Engine for FailureRecorder<E> {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = E::WIRE_CONTRACT_DIGEST_C;
  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    self.engine.connect(message)
  }
  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    if matches!(
      CoreClientMessageView::read(bytes),
      Ok(CoreClientMessageView::BatchFailed(_))
    ) {
      self.failure = Some(bytes.to_vec());
    }
    self.engine.submit(bytes)
  }
  fn submit_ui_event(
    &mut self,
    event: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    self.engine.submit_ui_event(event)
  }
  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    if let Some(bytes) = self.replay.take() {
      return Ok(Some(
        self
          .engine
          .submit(&bytes)
          .expect("valid recorded host failure"),
      ));
    }
    self.engine.poll()
  }
}
