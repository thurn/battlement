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

#[derive(Clone)]
pub(super) struct TextFieldInteraction {
    pub(super) committed: String,
    pub(super) draft: String,
}

impl TextFieldInteraction {
    fn proposed_is_unchanged(&self) -> bool {
        self.committed == self.draft
    }
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

    /// Activates a Button through keyboard or gamepad submit.
    pub fn navigation_submit(&mut self, object_id: battlement::ObjectId) {
        if !self.client.world.input_enabled() {
            return;
        }
        let target = self.element(object_id);
        assert_eq!(
            target.kind(),
            battlement::UiElementKind::Button,
            "UI navigation submit target is not a button: {object_id}"
        );
        assert!(
            target.is_enabled().unwrap_or(true),
            "UI navigation submit target is disabled: {object_id}"
        );
        if let Some(target_id) = self
            .client
            .ui_world
            .first_subscription(object_id, battlement::UiEventKind::Click)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent::click(
                    target_id,
                    battlement::ClickEvent::NavigationSubmit,
                )));
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

    /// Applies one native text edit and optionally forwards the complete local draft.
    pub fn text_input(&mut self, object_id: battlement::ObjectId, draft: impl Into<String>) {
        self.require_text_field(object_id);
        if !self.text_edit_available(object_id) {
            self.client.text_field_interactions.remove(&object_id);
            return;
        }
        let draft = draft.into();
        let committed = self.text_value(object_id).to_owned();
        self.client
            .text_field_interactions
            .entry(object_id)
            .and_modify(|state| state.draft.clone_from(&draft))
            .or_insert(TextFieldInteraction {
                committed,
                draft: draft.clone(),
            });
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::Input)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::Input(battlement::TextInputEvent {
                        value: draft,
                    }),
                }));
        }
    }

    /// Returns the native draft, or the committed value when no edit is active.
    #[must_use]
    pub fn text_draft(&self, object_id: battlement::ObjectId) -> &str {
        self.require_text_field(object_id);
        self.client
            .text_field_interactions
            .get(&object_id)
            .map_or_else(|| self.text_value(object_id), |state| state.draft.as_str())
    }

    /// Commits one local text draft and immediately restores authored fake state.
    pub fn text_commit(&mut self, object_id: battlement::ObjectId) {
        self.require_text_field(object_id);
        let Some(state) = self.client.text_field_interactions.remove(&object_id) else {
            return;
        };
        if !self.text_edit_available(object_id) || state.proposed_is_unchanged() {
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
                        previous: battlement::UiValue::String(state.committed),
                        proposed: battlement::UiValue::String(state.draft),
                    }),
                }));
        }
    }

    /// Cancels one local text draft without emitting an action.
    pub fn text_escape(&mut self, object_id: battlement::ObjectId) {
        self.require_text_field(object_id);
        self.client.text_field_interactions.remove(&object_id);
    }

    /// Emits one logical native caret or selection mutation when subscribed.
    pub fn text_selection(
        &mut self,
        object_id: battlement::ObjectId,
        cursor_index: u32,
        select_index: u32,
    ) {
        self.require_text_field(object_id);
        let length = self.text_draft(object_id).encode_utf16().count();
        assert!(
            (cursor_index as usize) <= length,
            "text cursor index is out of range"
        );
        assert!(
            (select_index as usize) <= length,
            "text select index is out of range"
        );
        if !self.input_available(object_id) {
            return;
        }
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::SelectionChanged)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::SelectionChanged(
                        battlement::TextSelectionEvent {
                            cursor_index,
                            select_index,
                        },
                    ),
                }));
        }
    }

    /// Proposes a controlled active-tab change without mutating authored state.
    pub fn tab_select(&mut self, object_id: battlement::ObjectId, proposed_index: u32) {
        self.require_tab_view(object_id);
        if !self.input_available(object_id) {
            return;
        }
        let children = self.element(object_id).children();
        let proposed_tab_id = *children
            .get(proposed_index as usize)
            .unwrap_or_else(|| panic!("tab selection is out of range: {proposed_index}"));
        let previous_index = self.tab_selected_index(object_id);
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::TabSelectionRequested)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::TabSelectionRequested(
                        battlement::TabSelectionEvent {
                            previous_index,
                            proposed_index,
                            proposed_tab_id,
                        },
                    ),
                }));
        }
    }

    /// Proposes closing one tab while preserving it until Rust destroys it.
    pub fn tab_close(&mut self, object_id: battlement::ObjectId, index: u32) {
        self.require_tab_view(object_id);
        if !self.input_available(object_id) {
            return;
        }
        let tab_id = *self
            .element(object_id)
            .children()
            .get(index as usize)
            .unwrap_or_else(|| panic!("tab close index is out of range: {index}"));
        let battlement::UiElement::Tab(tab) = self.element(tab_id).element() else {
            panic!("TabView child is not a Tab: {tab_id}");
        };
        if tab.closeable != Some(true) {
            return;
        }
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::TabCloseRequested)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::TabCloseRequested(battlement::TabCloseEvent {
                        tab_id,
                        index,
                    }),
                }));
        }
    }

    /// Proposes moving a tab header while preserving authored logical order.
    pub fn tab_reorder(
        &mut self,
        object_id: battlement::ObjectId,
        previous_index: u32,
        proposed_index: u32,
    ) {
        self.require_tab_view(object_id);
        if !self.input_available(object_id) {
            return;
        }
        let battlement::UiElement::TabView(tab_view) = self.element(object_id).element() else {
            unreachable!("tab view kind changed after validation");
        };
        if tab_view.reorderable != Some(true) {
            return;
        }
        let children = self.element(object_id).children();
        let tab_id = *children
            .get(previous_index as usize)
            .unwrap_or_else(|| panic!("tab source index is out of range: {previous_index}"));
        assert!(
            (proposed_index as usize) < children.len(),
            "tab destination index is out of range: {proposed_index}"
        );
        if self
            .client
            .ui_world
            .has_subscription(object_id, battlement::UiEventKind::TabReorderRequested)
        {
            self.client
                .submit_action(ActionBody::VisualElement(battlement::UiEvent {
                    target_id: object_id,
                    body: battlement::UiEventBody::TabReorderRequested(
                        battlement::TabReorderEvent {
                            tab_id,
                            previous_index,
                            proposed_index,
                        },
                    ),
                }));
        }
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

    fn require_tab_view(&self, object_id: battlement::ObjectId) {
        assert_eq!(
            self.element(object_id).kind(),
            battlement::UiElementKind::TabView
        );
    }

    fn require_text_field(&self, object_id: battlement::ObjectId) {
        assert_eq!(
            self.element(object_id).kind(),
            battlement::UiElementKind::TextField
        );
    }

    fn input_available(&self, object_id: battlement::ObjectId) -> bool {
        self.client.world.input_enabled() && self.element_enabled_in_hierarchy(object_id)
    }

    fn text_edit_available(&self, object_id: battlement::ObjectId) -> bool {
        if !self.input_available(object_id) {
            return false;
        }
        let battlement::UiElement::TextField(value) = self.element(object_id).element() else {
            unreachable!("validated text field kind changed")
        };
        value.read_only != Some(true)
    }

    fn text_value(&self, object_id: battlement::ObjectId) -> &str {
        let battlement::UiElement::TextField(value) = self.element(object_id).element() else {
            unreachable!("validated text field kind changed")
        };
        value.value.as_deref().unwrap_or_default()
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

    fn tab_selected_index(&self, object_id: battlement::ObjectId) -> u32 {
        let battlement::UiElement::TabView(value) = self.element(object_id).element() else {
            unreachable!("validated tab view kind changed")
        };
        value.selected_tab_index.unwrap_or_default()
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
            self.text_field_interactions.clear();
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
            if matches!(&**element, battlement::UiElement::TextField(value) if value.value.is_some())
            {
                self.text_field_interactions.remove(object_id);
            }
        }
        let world = &self.ui_world;
        self.scroll_interactions
            .retain(|object_id, _| ui_element_enabled(world, *object_id));
        self.scroller_interactions
            .retain(|object_id, _| ui_element_enabled(world, *object_id));
        self.text_field_interactions
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
