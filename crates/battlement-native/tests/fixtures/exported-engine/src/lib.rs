#![deny(missing_docs)]

//! Dynamically loaded engine fixture for the exported Battlement C ABI.

mod fixture_response;
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod fixture_response_generated;
mod release_scenarios;

use std::sync::{
  Mutex, OnceLock,
  atomic::{AtomicUsize, Ordering},
};

use battlement::{
  AnyCommand, Batch, BatchId, Command, CommandBody, CommandId, ParallelCommandGroup, Response,
  ResponseMessage, SessionId, TextContentPayload,
};
use battlement_native::{
  ConnectView, CoreActionBodyView, CoreClientMessageView, Engine, EngineError, EngineResponse,
  FlatBufferSubmitError, MessageWriter, NativeBatchStart, NativeDragMode, NativeImageFit,
  NativeObjectPlacement, NativeParentScene, NativePointerEvent, NativePreparedAssetKind,
  NativeTransform, UiEventActionView, UiEventResult,
};
pub use reactant_rules::platform_proof::{ProofObserver, ProofSnapshot};
use reactant_rules::platform_proof::{ProofTask, WorkerProof};

pub use release_scenarios::FlashPayload;
use release_scenarios::ReleaseScenario;

/// Custom command type that selects the exported worker-cancellation scenario.
pub const WORKER_COMMAND_TYPE: &str = "fixture.release.worker-cancellation";
/// Scene address required by the exported worker-cancellation scenario.
pub const WORKER_SCENE_ADDRESS: &str = "battlement/integration/scene";

static SUBMIT_CALLS: AtomicUsize = AtomicUsize::new(0);
static CONNECT_CALLS: AtomicUsize = AtomicUsize::new(0);
static NEXT_ENGINE_ID: AtomicUsize = AtomicUsize::new(1);
static WORKER_OBSERVER: OnceLock<Mutex<Option<ProofObserver>>> = OnceLock::new();

/// Stateful fixture exposed through the native plugin ABI.
pub struct FixtureEngine {
  engine_id: usize,
  mode: String,
  session_id: SessionId,
  release_scenario: Option<ReleaseScenario>,
  connect_count: usize,
  poll_count: usize,
  worker_proof: Option<WorkerProof>,
  worker_action_started: bool,
}

/// Explicit engine and observer service for worker-lifecycle tests.
pub struct WorkerFixture {
  engine: FixtureEngine,
  observer: ProofObserver,
}

impl WorkerFixture {
  /// Creates a fresh exported engine with an attached worker-proof observer.
  pub fn create() -> Result<Self, EngineError> {
    let mut engine = create_engine()?;
    let proof = WorkerProof::new();
    let observer = proof.observer();
    *worker_observer().lock().unwrap() = Some(observer.clone());
    engine.worker_proof = Some(proof);
    Ok(Self { engine, observer })
  }

  /// Separates the real exported engine from its event-driven observer.
  #[must_use]
  pub fn into_parts(self) -> (FixtureEngine, ProofObserver) {
    (self.engine, self.observer)
  }
}

/// Returns the object whose public click action enters the waiting proof barrier.
#[must_use]
pub fn worker_action_object_id() -> battlement::ObjectId {
  release_scenarios::object_id(70)
}

impl Drop for FixtureEngine {
  fn drop(&mut self) {
    tracing::event!(
      name: "fixture.engine.destroyed",
      tracing::Level::INFO,
      engine_id = self.engine_id,
      "Destroyed fixture engine"
    );
    if self.mode == "panic-destroy" {
      panic!("fixture destroy panic");
    }
  }
}

impl Engine for FixtureEngine {
  const WIRE_DIGEST_C: &'static [u8; 65] = fixture_response::WIRE_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    if ReleaseScenario::from_connect(message).is_some_and(ReleaseScenario::is_integration) {
      CONNECT_CALLS.fetch_add(1, Ordering::Relaxed);
      tracing::event!(
        name: "fixture.engine.connected",
        tracing::Level::INFO,
        engine_id = self.engine_id,
        platform = %message.platform(),
        "Connected fixture engine"
      );
      self.mode = message.platform().to_owned();
      self.connect_count += 1;
      self.poll_count = 0;
      self.release_scenario = Some(ReleaseScenario::IntegrationFixture);
      self.session_id = SessionId::new_v4();
      return direct_integration_snapshot(*self.session_id.as_uuid().as_bytes());
    }
    CONNECT_CALLS.fetch_add(1, Ordering::Relaxed);
    self.mode = message.platform().to_owned();
    tracing::event!(
      name: "fixture.engine.connected",
      tracing::Level::INFO,
      engine_id = self.engine_id,
      platform = %message.platform(),
      "Connected fixture engine"
    );
    self.connect_count += 1;
    self.poll_count = 0;
    self.release_scenario = ReleaseScenario::from_connect(message);
    if let Some(scenario) = self.release_scenario {
      if matches!(scenario, ReleaseScenario::WorkerCancellation) {
        self.prepare_worker_proof();
      }
      self.session_id = SessionId::new_v4();
      return native_response(scenario.connect_response(self.session_id));
    }
    if self.mode == "panic-connect" {
      tracing::event!(
        name: "fixture.connect.panic.preparing",
        tracing::Level::INFO,
        "Preparing fixture connect panic"
      );
      tracing::event!(
        name: "fixture.connect.panic.triggering",
        tracing::Level::INFO,
        "Triggering fixture connect panic"
      );
      panic!("fixture connect panic");
    }
    if self.mode == "engine-error" {
      return Err(EngineError::new("fixture engine error"));
    }
    if self.mode == "maximum-response" {
      return native_response(sized_response(self.session_id, 16 * 1024 * 1024));
    }
    if self.mode == "oversized-response" {
      return native_response(sized_response(self.session_id, 16 * 1024 * 1024 + 1));
    }
    direct_empty_response(*self.session_id.as_uuid().as_bytes())
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    if matches!(
      self.mode.as_str(),
      "direct-native-label"
        | "direct-native-image"
        | "direct-native-transforms"
        | "direct-native-ui-scalars"
    ) {
      let message = CoreClientMessageView::read(bytes)
        .map_err(|error| FlatBufferSubmitError::invalid_argument(error.to_string()))?;
      let CoreClientMessageView::Action(action) = message else {
        return Err(FlatBufferSubmitError::invalid_argument(
          "direct command mode requires a core action",
        ));
      };
      if action.session_id() != *self.session_id.as_uuid().as_bytes() {
        return Err(FlatBufferSubmitError::engine(EngineError::new(
          "fixture client message session mismatch",
        )));
      }
      SUBMIT_CALLS.fetch_add(1, Ordering::Relaxed);
      return match self.mode.as_str() {
        "direct-native-image" => direct_image_response(*self.session_id.as_uuid().as_bytes()),
        "direct-native-transforms" => {
          direct_transform_response(*self.session_id.as_uuid().as_bytes())
        }
        "direct-native-ui-scalars" => {
          direct_ui_scalar_response(*self.session_id.as_uuid().as_bytes())
        }
        _ => direct_label_response(*self.session_id.as_uuid().as_bytes()),
      }
      .map_err(FlatBufferSubmitError::engine);
    }
    if self
      .release_scenario
      .is_some_and(ReleaseScenario::is_integration)
    {
      let message = CoreClientMessageView::read(bytes)
        .map_err(|error| FlatBufferSubmitError::invalid_argument(error.to_string()))?;
      let CoreClientMessageView::Action(action) = message else {
        return direct_empty_response(*self.session_id.as_uuid().as_bytes())
          .map_err(FlatBufferSubmitError::engine);
      };
      if action.session_id() != *self.session_id.as_uuid().as_bytes() {
        return Err(FlatBufferSubmitError::engine(EngineError::new(
          "fixture client message session mismatch",
        )));
      }
      SUBMIT_CALLS.fetch_add(1, Ordering::Relaxed);
      let CoreActionBodyView::PointerClick(payload) = action.body() else {
        return direct_empty_response(*self.session_id.as_uuid().as_bytes())
          .map_err(FlatBufferSubmitError::engine);
      };
      if payload.object_id() != *release_scenarios::object_id(3701).as_uuid().as_bytes() {
        return direct_empty_response(*self.session_id.as_uuid().as_bytes())
          .map_err(FlatBufferSubmitError::engine);
      }
      return direct_integration_response(*self.session_id.as_uuid().as_bytes())
        .map_err(FlatBufferSubmitError::engine);
    }
    if let Ok(message) = CoreClientMessageView::read(bytes) {
      SUBMIT_CALLS.fetch_add(1, Ordering::Relaxed);
      if self.mode == "panic-submit" {
        panic!("fixture submit panic");
      }
      if matches!(
        self.release_scenario,
        Some(ReleaseScenario::WorkerCancellation)
      ) {
        self.apply_worker_action(message)?;
      }
      let response = self.release_scenario.map_or_else(
        || Response::new(self.session_id, Vec::new()),
        |scenario| scenario.submit_core_response(self.session_id, message),
      );
      return native_response(response).map_err(FlatBufferSubmitError::engine);
    }
    let session = fixture_response::validate_fixture_client(bytes)
      .map_err(|error| FlatBufferSubmitError::invalid_argument(error.to_string()))?;
    if session != *self.session_id.as_uuid().as_bytes() {
      return Err(FlatBufferSubmitError::engine(EngineError::new(
        "fixture client message session mismatch",
      )));
    }
    SUBMIT_CALLS.fetch_add(1, Ordering::Relaxed);
    if self.mode == "panic-submit" {
      panic!("fixture submit panic");
    }
    direct_empty_response(session).map_err(FlatBufferSubmitError::engine)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    if action.session_id() != *self.session_id.as_uuid().as_bytes() {
      return Err(EngineError::new("fixture UI event session mismatch"));
    }
    if self.mode == "panic-ui-event" {
      panic!("fixture UI event panic");
    }
    if self.mode == "ui-event-error" {
      return Err(EngineError::new("fixture UI event error"));
    }
    Ok(UiEventResult {
      disposition: if action.default_prevented() {
        battlement::UiEventDisposition::PreventDefault
      } else {
        battlement::UiEventDisposition::Continue
      },
      response: direct_empty_response(*self.session_id.as_uuid().as_bytes())?,
    })
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    if let Some(scenario) = self.release_scenario {
      let result = scenario.poll_response(self.session_id, self.connect_count, self.poll_count)?;
      self.poll_count += 1;
      return result.map(native_response).transpose();
    }
    if self.mode == "panic-poll" {
      panic!("fixture poll panic");
    }
    if self.mode == "poll-response" {
      return direct_empty_response(*self.session_id.as_uuid().as_bytes()).map(Some);
    }
    Ok(None)
  }
}

impl FixtureEngine {
  fn prepare_worker_proof(&mut self) {
    let proof = self.worker_proof.get_or_insert_with(|| {
      let proof = WorkerProof::new();
      *worker_observer().lock().unwrap() = Some(proof.observer());
      proof
    });
    let task = match self.connect_count {
      2 | 8 => Some(ProofTask::Computation),
      3..=5 | 7 => Some(ProofTask::Complete),
      6 => Some(ProofTask::Panic),
      _ => None,
    };
    if let Some(task) = task {
      proof.replace(task);
    }
  }

  fn apply_worker_action(
    &mut self,
    message: CoreClientMessageView<'_>,
  ) -> Result<(), FlatBufferSubmitError> {
    let CoreClientMessageView::Action(action) = message else {
      return Ok(());
    };
    if action.session_id() != *self.session_id.as_uuid().as_bytes() {
      return Err(FlatBufferSubmitError::engine(EngineError::new(
        "fixture client message session mismatch",
      )));
    }
    let CoreActionBodyView::PointerClick(payload) = action.body() else {
      return Ok(());
    };
    let proof = self
      .worker_proof
      .as_ref()
      .expect("worker scenario must create its proof");
    let object_id = payload.object_id();
    if !self.worker_action_started
      && object_id == *release_scenarios::object_id(70).as_uuid().as_bytes()
    {
      self.worker_action_started = true;
      proof.replace(ProofTask::Wait);
    }
    Ok(())
  }
}

fn worker_observer() -> &'static Mutex<Option<ProofObserver>> {
  WORKER_OBSERVER.get_or_init(|| Mutex::new(None))
}

fn direct_label_response(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  let mut writer = MessageWriter::default();
  let label = writer.label("direct native label");
  let command = writer
    .update_visual_element([0x41; 16], true, [0x42; 16], label)
    .map_err(|error| EngineError::new(error.to_string()))?;
  let group = writer
    .parallel_group(&[command])
    .map_err(|error| EngineError::new(error.to_string()))?;
  let batch = writer
    .batch(
      [0x43; 16],
      session_id,
      None,
      NativeBatchStart::Now,
      &[group],
    )
    .map_err(|error| EngineError::new(error.to_string()))?;
  let message = writer
    .finish(session_id, &[batch])
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_composed(
    session_id,
    message,
    fixture_response::verify_fixture_response,
  )
}

fn direct_ui_scalar_response(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  let mut writer = MessageWriter::default();
  let elements = [
    writer
      .text_field_builder()
      .text_value("borrowed text")
      .finish(),
    writer.toggle_builder().bool_value(true).finish(),
    writer
      .radio_button_group_builder()
      .selected_index(Some(3))
      .finish(),
    writer
      .toggle_button_group_builder()
      .selected_indices([1, 4, 7])
      .finish(),
    writer
      .dropdown_field_builder()
      .selection(Some(2), Some("third"))
      .finish(),
    writer.scroller_builder().float_value(4.5).finish(),
    writer.slider_builder().float_value(6.25).finish(),
    writer.slider_int_builder().int_value(-12).finish(),
    writer.min_max_slider_builder().range(2.0, 8.0).finish(),
    writer.tab_view_builder().selected_tab_index(5).finish(),
    writer.button_builder().text("text only").finish(),
    writer.button_builder().enabled(true).finish(),
    writer
      .button_builder()
      .text("ready")
      .enabled(false)
      .finish(),
    writer
      .repeat_button_builder()
      .repeat_timing(200, std::num::NonZeroU32::new(100).unwrap())
      .finish(),
  ];
  let mut commands = Vec::with_capacity(elements.len());
  for (index, element) in elements.into_iter().enumerate() {
    commands.push(
      writer
        .update_visual_element(
          [0x81 + index as u8; 16],
          true,
          [0xa1 + index as u8; 16],
          element,
        )
        .map_err(writer_error)?,
    );
  }
  finish_direct_commands(writer, session_id, &commands)
}

fn direct_image_response(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  let mut writer = MessageWriter::default();
  let image = writer
    .image_object(
      [0x61; 16],
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [1.0, 2.0, 3.0],
          scale: [2.0, 3.0, 4.0],
          ..Default::default()
        },
        pointer_events: &[NativePointerEvent::Click],
        drag_mode: NativeDragMode::PreserveOffset,
        ..Default::default()
      },
      "fixture-texture",
      12.0,
      8.0,
      NativeImageFit::Contain,
    )
    .map_err(writer_error)?;
  let command = writer
    .create_object([0x62; 16], true, image)
    .map_err(writer_error)?;
  finish_direct_response(writer, session_id, command)
}

fn direct_transform_response(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  let mut writer = MessageWriter::default();
  let object_id = [0x72; 16];
  let commands = [
    writer
      .set_local_rotation([0x73; 16], true, object_id, [0.0, 0.0, 0.5, 0.5])
      .map_err(writer_error)?,
    writer
      .set_world_rotation([0x74; 16], false, object_id, [0.0, 0.25, 0.0, 0.75])
      .map_err(writer_error)?,
    writer
      .set_local_scale([0x75; 16], true, object_id, [2.0, 3.0, 4.0])
      .map_err(writer_error)?,
    writer
      .set_object_active([0x76; 16], false, object_id, false)
      .map_err(writer_error)?,
    writer
      .tween_world_position(
        [0x77; 16],
        true,
        object_id,
        [5.0, 6.0, 7.0],
        250,
        battlement_native::NativeEasing::InOutSine,
      )
      .map_err(writer_error)?,
    writer
      .tween_local_rotation(
        [0x78; 16],
        true,
        object_id,
        [0.0, 0.0, 0.25, 0.75],
        300,
        battlement_native::NativeEasing::InOutSine,
      )
      .map_err(writer_error)?,
    writer
      .tween_world_rotation(
        [0x79; 16],
        false,
        object_id,
        [0.0, 0.5, 0.0, 0.5],
        350,
        battlement_native::NativeEasing::InOutSine,
      )
      .map_err(writer_error)?,
    writer
      .tween_local_scale(
        [0x7a; 16],
        true,
        object_id,
        [6.0, 7.0, 8.0],
        400,
        battlement_native::NativeEasing::InOutSine,
      )
      .map_err(writer_error)?,
    {
      let plane = writer
        .plane_object(
          [0x7b; 16],
          NativeObjectPlacement {
            parent_scene: NativeParentScene::Persistent,
            pointer_events: &[NativePointerEvent::Click],
            ..Default::default()
          },
          &[(0, "fixture-material")],
        )
        .map_err(writer_error)?;
      writer
        .create_object([0x7c; 16], true, plane)
        .map_err(writer_error)?
    },
    {
      let prefab = writer
        .prefab_object(
          [0x7d; 16],
          NativeObjectPlacement {
            parent_scene: NativeParentScene::Persistent,
            ..Default::default()
          },
          "fixture-prefab",
          &[(1, "fixture-material")],
          Some("Idle"),
        )
        .map_err(writer_error)?;
      writer
        .create_object([0x7e; 16], false, prefab)
        .map_err(writer_error)?
    },
    {
      let empty = writer
        .empty_object(
          [0x81; 16],
          NativeObjectPlacement {
            parent_scene: NativeParentScene::Persistent,
            ..Default::default()
          },
        )
        .map_err(writer_error)?;
      writer
        .create_object([0x82; 16], false, empty)
        .map_err(writer_error)?
    },
    {
      let text = writer
        .text_object_with_color(
          [0x83; 16],
          NativeObjectPlacement {
            parent_scene: NativeParentScene::Persistent,
            ..Default::default()
          },
          "fixture text",
          "fixture-font",
          2.5,
          Some(9.0),
          [0.1, 0.2, 0.3, 0.4],
        )
        .map_err(writer_error)?;
      writer
        .create_object([0x84; 16], false, text)
        .map_err(writer_error)?
    },
    {
      let camera = writer
        .camera_object(
          [0x85; 16],
          NativeObjectPlacement {
            parent_scene: NativeParentScene::Persistent,
            ..Default::default()
          },
          70.0,
          [0.2, 0.3, 0.4, 1.0],
        )
        .map_err(writer_error)?;
      writer
        .create_object([0x86; 16], false, camera)
        .map_err(writer_error)?
    },
    {
      let light = writer
        .point_light_object(
          [0x87; 16],
          NativeObjectPlacement {
            parent_scene: NativeParentScene::Persistent,
            ..Default::default()
          },
          3.0,
          12.0,
        )
        .map_err(writer_error)?;
      writer
        .create_object([0x88; 16], false, light)
        .map_err(writer_error)?
    },
    writer
      .reparent_object([0x89; 16], false, object_id, Some([0x81; 16]), true)
      .map_err(writer_error)?,
    writer
      .spawn_particle_at_world_position([0x8a; 16], true, "fixture-effect", [9.0, 10.0, 11.0], 750)
      .map_err(writer_error)?,
    writer
      .play_audio(
        [0x8b; 16],
        false,
        "fixture-audio",
        0.75,
        1.25,
        true,
        125,
        battlement::AudioBus::Effects,
      )
      .map_err(writer_error)?,
    writer
      .play_particles([0x8c; 16], false, object_id, true)
      .map_err(writer_error)?,
    writer
      .stop_audio([0x8d; 16], false, [0x8b; 16], 200)
      .map_err(writer_error)?,
    writer
      .set_audio_volume([0x8e; 16], true, [0x8b; 16], 0.5)
      .map_err(writer_error)?,
    writer.wait([0x8f; 16], 300).map_err(writer_error)?,
    writer
      .vibrate_controller([0x90; 16], false, 0.25, 0.75, 400)
      .map_err(writer_error)?,
    writer
      .set_debug_ui([0x91; 16], true, true, false)
      .map_err(writer_error)?,
    writer
      .pause_audio([0x92; 16], true, [0x8b; 16])
      .map_err(writer_error)?,
    writer
      .resume_audio([0x93; 16], true, [0x8b; 16])
      .map_err(writer_error)?,
    writer
      .seek_audio([0x94; 16], true, [0x8b; 16], 500)
      .map_err(writer_error)?,
    writer
      .set_audio_buffering([0x95; 16], true, [0x8b; 16], true)
      .map_err(writer_error)?,
    writer
      .replace_audio([0x96; 16], true, [0x8b; 16], "replacement-audio")
      .map_err(writer_error)?,
    writer
      .tween_audio_volume(
        [0x97; 16],
        true,
        [0x8b; 16],
        0.25,
        600,
        battlement_native::NativeEasing::InOutSine,
      )
      .map_err(writer_error)?,
    writer
      .open_external_url([0x98; 16], true, "https://example.invalid/fixture")
      .map_err(writer_error)?,
    writer
      .load_scene([0x99; 16], true, [0xaa; 16], "fixture-scene", true)
      .map_err(writer_error)?,
    writer
      .unload_scene([0x9a; 16], true, [0xaa; 16])
      .map_err(writer_error)?,
    writer
      .set_primary_scene([0x9b; 16], true, [0xaa; 16])
      .map_err(writer_error)?,
    writer
      .stop_particles([0x9c; 16], false, object_id, true)
      .map_err(writer_error)?,
    writer
      .cancel_operation([0x9d; 16], true, [0x97; 16])
      .map_err(writer_error)?,
  ];
  finish_direct_commands(writer, session_id, &commands)
}

fn direct_integration_response(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  let mut writer = MessageWriter::default();
  let command = writer
    .set_local_position(
      [0x51; 16],
      true,
      *release_scenarios::object_id(3701).as_uuid().as_bytes(),
      [0.0, 1.25, 0.0],
    )
    .map_err(|error| EngineError::new(error.to_string()))?;
  finish_direct_response(writer, session_id, command)
}

fn direct_integration_snapshot(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  use release_scenarios::{
    INTEGRATION_AUDIO, INTEGRATION_EFFECT, INTEGRATION_FONT, INTEGRATION_MATERIAL,
    INTEGRATION_PREFAB, INTEGRATION_SCENE, INTEGRATION_TEXTURE,
  };

  let mut writer = MessageWriter::with_capacity(16 * 1024);
  let assets = [
    writer.prepared_asset(NativePreparedAssetKind::Scene, INTEGRATION_SCENE),
    writer.prepared_asset(NativePreparedAssetKind::Prefab, INTEGRATION_PREFAB),
    writer.prepared_asset(NativePreparedAssetKind::ParticleEffect, INTEGRATION_EFFECT),
    writer.prepared_asset(NativePreparedAssetKind::Material, INTEGRATION_MATERIAL),
    writer.prepared_asset(NativePreparedAssetKind::Texture, INTEGRATION_TEXTURE),
    writer.prepared_asset(NativePreparedAssetKind::AudioClip, INTEGRATION_AUDIO),
    writer.prepared_asset(NativePreparedAssetKind::TextMeshProFont, INTEGRATION_FONT),
  ];
  let scene_id = *release_scenarios::scene_id(3790).as_uuid().as_bytes();
  let scene = writer
    .scene(scene_id, INTEGRATION_SCENE)
    .map_err(writer_error)?;
  let camera = writer
    .orthographic_camera_object(
      *release_scenarios::object_id(3700).as_uuid().as_bytes(),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 0.0, -10.0],
          ..Default::default()
        },
        ..Default::default()
      },
      4.0,
      [0.015, 0.025, 0.07, 1.0],
    )
    .map_err(writer_error)?;
  let light = writer
    .point_light_object(
      *release_scenarios::object_id(3705).as_uuid().as_bytes(),
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        transform: NativeTransform {
          position: [0.0, 2.5, -3.0],
          ..Default::default()
        },
        ..Default::default()
      },
      3.0,
      20.0,
    )
    .map_err(writer_error)?;
  let scene_parent = NativeParentScene::Scene(scene_id);
  let target = writer
    .prefab_object(
      *release_scenarios::object_id(3701).as_uuid().as_bytes(),
      NativeObjectPlacement {
        parent_scene: scene_parent,
        pointer_events: &[
          NativePointerEvent::Enter,
          NativePointerEvent::Down,
          NativePointerEvent::Up,
          NativePointerEvent::Click,
        ],
        ..Default::default()
      },
      INTEGRATION_PREFAB,
      &[(0, INTEGRATION_MATERIAL)],
      Some("Idle"),
    )
    .map_err(writer_error)?;
  let image = writer
    .image_object(
      *release_scenarios::object_id(3702).as_uuid().as_bytes(),
      NativeObjectPlacement {
        parent_scene: scene_parent,
        transform: NativeTransform {
          position: [-2.2, 0.0, 0.0],
          ..Default::default()
        },
        ..Default::default()
      },
      INTEGRATION_TEXTURE,
      1.7,
      1.7,
      NativeImageFit::Stretch,
    )
    .map_err(writer_error)?;
  let text = writer
    .text_object(
      *release_scenarios::object_id(3703).as_uuid().as_bytes(),
      NativeObjectPlacement {
        parent_scene: scene_parent,
        transform: NativeTransform {
          position: [0.0, 2.2, 0.0],
          ..Default::default()
        },
        ..Default::default()
      },
      "REAL CONTENT",
      INTEGRATION_FONT,
      1.0,
      None,
    )
    .map_err(writer_error)?;
  let material_cube = writer
    .cube_object(
      *release_scenarios::object_id(3704).as_uuid().as_bytes(),
      NativeObjectPlacement {
        parent_scene: scene_parent,
        transform: NativeTransform {
          position: [2.2, 0.0, 0.0],
          ..Default::default()
        },
        ..Default::default()
      },
      &[(0, INTEGRATION_MATERIAL)],
    )
    .map_err(writer_error)?;
  let document_id = *release_scenarios::object_id(3706).as_uuid().as_bytes();
  let root_id = *release_scenarios::object_id(3707).as_uuid().as_bytes();
  let label_id = *release_scenarios::object_id(3708).as_uuid().as_bytes();
  let document_host = writer
    .ui_document_object(
      document_id,
      NativeObjectPlacement {
        parent_scene: NativeParentScene::Persistent,
        ..Default::default()
      },
      root_id,
    )
    .map_err(writer_error)?;
  let document_root = writer.visual_element();
  let label = writer.label("FLAT UI");
  let label_node = writer.ui_node(label_id, label, &[]).map_err(writer_error)?;
  let document = writer
    .ui_document(
      document_id,
      root_id,
      document_root,
      &[label_id],
      &[label_node],
    )
    .map_err(writer_error)?;
  let snapshot = writer
    .snapshot_with_ui(
      session_id,
      &assets,
      &[scene],
      Some(scene_id),
      &[
        camera,
        light,
        target,
        image,
        text,
        material_cube,
        document_host,
      ],
      Some(*release_scenarios::object_id(3700).as_uuid().as_bytes()),
      &[document],
    )
    .map_err(writer_error)?;
  let message = writer
    .finish(session_id, &[snapshot])
    .map_err(writer_error)?;
  EngineResponse::from_composed(
    session_id,
    message,
    fixture_response::verify_fixture_response,
  )
}

fn writer_error(error: impl std::fmt::Display) -> EngineError {
  EngineError::new(error.to_string())
}

fn direct_empty_response(session_id: [u8; 16]) -> Result<EngineResponse, EngineError> {
  let message = battlement_flatbuffers::write_empty_response(session_id)
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_composed(
    session_id,
    message,
    fixture_response::verify_fixture_response,
  )
}

fn finish_direct_response(
  writer: MessageWriter,
  session_id: [u8; 16],
  command: battlement_flatbuffers::CoreCommandOffset,
) -> Result<EngineResponse, EngineError> {
  finish_direct_commands(writer, session_id, &[command])
}

fn finish_direct_commands(
  mut writer: MessageWriter,
  session_id: [u8; 16],
  commands: &[battlement_flatbuffers::CoreCommandOffset],
) -> Result<EngineResponse, EngineError> {
  let group = writer
    .parallel_group(commands)
    .map_err(|error| EngineError::new(error.to_string()))?;
  let batch = writer
    .batch(
      [0x53; 16],
      session_id,
      None,
      NativeBatchStart::Now,
      &[group],
    )
    .map_err(|error| EngineError::new(error.to_string()))?;
  let message = writer
    .finish(session_id, &[batch])
    .map_err(|error| EngineError::new(error.to_string()))?;
  EngineResponse::from_composed(
    session_id,
    message,
    fixture_response::verify_fixture_response,
  )
}

fn native_response(
  response: Response<AnyCommand<FlashPayload>>,
) -> Result<EngineResponse, EngineError> {
  let session_id = *response.session_id.as_uuid().as_bytes();
  EngineResponse::from_composed(
    session_id,
    fixture_response::write_response(&response)?,
    fixture_response::verify_fixture_response,
  )
}

fn sized_response(session_id: SessionId, target: usize) -> Response<AnyCommand<FlashPayload>> {
  if target > battlement_flatbuffers::MAXIMUM_MESSAGE_BYTES {
    return text_response(session_id, "x".repeat(target));
  }
  let mut payload = "x".repeat(target.saturating_sub(4 * 1024));
  loop {
    let response = text_response(session_id, payload);
    let length = fixture_response::write_response(&response)
      .unwrap()
      .as_bytes()
      .len();
    if length == target {
      return response;
    }

    let command = match &response.messages[0] {
      ResponseMessage::Batch(batch) => batch.groups[0].commands[0].clone(),
      ResponseMessage::Snapshot(_) => unreachable!(),
    };
    payload = match command {
      AnyCommand::Core(Command {
        body: CommandBody::TextSetContent(text),
        ..
      }) => {
        let new_length = text.content.len() + target - length;
        "x".repeat(new_length)
      }
      _ => unreachable!(),
    };
  }
}

fn text_response(session_id: SessionId, content: String) -> Response<AnyCommand<FlashPayload>> {
  Response::new(
    session_id,
    vec![ResponseMessage::Batch(Batch::new(
      BatchId::new_v4(),
      session_id,
      vec![ParallelCommandGroup::new(vec![AnyCommand::Core(
        Command::new(
          CommandId::new_v4(),
          CommandBody::TextSetContent(TextContentPayload {
            object_id: release_scenarios::object_id(999),
            content,
          }),
        ),
      )])],
    ))],
  )
}

/// Creates a fresh engine with no active session.
pub fn create_engine() -> Result<FixtureEngine, EngineError> {
  match std::env::var("BATTLEMENT_EXPORT_FIXTURE_CREATE").as_deref() {
    Ok("panic") => panic!("fixture create panic"),
    Ok("error") => Err(EngineError::new("fixture create error")),
    _ => {
      let engine_id = NEXT_ENGINE_ID.fetch_add(1, Ordering::Relaxed);
      tracing::event!(
        name: "fixture.engine.created",
        tracing::Level::INFO,
        engine_id,
        "Created fixture engine"
      );
      Ok(FixtureEngine {
        engine_id,
        mode: String::new(),
        session_id: SessionId::new_v4(),
        release_scenario: None,
        connect_count: 0,
        poll_count: 0,
        worker_proof: None,
        worker_action_started: false,
      })
    }
  }
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

#[unsafe(no_mangle)]
/// Returns the fixture adapter's live output allocation count.
pub extern "C" fn fixture_outstanding_buffers() -> usize {
  battlement_native::outstanding_buffer_count()
}

#[unsafe(no_mangle)]
/// Returns the number of times the fixture engine received submit.
pub extern "C" fn fixture_submit_calls() -> usize {
  SUBMIT_CALLS.load(Ordering::Relaxed)
}

#[unsafe(no_mangle)]
/// Returns the number of times the fixture engine received connect.
pub extern "C" fn fixture_connect_calls() -> usize {
  CONNECT_CALLS.load(Ordering::Relaxed)
}

#[unsafe(no_mangle)]
/// Returns one deterministic worker-proof observation selected by index.
pub extern "C" fn fixture_worker_observation(index: u32) -> usize {
  let observer = worker_observer().lock().unwrap().clone();
  let Some(observer) = observer else {
    return 0;
  };
  let snapshot = observer.snapshot();
  match index {
    0 => snapshot.started,
    1 => snapshot.stopped,
    2 => snapshot.cancelled,
    3 => snapshot.failed,
    4 => snapshot.completed,
    5 => snapshot.drops,
    6 => usize::from(snapshot.waiting),
    7 => usize::from(snapshot.computing),
    8 => usize::try_from(snapshot.last_started).unwrap(),
    9 => usize::from(snapshot.off_creator_thread),
    _ => 0,
  }
}

#[unsafe(no_mangle)]
/// Releases the platform proof's ordinary-computation barrier.
pub extern "C" fn fixture_worker_release_computation() {
  if let Some(observer) = worker_observer().lock().unwrap().clone() {
    observer.release_computation();
  }
}

#[unsafe(no_mangle)]
/// Emits one structured Rust tracing event.
pub extern "C" fn fixture_trace() {
  tracing::event!(
      name: "fixture.rust_event",
      tracing::Level::INFO,
      mode = "test",
      "native trace"
  );
}
