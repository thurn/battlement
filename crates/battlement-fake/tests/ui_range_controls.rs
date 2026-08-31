use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
  ActionBody, CameraState, ClientMessage, Command, Connect, F32Range, GameObject, LowerLimit,
  ObjectId, ParentScene, PreparedAsset, Prop, Response, Scene, SceneId, SessionId, Snapshot,
  UiDocument, UiElement, UiEventBody, UiEventKind, UiMinMaxSlider, UiNode, UiProgressBar, UiValue,
  UpperLimit,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_native::{Engine, EngineError};

struct RangeEngine {
  session_id: SessionId,
  snapshot: Option<Snapshot>,
  accepted_id: ObjectId,
  events: Rc<RefCell<Vec<(ObjectId, UiEventBody)>>>,
}

impl Engine for RangeEngine {
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
    self
      .events
      .borrow_mut()
      .push((event.target_id, event.body.clone()));
    let UiEventBody::ValueCommitted(commit) = event.body else {
      return Ok(Response::empty(self.session_id));
    };
    if event.target_id != self.accepted_id {
      return Ok(Response::empty(self.session_id));
    }
    let UiValue::F32Range(proposed) = commit.proposed else {
      return Err(EngineError::new("unexpected range proposal"));
    };
    Ok(Response::commands_for_action(
      self.session_id,
      action.action_id,
      vec![
        Command::update_visual_element(
          event.target_id,
          UiMinMaxSlider::new()
            .min_value(proposed.min)
            .max_value(proposed.max),
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
fn fake_range_slider_clamps_typed_gestures_and_progress_remains_output_only() {
  let session_id = SessionId::new_v4();
  let scene_id = SceneId::new_v4();
  let camera_id = ObjectId::new_v4();
  let accepted_id = ObjectId::new_v4();
  let rejected_id = ObjectId::new_v4();
  let progress_id = ObjectId::new_v4();
  let document = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(
      accepted_id,
      UiMinMaxSlider::new()
        .low_limit(LowerLimit::Inclusive(0.0))
        .high_limit(UpperLimit::Inclusive(100.0))
        .min_value(20.0)
        .max_value(80.0)
        .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      rejected_id,
      UiMinMaxSlider::new()
        .min_value(-5.0)
        .max_value(15.0)
        .events([UiEventKind::ValueCommitted]),
    ))
    .child(UiNode::new(
      progress_id,
      UiProgressBar::new()
        .low_value(0.0)
        .high_value(100.0)
        .value(62.0)
        .title("Streaming 62%"),
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
    RangeEngine {
      session_id,
      snapshot: Some(snapshot),
      accepted_id,
      events: Rc::clone(&events),
    },
    Arc::new(catalog),
  );

  client.ui().min_max_slider_begin(accepted_id);
  client.ui().min_max_slider_change(accepted_id, -20.0, 120.0);
  assert_eq!(
    range_value(&mut client, accepted_id),
    F32Range::new(20.0, 80.0)
  );
  client.ui().min_max_slider_commit(accepted_id);
  assert_eq!(
    range_value(&mut client, accepted_id),
    F32Range::new(0.0, 100.0)
  );

  client.ui().min_max_slider_begin(rejected_id);
  client.ui().min_max_slider_change(rejected_id, -10.0, 25.0);
  client.ui().min_max_slider_commit(rejected_id);
  assert_eq!(
    range_value(&mut client, rejected_id),
    F32Range::new(-5.0, 15.0)
  );

  let progress = client.ui().element(progress_id).element().clone();
  let UiElement::ProgressBar(progress) = progress else {
    unreachable!("progress bar kind changed")
  };
  assert_eq!(progress.value, Prop::Set(62.0));
  assert_eq!(progress.title, Prop::Set("Streaming 62%".to_owned()));
  assert_eq!(progress.element.events, Prop::Unset);

  let events = events.borrow();
  assert_eq!(events.len(), 3);
  assert!(matches!(
      &events[0].1,
      UiEventBody::ValueChanging(value)
          if value.proposed == UiValue::F32Range(F32Range::new(0.0, 100.0))
  ));
  assert!(matches!(
      &events[1].1,
      UiEventBody::ValueCommitted(value)
          if value.previous == UiValue::F32Range(F32Range::new(20.0, 80.0))
              && value.proposed == UiValue::F32Range(F32Range::new(0.0, 100.0))
  ));
}

fn range_value<E>(client: &mut FakeClient<E>, object_id: ObjectId) -> F32Range
where
  E: Engine<Command = Command>,
{
  let ui = client.ui();
  let UiElement::MinMaxSlider(value) = ui.element(object_id).element() else {
    unreachable!("range slider kind changed")
  };
  F32Range::new(
    prop_f32(&value.min_value, 0.0),
    prop_f32(&value.max_value, 10.0),
  )
}

fn prop_f32(value: &Prop<f32>, reset: f32) -> f32 {
  match value {
    Prop::Set(value) => *value,
    Prop::Unset | Prop::Reset => reset,
  }
}
