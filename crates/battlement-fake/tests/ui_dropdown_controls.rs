use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
  ActionBody, CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, ParentScene,
  PreparedAsset, Response, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDropdownField,
  UiEventBody, UiEventKind, UiNode, UiValue,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct DropdownEngine {
  session_id: SessionId,
  snapshot: Option<Snapshot>,
  accepted_id: ObjectId,
  events: Rc<RefCell<Vec<(ObjectId, UiValue, UiValue)>>>,
}

impl Engine for DropdownEngine {
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
    let UiEventBody::ValueCommitted(value) = event.body else {
      return Err(EngineError::new("unexpected UI event"));
    };
    self.events.borrow_mut().push((
      event.target_id,
      value.previous.clone(),
      value.proposed.clone(),
    ));
    if event.target_id != self.accepted_id {
      return Ok(Response::empty(self.session_id));
    }
    let UiValue::Choice(selection) = value.proposed else {
      return Err(EngineError::new("unexpected dropdown proposal"));
    };
    Ok(Response::commands_for_action(
      self.session_id,
      action.action_id,
      vec![
        Command::update_visual_element(
          event.target_id,
          UiDropdownField::new().selection_value(selection),
        )
        .body,
      ],
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn fake_dropdown_proposals_preserve_rejected_state_and_accept_clears() {
  let session_id = SessionId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let accepted_id = ObjectId::new_v4();
  let rejected_id = ObjectId::new_v4();
  let document = UiDocument::new(ObjectId::new_v4())
    .child(dropdown(accepted_id))
    .child(dropdown(rejected_id));
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
    DropdownEngine {
      session_id,
      snapshot: Some(snapshot),
      accepted_id,
      events: Rc::clone(&events),
    },
    Arc::new(catalog),
  );

  client.ui().dropdown_select(accepted_id, 2);
  assert_eq!(
    client.ui().element(accepted_id).choice(),
    Some(&battlement::Choice::selected(2, "Dense"))
  );
  client.ui().dropdown_select(rejected_id, 1);
  assert_eq!(
    client.ui().element(rejected_id).choice(),
    Some(&battlement::Choice::selected(0, "Comfort"))
  );
  client.ui().dropdown_clear(accepted_id);
  assert_eq!(
    client.ui().element(accepted_id).choice(),
    Some(&battlement::Choice::none())
  );
  assert_eq!(events.borrow().len(), 3);
}

fn dropdown(object_id: ObjectId) -> UiNode {
  UiNode::new(
    object_id,
    UiDropdownField::new()
      .choices(["Comfort", "Compact", "Dense"])
      .selection(0, "Comfort")
      .events([UiEventKind::ValueCommitted]),
  )
}
