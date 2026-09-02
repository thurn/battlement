use std::{cell::RefCell, num::NonZeroU32, rc::Rc, sync::Arc};

use battlement::{
  CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, PreparedAsset, Response,
  Scene, SceneId, SessionId, Snapshot, UiDocument, UiElementKind, UiEvent, UiEventAction,
  UiEventBody, UiEventKind, UiEventPhase, UiEventResponse, UiEventSubscription, UiNode,
  UiRepeatButton, UiVisualElement,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct RecordingEngine {
  session_id: SessionId,
  snapshot: Option<Snapshot>,
  actions: Rc<RefCell<Vec<UiEvent>>>,
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

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Ok(Response::empty(self.session_id))
  }

  fn submit_ui_event(&mut self, action: UiEventAction) -> Result<UiEventResponse, EngineError> {
    let response = UiEventResponse::from_event(&action.event, Response::empty(self.session_id));
    self.actions.borrow_mut().push(action.event);
    Ok(response)
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn navigation_submit_and_repeat_hold_emit_exact_fake_actions() {
  let session_id = SessionId::new_v4();
  let document_id = ObjectId::new_v4();
  let root_id = ObjectId::new_v4();
  let container_id = ObjectId::new_v4();
  let button_id = ObjectId::new_v4();
  let repeat_id = ObjectId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let document = UiDocument::with_root_id(document_id, root_id).child(
    UiNode::new(
      container_id,
      UiVisualElement::new()
        .events([UiEventKind::Click])
        .event_subscriptions([UiEventSubscription::new(
          UiEventKind::Click,
          UiEventPhase::Bubble,
        )]),
    )
    .child(UiNode::new(button_id, battlement::UiButton::new("Submit")))
    .child(UiNode::new(
      repeat_id,
      UiRepeatButton::new(
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
  assert_eq!(client.ui().repeat_hold(repeat_id, 650), 5);

  let values = actions.borrow();
  assert_eq!(values[0].target_id, button_id);
  assert!(matches!(
    values[0].body,
    UiEventBody::Click(battlement::ClickEvent::NavigationSubmit)
  ));
  assert_eq!(
    values
      .iter()
      .filter(|value| {
        matches!(
          value.body,
          UiEventBody::Click(battlement::ClickEvent::Repeat)
        )
      })
      .count(),
    5
  );
}
