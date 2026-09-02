//! Synthetic gestures for controlled dual-thumb range sliders.

use battlement::{
  F32Range, LowerLimit, UiElement, UiElementKind, UiEventKind, UiValue, UpperLimit,
};
use battlement_native::Engine;

use crate::client::ui::{MinMaxSliderInteraction, UiClient};

impl<E> UiClient<'_, E>
where
  E: Engine<Command = battlement::Command>,
{
  /// Begins a controlled UiMinMaxSlider drag.
  pub fn min_max_slider_begin(&mut self, object_id: battlement::ObjectId) {
    assert_eq!(self.element(object_id).kind(), UiElementKind::MinMaxSlider);
    if !self.input_available(object_id) {
      return;
    }
    let committed = self.min_max_slider_value(object_id);
    self.client.min_max_slider_interactions.insert(
      object_id,
      MinMaxSliderInteraction {
        committed,
        proposed: committed,
      },
    );
  }

  /// Changes the local ordered range proposal during capture.
  pub fn min_max_slider_change(&mut self, object_id: battlement::ObjectId, min: f32, max: f32) {
    assert!(
      min.is_finite() && max.is_finite(),
      "range proposal must be finite"
    );
    let proposed = self.clamp_min_max_slider_value(object_id, min, max);
    self
      .client
      .min_max_slider_interactions
      .get_mut(&object_id)
      .unwrap_or_else(|| panic!("range slider is not captured: {object_id}"))
      .proposed = proposed;
    if self.input_available(object_id)
      && self
        .client
        .ui_world
        .has_subscription(object_id, UiEventKind::ValueChanging)
    {
      self.client.submit_ui_event(battlement::UiEvent {
        target_id: object_id,
        cancelable: false,
        default_prevented: false,
        body: battlement::UiEventBody::ValueChanging(battlement::ValueChangingEvent {
          proposed: UiValue::F32Range(proposed),
        }),
      });
    }
  }

  /// Releases a UiMinMaxSlider and submits its final proposal.
  pub fn min_max_slider_commit(&mut self, object_id: battlement::ObjectId) {
    let state = self
      .client
      .min_max_slider_interactions
      .remove(&object_id)
      .unwrap_or_else(|| panic!("range slider is not captured: {object_id}"));
    if self.input_available(object_id)
      && self
        .client
        .ui_world
        .has_subscription(object_id, UiEventKind::ValueCommitted)
    {
      self.client.submit_ui_event(battlement::UiEvent {
        target_id: object_id,
        cancelable: false,
        default_prevented: false,
        body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
          previous: UiValue::F32Range(state.committed),
          proposed: UiValue::F32Range(state.proposed),
        }),
      });
    }
  }

  /// Cancels a UiMinMaxSlider drag without a final proposal.
  pub fn min_max_slider_cancel(&mut self, object_id: battlement::ObjectId) {
    self.client.min_max_slider_interactions.remove(&object_id);
  }

  fn min_max_slider_value(&self, object_id: battlement::ObjectId) -> F32Range {
    let UiElement::MinMaxSlider(value) = self.element(object_id).element() else {
      unreachable!("validated range slider kind changed")
    };
    F32Range::new(
      prop_f32(&value.min_value, 0.0),
      prop_f32(&value.max_value, 10.0),
    )
  }

  fn clamp_min_max_slider_value(
    &self,
    object_id: battlement::ObjectId,
    min: f32,
    max: f32,
  ) -> F32Range {
    let UiElement::MinMaxSlider(value) = self.element(object_id).element() else {
      unreachable!("validated range slider kind changed")
    };
    let low = match &value.low_limit {
      battlement::Prop::Set(LowerLimit::Inclusive(value)) => *value,
      battlement::Prop::Set(LowerLimit::Unbounded)
      | battlement::Prop::Unset
      | battlement::Prop::Reset => f32::MIN,
    };
    let high = match &value.high_limit {
      battlement::Prop::Set(UpperLimit::Inclusive(value)) => *value,
      battlement::Prop::Set(UpperLimit::Unbounded)
      | battlement::Prop::Unset
      | battlement::Prop::Reset => f32::MAX,
    };
    let min = min.clamp(low, high);
    F32Range::new(min, max.clamp(min, high))
  }
}

fn prop_f32(value: &battlement::Prop<f32>, reset: f32) -> f32 {
  match value {
    battlement::Prop::Set(value) => *value,
    battlement::Prop::Unset | battlement::Prop::Reset => reset,
  }
}
