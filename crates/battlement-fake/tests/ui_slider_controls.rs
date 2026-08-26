use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
    ActionBody, CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, ParentScene,
    PreparedAsset, Response, Scene, SceneId, SessionId, Slider, SliderInt, Snapshot, UiDocument,
    UiEventBody, UiEventKind, UiNode, UiValue,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct SliderEngine {
    session_id: SessionId,
    snapshot: Option<Snapshot>,
    accepted_id: ObjectId,
    events: Rc<RefCell<Vec<UiEventBody>>>,
}

impl Engine for SliderEngine {
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
        let UiEventBody::ValueCommitted(commit) = event.body else {
            return Ok(Response::empty(self.session_id));
        };
        if event.target_id != self.accepted_id {
            return Ok(Response::empty(self.session_id));
        }
        let UiValue::F32(proposed) = commit.proposed else {
            return Err(EngineError::new("unexpected slider proposal"));
        };
        Ok(Response::commands_for_action(
            self.session_id,
            action.action_id,
            vec![
                Command::update_visual_element(event.target_id, Slider::new().value(proposed)).body,
            ],
        ))
    }

    fn poll(&mut self) -> Result<Option<Response>, EngineError> {
        Ok(None)
    }
}

#[test]
fn fake_sliders_keep_drag_values_local_and_commit_clamped_typed_proposals() {
    let session_id = SessionId::new_v4();
    let scene_id = SceneId::new_v4();
    let camera_id = ObjectId::new_v4();
    let float_id = ObjectId::new_v4();
    let int_id = ObjectId::new_v4();
    let document = UiDocument::new(ObjectId::new_v4())
        .child(UiNode::new(
            float_id,
            Slider::new()
                .low_value(0.0)
                .high_value(1.0)
                .value(0.25)
                .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted]),
        ))
        .child(UiNode::new(
            int_id,
            SliderInt::new()
                .low_value(0)
                .high_value(8)
                .value(3)
                .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted]),
        ));
    let snapshot = Snapshot::new(
        session_id,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id, "test/scene")],
        vec![GameObject::new(camera_id, CameraState::new())],
        camera_id,
    )
    .ui_document_with(document, ParentScene::Persistent, |state| state);
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut catalog = FakeAssetCatalog::new();
    catalog.add_scene("test/scene");
    let mut client = FakeClient::connect(
        SliderEngine {
            session_id,
            snapshot: Some(snapshot),
            accepted_id: float_id,
            events: Rc::clone(&events),
        },
        Arc::new(catalog),
    );

    client.ui().slider_begin(float_id);
    client.ui().slider_change(float_id, 2.0);
    assert_eq!(float_value(&mut client, float_id), 0.25);
    client.ui().slider_commit(float_id);
    assert_eq!(float_value(&mut client, float_id), 1.0);

    client.ui().slider_int_begin(int_id);
    client.ui().slider_int_change(int_id, 6.6);
    assert_eq!(int_value(&mut client, int_id), 3);
    client.ui().slider_int_commit(int_id);
    assert_eq!(int_value(&mut client, int_id), 3);

    let events = events.borrow();
    assert_eq!(events.len(), 4);
    assert!(matches!(
        &events[0],
        UiEventBody::ValueChanging(value) if value.proposed == UiValue::F32(1.0)
    ));
    assert!(matches!(
        &events[2],
        UiEventBody::ValueChanging(value) if value.proposed == UiValue::I32(7)
    ));
}

fn float_value<E>(client: &mut FakeClient<E>, object_id: ObjectId) -> f32
where
    E: Engine<Command = Command>,
{
    let ui = client.ui();
    let battlement::UiElement::Slider(value) = ui.element(object_id).element() else {
        unreachable!("slider kind changed")
    };
    value.value.unwrap_or_default()
}

fn int_value<E>(client: &mut FakeClient<E>, object_id: ObjectId) -> i32
where
    E: Engine<Command = Command>,
{
    let ui = client.ui();
    let battlement::UiElement::SliderInt(value) = ui.element(object_id).element() else {
        unreachable!("slider kind changed")
    };
    value.value.unwrap_or_default()
}
