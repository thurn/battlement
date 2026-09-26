mod app_support;

use battlement::{
  Command, CommandBody, CommandId, ScreenSize,
  application::ApplicationState,
  display::{DisplayCommand, DisplayPreviewState},
  frame_pacing::FramePacing,
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
    SettingAvailability,
  },
};
use battlement_fake::client::FakeClient;
use reactant_core::{app::App, app_context::HostEnvironment, prelude::*};
use std::{cell::RefCell, rc::Rc, time::Duration};

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
fn configuration(width: u32, height: u32) -> DisplayConfiguration {
  DisplayConfiguration {
    mode: DisplayMode::Windowed,
    resolution: DisplayResolution {
      width,
      height,
      refresh_numerator: 60,
      refresh_denominator: 1,
    },
  }
}
fn send(app: &AppHandle, operation: DisplayCommand) -> CommandId {
  let command = Command::new_v4(CommandBody::ApplicationDisplay(operation));
  let id = command.command_id;
  app.send(command);
  id
}
#[test]
fn display_transactions_cross_the_wire_and_timeout_without_advancing_rules_time() {
  let handle = Rc::new(RefCell::new(None));
  let observed = Rc::new(RefCell::new(HostSettings::default()));
  let app = App::new("app/content").ui(Consumer {
    app: Rc::clone(&handle),
    observed: Rc::clone(&observed),
  });
  let mut connect = app_support::connect();
  let prior = self::configuration(1280, 720);
  let target = self::configuration(1024, 768);
  connect.host_settings = HostSettings {
    platform: HostPlatform::MacOs,
    display: SettingAvailability::Available,
    frame_pacing: SettingAvailability::Available,
    frame_rates: vec![60, 120],
    display_modes: vec![DisplayMode::Windowed, DisplayMode::Borderless],
    resolutions: vec![prior.resolution, target.resolution],
    applied_display: Some(prior),
    window_bounds: Some(ScreenSize {
      width: 1920,
      height: 1080,
    }),
    ..HostSettings::default()
  };
  let mut client = FakeClient::connect_with(app, app_support::catalog(), connect);
  let app = handle.borrow().as_ref().unwrap().clone();
  app.send(Command::new_v4(CommandBody::ApplicationSetFramePacing(
    FramePacing {
      maximum_frame_rate: 60,
      vsync: false,
    },
  )));
  client.poll();
  let first = self::send(&app, DisplayCommand::Preview(target));
  client.poll();
  assert_eq!(observed.borrow().applied_display, Some(target));
  assert_eq!(
    observed.borrow().display_preview.unwrap().state,
    DisplayPreviewState::Confirmable
  );
  let current = self::send(&app, DisplayCommand::Preview(target));
  client.poll();
  self::send(&app, DisplayCommand::Confirm(first));
  self::send(&app, DisplayCommand::Cancel(first));
  client.poll();
  assert_eq!(
    observed.borrow().display_preview.unwrap().request_id,
    current
  );
  client.advance_host_time(Duration::from_secs(14));
  client.poll();
  assert_eq!(
    observed.borrow().display_preview.unwrap().remaining_seconds,
    1
  );
  client.advance_host_time(Duration::from_secs(1));
  client.poll();
  assert_eq!(observed.borrow().applied_display, Some(prior));
  assert!(observed.borrow().display_preview.is_none());
  let preview = self::send(&app, DisplayCommand::Preview(target));
  client.poll();
  let confirmation = self::send(&app, DisplayCommand::Confirm(preview));
  client.poll();
  let host = observed.borrow().clone();
  client.set_host_settings(host);
  client.poll();
  assert_eq!(
    observed.borrow().last_result.as_ref().unwrap().request_id,
    confirmation
  );
  client.advance_host_time(Duration::from_secs(30));
  client.poll();
  assert_eq!(observed.borrow().applied_display, Some(target));

  let preview = self::send(&app, DisplayCommand::Preview(prior));
  client.poll();
  client.fail_next_display_save();
  self::send(&app, DisplayCommand::Confirm(preview));
  client.poll();
  assert_eq!(observed.borrow().applied_display, Some(target));
  assert!(
    observed
      .borrow()
      .last_result
      .as_ref()
      .unwrap()
      .error
      .is_some()
  );
  self::send(&app, DisplayCommand::Preview(prior));
  client.poll();
  client.set_application_state(ApplicationState {
    focused: false,
    paused: false,
  });
  client.poll();
  assert_eq!(observed.borrow().applied_display, Some(target));
  assert!(observed.borrow().display_preview.is_none());
}
