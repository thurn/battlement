mod app_support;

use std::{cell::RefCell, rc::Rc};

use battlement::{
  Command, CommandBody,
  frame_pacing::FramePacing,
  host_settings::{HostPlatform, HostSettings, SettingAvailability},
};
use battlement_fake::client::FakeClient;
use reactant_core::{app::App, app_context::HostEnvironment, prelude::*};

struct Consumer {
  app: Rc<RefCell<Option<AppHandle>>>,
  observed: Rc<RefCell<HostSettings>>,
}

impl Component for Consumer {
  fn render(&self) -> impl Render {
    *self.app.borrow_mut() = Some(use_app());
    *self.observed.borrow_mut() = use_required_context::<HostEnvironment>().settings;
    View::new()
  }
}

#[test]
fn pacing_crosses_the_wire_and_reconciles_capabilities_without_erasing_intent() {
  let handle = Rc::new(RefCell::new(None));
  let observed = Rc::new(RefCell::new(HostSettings::default()));
  let app = App::new("app/content").ui(Consumer {
    app: Rc::clone(&handle),
    observed: Rc::clone(&observed),
  });
  let mut connect = app_support::connect();
  let host = HostSettings {
    platform: HostPlatform::Ios,
    frame_pacing: SettingAvailability::Available,
    frame_rates: vec![30, 60, 120],
    ..HostSettings::default()
  };
  connect.host_settings = host.clone();
  let mut client = FakeClient::connect_with(app, app_support::catalog(), connect);
  let app = handle.borrow().as_ref().unwrap().clone();
  let request = Command::new_v4(CommandBody::ApplicationSetFramePacing(FramePacing {
    maximum_frame_rate: 144,
    vsync: true,
  }));
  let id = request.command_id;
  app.send(request);
  client.poll();
  assert_eq!(observed.borrow().applied_frame_rate, 120);
  assert!(!observed.borrow().applied_vsync);
  assert_eq!(
    observed.borrow().last_result.as_ref().unwrap().request_id,
    id
  );
  assert!(
    observed
      .borrow()
      .last_result
      .as_ref()
      .unwrap()
      .error
      .is_none()
  );
  client.set_host_settings(HostSettings {
    frame_rates: vec![30, 60],
    ..host.clone()
  });
  client.poll();
  assert_eq!(observed.borrow().applied_frame_rate, 60);
  client.set_host_settings(host.clone());
  client.poll();
  assert_eq!(observed.borrow().applied_frame_rate, 120);
  client.set_host_settings(HostSettings {
    platform: HostPlatform::MacOs,
    vsync_available: true,
    frame_rates: vec![60, 120, 144, 240],
    ..host
  });
  client.poll();
  assert_eq!(observed.borrow().applied_frame_rate, -1);
  assert!(observed.borrow().applied_vsync);
  app.send(Command::new_v4(CommandBody::ApplicationSetFramePacing(
    FramePacing {
      maximum_frame_rate: 144,
      vsync: false,
    },
  )));
  client.poll();
  assert_eq!(observed.borrow().applied_frame_rate, 144);
  assert!(!observed.borrow().applied_vsync);
}

#[test]
fn malformed_pacing_is_rejected_at_the_wire_boundary() {
  let response = battlement::Response::commands(
    battlement::SessionId::new_v4(),
    [CommandBody::ApplicationSetFramePacing(FramePacing {
      maximum_frame_rate: 0,
      vsync: false,
    })],
  );
  assert!(battlement_flatbuffers::test_support::core_response(&response).is_err());
}
