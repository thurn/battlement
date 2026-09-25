use battlement::{
  ActionBody, ClickEvent, ControllerNavigationSource, InputCaptureCancellation,
  InputCaptureCommand, InputCaptureDevice, InputCaptureEvent, InputCaptureRequest,
  InputCaptureResult, PhysicalKey, UiEventBody,
};
use battlement_native::Engine;

use crate::client::FakeClient;

#[derive(Default)]
pub(crate) struct Capture {
  request: Option<InputCaptureRequest>,
  terminal: bool,
  armed: bool,
  fence: bool,
  neutral_frames: u8,
}

impl Capture {
  pub(crate) fn blocks(&self) -> bool {
    self.request.is_some() || self.fence
  }
}

impl<E: Engine> FakeClient<E> {
  pub(crate) fn capture_command(&mut self, command: InputCaptureCommand) {
    match command {
      InputCaptureCommand::Begin(request) => {
        let previous = self.capture.request.filter(|_| !self.capture.terminal);
        self.capture = Capture {
          request: Some(request),
          armed: self.input_neutral(),
          fence: true,
          ..Default::default()
        };
        if let Some(previous) = previous {
          self.submit_action(ActionBody::InputCaptured(InputCaptureEvent {
            id: previous.id,
            result: InputCaptureResult::Cancelled(InputCaptureCancellation::Superseded),
          }));
        }
        if !self.connect.application_state.focused || self.connect.application_state.paused {
          self.complete_capture(InputCaptureResult::Cancelled(
            InputCaptureCancellation::FocusLost,
          ));
        }
      }
      InputCaptureCommand::End(id) => {
        if self.capture.request.is_some_and(|request| request.id == id) {
          self.capture.request = None;
          self.capture.fence = true;
          self.capture.neutral_frames = 0;
        }
      }
    }
  }

  pub(super) fn capture_input(&mut self, input: InputCaptureResult) -> bool {
    if !self.capture.blocks() {
      return false;
    }
    let Some(request) = self.capture.request else {
      return true;
    };
    if !self.capture.armed || self.capture.terminal {
      return true;
    }
    let eligible = match input {
      InputCaptureResult::Key { key, .. } if request.device == InputCaptureDevice::Keyboard => {
        let modifier = matches!(
          key,
          PhysicalKey::ShiftLeft
            | PhysicalKey::ShiftRight
            | PhysicalKey::ControlLeft
            | PhysicalKey::ControlRight
            | PhysicalKey::AltLeft
            | PhysicalKey::AltRight
            | PhysicalKey::MetaLeft
            | PhysicalKey::MetaRight
        );
        if modifier || self.held_keys.len() != 1 {
          self.capture.armed = false;
          false
        } else {
          true
        }
      }
      InputCaptureResult::Button(_) if request.device == InputCaptureDevice::Controller => {
        let dpad_held = self
          .held_navigation
          .is_some_and(|value| value.source == ControllerNavigationSource::Dpad);
        if self.held_controller_buttons.len() != 1 || dpad_held {
          self.capture.armed = false;
          false
        } else {
          true
        }
      }
      InputCaptureResult::Direction(value) if request.device == InputCaptureDevice::Controller => {
        if !self.held_controller_buttons.is_empty() {
          self.capture.armed = false;
          false
        } else {
          value.source == ControllerNavigationSource::Dpad && !value.repeat
        }
      }
      _ => false,
    };
    if eligible {
      self.complete_capture(input);
    }
    true
  }

  pub(super) fn complete_capture(&mut self, result: InputCaptureResult) {
    if let Some(request) = self.capture.request.filter(|_| !self.capture.terminal) {
      self.capture.terminal = true;
      self.submit_action(ActionBody::InputCaptured(InputCaptureEvent {
        id: request.id,
        result,
      }));
    }
  }

  pub(super) fn capture_released(&mut self) {
    if self.input_neutral() {
      self.capture.armed = true;
    }
  }

  pub(super) fn capture_frame(&mut self) {
    self.capture.neutral_frames = if self.input_neutral() {
      self.capture.neutral_frames.saturating_add(1)
    } else {
      0
    };
    if self.capture.neutral_frames >= 2 {
      self.capture.fence = false;
    }
  }

  fn input_neutral(&self) -> bool {
    self.held_keys.is_empty()
      && self.held_controller_buttons.is_empty()
      && self.held_navigation.is_none()
  }

  /// Releases the physical direction held by the fake controller.
  pub fn release_controller_navigation(&mut self) {
    self.held_navigation = None;
    self.capture_released();
  }

  /// Removes the selected device family and cancels its active capture.
  pub fn disconnect_input_device(&mut self, device: InputCaptureDevice) {
    match device {
      InputCaptureDevice::Keyboard => self.connect.host_settings.keyboard_connected = false,
      InputCaptureDevice::Controller => self.connect.host_settings.controller_count = 0,
    }
    self.remove_input_device(device);
    self.submit_action(ActionBody::HostSettingsChanged(
      self.connect.host_settings.clone(),
    ));
  }

  pub(super) fn remove_input_device(&mut self, device: InputCaptureDevice) {
    if device == InputCaptureDevice::Keyboard {
      self.held_keys.clear();
    } else {
      self.held_controller_buttons.clear();
      self.held_navigation = None;
    }
    if self
      .capture
      .request
      .is_some_and(|request| request.device == device)
    {
      self.complete_capture(InputCaptureResult::Cancelled(
        InputCaptureCancellation::DeviceDisconnected,
      ));
    }
    self.capture_released();
  }

  pub(super) fn suppress_captured_ui(&self, event: &UiEventBody) -> bool {
    self.capture.blocks()
      && matches!(
        event,
        UiEventBody::KeyDown(_)
          | UiEventBody::KeyUp(_)
          | UiEventBody::NavigationMove(_)
          | UiEventBody::NavigationCancel(_)
          | UiEventBody::Click(ClickEvent::NavigationSubmit)
      )
  }
}
