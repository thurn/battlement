use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
  CameraState, ClientMessage, Command, Connect, GameObject, ObjectId, PreparedAsset, Response,
  Scene, SceneId, SessionId, Snapshot, UiDocument, UiEventAction, UiEventBody, UiEventDisposition,
  UiEventKind, UiEventResponse, UiNode, UiTextField,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct TextEngine {
  session_id: SessionId,
  snapshot: Option<Snapshot>,
  normalized_id: ObjectId,
  events: Rc<RefCell<Vec<UiEventBody>>>,
}

impl Engine for TextEngine {
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
    self.events.borrow_mut().push(event.body.clone());
    let UiEventBody::ValueCommitted(value) = event.body else {
      return Ok(UiEventResponse::new(
        disposition,
        Response::empty(self.session_id),
      ));
    };
    let battlement::UiValue::String(proposed) = value.proposed else {
      return Err(EngineError::new("unexpected numeric text commit"));
    };
    if event.target_id != self.normalized_id {
      return Ok(UiEventResponse::new(
        disposition,
        Response::empty(self.session_id),
      ));
    }
    Ok(UiEventResponse::new(
      disposition,
      Response::commands_for_action(
        self.session_id,
        action.action_id,
        [Command::update_visual_element(
          event.target_id,
          UiTextField::new().value(proposed.trim().to_uppercase()),
        )
        .body],
      ),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    Ok(None)
  }
}

#[test]
fn fake_text_drafts_commits_selection_and_reconciliation_match_native_contract() {
  let session_id = SessionId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let normalized_id = ObjectId::new_v4();
  let rejected_id = ObjectId::new_v4();
  let quiet_id = ObjectId::new_v4();
  let read_only_id = ObjectId::new_v4();
  let document = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(
      normalized_id,
      UiTextField::new().value("Rook").events([
        UiEventKind::Input,
        UiEventKind::ValueCommitted,
        UiEventKind::SelectionChanged,
      ]),
    ))
    .child(UiNode::new(
      rejected_id,
      UiTextField::new()
        .value("Guard")
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      quiet_id,
      UiTextField::new()
        .value("Silent")
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      read_only_id,
      UiTextField::new().value("Locked").read_only(true),
    ));
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
    TextEngine {
      session_id,
      snapshot: Some(snapshot),
      normalized_id,
      events: Rc::clone(&events),
    },
    Arc::new(catalog),
  );

  client.ui().text_input(quiet_id, "local draft");
  assert_eq!(
    events.borrow().len(),
    0,
    "unsubscribed input sends no traffic"
  );
  assert_eq!(client.ui().text_draft(quiet_id), "local draft");
  client.ui().text_escape(quiet_id);
  assert_eq!(client.ui().text_draft(quiet_id), "Silent");

  client.ui().text_input(normalized_id, "  knight  ");
  assert_eq!(client.ui().text_draft(normalized_id), "  knight  ");
  client.ui().text_selection(normalized_id, 8, 2);
  client.ui().text_commit(normalized_id);
  assert_eq!(client.ui().text_draft(normalized_id), "KNIGHT");

  client.ui().text_input(rejected_id, "Scout");
  client.ui().text_commit(rejected_id);
  assert_eq!(client.ui().text_draft(rejected_id), "Guard");

  client.ui().text_input(read_only_id, "Changed");
  assert_eq!(client.ui().text_draft(read_only_id), "Locked");

  let values = events.borrow();
  assert!(matches!(values[0], UiEventBody::Input(_)));
  assert!(matches!(values[1], UiEventBody::SelectionChanged(_)));
  assert!(matches!(
      values[2],
      UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
          previous: battlement::UiValue::String(ref previous),
          proposed: battlement::UiValue::String(ref proposed),
      }) if previous == "Rook" && proposed == "  knight  "
  ));
  assert!(matches!(values[3], UiEventBody::ValueCommitted(_)));
}
