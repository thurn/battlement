use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
  CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, ParentScene, PreparedAsset,
  Response, Scene, SceneId, SessionId, Snapshot, UiButton, UiDocument, UiElement, UiEventAction,
  UiEventBody, UiEventDisposition, UiEventKind, UiEventResponse, UiNode, UiRadioButtonGroup,
  UiToggleButtonGroup, UiValue,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct ChoiceEngine {
  session_id: SessionId,
  snapshot: Option<Snapshot>,
  radio_id: ObjectId,
  accepted_toggle_id: ObjectId,
  events: Rc<RefCell<Vec<(ObjectId, UiValue, UiValue)>>>,
}

impl Engine for ChoiceEngine {
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
    let disposition = if action.event.default_prevented {
      UiEventDisposition::PreventDefault
    } else {
      UiEventDisposition::Continue
    };
    let event = action.event;
    let UiEventBody::ValueCommitted(value) = event.body else {
      return Err(EngineError::new("unexpected UI event"));
    };
    self.events.borrow_mut().push((
      event.target_id,
      value.previous.clone(),
      value.proposed.clone(),
    ));
    let update: Option<UiElement> = if event.target_id == self.radio_id {
      let UiValue::Index(Some(index)) = value.proposed else {
        return Err(EngineError::new("unexpected radio proposal"));
      };
      Some(UiRadioButtonGroup::new().selected_index(index).into())
    } else if event.target_id == self.accepted_toggle_id {
      let UiValue::Indices(indices) = value.proposed else {
        return Err(EngineError::new("unexpected toggle proposal"));
      };
      Some(UiToggleButtonGroup::new().selected_indices(indices).into())
    } else {
      None
    };
    let Some(element) = update else {
      return Ok(UiEventResponse::new(
        disposition,
        Response::empty(self.session_id),
      ));
    };
    Ok(UiEventResponse::new(
      disposition,
      Response::commands_for_action(
        self.session_id,
        action.action_id,
        vec![Command::update_visual_element(event.target_id, element).body],
      ),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn fake_choice_groups_propose_without_mutating_rejected_state() {
  let session_id = SessionId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let radio_id = ObjectId::new_v4();
  let accepted_toggle_id = ObjectId::new_v4();
  let rejected_toggle_id = ObjectId::new_v4();
  let disabled_toggle_id = ObjectId::new_v4();
  let controls = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(
      radio_id,
      UiRadioButtonGroup::new()
        .choices(["Line", "Wedge", "Column"])
        .selected_index(0)
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(toggle_group(accepted_toggle_id))
    .child(toggle_group(rejected_toggle_id))
    .child(
      UiNode::new(
        disabled_toggle_id,
        UiToggleButtonGroup::new()
          .multiple_selection(true)
          .allow_empty_selection(true)
          .selected_indices([0])
          .events([UiEventKind::ValueCommitted]),
      )
      .children([
        UiNode::new(ObjectId::new_v4(), UiButton::new("Enabled")),
        UiNode::new(ObjectId::new_v4(), UiButton::new("Disabled").enabled(false)),
      ]),
    );
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
    ChoiceEngine {
      session_id,
      snapshot: Some(snapshot),
      radio_id,
      accepted_toggle_id,
      events: Rc::clone(&events),
    },
    Arc::new(catalog),
  );

  client.ui().radio_group_select(radio_id, 2);
  assert_eq!(client.ui().element(radio_id).selected_index(), Some(2));
  client.ui().toggle_group_click(accepted_toggle_id, 2);
  assert_eq!(
    client.ui().element(accepted_toggle_id).selected_indices(),
    Some(&[0, 2][..])
  );
  client.ui().toggle_group_click(rejected_toggle_id, 1);
  assert_eq!(
    client.ui().element(rejected_toggle_id).selected_indices(),
    Some(&[0][..])
  );
  client.ui().toggle_group_click(disabled_toggle_id, 1);
  assert_eq!(
    client.ui().element(disabled_toggle_id).selected_indices(),
    Some(&[0][..])
  );
  assert_eq!(events.borrow().len(), 3);
}

fn toggle_group(object_id: ObjectId) -> UiNode {
  UiNode::new(
    object_id,
    UiToggleButtonGroup::new()
      .multiple_selection(true)
      .allow_empty_selection(true)
      .selected_indices([0])
      .events([UiEventKind::ValueCommitted]),
  )
  .children([
    UiNode::new(ObjectId::new_v4(), UiButton::new("Air")),
    UiNode::new(ObjectId::new_v4(), UiButton::new("Land")),
    UiNode::new(ObjectId::new_v4(), UiButton::new("Sea")),
  ])
}
