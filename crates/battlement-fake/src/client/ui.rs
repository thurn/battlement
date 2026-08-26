//! Typed UI inspection, synthetic gestures, and interaction reconciliation.

use std::time::Instant;

use battlement::{ActionBody, Command, PointerButton};
use battlement_native::Engine;
use battlement_ui_fake::{UiElementState, UiJournalEntry, UiWorld};

use crate::client::FakeClient;

#[derive(Clone, Copy)]
pub(super) struct ScrollInteraction {
    pub(super) latest: battlement::Vector,
    pub(super) last_changed: Instant,
    pub(super) captured: bool,
    pub(super) armed: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ScrollerInteraction {
    pub(super) committed: f32,
    pub(super) proposed: f32,
}

/// Typed access to fake UI state and synthetic UI gestures.
pub struct UiClient<'a, E>
where
    E: Engine<Command = Command>,
{
    pub(super) client: &'a mut FakeClient<E>,
}

impl<E> UiClient<'_, E>
where
    E: Engine<Command = Command>,
{
    /// Returns whether an identity belongs to a live logical UI element.
    #[must_use]
    pub fn contains(&self, object_id: battlement::ObjectId) -> bool {
        self.client.ui_world.element(object_id).is_some()
    }

    /// Returns one live logical UI element.
    #[must_use]
    pub fn element(&self, object_id: battlement::ObjectId) -> &UiElementState {
        self.client
            .ui_world
            .element(object_id)
            .unwrap_or_else(|| panic!("UI element does not exist: {object_id}"))
    }

    /// Returns successfully executed UI commands.
    #[must_use]
    pub fn journal(&self) -> &[UiJournalEntry] {
        self.client.ui_world.journal()
    }

    /// Sends one pointer-style click when the button is enabled and subscribed.
    pub fn click(&mut self, object_id: battlement::ObjectId) {
        if !self.client.world.input_enabled() {
            return;
        }
        let target = self.element(object_id);
        assert_eq!(
            target.kind(),
            battlement::UiElementKind::Button,
            "UI click target is not a button: {object_id}"
        );
        assert!(
            target.is_enabled().unwrap_or(true),
            "UI click target is disabled: {object_id}"
        );
        if !self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::Click)
        {
            return;
        }
        self.client
            .submit_action(ActionBody::VisualElement(battlement::UiEvent::click(
                object_id,
                battlement::ClickEvent::pointer(
                    0,
                    battlement::PanelPoint::default(),
                    PointerButton::Left,
                    1,
                    battlement::KeyModifiers::default(),
                ),
            )));
    }

    /// Sends one keyboard/gamepad submit using route-wide Button click precedence.
    pub fn navigation_submit(&mut self, object_id: battlement::ObjectId) {
        if !self.client.world.input_enabled() {
            return;
        }
        let target = self.element(object_id);
        let button_target = target.kind() == battlement::UiElementKind::Button;
        assert!(
            target.is_enabled().unwrap_or(true),
            "UI navigation submit target is disabled: {object_id}"
        );
        if button_target
            && let Some(target_id) = self
                .client
                .ui_world
                .first_subscription(object_id, battlement::UiEventKind::Click)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent::click(
                    target_id,
                    battlement::ClickEvent::NavigationSubmit,
                )));
            return;
        }
        if let Some(target_id) = self
            .client
            .ui_world
            .first_subscription(object_id, battlement::UiEventKind::NavigationSubmit)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id,
                    body: battlement::UiEventBody::NavigationSubmit,
                }));
        }
    }

    /// Presses and holds a repeat button for an exact number of milliseconds.
    ///
    /// The returned count includes the immediate press callback and every timer
    /// callback whose deadline is at or before `held_ms`. Release adds nothing.
    pub fn repeat_hold(&mut self, object_id: battlement::ObjectId, held_ms: u64) -> usize {
        if !self.client.world.input_enabled() {
            return 0;
        }
        let target = self.element(object_id);
        assert_eq!(
            target.kind(),
            battlement::UiElementKind::RepeatButton,
            "UI repeat target is not a repeat button: {object_id}"
        );
        assert!(
            target.is_enabled().unwrap_or(true),
            "UI repeat target is disabled: {object_id}"
        );
        let battlement::UiElement::RepeatButton(value) = target.element() else {
            unreachable!("validated repeat kind changed")
        };
        let delay = u64::from(value.delay_ms.expect("repeat delay missing"));
        let interval = u64::from(value.interval_ms.expect("repeat interval missing").get());
        let callbacks = 1 + usize::try_from(
            held_ms
                .checked_sub(delay)
                .map_or(0, |elapsed| elapsed / interval + 1),
        )
        .expect("repeat callback count exceeds usize");
        let Some(target_id) = self
            .client
            .ui_world
            .first_subscription(object_id, battlement::UiEventKind::Click)
        else {
            return 0;
        };
        for _ in 0..callbacks {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent::click(
                    target_id,
                    battlement::ClickEvent::Repeat,
                )));
        }
        callbacks
    }

    /// Begins pointer capture for a scroll gesture without changing the offset.
    pub fn scroll_begin(&mut self, object_id: battlement::ObjectId) {
        self.require_scroll_view(object_id);
        if !self.input_available(object_id) {
            return;
        }
        let now = self.clock_now();
        let latest = self.scroll_offset(object_id);
        self.client
            .scroll_interactions
            .entry(object_id)
            .and_modify(|state| state.captured = true)
            .or_insert(ScrollInteraction {
                latest,
                last_changed: now,
                captured: true,
                armed: false,
            });
    }

    /// Applies one user-originated scroll offset and emits an optional live event.
    pub fn scroll_change(&mut self, object_id: battlement::ObjectId, offset: battlement::Vector) {
        self.require_scroll_view(object_id);
        assert!(
            offset.x.is_finite() && offset.y.is_finite(),
            "scroll offset must be finite"
        );
        if !self.input_available(object_id) {
            self.client.scroll_interactions.remove(&object_id);
            return;
        }
        let now = self.clock_now();
        let captured = self
            .client
            .scroll_interactions
            .get(&object_id)
            .is_some_and(|state| state.captured);
        self.client.scroll_interactions.insert(
            object_id,
            ScrollInteraction {
                latest: offset,
                last_changed: now,
                captured,
                armed: true,
            },
        );
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::ScrollChanged)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::ScrollChanged(battlement::ScrollEvent {
                        offset,
                    }),
                }));
        }
    }

    /// Releases pointer capture for a scroll gesture without forcing settlement.
    pub fn scroll_end(&mut self, object_id: battlement::ObjectId) {
        if let Some(state) = self.client.scroll_interactions.get_mut(&object_id) {
            state.captured = false;
        }
    }

    /// Emits scroll settlements whose exact 100-millisecond deadline has elapsed.
    pub fn advance(&mut self) {
        let now = self.clock_now();
        let settled = self
            .client
            .scroll_interactions
            .iter()
            .filter_map(|(object_id, state)| {
                let elapsed = now.duration_since(state.last_changed);
                (state.armed && !state.captured && elapsed >= std::time::Duration::from_millis(100))
                    .then_some((*object_id, state.latest))
            })
            .collect::<Vec<_>>();
        for (object_id, offset) in settled {
            if let Some(state) = self.client.scroll_interactions.get_mut(&object_id) {
                state.armed = false;
            }
            if self.input_available(object_id)
                && self
                    .client
                    .ui_world
                    .has_subscription(object_id, battlement::UiEventKind::ScrollSettled)
            {
                self.client
                    .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                        target_id: object_id,
                        body: battlement::UiEventBody::ScrollSettled(battlement::ScrollEvent {
                            offset,
                        }),
                    }));
            }
        }
    }

    /// Begins a controlled Scroller drag from its latest Rust-authored value.
    pub fn scroller_begin(&mut self, object_id: battlement::ObjectId) {
        self.require_scroller(object_id);
        if !self.input_available(object_id) {
            return;
        }
        let committed = self.scroller_value(object_id);
        self.client.scroller_interactions.insert(
            object_id,
            ScrollerInteraction {
                committed,
                proposed: committed,
            },
        );
    }

    /// Changes a controlled Scroller's local proposal during pointer capture.
    pub fn scroller_change(&mut self, object_id: battlement::ObjectId, proposed: f32) {
        assert!(proposed.is_finite(), "scroller proposal must be finite");
        let proposed = self.clamp_scroller_value(object_id, proposed);
        self.client
            .scroller_interactions
            .get_mut(&object_id)
            .unwrap_or_else(|| panic!("scroller is not captured: {object_id}"))
            .proposed = proposed;
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::ValueChanging)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::ValueChanging(battlement::ValueChangingEvent {
                        proposed: battlement::UiValue::F32(proposed),
                    }),
                }));
        }
    }

    /// Releases a controlled Scroller and submits one final proposal when subscribed.
    pub fn scroller_commit(&mut self, object_id: battlement::ObjectId) {
        let state = self
            .client
            .scroller_interactions
            .remove(&object_id)
            .unwrap_or_else(|| panic!("scroller is not captured: {object_id}"));
        if !self.input_available(object_id) {
            return;
        }
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::ValueCommitted)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
                        previous: battlement::UiValue::F32(state.committed),
                        proposed: battlement::UiValue::F32(state.proposed),
                    }),
                }));
        }
    }

    /// Cancels a controlled Scroller gesture without emitting a final proposal.
    pub fn scroller_cancel(&mut self, object_id: battlement::ObjectId) {
        self.client.scroller_interactions.remove(&object_id);
    }

    fn require_scroll_view(&self, object_id: battlement::ObjectId) {
        assert_eq!(
            self.element(object_id).kind(),
            battlement::UiElementKind::ScrollView
        );
    }

    fn require_scroller(&self, object_id: battlement::ObjectId) {
        assert_eq!(
            self.element(object_id).kind(),
            battlement::UiElementKind::Scroller
        );
    }

    fn input_available(&self, object_id: battlement::ObjectId) -> bool {
        self.client.world.input_enabled() && self.element_enabled_in_hierarchy(object_id)
    }

    fn element_enabled_in_hierarchy(&self, object_id: battlement::ObjectId) -> bool {
        let mut current = Some(object_id);
        while let Some(value) = current {
            let element = self.element(value);
            if element.is_enabled() == Some(false) {
                return false;
            }
            current = element.parent_id();
        }
        true
    }

    fn clock_now(&self) -> Instant {
        self.client
            .clock
            .as_ref()
            .expect("scroll settlement requires FakeClient::connect_clocked")
            .now()
    }

    fn scroll_offset(&self, object_id: battlement::ObjectId) -> battlement::Vector {
        let battlement::UiElement::ScrollView(value) = self.element(object_id).element() else {
            unreachable!("validated scroll view kind changed")
        };
        value.scroll_offset.unwrap_or_default()
    }

    fn scroller_value(&self, object_id: battlement::ObjectId) -> f32 {
        let battlement::UiElement::Scroller(value) = self.element(object_id).element() else {
            unreachable!("validated scroller kind changed")
        };
        value.value.unwrap_or_default()
    }

    fn clamp_scroller_value(&self, object_id: battlement::ObjectId, proposed: f32) -> f32 {
        let battlement::UiElement::Scroller(value) = self.element(object_id).element() else {
            unreachable!("validated scroller kind changed")
        };
        let low = value.low_value.unwrap_or_default();
        let high = value.high_value.unwrap_or_default();
        proposed.clamp(low.min(high), low.max(high))
    }

    /// Sends a subscribed native transition-start event.
    pub fn transition_start(
        &mut self,
        object_id: battlement::ObjectId,
        value: battlement::TransitionEvent,
    ) {
        self.transition(
            object_id,
            battlement::UiEventKind::TransitionStart,
            battlement::UiEventBody::TransitionStart(value),
        );
    }

    /// Sends a subscribed native transition-end event.
    pub fn transition_end(
        &mut self,
        object_id: battlement::ObjectId,
        value: battlement::TransitionEvent,
    ) {
        self.transition(
            object_id,
            battlement::UiEventKind::TransitionEnd,
            battlement::UiEventBody::TransitionEnd(value),
        );
    }

    /// Sends a subscribed native transition-cancel event.
    pub fn transition_cancel(
        &mut self,
        object_id: battlement::ObjectId,
        value: battlement::TransitionEvent,
    ) {
        self.transition(
            object_id,
            battlement::UiEventKind::TransitionCancel,
            battlement::UiEventBody::TransitionCancel(value),
        );
    }

    fn transition(
        &mut self,
        object_id: battlement::ObjectId,
        kind: battlement::UiEventKind,
        body: battlement::UiEventBody,
    ) {
        if !self.client.world.input_enabled() {
            return;
        }
        let _ = self.element(object_id);
        if !self.client.ui_world.has_subscription(object_id, kind) {
            return;
        }
        self.client
            .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                target_id: object_id,
                body,
            }));
    }
}

impl<E> FakeClient<E>
where
    E: Engine<Command = Command>,
{
    pub(crate) fn reconcile_ui_interactions(&mut self, body: &battlement::CommandBody) {
        if matches!(body, battlement::CommandBody::InputSetEnabled(value) if !value.enabled) {
            self.scroll_interactions.clear();
            self.scroller_interactions.clear();
            return;
        }
        if let battlement::CommandBody::VisualElementUpdate(value) = body
            && let battlement::VisualElementUpdate::Properties { object_id, element } =
                value.as_ref()
        {
            if matches!(&**element, battlement::UiElement::ScrollView(value) if value.scroll_offset.is_some())
            {
                self.scroll_interactions.remove(object_id);
            }
            if let battlement::UiElement::Scroller(value) = &**element
                && let Some(committed) = value.value
                && let Some(state) = self.scroller_interactions.get_mut(object_id)
            {
                state.committed = committed;
            }
        }
        let world = &self.ui_world;
        self.scroll_interactions
            .retain(|object_id, _| ui_element_enabled(world, *object_id));
        self.scroller_interactions
            .retain(|object_id, _| ui_element_enabled(world, *object_id));
    }
}

fn ui_element_enabled(world: &UiWorld, object_id: battlement::ObjectId) -> bool {
    let mut current = Some(object_id);
    while let Some(value) = current {
        let Some(element) = world.element(value) else {
            return false;
        };
        if element.is_enabled() == Some(false) {
            return false;
        }
        current = element.parent_id();
    }
    true
}
