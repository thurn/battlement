//! Native rules engine for the standalone basic sample.

use battlement::{ObjectId, SceneId, SessionId, object_id, scene_id};
use battlement_native::{
  CoreActionBodyView, CoreClientMessageView, Engine, EngineError, EngineResponse,
  FlatBufferSubmitError, MessageWriter, NativeBatchStart, NativeDragMode, NativeEasing,
  NativeObjectPlacement, NativeParentScene, NativePointerEvent, NativePreparedAssetKind,
  NativeTransform, UiEventActionView, UiEventResult,
};

const SCENE_ID: SceneId = scene_id!("cfd68d2d-e6d4-4b6c-a259-c729cd7e190c");

/// Address of the sample's content scene.
pub const CONTENT_SCENE: &str = "basic/content";
/// Machine-readable registry consumed by the Ditto coverage checker.
pub const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");
/// Address of the cubes' initial material.
pub const WHITE_MATERIAL: &str = "basic/material/white";
/// Address of the cubes' hover material.
pub const YELLOW_MATERIAL: &str = "basic/material/yellow";
/// Address of the material applied by the polled response.
pub const BLUE_MATERIAL: &str = "basic/material/blue";
/// Address of the sample's text font.
pub const FONT: &str = "basic/font";
/// Stable identity of the sample's input camera.
pub const CAMERA_ID: ObjectId = object_id!("54ad5cfa-5698-42e5-b32d-01da99539bfc");
/// Stable identity of the visible diagnostic status text.
pub const STATUS_ID: ObjectId = object_id!("2a188803-9663-43a0-b79b-7884f44d23a8");
/// Stable identities of the sample's interactive cubes.
pub const CUBE_IDS: [ObjectId; 3] = [
  object_id!("9c8921d4-ab2a-4287-a678-68ae3880a6f7"),
  object_id!("93c29a0f-1d4e-4aed-b797-011d730036cc"),
  object_id!("ab96efc3-f6f8-46b8-ad99-3e8f4319c2a0"),
];

/// Finite user-visible states recognized by the Basic engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualState {
  Connected,
  Hovered,
  HoverRestored,
  ClickPlaced,
  ClickRestored,
  DragInFlight,
  DragPlaced,
}

impl VisualState {
  /// Every visual state in registry order.
  pub const ALL: [Self; 7] = [
    Self::Connected,
    Self::Hovered,
    Self::HoverRestored,
    Self::ClickPlaced,
    Self::ClickRestored,
    Self::DragInFlight,
    Self::DragPlaced,
  ];

  /// Returns the canonical Ditto registry key.
  pub const fn registry_key(self) -> &'static str {
    match self {
      Self::Connected => "connected.initial",
      Self::Hovered => "hover.changed",
      Self::HoverRestored => "hover.restored",
      Self::ClickPlaced => "click.placed",
      Self::ClickRestored => "click.restored",
      Self::DragInFlight => "drag.in-flight",
      Self::DragPlaced => "drag.placed",
    }
  }
}

/// Native basic-sample rules engine.
pub struct BasicEngine {
  session_id: SessionId,
  positions: [bool; 3],
  poll_target: Option<ObjectId>,
  polled_change_delivered: bool,
  last_action: &'static str,
  visual_state: VisualState,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<BasicEngine, EngineError> {
  Ok(BasicEngine {
    session_id: SessionId::new_v4(),
    positions: [false; 3],
    poll_target: None,
    polled_change_delivered: false,
    last_action: "none",
    visual_state: VisualState::Connected,
  })
}

impl BasicEngine {
  /// Returns the current user-visible state classification.
  pub const fn visual_state(&self) -> VisualState {
    self.visual_state
  }
}

impl Engine for BasicEngine {
  const WIRE_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_DIGEST_C;

  fn connect(
    &mut self,
    _message: battlement_native::ConnectView<'_>,
  ) -> Result<EngineResponse, EngineError> {
    self.session_id = SessionId::new_v4();
    self.positions = [false; 3];
    self.poll_target = None;
    self.polled_change_delivered = false;
    self.last_action = "none";
    self.visual_state = VisualState::Connected;
    self::native_snapshot(self.session_id)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    let message = CoreClientMessageView::read(bytes)
      .map_err(|error| FlatBufferSubmitError::invalid_argument(error.to_string()))?;
    let CoreClientMessageView::Action(action) = message else {
      return self::native_empty(self.session_id).map_err(FlatBufferSubmitError::engine);
    };
    if action.session_id() != self::session_bytes(self.session_id) {
      return Err(FlatBufferSubmitError::engine(EngineError::new(
        "basic action session mismatch",
      )));
    }
    let (object_id, action_name, command_name, response_command) = match action.body() {
      CoreActionBodyView::PointerEnter(payload) => {
        self.visual_state = VisualState::Hovered;
        (
          payload.object_id(),
          "pointer enter",
          "target → yellow",
          NativeBasicCommand::Material(YELLOW_MATERIAL),
        )
      }
      CoreActionBodyView::PointerExit(payload) => {
        self.visual_state = VisualState::HoverRestored;
        (
          payload.object_id(),
          "pointer exit",
          "target → white",
          NativeBasicCommand::Material(WHITE_MATERIAL),
        )
      }
      CoreActionBodyView::Activate(payload) => {
        let object_id = payload.object_id();
        let Some(index) = self::cube_index_bytes(object_id) else {
          return self::native_empty(self.session_id).map_err(FlatBufferSubmitError::engine);
        };
        self.positions[index] = !self.positions[index];
        self.visual_state = if self.positions[index] {
          VisualState::ClickPlaced
        } else {
          VisualState::ClickRestored
        };
        let x = -2.0 + index as f64 * 2.0;
        let z = if self.positions[index] { 2.0 } else { 0.0 };
        (
          object_id,
          "pointer click",
          "500 ms move tween",
          NativeBasicCommand::Tween([x, 0.0, z]),
        )
      }
      CoreActionBodyView::PointerClick(payload) => {
        let object_id = payload.object_id();
        let Some(index) = self::cube_index_bytes(object_id) else {
          return self::native_empty(self.session_id).map_err(FlatBufferSubmitError::engine);
        };
        self.positions[index] = !self.positions[index];
        self.visual_state = if self.positions[index] {
          VisualState::ClickPlaced
        } else {
          VisualState::ClickRestored
        };
        let x = -2.0 + index as f64 * 2.0;
        let z = if self.positions[index] { 2.0 } else { 0.0 };
        (
          object_id,
          "pointer click",
          "500 ms move tween",
          NativeBasicCommand::Tween([x, 0.0, z]),
        )
      }
      CoreActionBodyView::DragStart(payload) => {
        self.visual_state = VisualState::DragInFlight;
        (
          payload.object_id(),
          "drag start",
          "local pointer capture",
          NativeBasicCommand::None,
        )
      }
      CoreActionBodyView::DragEnd(payload) => {
        self.visual_state = VisualState::DragPlaced;
        (
          payload.object_id(),
          "drag end",
          "commit world position",
          NativeBasicCommand::WorldPosition(payload.world_position()),
        )
      }
      _ => return self::native_empty(self.session_id).map_err(FlatBufferSubmitError::engine),
    };
    if !self.polled_change_delivered && self.poll_target.is_none() {
      self.poll_target = self::cube_index_bytes(object_id)
        .map(|index| self::cube_id((index + 2) % self.positions.len()));
    }
    self.last_action = action_name;
    self::native_commands(
      self.session_id,
      Some(action.action_id()),
      object_id,
      response_command,
      self.visual_state,
      action_name,
      command_name,
      "immediate",
    )
    .map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    if action.session_id() != self::session_bytes(self.session_id) {
      return Err(EngineError::new("UI event session mismatch"));
    }
    Ok(UiEventResult {
      disposition: if action.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      response: self::native_empty(self.session_id)?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    let Some(object_id) = self.poll_target.take() else {
      return Ok(None);
    };
    self.polled_change_delivered = true;
    let label = (b'A' + self::cube_index(object_id).expect("poll target is a cube") as u8) as char;
    let command = format!("cube {label} → blue");
    self::native_commands(
      self.session_id,
      None,
      self::object_bytes(object_id),
      NativeBasicCommand::Material(BLUE_MATERIAL),
      self.visual_state,
      self.last_action,
      &command,
      "polled",
    )
    .map(Some)
  }
}

#[derive(Clone, Copy)]
enum NativeBasicCommand<'a> {
  None,
  Material(&'a str),
  Tween([f64; 3]),
  WorldPosition([f64; 3]),
}

#[allow(clippy::too_many_arguments)]
fn native_commands(
  session_id: SessionId,
  action_id: Option<[u8; 16]>,
  object_id: [u8; 16],
  response_command: NativeBasicCommand<'_>,
  state: VisualState,
  action: &str,
  command: &str,
  response: &str,
) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let mut writer = MessageWriter::default();
  let mut commands = Vec::with_capacity(2);
  let command_id = || *battlement::CommandId::new_v4().as_uuid().as_bytes();
  match response_command {
    NativeBasicCommand::None => {}
    NativeBasicCommand::Material(address) => commands.push(
      writer
        .set_material(command_id(), true, object_id, address, None)
        .map_err(self::writer_error)?,
    ),
    NativeBasicCommand::Tween(position) => commands.push(
      writer
        .tween_local_position(
          command_id(),
          true,
          object_id,
          position,
          500,
          NativeEasing::InOutSine,
        )
        .map_err(self::writer_error)?,
    ),
    NativeBasicCommand::WorldPosition(position) => commands.push(
      writer
        .set_world_position(command_id(), true, object_id, position)
        .map_err(self::writer_error)?,
    ),
  }
  commands.push(
    writer
      .text_set_content(
        command_id(),
        true,
        self::object_bytes(STATUS_ID),
        &self::status(state, action, command, response),
      )
      .map_err(self::writer_error)?,
  );
  let group = writer
    .parallel_group(&commands)
    .map_err(self::writer_error)?;
  let batch = writer
    .batch(
      *battlement::BatchId::new_v4().as_uuid().as_bytes(),
      session,
      action_id,
      NativeBatchStart::Now,
      &[group],
    )
    .map_err(self::writer_error)?;
  let message = writer
    .finish(session, &[batch])
    .map_err(self::writer_error)?;
  EngineResponse::from_core(session, message)
}

fn native_empty(session_id: SessionId) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let message = MessageWriter::default()
    .finish(session, &[])
    .map_err(self::writer_error)?;
  EngineResponse::from_core(session, message)
}

fn native_snapshot(session_id: SessionId) -> Result<EngineResponse, EngineError> {
  let session = self::session_bytes(session_id);
  let mut writer = MessageWriter::with_capacity(16 * 1024);
  let assets = [
    writer.prepared_asset(NativePreparedAssetKind::Scene, CONTENT_SCENE),
    writer.prepared_asset(NativePreparedAssetKind::Material, WHITE_MATERIAL),
    writer.prepared_asset(NativePreparedAssetKind::Material, YELLOW_MATERIAL),
    writer.prepared_asset(NativePreparedAssetKind::Material, BLUE_MATERIAL),
    writer.prepared_asset(NativePreparedAssetKind::TextMeshProFont, FONT),
  ];
  let scene = writer
    .scene(self::scene_bytes(SCENE_ID), CONTENT_SCENE)
    .map_err(self::writer_error)?;
  let camera = writer
    .camera_object(
      self::object_bytes(CAMERA_ID),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 2.8, -11.0],
          rotation: [0.12, 0.0, 0.0, 0.993],
          ..Default::default()
        },
        ..Default::default()
      },
      52.0,
      [0.025, 0.035, 0.065, 1.0],
    )
    .map_err(self::writer_error)?;
  let status = writer
    .text_object(
      self::object_bytes(STATUS_ID),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 3.25, 1.0],
          ..Default::default()
        },
        ..Default::default()
      },
      &self::status(
        VisualState::Connected,
        "none",
        "initial snapshot",
        "connect",
      ),
      FONT,
      1.8,
      Some(18.0),
    )
    .map_err(self::writer_error)?;
  let mut objects = vec![camera, status];
  const POINTER_EVENTS: &[NativePointerEvent] = &[
    NativePointerEvent::Enter,
    NativePointerEvent::Exit,
    NativePointerEvent::Click,
  ];
  for index in 0..3 {
    let cube = writer
      .cube_object(
        self::object_bytes(self::cube_id(index)),
        NativeObjectPlacement {
          transform: NativeTransform {
            position: [-2.0 + index as f64 * 2.0, 0.0, 0.0],
            scale: [1.4; 3],
            ..Default::default()
          },
          pointer_events: POINTER_EVENTS,
          drag_mode: match index {
            0 => NativeDragMode::SnapToPointer,
            1 => NativeDragMode::PreserveOffset,
            _ => NativeDragMode::None,
          },
          ..Default::default()
        },
        &[(0, WHITE_MATERIAL)],
      )
      .map_err(self::writer_error)?;
    objects.push(cube);
    let label = writer
      .text_object(
        self::object_bytes(self::label_id(index)),
        NativeObjectPlacement {
          transform: NativeTransform {
            position: [-2.0 + index as f64 * 2.0, 1.3, 0.0],
            ..Default::default()
          },
          ..Default::default()
        },
        &((b'A' + index as u8) as char).to_string(),
        FONT,
        2.5,
        None,
      )
      .map_err(self::writer_error)?;
    objects.push(label);
  }
  let snapshot = writer
    .snapshot(
      session,
      &assets,
      &[scene],
      Some(self::scene_bytes(SCENE_ID)),
      &objects,
      Some(self::object_bytes(CAMERA_ID)),
    )
    .map_err(self::writer_error)?;
  let message = writer
    .finish(session, &[snapshot])
    .map_err(self::writer_error)?;
  EngineResponse::from_core(session, message)
}

fn writer_error(error: impl std::fmt::Display) -> EngineError {
  EngineError::new(error.to_string())
}

fn session_bytes(id: SessionId) -> [u8; 16] {
  *id.as_uuid().as_bytes()
}

fn object_bytes(id: ObjectId) -> [u8; 16] {
  *id.as_uuid().as_bytes()
}

fn scene_bytes(id: SceneId) -> [u8; 16] {
  *id.as_uuid().as_bytes()
}

fn cube_index_bytes(id: [u8; 16]) -> Option<usize> {
  (0..3).find(|index| self::object_bytes(self::cube_id(*index)) == id)
}

fn status(state: VisualState, action: &str, command: &str, response: &str) -> String {
  format!(
    "Battlement — Basic Native Sample\nA: snap drag  •  B: offset drag  •  C: click tween\n\
         Running  •  native battlement_rules\n\
         visual state: {}\n\
         last action: {action}  •  last command: {command}  •  response: {response}",
    state.registry_key()
  )
}

fn cube_index(id: ObjectId) -> Option<usize> {
  (0..3).find(|index| self::cube_id(*index) == id)
}

fn cube_id(index: usize) -> ObjectId {
  CUBE_IDS[index]
}

fn label_id(index: usize) -> ObjectId {
  [
    object_id!("8aaf3f5a-c30a-4b57-9e57-83492ae48f92"),
    object_id!("1a4b48f1-d599-470e-9388-6965edb45798"),
    object_id!("6812d3b2-151e-46d7-a4b9-d41c36c44f33"),
  ][index]
}

battlement_native::export_deterministic_engine!(
  create_engine,
  clock = virtualized,
  randomness = seeded,
  external_state = isolated,
  persistent_state = reset,
  input = semantic,
  visible_output = flatbuffers,
);
