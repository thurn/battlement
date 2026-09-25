use std::{
  cell::{Cell, RefCell},
  rc::Rc,
};

use battlement::{
  ControllerButton, ControllerButtonPayload, ControllerDirection, ControllerInputSettings,
  ControllerNavigationPayload, ControllerNavigationSource, KeyPayload, ObjectId, PhysicalKey,
  PreparedAsset, application::ApplicationState, host_settings::HostSettings,
};
use reactant::{
  Application, GlobalInput, InputCaptureCancellation, InputCaptureDevice, InputCaptureResult,
  InputSubscription, host::ButtonHost, prelude::*,
};
use reactant_core::{app_context, hooks};
use reactant_testing::{Display, assets};

const CAPTURE_ROOT: ObjectId = battlement::object_id!("8da18c98-1bda-4938-adde-358e55a34982");

struct InputObserver(Rc<RefCell<Vec<GlobalInput>>>);

impl Component for InputObserver {
  fn render(&self) -> impl Render {
    let events = self.0.clone();
    reactant::use_global_input(move |input| events.borrow_mut().push(input));
    View::new()
  }
}

#[test]
fn global_input_preserves_metadata_and_clears_held_input_on_focus_loss() {
  let events = Rc::new(RefCell::new(Vec::new()));
  let observed = events.clone();
  let mut display = Display::mount(
    move || {
      Application::new("input/scene")
        .global_keys([PhysicalKey::KeyK])
        .controller_input(ControllerInputSettings::new().buttons([ControllerButton::North]))
        .child(InputObserver(observed.clone()))
    },
    assets::catalog(&[PreparedAsset::scene("input/scene")]),
  );
  display.flush();
  let navigation = ControllerNavigationPayload {
    controller_id: 42,
    direction: ControllerDirection::Left,
    source: ControllerNavigationSource::LeftStick,
    repeat: true,
  };
  display.key_down(PhysicalKey::KeyK);
  display.controller_button_down(42, ControllerButton::North);
  display.controller_navigation(navigation);
  display.set_application_state(ApplicationState {
    focused: false,
    paused: false,
  });
  display.flush();
  assert_eq!(
    *events.borrow(),
    vec![
      GlobalInput::KeyDown(KeyPayload {
        key: PhysicalKey::KeyK
      }),
      GlobalInput::ControllerButtonDown(ControllerButtonPayload {
        controller_id: 42,
        button: ControllerButton::North,
      }),
      GlobalInput::ControllerNavigate(navigation),
      GlobalInput::Reset,
    ]
  );
}

struct SubscribedInput(InputSubscription);

impl Component for SubscribedInput {
  fn render(&self) -> impl Render {
    reactant::use_input_subscription(self.0.clone());
    View::new()
  }
}

struct ChangingBindings;

impl Component for ChangingBindings {
  fn render(&self) -> impl Render {
    let (phase, set_phase) = hooks::use_state(0_u8);
    let app = app_context::use_app();
    (
      ButtonHost::new(trox::ls("Next"))
        .name("Next")
        .on_click(move || set_phase.update(|value| value + 1)),
      ButtonHost::new(trox::ls("Refresh"))
        .name("Refresh")
        .on_click(move || app.refresh_snapshot()),
      SubscribedInput(InputSubscription {
        keys: vec![PhysicalKey::KeyJ],
        buttons: vec![ControllerButton::North],
        navigation: true,
      }),
      (phase < 2).then(|| {
        SubscribedInput(InputSubscription {
          keys: vec![
            PhysicalKey::KeyJ,
            if phase == 0 {
              PhysicalKey::KeyK
            } else {
              PhysicalKey::KeyL
            },
          ],
          buttons: vec![ControllerButton::West],
          navigation: false,
        })
      }),
    )
  }
}

#[test]
fn mounted_bindings_update_without_removing_other_owners_and_survive_refresh() {
  let mut display = Display::mount(
    || {
      Application::new("input/scene")
        .global_keys([PhysicalKey::Escape])
        .controller_input(ControllerInputSettings::new().buttons([ControllerButton::South]))
        .child(ChangingBindings)
        .document(|mut document| {
          document.root_id = CAPTURE_ROOT;
          document
        })
    },
    assets::catalog(&[PreparedAsset::scene("input/scene")]),
  );
  display.flush();
  assert!(display.world().global_keys().contains(&PhysicalKey::KeyK));
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Next"));
  display.flush();
  assert!(!display.world().global_keys().contains(&PhysicalKey::KeyK));
  assert!(display.world().global_keys().contains(&PhysicalKey::KeyL));
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Refresh"));
  display.flush();
  assert!(display.world().global_keys().contains(&PhysicalKey::KeyL));
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Next"));
  display.flush();
  assert_eq!(
    display.world().global_keys(),
    &[PhysicalKey::Escape, PhysicalKey::KeyJ]
  );
  let controller = display.world().controller_input().unwrap();
  assert_eq!(
    controller.buttons,
    [ControllerButton::South, ControllerButton::North]
  );
  assert!(controller.navigation_enabled);
}

struct CaptureObserver {
  device: InputCaptureDevice,
  results: Rc<RefCell<Vec<InputCaptureResult>>>,
  close: StateSetter<Option<InputCaptureDevice>>,
}

impl Component for CaptureObserver {
  fn render(&self) -> impl Render {
    let results = self.results.clone();
    let close = self.close.clone();
    reactant::use_input_capture(self.device, move |result| {
      results.borrow_mut().push(result);
      close.set(None);
    });
    View::new().name("capturing")
  }
}

struct CaptureSurface {
  results: Rc<RefCell<Vec<InputCaptureResult>>>,
  globals: Rc<RefCell<Vec<GlobalInput>>>,
  activations: Rc<Cell<u32>>,
}

impl Component for CaptureSurface {
  fn render(&self) -> impl Render {
    let (device, set_device) = hooks::use_state(None::<InputCaptureDevice>);
    let keyboard = set_device.clone();
    let controller = set_device.clone();
    let cancel = set_device.clone();
    let globals = self.globals.clone();
    let activations = self.activations.clone();
    reactant::use_global_input(move |input| globals.borrow_mut().push(input));
    (
      ButtonHost::new(trox::ls("Capture keyboard"))
        .name("Capture keyboard")
        .on_click(move || keyboard.set(Some(InputCaptureDevice::Keyboard))),
      ButtonHost::new(trox::ls("Capture controller"))
        .name("Capture controller")
        .on_click(move || controller.set(Some(InputCaptureDevice::Controller))),
      ButtonHost::new(trox::ls("Cancel capture"))
        .name("Cancel capture")
        .on_click(move || cancel.set(None)),
      ButtonHost::new(trox::ls("Underlying"))
        .name("underlying")
        .on_click(move || activations.set(activations.get() + 1)),
      device.map(|device| CaptureObserver {
        device,
        results: self.results.clone(),
        close: set_device,
      }),
    )
  }
}

fn capture_display(
  results: Rc<RefCell<Vec<InputCaptureResult>>>,
  globals: Rc<RefCell<Vec<GlobalInput>>>,
  activations: Rc<Cell<u32>>,
) -> Display {
  let mut display = Display::mount(
    move || {
      Application::new("input/scene")
        .global_keys([PhysicalKey::KeyK])
        .controller_input(ControllerInputSettings::new().buttons([ControllerButton::South]))
        .child(CaptureSurface {
          results: results.clone(),
          globals: globals.clone(),
          activations: activations.clone(),
        })
        .document(|mut document| {
          document.root_id = CAPTURE_ROOT;
          document
        })
    },
    assets::catalog(&[PreparedAsset::scene("input/scene")]),
  );
  display.flush();
  display
}

#[test]
fn exclusive_capture_swallows_opener_repeat_and_underlying_activation_until_release() {
  let results = Rc::new(RefCell::new(Vec::new()));
  let globals = Rc::new(RefCell::new(Vec::new()));
  let activations = Rc::new(Cell::new(0));
  let mut display = capture_display(results.clone(), globals.clone(), activations.clone());
  display.controller_button_down(42, ControllerButton::South);
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Capture controller"));
  display.flush();
  let underlying = display.find_ui(CAPTURE_ROOT, "underlying");
  globals.borrow_mut().clear();
  display.controller_button_down(42, ControllerButton::South);
  display.controller_navigation(ControllerNavigationPayload {
    controller_id: 42,
    direction: ControllerDirection::Left,
    source: ControllerNavigationSource::Dpad,
    repeat: true,
  });
  display.ui().navigation_submit(underlying);
  assert!(results.borrow().is_empty());
  assert_eq!(activations.get(), 0);
  display.controller_button_up(42, ControllerButton::South);
  display.release_controller_navigation();
  display.controller_navigation(ControllerNavigationPayload {
    controller_id: 42,
    direction: ControllerDirection::Left,
    source: ControllerNavigationSource::LeftStick,
    repeat: false,
  });
  assert!(results.borrow().is_empty());
  display.release_controller_navigation();
  display.controller_button_down(42, ControllerButton::North);
  display.flush();
  assert_eq!(
    *results.borrow(),
    vec![InputCaptureResult::Button(ControllerButtonPayload {
      controller_id: 42,
      button: ControllerButton::North
    })]
  );
  assert!(
    globals
      .borrow()
      .iter()
      .all(|input| *input == GlobalInput::Reset)
  );
  display.ui().navigation_submit(underlying);
  assert_eq!(activations.get(), 0);
  display.controller_button_up(42, ControllerButton::North);
  display.advance_frame();
  display.ui().navigation_submit(underlying);
  assert_eq!(activations.get(), 0);
  display.advance_frame();
  display.ui().navigation_submit(underlying);
  assert_eq!(activations.get(), 1);
}

#[test]
fn capture_cancels_on_focus_or_disconnect_and_can_be_remounted_after_pointer_cancel() {
  let results = Rc::new(RefCell::new(Vec::new()));
  let mut display = capture_display(results.clone(), Rc::default(), Rc::default());
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Capture keyboard"));
  display.flush();
  display.set_application_state(ApplicationState {
    focused: false,
    paused: false,
  });
  display.flush();
  assert_eq!(
    results.borrow().last(),
    Some(&InputCaptureResult::Cancelled(
      InputCaptureCancellation::FocusLost
    ))
  );
  display.set_application_state(ApplicationState {
    focused: true,
    paused: false,
  });
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Capture controller"));
  display.flush();
  display.disconnect_input_device(InputCaptureDevice::Controller);
  display.flush();
  assert_eq!(
    results.borrow().last(),
    Some(&InputCaptureResult::Cancelled(
      InputCaptureCancellation::DeviceDisconnected
    ))
  );
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Capture keyboard"));
  display.flush();
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Cancel capture"));
  display.flush();
  display.click_ui(display.find_ui(CAPTURE_ROOT, "Capture keyboard"));
  display.flush();
  display.key_down(PhysicalKey::KeyJ);
  display.flush();
  assert_eq!(
    results.borrow().last(),
    Some(&InputCaptureResult::Key {
      device_id: 0,
      key: PhysicalKey::KeyJ
    })
  );
}

#[test]
fn device_availability_observations_cancel_capture_and_clear_global_held_state() {
  let results = Rc::new(RefCell::new(Vec::new()));
  let globals = Rc::new(RefCell::new(Vec::new()));
  let mut display = capture_display(results.clone(), globals.clone(), Rc::default());
  let mut settings = HostSettings {
    keyboard_connected: true,
    controller_count: 1,
    ..Default::default()
  };
  display.set_host_settings(settings.clone());
  display.flush();
  for (label, device) in [
    ("Capture controller", InputCaptureDevice::Controller),
    ("Capture keyboard", InputCaptureDevice::Keyboard),
  ] {
    display.click_ui(display.find_ui(CAPTURE_ROOT, label));
    display.flush();
    globals.borrow_mut().clear();
    match device {
      InputCaptureDevice::Keyboard => settings.keyboard_connected = false,
      InputCaptureDevice::Controller => settings.controller_count = 0,
    }
    display.set_host_settings(settings.clone());
    display.flush();
    assert_eq!(
      results.borrow().last(),
      Some(&InputCaptureResult::Cancelled(
        InputCaptureCancellation::DeviceDisconnected
      ))
    );
    assert!(globals.borrow().contains(&GlobalInput::Reset));
  }
  assert_eq!(results.borrow().len(), 2);
}
