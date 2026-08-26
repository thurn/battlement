use std::{cell::RefCell, num::NonZeroU32, rc::Rc, sync::Arc};

use battlement::{
    ActionBody, CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, PreparedAsset,
    RepeatButton, Response, Scene, SceneId, SessionId, Snapshot, UiDocument, UiElementKind,
    UiEventBody, UiEventKind, UiNode, VisualElement,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct RecordingEngine {
    session_id: SessionId,
    snapshot: Option<Snapshot>,
    actions: Rc<RefCell<Vec<UiEventBody>>>,
}

impl Engine for RecordingEngine {
    type ActionPayload = ();
    type ErrorCode = ();
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response, EngineError> {
        Ok(Response::snapshot(
            self.snapshot.take().expect("engine connected twice"),
        ))
    }

    fn submit(&mut self, message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
        let ClientMessage::Action(action) = message else {
            return Err(EngineError::new("unexpected client failure"));
        };
        let ActionBody::VisualElement(event) = action.body else {
            return Err(EngineError::new("unexpected non-UI action"));
        };
        self.actions.borrow_mut().push(event.body);
        Ok(Response::empty(self.session_id))
    }

    fn poll(&mut self) -> Result<Option<Response>, EngineError> {
        Ok(None)
    }
}

#[test]
fn navigation_precedence_and_repeat_hold_emit_exact_fake_actions() {
    let session_id = SessionId::new_v4();
    let document_id = ObjectId::new_v4();
    let root_id = ObjectId::new_v4();
    let container_id = ObjectId::new_v4();
    let button_id = ObjectId::new_v4();
    let repeat_id = ObjectId::new_v4();
    let navigation_id = ObjectId::new_v4();
    let scene_id = SceneId::new_v4();
    let camera_id = ObjectId::new_v4();
    let document = UiDocument::with_root_id(document_id, root_id).child(
        UiNode::new(
            container_id,
            VisualElement::new().events([UiEventKind::Click, UiEventKind::NavigationSubmit]),
        )
        .child(UiNode::new(button_id, battlement::Button::new("Submit")))
        .child(UiNode::new(
            navigation_id,
            VisualElement::new().events([UiEventKind::NavigationSubmit]),
        ))
        .child(UiNode::new(
            repeat_id,
            RepeatButton::new(
                "Hold",
                300,
                NonZeroU32::new(100).expect("constant interval is positive"),
            ),
        )),
    );
    let snapshot = Snapshot::new(
        session_id,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id, "test/scene")],
        vec![GameObject::new(camera_id, CameraState::new())],
        camera_id,
    )
    .ui_document_with(document, battlement::ParentScene::Persistent, |state| state);
    let actions = Rc::new(RefCell::new(Vec::new()));
    let engine = RecordingEngine {
        session_id,
        snapshot: Some(snapshot),
        actions: Rc::clone(&actions),
    };
    let mut catalog = FakeAssetCatalog::new();
    catalog.add_scene("test/scene");
    let mut client = FakeClient::connect(engine, Arc::new(catalog));

    assert_eq!(client.ui().element(button_id).kind(), UiElementKind::Button);
    client.ui().navigation_submit(button_id);
    client.ui().navigation_submit(navigation_id);
    assert_eq!(client.ui().repeat_hold(repeat_id, 650), 5);

    let values = actions.borrow();
    assert!(matches!(
        values[0],
        UiEventBody::Click(battlement::ClickEvent::NavigationSubmit)
    ));
    assert!(matches!(values[1], UiEventBody::NavigationSubmit));
    assert_eq!(
        values
            .iter()
            .filter(|value| matches!(value, UiEventBody::Click(battlement::ClickEvent::Repeat)))
            .count(),
        5
    );
    assert_eq!(
        values
            .iter()
            .filter(|value| matches!(value, UiEventBody::NavigationSubmit))
            .count(),
        1
    );
}
