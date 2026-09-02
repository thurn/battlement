use std::{mem, thread};

use battlement::{
  ActionBody, ActionId, ClientMessage, Command, Connect, CoreErrorCode, Response, SessionId,
  Snapshot, UiEventAction, UiEventResponse,
};
use battlement_native::{Engine, EngineError};

use crate::{action_context, app::App, app_delivery};

impl<G: 'static> Engine for App<G> {
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, message: Connect) -> Result<Response, EngineError> {
    self.healthy = false;
    if self.session.is_none() {
      for root in &self.roots {
        root.register(&mut self.runtime, &self.observations, &self.queue);
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
      let mut observations = self.observations.borrow_mut();
      observations.application = message.application_state;
      observations.screen = message.screen;
      if self.reset {
        observations.remount = queue.generation;
      }
    }
    let response = self.snapshot();
    self.healthy = true;
    Ok(self.delivery.prepare(response))
  }

  fn submit(&mut self, message: ClientMessage<()>) -> Result<Response, EngineError> {
    let session = self
      .session
      .expect("connect before submitting application messages");
    let Some(action) = message.into_action() else {
      return Ok(Response::empty(session));
    };
    self.check_session(action.session_id)?;
    self.healthy = false;
    let _action = action_context::enter(Some(action.action_id));
    let commit = match action.body {
      ActionBody::ApplicationStateChanged(state) => {
        self.observations.borrow_mut().application = state;
        self.runtime.refresh(&mut self.model)
      }
      ActionBody::GeometryObservations(batch) => {
        self.runtime.observe_geometry(&mut self.model, batch)
      }
      ActionBody::MotionEvents(batch) => self.runtime.motion_events(&mut self.model, batch),
      _ => {
        self.healthy = true;
        return Ok(Response::empty(session));
      }
    }
    .expect("application observation failed to render");
    let mut response = Response::empty(session);
    app_delivery::append(&mut response, Some(action.action_id), commit);
    self.settle(&mut response, Some(action.action_id), false);
    self.healthy = true;
    Ok(self.delivery.prepare(response))
  }

  fn submit_ui_event(&mut self, action: UiEventAction) -> Result<UiEventResponse, EngineError> {
    self.check_session(action.session_id)?;
    self.healthy = false;
    let _action = action_context::enter(Some(action.action_id));
    let event = self
      .runtime
      .dispatch(&mut self.model, action.event)
      .expect("application event failed to render");
    let disposition = event.disposition();
    let mut response = Response::empty(action.session_id);
    app_delivery::append(&mut response, Some(action.action_id), event.into_commit());
    self.settle(&mut response, Some(action.action_id), false);
    self.healthy = true;
    Ok(UiEventResponse::new(
      disposition,
      self.delivery.prepare(response),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    let Some(session) = self.session else {
      return Ok(None);
    };
    self.healthy = false;
    let mut response = Response::empty(session);
    self.settle(&mut response, None, true);
    self.healthy = true;
    if response.messages.is_empty() {
      return Ok(None);
    }
    Ok(Some(self.delivery.prepare(response)))
  }
}

impl<G: 'static> App<G> {
  fn check_session(&self, session: SessionId) -> Result<(), EngineError> {
    if self.session != Some(session) {
      return Err(EngineError::new("application event session mismatch"));
    }
    assert!(self.healthy, "application is poisoned");
    Ok(())
  }

  fn snapshot(&mut self) -> Response {
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
    self
      .runtime
      .begin_session(&mut self.model)
      .expect("application failed to render")
      .into_app_response(snapshot)
  }

  fn settle(&mut self, response: &mut Response, action: Option<ActionId>, poll: bool) {
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
    let (snapshot, snapshot_action, commands) = {
      let mut queue = self.queue.borrow_mut();
      (
        mem::take(&mut queue.snapshot),
        queue.snapshot_action.take(),
        mem::take(&mut queue.commands),
      )
    };
    if snapshot {
      let imperative = app_delivery::take_imperative(response);
      *response = self.snapshot();
      for message in &mut response.messages {
        if let battlement::ResponseMessage::Batch(batch) = message {
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
    self.queue.borrow_mut().generation += 1;
    if self.healthy && !thread::panicking() {
      let _ = self.runtime.shutdown(&mut self.model).into_groups();
    }
  }
}
