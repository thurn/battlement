//! Mount-scoped application and native drag input.

use std::rc::Rc;

use battlement::{ControllerButtonPayload, ControllerNavigationPayload, KeyPayload, Vector3};
use reactant_core::{app_runtime::ApplicationContext, application, hooks};

use crate::{game_app::ServicesContext, host_settings};

/// An unclaimed application-wide keyboard or controller action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalInput {
  /// Clears held input after capture, subscription, device, or focus changes.
  Reset,
  /// A configured physical key was pressed.
  KeyDown(KeyPayload),
  /// A configured physical key was released.
  KeyUp(KeyPayload),
  /// A configured controller button was pressed.
  ControllerButtonDown(ControllerButtonPayload),
  /// A configured controller button was released.
  ControllerButtonUp(ControllerButtonPayload),
  /// The native controller repeat/arbitration path produced one direction.
  ControllerNavigate(ControllerNavigationPayload),
}

/// Subscribes for unclaimed global input while the component is mounted.
pub fn use_global_input(handler: impl Fn(GlobalInput) + 'static) {
  let application = application::use_application_state();
  let host = host_settings::use_host_settings();
  let devices = (host.keyboard_connected, host.controller_count);
  let previous_devices = hooks::use_ref(devices);
  let services = hooks::use_required_context::<ApplicationContext>()
    .value::<ServicesContext>()
    .expect("use_global_input requires a Reactant Application root");
  let coordinator = services
    .coordinator
    .upgrade()
    .expect("application runtime ended while rendering");
  let handler: Rc<dyn Fn(GlobalInput)> = Rc::new(handler);
  let current = hooks::use_ref(handler.clone());
  let next = handler.clone();
  let committed = current.clone();
  hooks::use_effect_always(move || {
    committed.replace(next);
  });
  let reset = current.clone();
  hooks::use_effect(
    move || {
      if !application.focused || application.paused {
        reset.with(|handler| handler(GlobalInput::Reset));
      }
    },
    application,
  );
  let reset = current.clone();
  hooks::use_effect(
    move || {
      let previous = previous_devices.with(|previous| *previous);
      previous_devices.replace(devices);
      if previous != devices {
        reset.with(|handler| handler(GlobalInput::Reset));
      }
    },
    devices,
  );
  let identity = hooks::use_id();
  hooks::use_effect(
    move || {
      let current = current.clone();
      coordinator.input.register_global_input(
        identity.clone(),
        Rc::new(move |input| {
          current.with(|handler| handler(input));
        }),
      );
      move || coordinator.input.unregister_global_input(&identity)
    },
    (),
  );
}

#[derive(Clone, Default)]
pub(crate) struct DragCallbacks {
  pub(crate) start: Option<Rc<dyn Fn()>>,
  pub(crate) end: Option<Rc<dyn Fn(Vector3)>>,
}

impl PartialEq for DragCallbacks {
  fn eq(&self, other: &Self) -> bool {
    same_start(&self.start, &other.start) && same_callback(&self.end, &other.end)
  }
}

fn same_start(left: &Option<Rc<dyn Fn()>>, right: &Option<Rc<dyn Fn()>>) -> bool {
  match (left, right) {
    (None, None) => true,
    (Some(left), Some(right)) => Rc::ptr_eq(left, right),
    _ => false,
  }
}

fn same_callback<A>(left: &Option<Rc<dyn Fn(A)>>, right: &Option<Rc<dyn Fn(A)>>) -> bool {
  match (left, right) {
    (None, None) => true,
    (Some(left), Some(right)) => Rc::ptr_eq(left, right),
    _ => false,
  }
}

pub(crate) fn use_drag_callbacks(
  reference: reactant_core::native_host::ObjectRef,
  callbacks: Option<DragCallbacks>,
) {
  let coordinator = hooks::use_optional_context::<ApplicationContext>()
    .and_then(|context| context.value::<ServicesContext>())
    .and_then(|services| services.coordinator.upgrade());
  let dependency = (reference.clone(), callbacks.clone());
  hooks::use_effect(
    move || {
      let registration = callbacks.zip(coordinator).map(|(callbacks, coordinator)| {
        let object = reference
          .object_id()
          .expect("drag callback host must be committed before its effect");
        let token = coordinator.input.register_drag(object, callbacks);
        (coordinator, object, token)
      });
      move || {
        if let Some((coordinator, object, token)) = registration {
          coordinator.input.unregister_drag(object, token);
        }
      }
    },
    dependency,
  );
}
