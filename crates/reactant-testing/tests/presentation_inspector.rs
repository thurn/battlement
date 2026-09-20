#[path = "support/session_game.rs"]
#[allow(dead_code)]
mod session_game;

use std::num::NonZeroU64;
use std::{sync::Arc, sync::atomic::Ordering, time::Duration};

use battlement::{
  DisplayId, ElementGeometry, GeometryGeneration, GeometryObservationBatch,
  GeometryObservationResult, GeometryObservationTarget, GeometryObservationValue,
  GeometryUnavailable, GeometryValue, ObjectId, PresentationWorkGeometry, Projective2, Rect,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiEvent, UiEventBody, ViewportRect,
};
use battlement_fake::assets::FakeAssetCatalog;
use reactant::{
  DispatchResult, GameConsumer, GameStatus, InspectorObject, PresentationInspector, prelude::*,
  testing::App, testing::GameApp,
};
use reactant_testing::Display;
use session_game::{Action, Context, Counter, Probe};

const TIMEOUT: Duration = Duration::from_secs(10);

struct InspectorFixture;

struct WorkerInspector;

impl Component for InspectorFixture {
  fn render(&self) -> impl Render {
    let target = use_element_ref();
    (
      View::new()
        .name("inspected-card")
        .element_ref(target.clone()),
      PresentationInspector::new("card moved to hand", "waiting for player choice")
        .prompt(Some("choose one visible card"))
        .selection(Some(InspectorObject::ui(
          ObjectId::new_v4(),
          target,
          "face up",
          "hand slot 2",
        ))),
    )
  }
}

impl Component for WorkerInspector {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<Counter>();
    let observation = reactant::use_game_observation::<Counter>();
    let worker = if observation.worker.started && !observation.worker.stopped {
      "running rules action"
    } else if observation.worker.stopped && observation.status == GameStatus::Busy {
      "action complete; output awaiting submission"
    } else {
      "idle"
    };
    PresentationInspector::new(format!("counter {}", state.value), worker).prompt(None::<String>)
  }
}

#[test]
fn public_inspector_samples_only_while_open_and_renders_safe_observations() {
  let app = App::new("inspector/scene").ui(InspectorFixture);
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("inspector/scene");
  let mut display = Display::connect(app, assets);

  assert_eq!(display.geometry_registry().iter().count(), 0);
  semantic_activate(&mut display, root, "presentation-inspector-open");

  let observations = display
    .geometry_registry()
    .iter()
    .map(|(id, target)| (*id, target.clone()))
    .collect::<Vec<_>>();
  assert_eq!(observations.len(), 2);
  let changed = observations
    .iter()
    .map(|(id, target)| GeometryObservationValue {
      observation_id: *id,
      result: GeometryObservationResult::Current(match target {
        GeometryObservationTarget::UiElement { .. } => {
          GeometryValue::Element(element_geometry(root))
        }
        GeometryObservationTarget::PresentationWork => {
          GeometryValue::PresentationWork(PresentationWorkGeometry {
            queued_batches: 3,
            blocking_operations: 1,
            paused_scopes: 0,
          })
        }
        _ => panic!("inspector requested an unrelated geometry target"),
      }),
    })
    .collect();
  display.deliver_geometry(GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(1).unwrap()),
    changed,
  });

  let work = display.find_ui(root, "presentation-inspector-work");
  assert_eq!(
    display.ui_element(work).text(),
    Some("Native queue: 3 queued batch(es) · 1 blocking operation(s) · 0 paused scope(s)")
  );
  let pose = display.find_ui(root, "presentation-inspector-pose");
  assert_eq!(
    display.ui_element(pose).text(),
    Some("Displayed position: viewport (120.0, 80.0) · native visible")
  );

  semantic_activate(&mut display, root, "presentation-inspector-close");
  assert_eq!(display.geometry_registry().iter().count(), 0);

  semantic_activate(&mut display, root, "presentation-inspector-open");
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "presentation-inspector-work"))
      .text(),
    Some("Native queue: waiting for first sample")
  );
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "presentation-inspector-pose"))
      .text(),
    Some("Displayed position: waiting")
  );

  let unavailable = display
    .geometry_registry()
    .iter()
    .map(|(id, target)| GeometryObservationValue {
      observation_id: *id,
      result: GeometryObservationResult::Unavailable(match target {
        GeometryObservationTarget::UiElement { .. } => GeometryUnavailable::Hidden,
        GeometryObservationTarget::PresentationWork => GeometryUnavailable::ObjectMissing,
        _ => panic!("inspector requested an unrelated geometry target"),
      }),
    })
    .collect();
  display.deliver_geometry(GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(2).unwrap()),
    changed: unavailable,
  });
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "presentation-inspector-work"))
      .text(),
    Some("Native queue: unavailable (ObjectMissing)")
  );
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "presentation-inspector-pose"))
      .text(),
    Some("Displayed position: unavailable (Hidden)")
  );

  semantic_activate(&mut display, root, "presentation-inspector-close");
  assert_eq!(display.geometry_registry().iter().count(), 0);
}

#[test]
fn worker_observations_rerender_and_ordinary_replacement_releases_the_old_run() {
  let probe = Arc::new(Probe::default());
  let mut app =
    App::with_model("inspector/game", 0_usize).ui((GameRoot::new(View::new()), WorkerInspector));
  let game = app.start_game::<Counter>(probe.state(), |connection| {
    Context::interactive(connection, probe.clone())
  });
  let consumer = app.game_consumer::<Counter>();
  let root = app.root_document().root_id;
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("inspector/game");
  let mut display = Display::connect(app, assets);
  output(&consumer).submitted();
  display.poll();
  semantic_activate(&mut display, root, "presentation-inspector-open");

  assert_eq!(game.dispatch(Action::Add), DispatchResult::Started);
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  display.poll();
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "presentation-inspector-worker"))
      .text(),
    Some("Worker: action complete; output awaiting submission")
  );
  output(&consumer).submitted();
  assert_eq!(game.status(), GameStatus::Ready);

  assert_eq!(game.dispatch(Action::HoldThenFail), DispatchResult::Started);
  probe.wait_held();
  let drops_before = probe.drops.load(Ordering::SeqCst);
  let replacement_probe = probe.clone();
  let (replacement, replacement_consumer) = display.with_engine(|app| {
    let replacement = app.start_game::<Counter>(replacement_probe.state(), |connection| {
      Context::interactive(connection, replacement_probe.clone())
    });
    let consumer = app.game_consumer::<Counter>();
    (replacement, consumer)
  });
  assert_eq!(game.status(), GameStatus::Stopped);
  probe.release();
  assert!(consumer.wait_for_worker_stopped(TIMEOUT));
  assert!(probe.drops.load(Ordering::SeqCst) > drops_before);
  output(&replacement_consumer).submitted();
  display.poll();
  assert_eq!(replacement.status(), GameStatus::Ready);
  assert!(display.contains_ui(display.find_ui(root, "presentation-inspector-close")));
  assert_eq!(
    display
      .ui_element(display.find_ui(root, "presentation-inspector-worker"))
      .text(),
    Some("Worker: idle")
  );
}

fn output(consumer: &GameConsumer<Counter>) -> reactant::GameOutput<Counter> {
  assert!(consumer.wait_for_output(TIMEOUT));
  consumer.take_output().expect("published output")
}

fn semantic_activate<G: 'static>(display: &mut Display<App<G>>, root: ObjectId, name: &str) {
  let target_id = display.find_ui(root, name);
  display.deliver_ui_event(UiEvent {
    target_id,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::Activate,
    }),
  });
  display.poll();
}

fn element_geometry(panel_id: ObjectId) -> ElementGeometry {
  let identity = Projective2 {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
  };
  ElementGeometry {
    layout: Rect::new(0.0, 0.0, 80.0, 120.0),
    viewport_bound: ViewportRect {
      x: 120.0,
      y: 80.0,
      width: 80.0,
      height: 120.0,
      display_id: DisplayId(0),
    },
    viewport_from_local: identity,
    viewport_from_parent: identity,
    panel_id,
  }
}
