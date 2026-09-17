use std::{mem, rc::Rc, thread};

use battlement::application::{ApplicationState, ReducedMotionPreference};
use battlement::{ActionId, ScreenSize, SessionId, Snapshot, UiEventDisposition};
use battlement_native::{
  ConnectView, CoreActionBodyView, CoreClientMessageView, Engine, EngineError, EngineResponse,
  FlatBufferSubmitError, UiEventActionView, UiEventResult,
};

use crate::{
  action_context,
  app::App,
  app_delivery::{self, DeliveryMessage, DeliveryResponse},
};

struct DeliveryUiEvent {
  disposition: UiEventDisposition,
  response: DeliveryResponse,
}

impl<G: 'static> App<G> {
  fn connect_response(
    &mut self,
    message: ConnectView<'_>,
  ) -> Result<DeliveryResponse, EngineError> {
    self.healthy = false;
    if self.session.is_none() {
      for root in &self.roots {
        root.register(
          &mut self.runtime,
          &self.observations,
          &self.queue,
          &self.orchestration,
        );
      }
    } else {
      self.runtime.resources.reset();
    }
    self.session = Some(SessionId::new_v4());
    {
      let mut queue = self.queue.borrow_mut();
      queue.generation = queue
        .generation
        .checked_add(1)
        .expect("session generation overflow");
      queue.commands.clear();
      queue.snapshot = false;
      queue.snapshot_action = None;
      queue.localizer = None;
      let mut observations = self.observations.borrow_mut();
      observations.application = ApplicationState {
        focused: message.focused(),
        paused: message.paused(),
      };
      observations.reduced_motion = match message.reduced_motion_preference().ordinal() {
        0 => ReducedMotionPreference::Unavailable,
        1 => ReducedMotionPreference::Reduce,
        2 => ReducedMotionPreference::NoPreference,
        _ => unreachable!("ConnectView validates reduced-motion ordinals"),
      };
      observations.screen = ScreenSize::new(message.screen_width(), message.screen_height());
      if self.reset {
        observations.remount = queue.generation;
      }
    }
    let response = self.snapshot();
    self.healthy = true;
    Ok(response)
  }

  fn submit_view_response(
    &mut self,
    message: CoreClientMessageView<'_>,
  ) -> Result<DeliveryResponse, EngineError> {
    let session = self
      .session
      .expect("connect before submitting application messages");
    let CoreClientMessageView::Action(action) = message else {
      if let CoreClientMessageView::BatchFailed(failure) = message
        && failure.session_id() == *session.as_uuid().as_bytes()
      {
        self.report_batch_failure(failure.batch_id(), failure.message());
      }
      // OperationFailed reports nonblocking cosmetic work; the host retains its diagnostic.
      let mut response = DeliveryResponse::empty(session);
      self.settle(&mut response, None, false);
      return Ok(response);
    };
    let session_id = SessionId::from_uuid(uuid::Uuid::from_bytes(action.session_id()))
      .expect("core view validates nonzero session UUIDs");
    self.check_session(session_id)?;
    let action_id = ActionId::from_uuid(uuid::Uuid::from_bytes(action.action_id()))
      .expect("core view validates nonzero action UUIDs");
    self.healthy = false;
    let _action = action_context::enter(Some(action_id));
    let commit = match action.body() {
      CoreActionBodyView::ReducedMotionPreferenceChanged(preference) => {
        self.observations.borrow_mut().reduced_motion = match preference.value() {
          0 => ReducedMotionPreference::Unavailable,
          1 => ReducedMotionPreference::Reduce,
          2 => ReducedMotionPreference::NoPreference,
          _ => unreachable!("core view validates reduced-motion preferences"),
        };
        self.runtime.refresh(&mut self.model)
      }
      CoreActionBodyView::ApplicationStateChanged(state) => {
        self.observations.borrow_mut().application = ApplicationState {
          focused: state.focused(),
          paused: state.paused(),
        };
        self.runtime.refresh(&mut self.model)
      }
      CoreActionBodyView::GeometryObservations(batch) => {
        self.runtime.observe_geometry_view(&mut self.model, batch)
      }
      CoreActionBodyView::MotionEvents(batch) => {
        self.runtime.motion_events_view(&mut self.model, batch)
      }
      _ => {
        self.healthy = true;
        return Ok(DeliveryResponse::empty(session));
      }
    }
    .expect("application observation failed to render");
    let mut response = DeliveryResponse::empty(session);
    app_delivery::append(&mut response, Some(action_id), commit);
    self.settle(&mut response, Some(action_id), false);
    self.healthy = true;
    Ok(response)
  }

  fn submit_ui_event_response(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<DeliveryUiEvent, EngineError> {
    let session_id = SessionId::from_uuid(uuid::Uuid::from_bytes(action.session_id()))
      .expect("UI event view validates nonzero session UUIDs");
    self.check_session(session_id)?;
    let action_id = ActionId::from_uuid(uuid::Uuid::from_bytes(action.action_id()))
      .expect("UI event view validates nonzero action UUIDs");
    self.healthy = false;
    let _action = action_context::enter(Some(action_id));
    let _callback_scope = self.orchestration.borrow().enter();
    let event = self
      .runtime
      .dispatch_view(&mut self.model, action)
      .expect("application event failed to render");
    let disposition = event.disposition();
    let mut response = DeliveryResponse::empty(session_id);
    app_delivery::append(&mut response, Some(action_id), event.into_commit());
    self.settle(&mut response, Some(action_id), false);
    self.healthy = true;
    Ok(DeliveryUiEvent {
      disposition,
      response,
    })
  }

  fn poll_response(&mut self) -> Result<Option<DeliveryResponse>, EngineError> {
    let Some(session) = self.session else {
      return Ok(None);
    };
    self.healthy = false;
    let mut response = DeliveryResponse::empty(session);
    self.settle(&mut response, None, true);
    self.healthy = true;
    if response.messages.is_empty() {
      return Ok(None);
    }
    Ok(Some(response))
  }
}

impl<G: 'static> Engine for App<G> {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let response = self.connect_response(message)?;
    self.deliver(response)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let message = CoreClientMessageView::read(bytes).map_err(|error| {
      FlatBufferSubmitError::invalid_argument(format!("invalid client message: {error}"))
    })?;
    let response = self
      .submit_view_response(message)
      .map_err(FlatBufferSubmitError::engine)?;
    self
      .deliver(response)
      .map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    let result = self.submit_ui_event_response(action)?;
    Ok(UiEventResult {
      disposition: result.disposition,
      response: self.deliver(result.response)?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    if self.output.can_consume()
      && let Some(runtime) = self.orchestration.borrow().runtime()
    {
      self.output.publication = runtime.take_output();
    }
    let Some(session) = self.session else {
      return Ok(None);
    };
    let response = self
      .poll_response()?
      .unwrap_or_else(|| DeliveryResponse::empty(session));
    self.finish_response(response)
  }
}

impl<G: 'static> App<G> {
  fn deliver(&mut self, response: DeliveryResponse) -> Result<EngineResponse, EngineError> {
    let session = *response.session_id.as_uuid().as_bytes();
    self
      .finish_response(response)?
      .map_or_else(|| EngineResponse::empty(session), Ok)
  }

  fn finish_response(
    &mut self,
    mut response: DeliveryResponse,
  ) -> Result<Option<EngineResponse>, EngineError> {
    let runtime = self.orchestration.borrow().runtime();
    let active = runtime.as_ref().and_then(|runtime| runtime.work_scope());
    if let Some(scope) = self.output.ending_scope(active)
      && let Some(batch) = self.runtime.recover_scope(scope, response.session_id)
    {
      response.messages.push(DeliveryMessage::Batch(batch));
    }
    let response = self.delivery.prepare(response);
    self.output.finish(response, runtime)
  }

  fn report_batch_failure(&self, batch: [u8; 16], message: &str) {
    if let (Some(scope), Some(runtime)) = (
      self.output.failed_batch(batch),
      self.orchestration.borrow().runtime(),
    ) {
      runtime.fail_work(scope, message.to_owned());
    }
  }

  fn check_session(&self, session: SessionId) -> Result<(), EngineError> {
    if self.session != Some(session) {
      return Err(EngineError::new("application event session mismatch"));
    }
    assert!(self.healthy, "application is poisoned");
    Ok(())
  }

  fn snapshot(&mut self) -> DeliveryResponse {
    let mut objects = vec![self.camera.clone()];
    objects.extend(self.objects.clone());
    objects.extend(self.roots.iter().map(|root| root.object()));
    let snapshot = Snapshot::new(
      self.session.expect("connected session"),
      Vec::new(),
      vec![self.scene.clone()],
      objects,
      self.camera.object_id,
    );
    let (mut snapshot, commit) = self
      .runtime
      .begin_session(&mut self.model)
      .expect("application failed to render")
      .into_app_parts(snapshot);
    let batches = self.runtime.extract_scoped_snapshot(&mut snapshot);
    let mut response = DeliveryResponse::snapshot(snapshot);
    response
      .messages
      .extend(batches.into_iter().map(DeliveryMessage::Batch));
    app_delivery::append(&mut response, None, commit);
    response
  }

  fn settle(&mut self, response: &mut DeliveryResponse, action: Option<ActionId>, poll: bool) {
    if self.orchestration.borrow().poll() {
      let commit = self
        .runtime
        .refresh(&mut self.model)
        .expect("application context failed to render");
      app_delivery::append(response, action, commit);
    }
    for pass in 0..2 {
      let requested =
        self.executor.has_ready() || !self.runtime.resources.operations.borrow().is_empty();
      if !(requested || (poll && pass == 0)) {
        break;
      }
      self.executor.tick();
      let origin = self.runtime.resources.next_action().unwrap_or(action);
      let _action = action_context::enter(origin);
      let commit = self
        .runtime
        .poll(&mut self.model)
        .expect("application work failed to render");
      app_delivery::append(response, origin, commit);
    }
    let (snapshot, snapshot_action, commands, localizer) = {
      let mut queue = self.queue.borrow_mut();
      (
        mem::take(&mut queue.snapshot),
        queue.snapshot_action.take(),
        mem::take(&mut queue.commands),
        queue.localizer.take(),
      )
    };
    if let Some(localizer) = localizer {
      let localizer = Rc::try_unwrap(localizer)
        .unwrap_or_else(|_| panic!("queued Reactant localizer is still borrowed"));
      let commit = self
        .runtime
        .replace_localizer(&mut self.model, localizer)
        .expect("localizer replacement failed to render");
      app_delivery::append(response, action, commit);
    }
    if snapshot {
      let imperative = app_delivery::take_imperative(response);
      *response = self.snapshot();
      for message in &mut response.messages {
        if let DeliveryMessage::Batch(batch) = message {
          batch.caused_by_action_id = snapshot_action;
        }
      }
      response.messages.extend(imperative);
    }
    app_delivery::commands(response, commands, self.runtime.track_work_scopes);
  }
}

impl<G: 'static> Drop for App<G> {
  fn drop(&mut self) {
    self.orchestration.borrow().stop();
    self.queue.borrow_mut().generation += 1;
    if self.healthy && !thread::panicking() {
      let _ = self.runtime.shutdown(&mut self.model).into_groups();
    }
  }
}
