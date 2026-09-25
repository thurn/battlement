use std::rc::Rc;

use battlement::{InputCaptureDevice, InputCaptureRequest, InputCaptureResult, ObjectId};
use reactant_core::{app_context::AppHandle, app_runtime::ApplicationContext, hooks};

use crate::game_app::ServicesContext;

/// Captures one physical input before native menu and global routing while mounted.
/// The handler receives one binding or cancellation. Unmounting releases capture;
/// the host continues swallowing already-held input until it is released.
/// Handlers run during a UI commit and may update state or announce localized feedback.
pub fn use_input_capture(
  device: InputCaptureDevice,
  handler: impl Fn(InputCaptureResult) + 'static,
) {
  let services = hooks::use_required_context::<ApplicationContext>()
    .value::<ServicesContext>()
    .expect("input capture requires a Reactant Application root");
  let input = services
    .coordinator
    .upgrade()
    .expect("application runtime ended while rendering")
    .input
    .clone();
  let app = hooks::use_required_context::<AppHandle>();
  let handler = Rc::new(handler) as Rc<dyn Fn(InputCaptureResult)>;
  let current = hooks::use_ref(handler.clone());
  let committed = current.clone();
  hooks::use_effect_always(move || {
    committed.replace(handler);
  });
  let id = hooks::use_memo(ObjectId::new_v4, device);
  let (result, set_result) = hooks::use_state(None::<(ObjectId, InputCaptureResult)>);
  hooks::use_effect(
    move || {
      if let Some((owner, result)) = result
        && owner == id
      {
        current.with(|handler| handler(result));
      }
    },
    (id, result),
  );
  hooks::use_effect(
    move || {
      input.begin_capture(
        InputCaptureRequest { id, device },
        app,
        Rc::new(move |result| set_result.set(Some((id, result)))),
      );
      move || input.end_capture(id)
    },
    (id, device),
  );
}
