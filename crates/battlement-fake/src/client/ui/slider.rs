//! Synthetic gestures for controlled floating-point and integer sliders.

use battlement::ActionBody;
use battlement_native::Engine;

use crate::client::ui::{ScrollerInteraction, SliderIntInteraction, UiClient};

impl<E> UiClient<'_, E>
where
  E: Engine<Command = battlement::Command>,
{
  /// Begins a controlled floating-point Slider drag.
  pub fn slider_begin(&mut self, object_id: battlement::ObjectId) {
    self.require_slider(object_id, battlement::UiElementKind::Slider);
    if !self.input_available(object_id) {
      return;
    }
    let committed = self.slider_value(object_id);
    self.client.slider_interactions.insert(
      object_id,
      ScrollerInteraction {
        committed,
        proposed: committed,
      },
    );
  }

  /// Changes a floating-point Slider's local proposal during capture.
  pub fn slider_change(&mut self, object_id: battlement::ObjectId, proposed: f32) {
    assert!(proposed.is_finite(), "slider proposal must be finite");
    let proposed = self.clamp_slider_value(object_id, proposed);
    self
      .client
      .slider_interactions
      .get_mut(&object_id)
      .unwrap_or_else(|| panic!("slider is not captured: {object_id}"))
      .proposed = proposed;
    self.submit_live_value(object_id, battlement::UiValue::F32(proposed));
  }

  /// Releases a floating-point Slider and submits its final proposal.
  pub fn slider_commit(&mut self, object_id: battlement::ObjectId) {
    let state = self
      .client
      .slider_interactions
      .remove(&object_id)
      .unwrap_or_else(|| panic!("slider is not captured: {object_id}"));
    self.submit_range_commit(
      object_id,
      battlement::UiValue::F32(state.committed),
      battlement::UiValue::F32(state.proposed),
    );
  }

  /// Cancels a floating-point Slider drag without a final proposal.
  pub fn slider_cancel(&mut self, object_id: battlement::ObjectId) {
    self.client.slider_interactions.remove(&object_id);
  }

  /// Begins a controlled integer Slider drag.
  pub fn slider_int_begin(&mut self, object_id: battlement::ObjectId) {
    self.require_slider(object_id, battlement::UiElementKind::SliderInt);
    if !self.input_available(object_id) {
      return;
    }
    let committed = self.slider_int_value(object_id);
    self.client.slider_int_interactions.insert(
      object_id,
      SliderIntInteraction {
        committed,
        proposed: committed,
      },
    );
  }

  /// Changes an integer Slider proposal, rounded and clamped like Unity.
  pub fn slider_int_change(&mut self, object_id: battlement::ObjectId, proposed: f32) {
    assert!(proposed.is_finite(), "slider proposal must be finite");
    let proposed = self.clamp_slider_int_value(object_id, proposed);
    self
      .client
      .slider_int_interactions
      .get_mut(&object_id)
      .unwrap_or_else(|| panic!("integer slider is not captured: {object_id}"))
      .proposed = proposed;
    self.submit_live_value(object_id, battlement::UiValue::I32(proposed));
  }

  /// Releases an integer Slider and submits its final proposal.
  pub fn slider_int_commit(&mut self, object_id: battlement::ObjectId) {
    let state = self
      .client
      .slider_int_interactions
      .remove(&object_id)
      .unwrap_or_else(|| panic!("integer slider is not captured: {object_id}"));
    self.submit_range_commit(
      object_id,
      battlement::UiValue::I32(state.committed),
      battlement::UiValue::I32(state.proposed),
    );
  }

  /// Cancels an integer Slider drag without a final proposal.
  pub fn slider_int_cancel(&mut self, object_id: battlement::ObjectId) {
    self.client.slider_int_interactions.remove(&object_id);
  }

  fn submit_live_value(&mut self, object_id: battlement::ObjectId, proposed: battlement::UiValue) {
    if self.input_available(object_id)
      && self
        .client
        .ui_world
        .has_subscription(object_id, battlement::UiEventKind::ValueChanging)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ValueChanging(battlement::ValueChangingEvent { proposed }),
        }));
    }
  }

  fn submit_range_commit(
    &mut self,
    object_id: battlement::ObjectId,
    previous: battlement::UiValue,
    proposed: battlement::UiValue,
  ) {
    if self.input_available(object_id)
      && self
        .client
        .ui_world
        .has_subscription(object_id, battlement::UiEventKind::ValueCommitted)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
            previous,
            proposed,
          }),
        }));
    }
  }

  fn require_slider(&self, object_id: battlement::ObjectId, expected: battlement::UiElementKind) {
    assert_eq!(self.element(object_id).kind(), expected);
  }

  fn slider_value(&self, object_id: battlement::ObjectId) -> f32 {
    let battlement::UiElement::Slider(value) = self.element(object_id).element() else {
      unreachable!("validated slider kind changed")
    };
    value.value.unwrap_or_default()
  }

  fn clamp_slider_value(&self, object_id: battlement::ObjectId, proposed: f32) -> f32 {
    let battlement::UiElement::Slider(value) = self.element(object_id).element() else {
      unreachable!("validated slider kind changed")
    };
    proposed.clamp(
      value.low_value.unwrap_or(0.0),
      value.high_value.unwrap_or(10.0),
    )
  }

  fn slider_int_value(&self, object_id: battlement::ObjectId) -> i32 {
    let battlement::UiElement::SliderInt(value) = self.element(object_id).element() else {
      unreachable!("validated integer slider kind changed")
    };
    value.value.unwrap_or_default()
  }

  fn clamp_slider_int_value(&self, object_id: battlement::ObjectId, proposed: f32) -> i32 {
    let battlement::UiElement::SliderInt(value) = self.element(object_id).element() else {
      unreachable!("validated integer slider kind changed")
    };
    let rounded = proposed.round();
    let low = value.low_value.unwrap_or(0);
    let high = value.high_value.unwrap_or(10);
    (rounded as i32).clamp(low, high)
  }
}
