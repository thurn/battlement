use std::collections::VecDeque;

use battlement::{BatchId, Command};
use battlement_native::{
  ConnectView, CoreClientMessageView, Engine, EngineError, EngineResponse, FlatBufferSubmitError,
  MessageWriter, NativeBatchStart, UiEventActionView, UiEventResult,
};

pub struct BatchEngine<E> {
  engine: E,
  session: [u8; 16],
  batches: VecDeque<Vec<Vec<Command>>>,
  pub failures: Vec<(bool, battlement::CommandId)>,
}

impl<E> BatchEngine<E> {
  pub fn new(engine: E) -> Self {
    Self {
      engine,
      session: [0; 16],
      batches: VecDeque::new(),
      failures: Vec::new(),
    }
  }
  pub fn queue(&mut self, groups: Vec<Vec<Command>>) {
    self.batches.push_back(groups);
  }
}

impl<E: Engine> Engine for BatchEngine<E> {
  const WIRE_DIGEST_C: &'static [u8; 65] = E::WIRE_DIGEST_C;
  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let response = self.engine.connect(message)?;
    self.session = response.session_id();
    Ok(response)
  }
  fn submit(&mut self, message: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    match CoreClientMessageView::read(message).expect("verified host submission") {
      CoreClientMessageView::BatchFailed(failure) => self.failures.push((
        true,
        battlement::CommandId::from_bytes(failure.command_id().expect("attributed motion failure"))
          .expect("valid command identity"),
      )),
      CoreClientMessageView::OperationFailed(failure) => self.failures.push((
        false,
        battlement::CommandId::from_bytes(failure.command_id()).expect("valid command identity"),
      )),
      CoreClientMessageView::Action(_) => {}
    }
    self.engine.submit(message)
  }
  fn submit_ui_event(
    &mut self,
    event: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    self.engine.submit_ui_event(event)
  }
  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    let Some(groups) = self.batches.pop_front() else {
      return self.engine.poll();
    };
    let mut writer = MessageWriter::default();
    let groups = groups
      .iter()
      .map(|group| {
        let commands = group
          .iter()
          .map(|command| writer.command(command).expect("valid fixture command"))
          .collect::<Vec<_>>();
        writer
          .parallel_group(&commands)
          .expect("valid fixture group")
      })
      .collect::<Vec<_>>();
    let batch = writer
      .batch(
        *BatchId::new_v4().as_uuid().as_bytes(),
        self.session,
        None,
        NativeBatchStart::Now,
        &groups,
      )
      .expect("valid fixture batch");
    let message = writer
      .finish(self.session, &[batch])
      .expect("verified fixture response");
    EngineResponse::from_core(self.session, message).map(Some)
  }
}
