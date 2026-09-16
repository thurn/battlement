use std::{mem, rc::Rc, thread};

use battlement::application::{ApplicationState, ReducedMotionPreference};
use battlement::{ActionId, ScreenSize, SessionId, Snapshot, UiEventDisposition};
use battlement_flatbuffers::{MessageWriter, NativeBatchStart};
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
    Ok(self.delivery.prepare(response))
  }

  fn submit_view_response(
    &mut self,
    message: CoreClientMessageView<'_>,
  ) -> Result<DeliveryResponse, EngineError> {
    let session = self
      .session
      .expect("connect before submitting application messages");
    let CoreClientMessageView::Action(action) = message else {
      return Ok(DeliveryResponse::empty(session));
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
    Ok(self.delivery.prepare(response))
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
      response: self.delivery.prepare(response),
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
    Ok(Some(self.delivery.prepare(response)))
  }
}

impl<G: 'static> Engine for App<G> {
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let response = self.connect_response(message)?;
    native_response(response)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let message = CoreClientMessageView::read(bytes).map_err(|error| {
      FlatBufferSubmitError::invalid_argument(format!("invalid client message: {error}"))
    })?;
    let response = self
      .submit_view_response(message)
      .map_err(FlatBufferSubmitError::engine)?;
    native_response(response).map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    let result = self.submit_ui_event_response(action)?;
    Ok(UiEventResult {
      disposition: result.disposition,
      response: native_response(result.response)?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    self.poll_response()?.map(native_response).transpose()
  }
}

fn native_response(response: DeliveryResponse) -> Result<EngineResponse, EngineError> {
  let session_id = *response.session_id.as_uuid().as_bytes();
  let mut writer = MessageWriter::default();
  let mut messages = Vec::with_capacity(response.messages.len());
  for message in &response.messages {
    let offset = match message {
      DeliveryMessage::Snapshot(snapshot) => writer.snapshot_message(snapshot),
      DeliveryMessage::Batch(batch) => {
        let mut groups = Vec::with_capacity(batch.groups.len());
        for group in &batch.groups {
          let commands = group
            .commands
            .iter()
            .map(|command| writer.command(command))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| EngineError::new(error.to_string()))?;
          groups.push(
            writer
              .parallel_group(&commands)
              .map_err(|error| EngineError::new(error.to_string()))?,
          );
        }
        writer.batch(
          *batch.batch_id.as_uuid().as_bytes(),
          *batch.session_id.as_uuid().as_bytes(),
          batch
            .caused_by_action_id
            .map(|value| *value.as_uuid().as_bytes()),
          match batch.start {
            battlement::BatchStart::Now => NativeBatchStart::Now,
            battlement::BatchStart::AfterEarlierBlockingWork => {
              NativeBatchStart::AfterEarlierBlockingWork
            }
            battlement::BatchStart::AfterEarlierAssetPreparation => {
              NativeBatchStart::AfterEarlierAssetPreparation
            }
          },
          &groups,
        )
      }
    }
    .map_err(|error| EngineError::new(error.to_string()))?;
    messages.push(offset);
  }
  let message = writer
    .finish(session_id, &messages)
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_core(session_id, message)
}

impl<G: 'static> App<G> {
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
    let (snapshot, commit) = self
      .runtime
      .begin_session(&mut self.model)
      .expect("application failed to render")
      .into_app_parts(snapshot);
    let mut response = DeliveryResponse::snapshot(snapshot);
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
    app_delivery::commands(response, commands);
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
