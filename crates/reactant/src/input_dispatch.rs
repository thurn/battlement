use std::{
  cell::{Cell, RefCell},
  collections::HashMap,
  rc::Rc,
};

use battlement::{
  Command, CommandBody, InputCaptureCommand, InputCaptureEvent, InputCaptureRequest,
  InputCaptureResult,
};
use battlement::{
  ControllerButtonPayload, ControllerNavigationPayload, KeyPayload, ObjectId, Vector3,
};
use battlement_native::CoreActionBodyView;
use reactant_core::app_context::AppHandle;

use crate::input::{DragCallbacks, GlobalInput};

type GlobalInputHandler = Rc<dyn Fn(GlobalInput)>;

#[derive(Default)]
pub(crate) struct InputDispatch {
  global_input: RefCell<HashMap<String, GlobalInputHandler>>,
  drag: RefCell<HashMap<ObjectId, (u64, DragCallbacks)>>,
  next_input: Cell<u64>,
  capture: RefCell<Option<CaptureOwner>>,
}

struct CaptureOwner {
  request: InputCaptureRequest,
  app: AppHandle,
  handler: Rc<dyn Fn(InputCaptureResult)>,
}

impl InputDispatch {
  pub(crate) fn begin_capture(
    &self,
    request: InputCaptureRequest,
    app: AppHandle,
    handler: Rc<dyn Fn(InputCaptureResult)>,
  ) {
    assert!(
      self.capture.borrow().is_none(),
      "only one exclusive input capture may be mounted"
    );
    self.reset_global_input();
    app.send(Command::new_v4(CommandBody::InputCapture(
      InputCaptureCommand::Begin(request),
    )));
    *self.capture.borrow_mut() = Some(CaptureOwner {
      request,
      app,
      handler,
    });
  }

  pub(crate) fn end_capture(&self, id: ObjectId) {
    let mut owner = self.capture.borrow_mut();
    if owner.as_ref().is_some_and(|owner| owner.request.id == id) {
      let owner = owner.take().expect("matching capture owner");
      owner.app.send(Command::new_v4(CommandBody::InputCapture(
        InputCaptureCommand::End(id),
      )));
    }
  }

  fn complete_capture(&self, event: InputCaptureEvent) {
    let handler = self
      .capture
      .borrow()
      .as_ref()
      .filter(|owner| owner.request.id == event.id)
      .map(|owner| owner.handler.clone());
    if let Some(handler) = handler {
      self.end_capture(event.id);
      self.reset_global_input();
      handler(event.result);
    }
  }

  pub(crate) fn reset_global_input(&self) {
    let handlers = self
      .global_input
      .borrow()
      .values()
      .cloned()
      .collect::<Vec<_>>();
    for handler in handlers {
      handler(GlobalInput::Reset);
    }
  }

  pub(crate) fn clear(&self) {
    self.capture.borrow_mut().take();
    self.global_input.borrow_mut().clear();
    self.drag.borrow_mut().clear();
  }

  pub(crate) fn register_global_input(&self, identity: String, handler: GlobalInputHandler) {
    self.global_input.borrow_mut().insert(identity, handler);
  }

  pub(crate) fn unregister_global_input(&self, identity: &str) {
    self.global_input.borrow_mut().remove(identity);
  }

  pub(crate) fn register_drag(&self, object: ObjectId, callbacks: DragCallbacks) -> u64 {
    let token = self
      .next_input
      .get()
      .checked_add(1)
      .expect("input identity overflow");
    self.next_input.set(token);
    self.drag.borrow_mut().insert(object, (token, callbacks));
    token
  }

  pub(crate) fn unregister_drag(&self, object: ObjectId, token: u64) {
    let mut drag = self.drag.borrow_mut();
    if drag
      .get(&object)
      .is_some_and(|registered| registered.0 == token)
    {
      drag.remove(&object);
    }
  }

  pub(crate) fn dispatch_core(&self, body: CoreActionBodyView<'_>) {
    let global = match body {
      CoreActionBodyView::InputCaptured(value) => {
        self.complete_capture(value);
        None
      }
      CoreActionBodyView::KeyDown(value) => Some(GlobalInput::KeyDown(KeyPayload {
        key: value.physical_key(),
      })),
      CoreActionBodyView::KeyUp(value) => Some(GlobalInput::KeyUp(KeyPayload {
        key: value.physical_key(),
      })),
      CoreActionBodyView::ControllerButtonDown(value) => {
        Some(GlobalInput::ControllerButtonDown(ControllerButtonPayload {
          controller_id: value.controller_id(),
          button: value.controller_button(),
        }))
      }
      CoreActionBodyView::ControllerButtonUp(value) => {
        Some(GlobalInput::ControllerButtonUp(ControllerButtonPayload {
          controller_id: value.controller_id(),
          button: value.controller_button(),
        }))
      }
      CoreActionBodyView::ControllerNavigate(value) => Some(GlobalInput::ControllerNavigate(
        ControllerNavigationPayload {
          controller_id: value.controller_id(),
          direction: value.controller_direction(),
          source: value.navigation_source(),
          repeat: value.is_repeat(),
        },
      )),
      CoreActionBodyView::DragStart(value) => {
        let object = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        let callback = self
          .drag
          .borrow()
          .get(&object)
          .and_then(|entry| entry.1.start.clone());
        if let Some(callback) = callback {
          callback();
        }
        None
      }
      CoreActionBodyView::DragEnd(value) => {
        let object = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        let callback = self
          .drag
          .borrow()
          .get(&object)
          .and_then(|entry| entry.1.end.clone());
        if let Some(callback) = callback {
          let [x, y, z] = value.world_position();
          callback(Vector3::new(x, y, z));
        }
        None
      }
      _ => None,
    };
    if let Some(input) = global
      && self.capture.borrow().is_none()
    {
      let handlers = self
        .global_input
        .borrow()
        .values()
        .cloned()
        .collect::<Vec<_>>();
      for handler in handlers {
        handler(input);
      }
    }
  }
}
