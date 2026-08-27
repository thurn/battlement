use battlement::{ActionBody, Command};
use battlement_native::Engine;

use super::UiClient;

impl<E> UiClient<'_, E>
where
    E: Engine<Command = Command>,
{
    /// Emits a target-only finite geometry change when subscribed.
    pub fn geometry_changed(
        &mut self,
        object_id: battlement::ObjectId,
        previous: battlement::Rect,
        current: battlement::Rect,
    ) {
        assert!(
            rect_finite(previous) && rect_finite(current),
            "geometry must be finite"
        );
        self.send_event(battlement::UiEvent {
            target_id: object_id,
            body: battlement::UiEventBody::GeometryChanged(battlement::GeometryEvent {
                previous,
                current,
            }),
        });
    }

    /// Emits one target-only panel attachment notification when subscribed.
    pub fn attach_to_panel(&mut self, object_id: battlement::ObjectId) {
        self.send_event(battlement::UiEvent {
            target_id: object_id,
            body: battlement::UiEventBody::AttachToPanel(battlement::LifecycleEvent {}),
        });
    }

    /// Emits one target-only panel detachment notification and clears link identity.
    pub fn detach_from_panel(&mut self, object_id: battlement::ObjectId) {
        self.send_event(battlement::UiEvent {
            target_id: object_id,
            body: battlement::UiEventBody::DetachFromPanel(battlement::LifecycleEvent {}),
        });
        self.client
            .ui_link_identities
            .retain(|(target_id, _), _| *target_id != object_id);
    }

    /// Enters a rich-text link and caches its identity for the matching pointer.
    pub fn link_enter(
        &mut self,
        object_id: battlement::ObjectId,
        pointer_id: i32,
        position: battlement::PanelPoint,
        link_id: impl Into<String>,
        link_text: impl Into<String>,
    ) {
        assert!(panel_point_finite(position), "link position must be finite");
        if !self.client.world.input_enabled() {
            return;
        }
        let identity = (link_id.into(), link_text.into());
        self.client
            .ui_link_identities
            .insert((object_id, pointer_id), identity.clone());
        self.send_link_event(
            object_id,
            battlement::UiEventKind::LinkEnter,
            pointer_id,
            position,
            identity,
            None,
        );
    }

    /// Leaves a rich-text link using only the matching cached pointer identity.
    pub fn link_leave(
        &mut self,
        object_id: battlement::ObjectId,
        pointer_id: i32,
        position: battlement::PanelPoint,
    ) {
        assert!(panel_point_finite(position), "link position must be finite");
        if !self.client.world.input_enabled() {
            return;
        }
        let Some(identity) = self
            .client
            .ui_link_identities
            .remove(&(object_id, pointer_id))
        else {
            return;
        };
        self.send_link_event(
            object_id,
            battlement::UiEventKind::LinkLeave,
            pointer_id,
            position,
            identity,
            None,
        );
    }

    /// Presses a button on one rich-text link.
    pub fn link_down(
        &mut self,
        object_id: battlement::ObjectId,
        pointer_id: i32,
        position: battlement::PanelPoint,
        link_id: impl Into<String>,
        link_text: impl Into<String>,
        button: battlement::PointerButton,
    ) {
        assert!(panel_point_finite(position), "link position must be finite");
        self.send_link_event(
            object_id,
            battlement::UiEventKind::LinkDown,
            pointer_id,
            position,
            (link_id.into(), link_text.into()),
            Some(button),
        );
    }

    /// Releases a button on one rich-text link.
    pub fn link_up(
        &mut self,
        object_id: battlement::ObjectId,
        pointer_id: i32,
        position: battlement::PanelPoint,
        link_id: impl Into<String>,
        link_text: impl Into<String>,
        button: battlement::PointerButton,
    ) {
        assert!(panel_point_finite(position), "link position must be finite");
        self.send_link_event(
            object_id,
            battlement::UiEventKind::LinkUp,
            pointer_id,
            position,
            (link_id.into(), link_text.into()),
            Some(button),
        );
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

    fn send_link_event(
        &mut self,
        object_id: battlement::ObjectId,
        kind: battlement::UiEventKind,
        pointer_id: i32,
        position: battlement::PanelPoint,
        identity: (String, String),
        button: Option<battlement::PointerButton>,
    ) {
        let value = battlement::LinkEvent {
            link_id: identity.0,
            link_text: identity.1,
            pointer_id,
            position,
            button,
        };
        let body = match kind {
            battlement::UiEventKind::LinkEnter => battlement::UiEventBody::LinkEnter(value),
            battlement::UiEventKind::LinkLeave => battlement::UiEventBody::LinkLeave(value),
            battlement::UiEventKind::LinkDown => battlement::UiEventBody::LinkDown(value),
            battlement::UiEventKind::LinkUp => battlement::UiEventBody::LinkUp(value),
            _ => panic!("unsupported fake link event kind"),
        };
        self.send_event(battlement::UiEvent {
            target_id: object_id,
            body,
        });
    }
}

fn panel_point_finite(value: battlement::PanelPoint) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

fn rect_finite(value: battlement::Rect) -> bool {
    value.x.is_finite()
        && value.y.is_finite()
        && value.width.is_finite()
        && value.height.is_finite()
}
