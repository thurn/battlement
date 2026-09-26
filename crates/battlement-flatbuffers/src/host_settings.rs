use battlement::{
  CommandId, ScreenSize,
  display::{DisplayPreview, DisplayPreviewState},
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
    HostSettingsResult, SettingAvailability,
  },
};
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use uuid::Uuid;

use crate::{ProtocolError, common_generated as wire};

/// Borrowed, semantically checked host settings observation.
#[derive(Clone, Copy)]
pub struct HostSettingsView<'a> {
  value: wire::HostSettings<'a>,
}

impl<'a> HostSettingsView<'a> {
  pub(crate) fn new(value: wire::HostSettings<'a>) -> Result<Self, ProtocolError> {
    let view = Self { value };
    view.decode()?;
    Ok(view)
  }

  /// Copies a bounded observation into the application model.
  pub fn to_owned(self) -> HostSettings {
    self.decode().expect("host settings view was validated")
  }

  fn decode(self) -> Result<HostSettings, ProtocolError> {
    let value = self.value;
    if value.display_modes().len() > 3 {
      return Err(self::error("too many display modes"));
    }
    if value.resolutions().len() > 4096 || value.frame_rates().len() > 256 {
      return Err(self::error("host settings choices exceed limits"));
    }
    let result = HostSettings {
      platform: match value.platform().0 {
        0 => HostPlatform::Unavailable,
        1 => HostPlatform::MacOs,
        2 => HostPlatform::Windows,
        3 => HostPlatform::Web,
        4 => HostPlatform::Ios,
        _ => return Err(self::error("unknown host platform")),
      },
      display: self::availability(value.display())?,
      display_modes: value
        .display_modes()
        .iter()
        .map(self::mode)
        .collect::<Result<_, _>>()?,
      resolutions: value
        .resolutions()
        .iter()
        .map(self::resolution)
        .collect::<Result<_, _>>()?,
      applied_display: value
        .applied_display()
        .map(|display| {
          Ok(DisplayConfiguration {
            mode: self::mode(display.mode())?,
            resolution: self::resolution(display.resolution())?,
          })
        })
        .transpose()?,
      window_bounds: value
        .window_bounds()
        .map(|size| ScreenSize::new(size.width(), size.height())),
      display_preview: value
        .display_preview()
        .map(|preview| {
          Ok(DisplayPreview {
            request_id: CommandId::from_uuid(Uuid::from_bytes(preview.request_id().0))
              .map_err(|_| self::error("display preview ID must be nonzero"))?,
            state: match preview.state().0 {
              0 => DisplayPreviewState::Applying,
              1 => DisplayPreviewState::Confirmable,
              2 => DisplayPreviewState::Reverting,
              _ => return Err(self::error("unknown display preview state")),
            },
            remaining_seconds: preview.remaining_seconds(),
          })
        })
        .transpose()?,
      frame_pacing: self::availability(value.frame_pacing())?,
      frame_rates: value.frame_rates().iter().collect(),
      applied_frame_rate: value.applied_frame_rate(),
      vsync_available: value.vsync_available(),
      applied_vsync: value.applied_vsync(),
      keyboard_connected: value.keyboard_connected(),
      controller_count: value.controller_count(),
      diagnostics: self::availability(value.diagnostics())?,
      diagnostics_configured: value.diagnostics_configured(),
      capture_exceptions: value.capture_exceptions(),
      performance_reporting: value.performance_reporting(),
      diagnostics_error: value.diagnostics_error().map(str::to_owned),
      observation_error: value.observation_error().map(str::to_owned),
      last_result: value
        .last_result()
        .map(|result| {
          Ok(HostSettingsResult {
            request_id: CommandId::from_uuid(Uuid::from_bytes(result.request_id().0))
              .map_err(|_| self::error("host settings request ID must be nonzero"))?,
            error: result.error().map(str::to_owned),
          })
        })
        .transpose()?,
    };
    self::validate(&result)?;
    Ok(result)
  }
}

pub(crate) fn write<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &HostSettings,
) -> Result<WIPOffset<wire::HostSettings<'a>>, ProtocolError> {
  self::validate(value)?;
  let modes = value
    .display_modes
    .iter()
    .map(|mode| wire::DisplayMode(*mode as u8))
    .collect::<Vec<_>>();
  let modes = builder.create_vector(&modes);
  let resolutions = value
    .resolutions
    .iter()
    .map(self::wire_resolution)
    .collect::<Vec<_>>();
  let resolutions = builder.create_vector(&resolutions);
  let rates = builder.create_vector(&value.frame_rates);
  let display = value.applied_display.map(|display| {
    wire::DisplayConfiguration::create(
      builder,
      &wire::DisplayConfigurationArgs {
        mode: wire::DisplayMode(display.mode as u8),
        resolution: Some(&self::wire_resolution(&display.resolution)),
      },
    )
  });
  let observation_error = value
    .observation_error
    .as_deref()
    .map(|message| builder.create_string(message));
  let last_result = value.last_result.as_ref().map(|result| {
    let error = result
      .error
      .as_deref()
      .map(|message| builder.create_string(message));
    wire::HostSettingsResult::create(
      builder,
      &wire::HostSettingsResultArgs {
        request_id: Some(&wire::Uuid(*result.request_id.as_uuid().as_bytes())),
        error,
      },
    )
  });
  let diagnostics_error = value
    .diagnostics_error
    .as_ref()
    .map(|value| builder.create_string(value));
  let window_bounds = value
    .window_bounds
    .map(|size| wire::ScreenSize::new(size.width, size.height));
  let display_preview = value.display_preview.map(|preview| {
    wire::DisplayPreview::create(
      builder,
      &wire::DisplayPreviewArgs {
        request_id: Some(&wire::Uuid(*preview.request_id.as_uuid().as_bytes())),
        state: wire::DisplayPreviewState(preview.state as u8),
        remaining_seconds: preview.remaining_seconds,
      },
    )
  });
  Ok(wire::HostSettings::create(
    builder,
    &wire::HostSettingsArgs {
      platform: wire::HostPlatform(value.platform as u8),
      display: wire::SettingAvailability(value.display as u8),
      display_modes: Some(modes),
      resolutions: Some(resolutions),
      applied_display: display,
      frame_pacing: wire::SettingAvailability(value.frame_pacing as u8),
      frame_rates: Some(rates),
      applied_frame_rate: value.applied_frame_rate,
      vsync_available: value.vsync_available,
      applied_vsync: value.applied_vsync,
      keyboard_connected: value.keyboard_connected,
      controller_count: value.controller_count,
      diagnostics: wire::SettingAvailability(value.diagnostics as u8),
      diagnostics_configured: value.diagnostics_configured,
      capture_exceptions: value.capture_exceptions,
      performance_reporting: value.performance_reporting,
      diagnostics_error,
      observation_error,
      last_result,
      window_bounds: window_bounds.as_ref(),
      display_preview,
    },
  ))
}

fn validate(value: &HostSettings) -> Result<(), ProtocolError> {
  if let Some(size) = value.window_bounds
    && (size.width == 0 || size.height == 0)
  {
    return Err(self::error("invalid window bounds"));
  }
  if value
    .display_preview
    .is_some_and(|preview| preview.remaining_seconds > 15)
  {
    return Err(self::error("invalid display preview deadline"));
  }
  if value.display_modes.len() > 3 || value.resolutions.len() > 4096 {
    return Err(self::error("too many display choices"));
  }
  for (index, mode) in value.display_modes.iter().enumerate() {
    if value.display_modes[..index].contains(mode) {
      return Err(self::error("duplicate display mode"));
    }
  }
  for resolution in &value.resolutions {
    self::validate_resolution(resolution)?;
  }
  if let Some(display) = value.applied_display {
    self::validate_resolution(&display.resolution)?;
  }
  if value.frame_rates.len() > 256 || value.frame_rates.contains(&0) {
    return Err(self::error("invalid frame rate choices"));
  }
  if value
    .frame_rates
    .windows(2)
    .any(|rates| rates[0] >= rates[1])
  {
    return Err(self::error("frame rate choices must be sorted and unique"));
  }
  if value.applied_frame_rate != -1 && value.applied_frame_rate <= 0 {
    return Err(self::error("invalid applied frame rate"));
  }
  Ok(())
}

fn validate_resolution(value: &DisplayResolution) -> Result<(), ProtocolError> {
  if value.width == 0 || value.height == 0 {
    return Err(self::error("invalid display dimensions"));
  }
  if value.refresh_denominator == 0 {
    return Err(self::error("invalid display refresh ratio"));
  }
  Ok(())
}

fn wire_resolution(value: &DisplayResolution) -> wire::DisplayResolution {
  wire::DisplayResolution::new(
    value.width,
    value.height,
    value.refresh_numerator,
    value.refresh_denominator,
  )
}

fn resolution(value: &wire::DisplayResolution) -> Result<DisplayResolution, ProtocolError> {
  let result = DisplayResolution {
    width: value.width(),
    height: value.height(),
    refresh_numerator: value.refresh_numerator(),
    refresh_denominator: value.refresh_denominator(),
  };
  self::validate_resolution(&result)?;
  Ok(result)
}

fn mode(value: wire::DisplayMode) -> Result<DisplayMode, ProtocolError> {
  match value.0 {
    0 => Ok(DisplayMode::Windowed),
    1 => Ok(DisplayMode::Borderless),
    2 => Ok(DisplayMode::Fullscreen),
    _ => Err(self::error("unknown display mode")),
  }
}

fn availability(value: wire::SettingAvailability) -> Result<SettingAvailability, ProtocolError> {
  match value.0 {
    0 => Ok(SettingAvailability::Unavailable),
    1 => Ok(SettingAvailability::Available),
    2 => Ok(SettingAvailability::Failed),
    _ => Err(self::error("unknown capability availability")),
  }
}

fn error(message: &str) -> ProtocolError {
  ProtocolError::new(message)
}
