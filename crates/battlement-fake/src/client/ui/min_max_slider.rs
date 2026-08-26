//! Synthetic gestures for controlled dual-thumb range sliders.

use battlement::{
    ActionBody, F32Range, LowerLimit, UiElement, UiElementKind, UiEventKind, UiValue, UpperLimit,
};
use battlement_native::Engine;

use crate::client::ui::{MinMaxSliderInteraction, UiClient};

impl<E> UiClient<'_, E>
where
    E: Engine<Command = battlement::Command>,
{
    /// Begins a controlled MinMaxSlider drag.
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
        self.client
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
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::ValueChanging(battlement::ValueChangingEvent {
                        proposed: UiValue::F32Range(proposed),
                    }),
                }));
        }
    }

    /// Releases a MinMaxSlider and submits its final proposal.
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
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
                        previous: UiValue::F32Range(state.committed),
                        proposed: UiValue::F32Range(state.proposed),
                    }),
                }));
        }
    }

    /// Cancels a MinMaxSlider drag without a final proposal.
    pub fn min_max_slider_cancel(&mut self, object_id: battlement::ObjectId) {
        self.client.min_max_slider_interactions.remove(&object_id);
    }

    fn min_max_slider_value(&self, object_id: battlement::ObjectId) -> F32Range {
        let UiElement::MinMaxSlider(value) = self.element(object_id).element() else {
            unreachable!("validated range slider kind changed")
        };
        F32Range::new(
            value.min_value.unwrap_or(0.0),
            value.max_value.unwrap_or(10.0),
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
        let low = match value.low_limit.unwrap_or_default() {
            LowerLimit::Unbounded => f32::MIN,
            LowerLimit::Inclusive(value) => value,
        };
        let high = match value.high_limit.unwrap_or_default() {
            UpperLimit::Unbounded => f32::MAX,
            UpperLimit::Inclusive(value) => value,
        };
        let min = min.clamp(low, high);
        F32Range::new(min, max.clamp(min, high))
    }
}
