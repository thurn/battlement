mod app_support;

use std::{cell::RefCell, num::NonZeroU64, rc::Rc};
use trox::ls;

use battlement::{
  Action, ActionBody, ActionId, ClientMessage, CommandBody, DisplayId, DisplayOrientation,
  GeometryGeneration, GeometryObservationBatch, GeometryObservationResult,
  GeometryObservationValue, GeometryValue, ResponseMessage, ScreenSize, ViewportGeometry,
  ViewportRect, application::ApplicationState, application::ReducedMotionPreference,
};
use battlement_native::Engine;
use battlement_reactant::{app::App, prelude::*};

type Observation = (ScreenSize, ApplicationState, ReducedMotionPreference, bool);

#[derive(PartialEq)]
struct Observed(Rc<RefCell<Vec<Observation>>>);

impl Component for Observed {
  fn render(&self) -> impl Render {
    let screen = use_viewport_size();
    let application = use_application_state();
    let reduced_motion = use_reduced_motion_preference();
    let effective_motion = use_reduced_motion();
    let values = Rc::clone(&self.0);
    use_effect(
      move || {
        values
          .borrow_mut()
          .push((screen, application, reduced_motion, effective_motion));
      },
      (screen, application, reduced_motion, effective_motion),
    );
    Label::new(ls(format!(
      "{}x{} {}",
      screen.width,
      screen.height,
      application.is_active()
    )))
  }
}

#[test]
fn host_observations_reach_memoized_components_and_reconnect_uses_new_dimensions() {
  let values = Rc::new(RefCell::new(Vec::new()));
  let mut app = App::new("app/content").ui(memo(Observed(Rc::clone(&values))));
  let initial = app.connect(app_support::connect()).unwrap();
  let _ = app.poll().unwrap();
  assert_eq!(values.borrow().last().unwrap().0, ScreenSize::new(800, 600));
  let observation = initial
    .messages
    .iter()
    .filter_map(|message| match message {
      ResponseMessage::Batch(batch) => Some(batch.groups.iter().flat_map(|group| &group.commands)),
      _ => None,
    })
    .flatten()
    .find_map(|command| match &command.body {
      CommandBody::GeometryObservationUpdate(update) => update.added.first(),
      _ => None,
    })
    .unwrap();
  let rect = ViewportRect {
    x: 0.0,
    y: 0.0,
    width: 1024.0,
    height: 768.0,
    display_id: DisplayId(0),
  };
  let geometry = GeometryObservationBatch {
    generation: GeometryGeneration(NonZeroU64::new(1).unwrap()),
    changed: vec![GeometryObservationValue {
      observation_id: observation.observation_id,
      result: GeometryObservationResult::Current(GeometryValue::Viewport(ViewportGeometry {
        viewport: rect,
        safe_area: rect,
        scale: 2.0,
        dpi: Some(192.0),
        orientation: DisplayOrientation::Landscape,
      })),
    }],
  };
  app
    .submit(ClientMessage::Action(Action::new(
      ActionId::new_v4(),
      initial.session_id,
      ActionBody::GeometryObservations(geometry),
    )))
    .unwrap();
  let _ = app.poll().unwrap();
  assert_eq!(values.borrow().last().unwrap().0, ScreenSize::new(512, 384));
  let inactive = ApplicationState {
    focused: false,
    paused: true,
  };
  app
    .submit(ClientMessage::Action(Action::new(
      ActionId::new_v4(),
      initial.session_id,
      ActionBody::ApplicationStateChanged(inactive),
    )))
    .unwrap();
  let _ = app.poll().unwrap();
  assert_eq!(values.borrow().last().unwrap().1, inactive);
  app
    .submit(ClientMessage::Action(Action::new(
      ActionId::new_v4(),
      initial.session_id,
      ActionBody::ReducedMotionPreferenceChanged(ReducedMotionPreference::Reduce),
    )))
    .unwrap();
  let _ = app.poll().unwrap();
  assert_eq!(
    values.borrow().last().unwrap().2,
    ReducedMotionPreference::Reduce
  );
  assert!(values.borrow().last().unwrap().3);
  let mut connect = app_support::connect();
  connect.screen = ScreenSize::new(400, 300);
  connect.reduced_motion_preference = ReducedMotionPreference::NoPreference;
  app.connect(connect).unwrap();
  let _ = app.poll().unwrap();
  assert_eq!(
    *values.borrow().last().unwrap(),
    (
      ScreenSize::new(400, 300),
      ApplicationState::default(),
      ReducedMotionPreference::NoPreference,
      false,
    )
  );
}
