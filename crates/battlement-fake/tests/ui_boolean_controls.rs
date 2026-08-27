use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
  ActionBody, CameraState, ClientMessage, Command, CommandBody, Connect, GameObject, ObjectId,
  ParentScene, PreparedAsset, RadioButton, Response, Scene, SceneId, SessionId, Snapshot, Toggle,
  UiDocument, UiElement, UiEventBody, UiEventKind, UiNode,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct BooleanEngine {
  session_id: SessionId,
  snapshot: Option<Snapshot>,
  accepted: Vec<ObjectId>,
  gating_id: ObjectId,
  events: Rc<RefCell<Vec<(ObjectId, bool, bool)>>>,
}

impl Engine for BooleanEngine {
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
    let (battlement::UiValue::Bool(previous), battlement::UiValue::Bool(proposed)) =
      (value.previous, value.proposed)
    else {
      return Err(EngineError::new("unexpected non-Boolean proposal"));
    };
    self
      .events
      .borrow_mut()
      .push((event.target_id, previous, proposed));
    if !self.accepted.contains(&event.target_id) {
      return Ok(Response::empty(self.session_id));
    }
    let update: UiElement = if event.target_id == self.gating_id {
      Toggle::new().value(proposed).into()
    } else if event.target_id == self.accepted[1] {
      RadioButton::new().value(proposed).into()
    } else {
      Toggle::new().value(proposed).into()
    };
    let mut commands = vec![Command::update_visual_element(event.target_id, update).body];
    if event.target_id == self.gating_id {
      commands.push(CommandBody::set_input_enabled(false));
    }
    Ok(Response::commands_for_action(
      self.session_id,
      action.action_id,
      commands,
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn fake_boolean_controls_remain_authored_and_obey_input_gating() {
  let session_id = SessionId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let accepted_toggle = ObjectId::new_v4();
  let accepted_radio = ObjectId::new_v4();
  let rejected_toggle = ObjectId::new_v4();
  let disabled_radio = ObjectId::new_v4();
  let gating_id = ObjectId::new_v4();
  let controls = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(
      accepted_toggle,
      Toggle::new()
        .value(false)
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      accepted_radio,
      RadioButton::new()
        .value(false)
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      rejected_toggle,
      Toggle::new()
        .value(false)
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      disabled_radio,
      RadioButton::new()
        .value(false)
        .enabled(false)
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      gating_id,
      Toggle::new()
        .value(false)
        .events([UiEventKind::ValueCommitted]),
    ));
  let snapshot = Snapshot::new(
    session_id,
    vec![PreparedAsset::Scene("test/scene".into())],
    vec![Scene::new(scene_id, "test/scene")],
    vec![GameObject::new(camera_id, CameraState::new())],
    camera_id,
  )
  .ui_document_with(controls, ParentScene::Persistent, |state| state);
  let events = Rc::new(RefCell::new(Vec::new()));
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene("test/scene");
  let mut client = FakeClient::connect(
    BooleanEngine {
      session_id,
      snapshot: Some(snapshot),
      accepted: vec![accepted_toggle, accepted_radio, gating_id],
      gating_id,
      events: Rc::clone(&events),
    },
    Arc::new(catalog),
  );

  client.ui().toggle_click(accepted_toggle);
  assert_eq!(
    client.ui().element(accepted_toggle).bool_value(),
    Some(true)
  );
  client.ui().toggle_click(rejected_toggle);
  assert_eq!(
    client.ui().element(rejected_toggle).bool_value(),
    Some(false)
  );
  client.ui().radio_click(accepted_radio);
  assert_eq!(client.ui().element(accepted_radio).bool_value(), Some(true));
  client.ui().radio_click(disabled_radio);
  assert_eq!(events.borrow().len(), 3);

  client.ui().toggle_click(gating_id);
  assert!(!client.world().input_enabled());
  client.ui().toggle_click(accepted_toggle);
  client.ui().radio_click(accepted_radio);
  assert_eq!(events.borrow().len(), 4);
  assert_eq!(events.borrow()[0], (accepted_toggle, false, true));
  assert_eq!(events.borrow()[1], (rejected_toggle, false, true));
  assert_eq!(events.borrow()[2], (accepted_radio, false, true));
}
