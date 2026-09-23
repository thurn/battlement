//! Complete gestures share the host's real input routes and finish before returning.
//! Raw transitions remain available for tests of capture and intermediate frames.
use crate::Display;
use battlement::{
  ClickEvent, ControllerButton, ControllerDirection, PhysicalKey, TextureAddress,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiEvent, UiEventBody, Vector3,
};
use battlement_native::Engine;

impl<E: Engine> Display<E> {
  /// Presses and releases one physical key.
  pub fn send_key(&mut self, key: PhysicalKey) {
    self.send_shortcut(&[key]);
  }

  /// Sends an ordered sequence of complete keystrokes.
  pub fn send_keys(&mut self, keys: &[PhysicalKey]) {
    for key in keys {
      self.send_key(*key);
    }
  }

  /// Holds a chord together, then releases keys in reverse order.
  pub fn send_shortcut(&mut self, keys: &[PhysicalKey]) {
    for key in keys {
      self.key_down(*key);
    }
    for key in keys.iter().rev() {
      self.key_up(*key);
    }
    self.settle();
  }

  /// Presses and releases a button on the primary controller.
  pub fn press_controller_button(&mut self, button: ControllerButton) {
    self.controller_button_down(0, button);
    self.controller_button_up(0, button);
    self.settle();
  }

  /// Sends an ordered sequence of D-pad directions.
  pub fn navigate_controller(&mut self, directions: &[ControllerDirection]) {
    for direction in directions {
      self.controller_navigate(0, *direction);
      self.settle();
    }
  }

  /// Projects through the displayed camera and crosses geometric picking.
  pub fn drag_world(&mut self, from: Vector3, to: Vector3) {
    self.begin_drag_world(from, to);
    self.pointer_up(
      0,
      self.project_world(to).expect("destination inside camera"),
    );
    self.settle();
  }

  /// Leaves the gesture captured so cancellation and interrupted input can be tested.
  pub fn begin_drag_world(&mut self, from: Vector3, to: Vector3) {
    let start = self.project_world(from).expect("source inside camera");
    let end = self.project_world(to).expect("destination inside camera");
    self.pointer_down(0, start);
    self.pointer_move(0, end, true);
  }

  /// Cancels pointer capture and completes restoration.
  pub fn cancel_drag(&mut self) {
    self.pointer_cancel(0);
    self.settle();
  }

  /// Clicks the unique displayed image through geometric picking.
  pub fn click_image(&mut self, texture: TextureAddress) {
    let positions = self
      .images()
      .filter(|(object, image)| object.active_in_hierarchy() && image.texture == texture)
      .map(|(object, _)| self.world_point(object.id(), Vector3::ZERO))
      .collect::<Vec<_>>();
    assert_eq!(
      positions.len(),
      1,
      "visible image must be unique: {texture:?}"
    );
    self.click_at(
      self
        .project_world(positions[0])
        .expect("image inside camera"),
    );
    self.settle();
  }

  /// Submits the unique accessible UI button.
  pub fn click_button(&mut self, label: &str) {
    let event = self.button_event(label);
    self.deliver_ui_event(event);
    self.settle();
  }

  /// Captures a real target for tests of delayed or stale input.
  pub fn button_event(&self, label: &str) -> UiEvent {
    UiEvent {
      target_id: self.semantic_node(label).object_id,
      cancelable: true,
      default_prevented: false,
      body: UiEventBody::Click(ClickEvent::NavigationSubmit),
    }
  }

  /// Invokes the accessibility activation route on a labeled control.
  pub fn activate_accessible(&mut self, label: &str) {
    self.deliver_ui_event(UiEvent {
      target_id: self.semantic_node(label).object_id,
      cancelable: true,
      default_prevented: false,
      body: UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
        backend_generation: 0,
        action: UiAccessibilityAction::Activate,
      }),
    });
    self.settle();
  }
}
