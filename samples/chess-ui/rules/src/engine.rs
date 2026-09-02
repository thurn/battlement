use battlement::application::ApplicationState;
use battlement::{
  ActionBody, CameraClearMode, CameraState, ClientMessage, Command, Connect, CoreErrorCode,
  GameObject, GameObjectKind, ObjectId, PanelScaleMode, PanelSettings, ParentScene, PickingMode,
  PreparedAsset, Response, Scene, SceneId, SessionId, Snapshot, UiDocument, UiDocumentState,
  UiEventAction, UiEventResponse, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};
use battlement_reactant::{
  application,
  executor::{BoxFuture, SpawnedTask, Spawner},
  key::KeyRenderExt,
  runtime::{Reactant, ResponseReactantExt},
};

use crate::{
  gallery::Gallery, review_surface::ReviewSurface, review_theme, setting_row::DISPLAY_FONT,
};

const CAMERA_ID: ObjectId = object_id!("a5572d68-1d85-448e-b233-b490b36222b9");
const DOCUMENT_ID: ObjectId = object_id!("182c5de9-22be-4ffd-806f-cdb05eaa5d80");
const ROOT_ID: ObjectId = object_id!("4ceeebde-e265-4bc5-b7ab-e64b8d5f2074");
const SCENE_ID: SceneId = scene_id!("35f67948-4d26-4f3c-9641-eb89a2406805");

/// Standalone Chess UI rules engine.
pub struct ChessUiEngine {
  session: SessionId,
  game: Game,
  runtime: Reactant<Game>,
  document: UiDocument,
}

pub(crate) struct Game {
  pub(crate) application: ApplicationState,
  connection: u64,
  pub(crate) width: f32,
  pub(crate) height: f32,
}

struct IdleSpawner;

/// Creates the standalone review gallery.
pub fn create_engine() -> Result<ChessUiEngine, EngineError> {
  let document = ReviewSurface::document(
    UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
      .name("chess-ui")
      .picking_mode(PickingMode::Ignore),
  );
  let mut runtime = Reactant::new(IdleSpawner);
  runtime.register_root(document.clone(), |game: &Game| {
    application::provider(game.application).child(
      Gallery {
        width: game.width,
        height: game.height,
      }
      .key(game.connection),
    )
  });
  Ok(ChessUiEngine {
    session: SessionId::new_v4(),
    game: Game {
      application: ApplicationState::default(),
      connection: 0,
      width: 1280.0,
      height: 800.0,
    },
    runtime,
    document,
  })
}

impl Engine for ChessUiEngine {
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, message: Connect) -> Result<Response, EngineError> {
    self.session = SessionId::new_v4();
    self.game.connection += 1;
    self.game.application = message.application_state;
    self.game.width = message.screen.width as f32;
    self.game.height = message.screen.height as f32;
    let snapshot = self.snapshot();
    Ok(
      self
        .runtime
        .begin_session(&mut self.game)
        .expect("gallery must render")
        .into_response(snapshot),
    )
  }

  fn submit(&mut self, message: ClientMessage<()>) -> Result<Response, EngineError> {
    let Some(action) = message.into_action() else {
      return Ok(Response::empty(self.session));
    };
    let commit = match action.body {
      ActionBody::ApplicationStateChanged(state) => {
        self.game.application = state;
        self.runtime.refresh(&mut self.game)
      }
      ActionBody::GeometryObservations(batch) => {
        self.runtime.observe_geometry(&mut self.game, batch)
      }
      ActionBody::MotionEvents(batch) => self.runtime.motion_events(&mut self.game, batch),
      _ => return Ok(Response::empty(self.session)),
    }
    .expect("gallery observation must render");
    Ok(Response::empty(self.session).append_reactant_for_action(action.action_id, commit))
  }

  fn submit_ui_event(&mut self, action: UiEventAction) -> Result<UiEventResponse, EngineError> {
    assert_eq!(action.session_id, self.session, "UI event session mismatch");
    let event = self
      .runtime
      .dispatch(&mut self.game, action.event)
      .expect("gallery event must render");
    Ok(UiEventResponse::new(
      event.disposition(),
      Response::empty(self.session)
        .append_reactant_for_action(action.action_id, event.into_commit()),
    ))
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    let commit = self
      .runtime
      .poll(&mut self.game)
      .expect("gallery effects must render");
    if commit.is_empty() {
      return Ok(None);
    }
    Ok(Some(Response::empty(self.session).append_reactant(commit)))
  }
}

impl ChessUiEngine {
  fn snapshot(&self) -> Snapshot {
    Snapshot::new(
      self.session,
      vec![
        PreparedAsset::scene("chess-ui/content"),
        PreparedAsset::UiFont(DISPLAY_FONT),
      ],
      vec![Scene::new(SCENE_ID, "chess-ui/content")],
      vec![
        GameObject::new(
          CAMERA_ID,
          CameraState::new()
            .clear_mode(CameraClearMode::SolidColor)
            .clear_color(review_theme::BACKGROUND),
        )
        .parent_scene(ParentScene::Persistent),
        GameObject::new(
          self.document.document_id,
          GameObjectKind::UiDocument(
            UiDocumentState::new(self.document.root_id)
              .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize)),
          ),
        )
        .parent_scene(ParentScene::Persistent),
      ],
      CAMERA_ID,
    )
  }
}

impl Drop for ChessUiEngine {
  fn drop(&mut self) {
    let _ = self.runtime.shutdown(&mut self.game).into_groups();
  }
}

impl Spawner for IdleSpawner {
  fn spawn(&self, _task: BoxFuture<'static, ()>) -> SpawnedTask {
    panic!("Chess UI has no asynchronous resources")
  }
}

battlement_native::export_engine!(self::create_engine);
