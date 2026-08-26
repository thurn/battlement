use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
    ActionBody, Batch, BatchId, CameraState, ClientMessage, Command, Connect, GameObject, Label,
    ObjectId, ParallelCommandGroup, PreparedAsset, Response, Scene, SceneId, SessionId, Snapshot,
    Tab, TabView, UiDocument, UiEventBody, UiEventKind, UiNode,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct TabEngine {
    session_id: SessionId,
    snapshot: Option<Snapshot>,
    rejected_tab_id: ObjectId,
    events: Rc<RefCell<Vec<UiEventBody>>>,
}

impl Engine for TabEngine {
    type ActionPayload = ();
    type ErrorCode = ();
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
        Ok(Response::snapshot(
            self.snapshot.take().expect("connected twice"),
        ))
    }

    fn submit(&mut self, message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
        let ClientMessage::Action(action) = message else {
            return Err(EngineError::new("unexpected client failure"));
        };
        let ActionBody::VisualElement(event) = action.body else {
            return Err(EngineError::new("unexpected non-UI action"));
        };
        self.events.borrow_mut().push(event.body.clone());
        let commands = match event.body {
            UiEventBody::TabSelectionRequested(value) => vec![Command::update_visual_element(
                event.target_id,
                TabView::new().selected_tab_index(value.proposed_index),
            )],
            UiEventBody::TabReorderRequested(value) => vec![Command::update_visual_element_index(
                value.tab_id,
                value.proposed_index,
            )],
            UiEventBody::TabCloseRequested(value) if value.tab_id != self.rejected_tab_id => {
                vec![Command::destroy_visual_element(value.tab_id)]
            }
            UiEventBody::TabCloseRequested(_) => Vec::new(),
            _ => return Err(EngineError::new("unexpected UI event")),
        };
        if commands.is_empty() {
            return Ok(Response::empty(self.session_id));
        }
        Ok(Response::batch(
            Batch::new(
                BatchId::new_v4(),
                self.session_id,
                vec![ParallelCommandGroup::new(commands)],
            )
            .caused_by_action_id(action.action_id),
        ))
    }

    fn poll(&mut self) -> Result<Option<Response>, EngineError> {
        Ok(None)
    }
}

#[test]
fn tab_proposals_remain_controlled_and_fake_order_matches_responses() {
    let session_id = SessionId::new_v4();
    let scene_id = SceneId::new_v4();
    let camera_id = ObjectId::new_v4();
    let view_id = ObjectId::new_v4();
    let first_id = ObjectId::new_v4();
    let second_id = ObjectId::new_v4();
    let third_id = ObjectId::new_v4();
    let document = UiDocument::new(ObjectId::new_v4()).child(
        UiNode::new(
            view_id,
            TabView::new()
                .selected_tab_index(0)
                .reorderable(true)
                .events([
                    UiEventKind::TabSelectionRequested,
                    UiEventKind::TabCloseRequested,
                    UiEventKind::TabReorderRequested,
                ]),
        )
        .child(tab(first_id, "BOARD"))
        .child(tab(second_id, "NOTES"))
        .child(tab(third_id, "LOG")),
    );
    let snapshot = Snapshot::new(
        session_id,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id, "test/scene")],
        vec![GameObject::new(camera_id, CameraState::new())],
        camera_id,
    )
    .ui_document_with(document, battlement::ParentScene::Persistent, |state| state);
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&events);
    let mut catalog = FakeAssetCatalog::new();
    catalog.add_scene("test/scene");
    let mut client = FakeClient::connect(
        TabEngine {
            session_id,
            snapshot: Some(snapshot),
            rejected_tab_id: first_id,
            events: recorded,
        },
        Arc::new(catalog),
    );

    client.ui().tab_select(view_id, 2);
    let ui = client.ui();
    let battlement::UiElement::TabView(state) = ui.element(view_id).element() else {
        panic!("tab view changed kind");
    };
    assert_eq!(state.selected_tab_index, Some(2));

    client.ui().tab_reorder(view_id, 2, 0);
    assert_eq!(client.ui().element(view_id).children()[0], third_id);
    let ui = client.ui();
    let battlement::UiElement::TabView(state) = ui.element(view_id).element() else {
        panic!("tab view changed kind");
    };
    assert_eq!(state.selected_tab_index, Some(2));

    client.ui().tab_close(view_id, 1);
    assert!(
        client.ui().contains(first_id),
        "rejected tab must remain live"
    );
    client.ui().tab_close(view_id, 2);
    assert!(
        !client.ui().contains(second_id),
        "accepted tab must be destroyed"
    );

    let values = events.borrow();
    assert!(matches!(values[0], UiEventBody::TabSelectionRequested(_)));
    assert!(matches!(values[1], UiEventBody::TabReorderRequested(_)));
    assert!(matches!(values[2], UiEventBody::TabCloseRequested(_)));
    assert!(matches!(values[3], UiEventBody::TabCloseRequested(_)));
}

#[test]
fn unavailable_native_tab_gestures_emit_no_fake_actions() {
    let session_id = SessionId::new_v4();
    let scene_id = SceneId::new_v4();
    let camera_id = ObjectId::new_v4();
    let view_id = ObjectId::new_v4();
    let first_id = ObjectId::new_v4();
    let second_id = ObjectId::new_v4();
    let document = UiDocument::new(ObjectId::new_v4()).child(
        UiNode::new(
            view_id,
            TabView::new()
                .selected_tab_index(0)
                .reorderable(false)
                .events([
                    UiEventKind::TabCloseRequested,
                    UiEventKind::TabReorderRequested,
                ]),
        )
        .child(
            UiNode::new(first_id, Tab::new("BOARD").closeable(false))
                .child(UiNode::new(ObjectId::new_v4(), Label::new("BOARD content"))),
        )
        .child(tab(second_id, "NOTES")),
    );
    let snapshot = Snapshot::new(
        session_id,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id, "test/scene")],
        vec![GameObject::new(camera_id, CameraState::new())],
        camera_id,
    )
    .ui_document_with(document, battlement::ParentScene::Persistent, |state| state);
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut catalog = FakeAssetCatalog::new();
    catalog.add_scene("test/scene");
    let mut client = FakeClient::connect(
        TabEngine {
            session_id,
            snapshot: Some(snapshot),
            rejected_tab_id: first_id,
            events: Rc::clone(&events),
        },
        Arc::new(catalog),
    );

    client.ui().tab_close(view_id, 0);
    client.ui().tab_reorder(view_id, 1, 0);

    assert!(events.borrow().is_empty());
    assert_eq!(
        client.ui().element(view_id).children(),
        [first_id, second_id]
    );
}

fn tab(object_id: ObjectId, text: &str) -> UiNode {
    UiNode::new(object_id, Tab::new(text).closeable(true)).child(UiNode::new(
        ObjectId::new_v4(),
        Label::new(format!("{text} content")),
    ))
}
