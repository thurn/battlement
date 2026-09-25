use battlement::{
  Action, ActionBody, ActionId, CommandId, Connect, ScreenSize, SessionId,
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
    HostSettingsResult, SettingAvailability,
  },
};
use battlement_flatbuffers::{self, ConnectView, CoreActionBodyView, CoreClientMessageView};

fn settings() -> HostSettings {
  let resolution = DisplayResolution {
    width: 1920,
    height: 1080,
    refresh_numerator: 60000,
    refresh_denominator: 1001,
  };
  HostSettings {
    platform: HostPlatform::Windows,
    display: SettingAvailability::Available,
    display_modes: vec![
      DisplayMode::Windowed,
      DisplayMode::Borderless,
      DisplayMode::Fullscreen,
    ],
    resolutions: vec![resolution],
    applied_display: Some(DisplayConfiguration {
      mode: DisplayMode::Windowed,
      resolution,
    }),
    frame_pacing: SettingAvailability::Available,
    frame_rates: vec![30, 60, 144],
    applied_frame_rate: 60,
    vsync_available: true,
    applied_vsync: true,
    keyboard_connected: true,
    controller_count: 2,
    diagnostics: SettingAvailability::Failed,
    diagnostics_configured: true,
    observation_error: Some("vendor unavailable".into()),
    last_result: Some(HostSettingsResult {
      request_id: CommandId::new_v4(),
      error: Some("denied".into()),
    }),
  }
}

#[test]
fn initial_and_changed_observations_preserve_every_field() {
  for settings in [HostSettings::default(), settings()] {
    let mut connect = Connect::new("test", "test", ScreenSize::new(1920, 1080));
    connect.host_settings = settings.clone();
    let bytes = battlement_flatbuffers::write_connect_request(&connect).unwrap();
    let view = ConnectView::read(bytes.as_bytes()).unwrap();
    assert_eq!(view.host_settings().to_owned(), settings);
    let action = Action::new(
      ActionId::new_v4(),
      SessionId::new_v4(),
      ActionBody::HostSettingsChanged(settings.clone()),
    );
    let bytes = battlement_flatbuffers::write_core_action(&action).unwrap();
    let CoreClientMessageView::Action(action) =
      CoreClientMessageView::read(bytes.as_bytes()).unwrap()
    else {
      panic!("expected action")
    };
    let CoreActionBodyView::HostSettingsChanged(view) = action.body() else {
      panic!("expected observation")
    };
    assert_eq!(view.to_owned(), settings);
    assert!(CoreClientMessageView::read(&bytes.as_bytes()[..bytes.as_bytes().len() - 1]).is_err());
  }
}

#[test]
fn malformed_choices_fail_before_crossing_the_transport() {
  let valid = settings();
  let mut invalid = Vec::new();
  let mut duplicate = valid.clone();
  duplicate.display_modes.push(DisplayMode::Windowed);
  invalid.push(duplicate);
  let mut bad_refresh = valid.clone();
  bad_refresh.resolutions[0].refresh_denominator = 0;
  invalid.push(bad_refresh);
  let mut bad_dimensions = valid.clone();
  bad_dimensions
    .applied_display
    .as_mut()
    .unwrap()
    .resolution
    .width = 0;
  invalid.push(bad_dimensions);
  let mut bad_rates = valid.clone();
  bad_rates.frame_rates = vec![60, 30];
  invalid.push(bad_rates);
  let mut bad_cap = valid;
  bad_cap.applied_frame_rate = 0;
  invalid.push(bad_cap);
  for settings in invalid {
    let mut connect = Connect::new("test", "test", ScreenSize::new(1920, 1080));
    connect.host_settings = settings.clone();
    assert!(battlement_flatbuffers::write_connect_request(&connect).is_err());
    let action = Action::new(
      ActionId::new_v4(),
      SessionId::new_v4(),
      ActionBody::HostSettingsChanged(settings),
    );
    assert!(battlement_flatbuffers::write_core_action(&action).is_err());
  }
}
