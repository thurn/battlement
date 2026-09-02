use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use battlement::{
  CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, PreparedAsset, Response,
  Scene, SceneId, SessionId, Snapshot, UiDocument, UiEventAction, UiEventBody, UiEventKind,
  UiEventResponse, UiNode, UiScrollView, UiScroller, Vector,
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
      self.snapshot.take().expect("connected twice"),
    ))
  }

  fn submit(&mut self, _message: ClientMessage<(), ()>) -> Result<Response, EngineError> {
    Ok(Response::empty(self.session_id))
  }

  fn submit_ui_event(&mut self, action: UiEventAction) -> Result<UiEventResponse, EngineError> {
    let response = UiEventResponse::from_event(&action.event, Response::empty(self.session_id));
    self.actions.borrow_mut().push(action.event.body);
    Ok(response)
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn manual_clock_scroll_settlement_and_scroller_commit_match_control_contract() {
  let session_id = SessionId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let scroll_id = ObjectId::new_v4();
  let scroller_id = ObjectId::new_v4();
  let document = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(
      scroll_id,
      UiScrollView::new().events([UiEventKind::ScrollChanged, UiEventKind::ScrollSettled]),
    ))
    .child(UiNode::new(
      scroller_id,
      UiScroller::new()
        .low_value(0.0)
        .high_value(100.0)
        .value(25.0)
        .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted]),
    ));
  let snapshot = Snapshot::new(
    session_id,
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
    vec![GameObject::new(camera_id, CameraState::new())],
    camera_id,
  )
  .ui_document_with(document, battlement::ParentScene::Persistent, |state| state);
  let actions = Rc::new(RefCell::new(Vec::new()));
  let recorded = Rc::clone(&actions);
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  let (mut client, clock) = FakeClient::connect_clocked(
    move |_| RecordingEngine {
      session_id,
      snapshot: Some(snapshot),
      actions: recorded,
    },
    Arc::new(catalog),
  );

  client.ui().scroll_begin(scroll_id);
  client
    .ui()
    .scroll_change(scroll_id, Vector::new(12.0, 48.0));
  clock.advance(Duration::from_millis(100));
  client.ui().advance();
  assert_eq!(actions.borrow().len(), 1, "capture suppresses settlement");
  client.ui().scroll_end(scroll_id);
  client.ui().advance();

  client.ui().scroller_begin(scroller_id);
  client.ui().scroller_change(scroller_id, 170.0);
  client.ui().scroller_commit(scroller_id);

  let values = actions.borrow();
  assert!(matches!(values[0], UiEventBody::ScrollChanged(_)));
  assert!(matches!(values[1], UiEventBody::ScrollSettled(_)));
  assert!(matches!(
    values[2],
    UiEventBody::ValueChanging(battlement::ValueChangingEvent {
      proposed: battlement::UiValue::F32(100.0)
    })
  ));
  assert!(matches!(
    values[3],
    UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
      previous: battlement::UiValue::F32(25.0),
      proposed: battlement::UiValue::F32(100.0)
    })
  ));
}
