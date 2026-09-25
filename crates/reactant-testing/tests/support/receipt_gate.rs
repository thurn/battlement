use std::{
  collections::VecDeque,
  ops::{Deref, DerefMut},
  time::Duration,
};

use battlement::{BatchFailed, CoreErrorCode};
use battlement_native::{
  ConnectView, CoreClientMessageView, Engine, EngineError, EngineResponse, FlatBufferSubmitError,
  MessageWriter, UiEventActionView, UiEventResult,
};
use reactant::ApplicationEngine;

pub struct ReceiptGate {
  engine: ApplicationEngine,
  session: [u8; 16],
  pub hold: bool,
  pub receipts: Vec<Vec<u8>>,
  replay: VecDeque<Vec<u8>>,
}
impl ReceiptGate {
  pub fn new(engine: ApplicationEngine) -> Self {
    Self {
      engine,
      session: [0; 16],
      hold: false,
      receipts: Vec::new(),
      replay: VecDeque::new(),
    }
  }
  pub fn replay(&mut self, bytes: Vec<u8>) {
    self.replay.push_back(bytes);
  }
  pub fn release_all(&mut self) {
    self.replay.extend(self.receipts.clone());
  }
  pub fn fail_last(&mut self) {
    let CoreClientMessageView::BatchCompleted(completed) =
      CoreClientMessageView::read(self.receipts.last().expect("host completion")).unwrap()
    else {
      unreachable!()
    };
    self.replay.push_back(
      battlement_flatbuffers::write_core_batch_failure(&BatchFailed::new(
        completed.session_id,
        completed.batch_id,
        None,
        CoreErrorCode::HandlerFailed,
        "injected host completion failure",
      ))
      .unwrap()
      .as_bytes()
      .to_vec(),
    );
  }
  fn empty(&self) -> EngineResponse {
    let writer = MessageWriter::default();
    let bytes = writer.finish(self.session, &[]).unwrap();
    EngineResponse::from_core(self.session, bytes).unwrap()
  }
}
impl Deref for ReceiptGate {
  type Target = ApplicationEngine;
  fn deref(&self) -> &Self::Target {
    &self.engine
  }
}
impl DerefMut for ReceiptGate {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.engine
  }
}
impl Engine for ReceiptGate {
  const WIRE_DIGEST_C: &'static [u8; 65] = ApplicationEngine::WIRE_DIGEST_C;
  fn connect(&mut self, value: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let response = self.engine.connect(value)?;
    self.session = response.session_id();
    Ok(response)
  }
  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    if matches!(
      CoreClientMessageView::read(bytes),
      Ok(CoreClientMessageView::BatchCompleted(_))
    ) {
      self.receipts.push(bytes.to_vec());
      if self.hold {
        return Ok(self.empty());
      }
    }
    self.engine.submit(bytes)
  }
  fn submit_ui_event(
    &mut self,
    value: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    self.engine.submit_ui_event(value)
  }
  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    if let Some(bytes) = self.replay.pop_front() {
      return Ok(Some(
        self.engine.submit(&bytes).expect("valid recorded receipt"),
      ));
    }
    self.engine.poll()
  }
  fn set_time(&mut self, time: Duration) {
    self.engine.set_time(time);
  }
}
