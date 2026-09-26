//! Capability-backed graphics controls and host-timed display confirmation.

use battlement::{
  Color, Position, Style, WhiteSpace,
  frame_pacing::FramePacing,
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
    SettingAvailability,
  },
};
use reactant::{control_behavior, portal::PortalTarget, prelude::*};
use trox::{LocalizedString, ls, tx, tx_args, txa};

use crate::{
  menu::{
    arcade_modal::ArcadeModal, arcade_route_transition, font_scale, select_control::SelectControl,
    toggle_control::ToggleControl,
  },
  settings::{self, SettingsChange, display},
};

#[builder]
pub struct GraphicsSettings {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for GraphicsSettings {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings();
    let host = reactant::use_host_settings();
    let display = display::use_display_control(&host);
    let navigation = arcade_route_transition::use_arcade_navigation();
    let desktop = matches!(host.platform, HostPlatform::MacOs | HostPlatform::Windows);
    let applied = host.applied_display;
    let resolutions = self::resolutions(&host);
    let modes = host.display_modes.clone();
    let display_available = desktop && host.display == SettingAvailability::Available;
    let rate_available =
      host.frame_pacing == SettingAvailability::Available && !host.frame_rates.is_empty();
    let rate_hidden = desktop && host.applied_vsync;
    let mut requested_pacing = host.clone();
    let pacing_supported = FramePacing {
      maximum_frame_rate: settings.desired.framerate(&host),
      vsync: settings.desired.vsync,
    }
    .apply_to(&mut requested_pacing);
    let pacing_pending = pacing_supported
      && (
        requested_pacing.applied_vsync,
        requested_pacing.applied_frame_rate,
      ) != (host.applied_vsync, host.applied_frame_rate);
    let scale = font_scale::use_font_scale().factor();
    View::new()
      .name("graphics-settings")
      .style(Style::new().position(Position::Relative).min_height(971))
      .child((
        display_available.then(|| {
          applied.map(|configuration| {
            SelectControl::new()
              .label(control_behavior::name_source_text(tx(
                "Resolution",
                "Graphics resolution setting label.",
              )))
              .value(self::resolution_id(configuration.resolution))
              .suspend_focus(display.pending)
              .option_label(self::resolution_label)
              .options(
                resolutions
                  .iter()
                  .copied()
                  .map(self::resolution_id)
                  .collect(),
              )
              .overlay(self.overlay.clone())
              .on_change(display.preview.clone().filter_map_input(move |id: String| {
                resolutions
                  .iter()
                  .find(|value| self::resolution_id(**value) == id)
                  .map(|resolution| DisplayConfiguration {
                    resolution: *resolution,
                    ..configuration
                  })
              }))
              .first(true)
          })
        }),
        (rate_available && !rate_hidden).then(|| {
          let rates = host.frame_rates.clone();
          SelectControl::new()
            .label(control_behavior::name_source_text(tx(
              "Max Framerate",
              "Graphics frame-rate setting label.",
            )))
            .value(self::selected_rate(&host, settings.desired.framerate(&host)).to_string())
            .option_label(self::framerate_label)
            .options(rates.iter().map(u32::to_string).collect())
            .overlay(self.overlay.clone())
            .on_change(
              settings
                .callback(SettingsChange::MaxFramerate)
                .filter_map_input(move |id: String| {
                  id.parse::<u32>().ok().filter(|rate| rates.contains(rate))
                }),
            )
            .first(!display_available)
        }),
        display_available.then(|| {
          applied.map(|configuration| {
            SelectControl::new()
              .label(control_behavior::name_source_text(tx(
                "Display Mode",
                "Graphics display-mode setting label.",
              )))
              .value(self::mode_id(configuration.mode).to_owned())
              .suspend_focus(display.pending)
              .option_label(self::mode_label)
              .options(
                modes
                  .iter()
                  .copied()
                  .map(|mode| self::mode_id(mode).to_owned())
                  .collect(),
              )
              .overlay(self.overlay.clone())
              .on_change(display.preview.clone().filter_map_input(move |id: String| {
                modes
                  .iter()
                  .find(|mode| self::mode_id(**mode) == id)
                  .map(|mode| DisplayConfiguration {
                    mode: *mode,
                    ..configuration
                  })
              }))
          })
        }),
        ToggleControl::new()
          .label(control_behavior::name_source_text(tx(
            "Screenshake",
            "Graphics screenshake setting label.",
          )))
          .checked(settings.desired.screenshake)
          .on_change(settings.callback(SettingsChange::Screenshake)),
        (desktop && host.vsync_available).then(|| {
          ToggleControl::new()
            .label(control_behavior::name_source_text(tx(
              "VSync",
              "Graphics vertical-sync setting label.",
            )))
            .checked(settings.desired.vsync)
            .on_change(settings.callback(SettingsChange::Vsync))
        }),
        display.failed.then(|| {
          Text::new(tx(
            "The display change was not kept. Current display settings are shown.",
            "Display preview failure or rollback feedback.",
          ))
          .style(
            Style::new()
              .font_size(30.0 * scale)
              .white_space(WhiteSpace::Normal)
              .color(Color::WHITE),
          )
        }),
        (pacing_pending || host.frame_pacing == SettingAvailability::Failed).then(|| {
          Text::new(tx(
            "Frame pacing has not matched your choice.",
            "Requested pacing differs from host readback.",
          ))
          .style(
            Style::new()
              .font_size(30.0 * scale)
              .white_space(WhiteSpace::Normal)
              .color(Color::WHITE),
          )
        }),
        (host.display == SettingAvailability::Failed).then(|| {
          Text::new(tx(
            "Display options are temporarily unavailable.",
            "Host display capability observation failed.",
          ))
          .style(
            Style::new()
              .font_size(30.0 * scale)
              .white_space(WhiteSpace::Normal)
              .color(Color::WHITE),
          )
        }),
        ArcadeModal::new()
          .open(display.pending)
          .title(tx("Keep changes?", "Display preview confirmation title."))
          .children(Text::new(if display.confirmable {
            txa(
              "Reverting in {seconds} seconds.",
              tx_args![seconds => display.remaining_seconds],
              "Display preview real-time countdown.",
            )
          } else {
            tx(
              "Waiting for the display…",
              "Display preview waiting for host readback or confirmation.",
            )
          }))
          .confirm_label(tx("Keep", "Keep the previewed display configuration."))
          .cancel_label(tx("Revert", "Revert the display preview."))
          .confirm_disabled(!display.confirmable)
          .busy(display.finishing)
          .reduce_motion(navigation.reduce_motion)
          .on_confirm(display.confirm)
          .on_close(display.cancel)
          .overlay(self.overlay.clone()),
      ))
  }
}

fn resolutions(host: &HostSettings) -> Vec<DisplayResolution> {
  let mut choices = host.resolutions.clone();
  if let Some(current) = host.applied_display {
    choices.insert(0, current.resolution);
  }
  let mut result = Vec::<DisplayResolution>::new();
  for value in choices {
    let current = host.applied_display.map(|display| display.resolution);
    let windowed = host
      .applied_display
      .is_some_and(|display| display.mode == DisplayMode::Windowed);
    let too_large = host
      .window_bounds
      .is_some_and(|bounds| value.width > bounds.width || value.height > bounds.height);
    if windowed && too_large && current != Some(value) {
      continue;
    }
    if !result
      .iter()
      .any(|other| other.width == value.width && other.height == value.height)
    {
      result.push(value);
    }
  }
  result
}

fn selected_rate(host: &HostSettings, requested: u32) -> u32 {
  if host.applied_frame_rate > 0 {
    return host.applied_frame_rate as u32;
  }
  host
    .frame_rates
    .iter()
    .copied()
    .filter(|rate| *rate <= requested)
    .max()
    .unwrap_or_else(|| *host.frame_rates.iter().min().expect("available rates"))
}

fn resolution_id(value: DisplayResolution) -> String {
  format!("{}x{}", value.width, value.height)
}

fn resolution_label(value: &str) -> LocalizedString {
  ls(value.replace('x', " × "))
}

fn framerate_label(value: &str) -> LocalizedString {
  let rate = value.parse::<u32>().expect("framerate option");
  txa(
    "{rate} FPS",
    tx_args![rate],
    "Frame rate in frames per second.",
  )
}

fn mode_id(mode: DisplayMode) -> &'static str {
  match mode {
    DisplayMode::Borderless => "borderless",
    DisplayMode::Fullscreen => "fullscreen",
    DisplayMode::Windowed => "windowed",
  }
}

fn mode_label(value: &str) -> LocalizedString {
  match value {
    "borderless" => tx("Borderless", "Borderless display mode."),
    "fullscreen" => tx("Fullscreen", "Exclusive fullscreen display mode."),
    "windowed" => tx("Windowed", "Windowed display mode."),
    _ => panic!("unknown display mode option"),
  }
}
