//! Complete settings-screen composition with host-owned navigation boundaries.

use battlement::{Position, Style};
use reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};
use trox::{LocalizedString, ls, opaque, tx, tx_args, txa};

use crate::menu::arcade_route_transition;
use crate::menu::settings_panel;

use crate::menu::{
  arcade_tab_transition::ArcadeTabTransition,
  erase_control::EraseControl,
  erase_dialog::EraseDialog,
  font_scale::FontScale,
  graphics_settings::GraphicsSettings,
  input_settings::InputSettings,
  privacy_policy::PrivacyPolicyHelp,
  return_button::ReturnButton,
  screen_header::{HeaderVariant, ScreenHeader},
  select_control::SelectControl,
  settings_panel::SettingsPanel,
  settings_tabs::{SettingsTab, SettingsTabs},
  sound_settings::SoundSettings,
  toggle_control::ToggleControl,
};
use crate::settings::{self, Language, SettingsChange, SettingsContext, SettingsSaveStatus};
use battlement::Overflow;
use battlement::host_settings::{DisplayMode, DisplayResolution, HostSettings};

/// The source settings screen; the host owns routing and external URL requests.
#[builder]
pub struct SettingsScreen {
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  on_return: EventCallback<()>,
  #[builder(required)]
  on_open_url: EventCallback<String>,
  autofocus_heading: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsModal {
  Erase,
  Privacy,
}

impl Component for SettingsScreen {
  fn render(&self) -> impl Render {
    let (active_tab, set_active_tab) = hooks::use_state(SettingsTab::Gameplay);
    let (tab_direction, set_tab_direction) = hooks::use_state(1_i32);
    let settings = settings::use_settings();
    let host = reactant::use_host_settings();
    let navigation = arcade_route_transition::use_arcade_navigation();
    let (active_modal, set_active_modal) = hooks::use_state(None::<SettingsModal>);

    Region::new(txa(
      "{category} settings",
      tx_args![category => opaque(active_tab.label())],
      "Settings panel region.",
    ))
    .host_name("settings-screen")
    .style(
      Style::new()
        .position(Position::Absolute)
        .inset(0)
        .overflow(Overflow::Hidden),
    )
    .child((
      ScreenHeader::new()
        .variant(HeaderVariant::Settings)
        .autofocus(self.autofocus_heading),
      View::new()
        .name("settings-screen-composition")
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(68)
            .top(if settings.desired.text_size.factor() > 1.0 {
              330
            } else {
              233
            })
            .width(887),
        )
        .child((
          SettingsTabs::new()
            .active_tab(active_tab)
            .on_select(EventCallback::new({
              let set_active_tab = set_active_tab.clone();
              let set_tab_direction = set_tab_direction.clone();
              move |tab: SettingsTab| {
                if tab != active_tab {
                  set_tab_direction.set(if tab.index() > active_tab.index() {
                    1
                  } else {
                    -1
                  });
                  set_active_tab.set(tab);
                }
              }
            })),
          SettingsPanel::new().children(
            ArcadeTabTransition::new()
              .active_key(active_tab)
              .direction(tab_direction)
              .reduce_motion(navigation.reduce_motion)
              .children(self::panel(
                active_tab,
                &settings,
                &host,
                &set_active_modal,
                self.overlay.clone(),
              )),
          ),
        )),
      SettingsSaveStatus,
      ReturnButton::new()
        .reduced_motion(navigation.reduce_motion)
        .on_press(self.on_return.clone()),
      EraseDialog::new()
        .open(active_modal == Some(SettingsModal::Erase))
        .on_close(set_active_modal.callback().map_input(|_| None))
        .overlay(self.overlay.clone()),
      PrivacyPolicyHelp::new()
        .open(active_modal == Some(SettingsModal::Privacy))
        .reduce_motion(navigation.reduce_motion)
        .on_open_url(self.on_open_url.clone())
        .on_close(set_active_modal.callback().map_input(|_| None))
        .overlay(self.overlay.clone()),
    ))
  }
}

#[allow(clippy::too_many_arguments)]
fn panel(
  active_tab: SettingsTab,
  settings: &SettingsContext,
  host: &HostSettings,
  set_active_modal: &StateSetter<Option<SettingsModal>>,
  overlay: PortalTarget,
) -> impl Render {
  let value = settings.desired;
  if active_tab == SettingsTab::Input {
    return Either::Left(InputSettings::new().overlay(overlay));
  }

  Either::Right(
    ScrollRegion::new(txa(
      "{category} settings controls",
      tx_args![category => opaque(active_tab.label())],
      "Settings panel scroll area.",
    ))
    .host_name(format!("{}-settings-scroll", active_tab.slug()))
    .style(
      Style::new()
        .width(839)
        .height(settings_panel::content_height(
          value.text_size,
          settings.failed || settings.pending,
        )),
    )
    .child(match active_tab {
      SettingsTab::Gameplay => Either::Left(self::gameplay(settings, set_active_modal, overlay)),
      SettingsTab::Graphics => Either::Right(Either::Left(
        GraphicsSettings::new()
          .resolution(
            value
              .resolution(host)
              .map(|resolution| format!("{} × {}", resolution.width, resolution.height))
              .unwrap_or_else(|| "Current display".to_owned()),
          )
          .max_framerate(format!("{} FPS", value.framerate(host)))
          .display_mode(
            match value.display_mode {
              DisplayMode::Borderless => "Borderless",
              DisplayMode::Fullscreen => "Fullscreen",
              DisplayMode::Windowed => "Windowed",
            }
            .to_owned(),
          )
          .screenshake(value.screenshake)
          .vsync(value.vsync)
          .overlay(overlay)
          .on_resolution_change(
            settings
              .callback(SettingsChange::Resolution)
              .map_input(self::resolution_from_label),
          )
          .on_max_framerate_change(settings.callback(SettingsChange::MaxFramerate).map_input(
            |label: String| {
              label
                .trim_end_matches(" FPS")
                .parse::<u32>()
                .expect("framerate option")
            },
          ))
          .on_display_mode_change(settings.callback(SettingsChange::DisplayMode).map_input(
            |label: String| match label.as_str() {
              "Fullscreen" => DisplayMode::Fullscreen,
              "Windowed" => DisplayMode::Windowed,
              _ => DisplayMode::Borderless,
            },
          ))
          .on_screenshake_change(settings.callback(SettingsChange::Screenshake))
          .on_vsync_change(settings.callback(SettingsChange::Vsync)),
      )),
      SettingsTab::Sound => Either::Right(Either::Right(
        SoundSettings::new()
          .master_volume(value.master_volume)
          .music_volume(value.music_volume)
          .effects_volume(value.effects_volume)
          .mute_in_background(value.mute_in_background)
          .on_master_volume_change(settings.callback(SettingsChange::MasterVolume))
          .on_music_volume_change(settings.callback(SettingsChange::MusicVolume))
          .on_effects_volume_change(settings.callback(SettingsChange::EffectsVolume))
          .on_mute_in_background_change(settings.callback(SettingsChange::MuteInBackground)),
      )),
      SettingsTab::Input => unreachable!(),
    }),
  )
}

#[allow(clippy::too_many_arguments)]
fn gameplay(
  settings: &SettingsContext,
  set_active_modal: &StateSetter<Option<SettingsModal>>,
  overlay: PortalTarget,
) -> impl Render {
  let value = settings.desired;
  let font_scale = value.text_size;
  (
    SelectControl::new()
      .label(control_behavior::name_source_text(tx(
        "Language",
        "Gameplay language setting label.",
      )))
      .value(
        match value.language {
          Language::English => "English",
          Language::French => "Français",
        }
        .to_owned(),
      )
      .options(["English", "Français"].map(String::from).to_vec())
      .option_label(self::language_label)
      .overlay(overlay.clone())
      .on_change(
        settings
          .callback(SettingsChange::Language)
          .map_input(|label: String| {
            if label == "Français" {
              Language::French
            } else {
              Language::English
            }
          }),
      )
      .first(true),
    SelectControl::new()
      .label(control_behavior::name_source_text(tx(
        "Text Size",
        "Gameplay text-size setting label.",
      )))
      .value(font_scale.label().to_owned())
      .option_label(|value| ls(value))
      .options(
        FontScale::ALL
          .map(|value| value.label().to_owned())
          .to_vec(),
      )
      .overlay(overlay)
      .on_change(
        settings
          .callback(SettingsChange::TextSize)
          .map_input(|label: String| match label.as_str() {
            "150%" => FontScale::Percent150,
            "200%" => FontScale::Percent200,
            _ => FontScale::Percent100,
          }),
      ),
    ToggleControl::new()
      .label(control_behavior::name_source_text(tx(
        "Reduce Motion",
        "Gameplay reduced-motion setting label.",
      )))
      .checked(value.reduce_motion)
      .on_change(settings.callback(SettingsChange::ReduceMotion)),
    ToggleControl::new()
      .label(control_behavior::name_source_text(tx(
        "Increase Move\nDuration",
        "Two-line gameplay duration setting label.",
      )))
      .aria_label(tx(
        "Increase Move Duration",
        "Gameplay duration checkbox accessibility label.",
      ))
      .row_height(self::multiline_row_height(font_scale))
      .checked(value.increase_move_duration)
      .on_change(settings.callback(SettingsChange::IncreaseMoveDuration)),
    ToggleControl::new()
      .label(control_behavior::name_source_text(tx(
        "Upload Crash\nReports",
        "Two-line crash-report setting label.",
      )))
      .aria_label(tx(
        "Upload Crash Reports",
        "Crash-report checkbox accessibility label.",
      ))
      .row_height(self::multiline_row_height(font_scale))
      .checked(value.upload_crash_reports)
      .with_info(true)
      .on_info_click(
        set_active_modal
          .callback()
          .map_input(|_| Some(SettingsModal::Privacy)),
      )
      .on_change(settings.callback(SettingsChange::UploadCrashReports)),
    EraseControl::new().on_click(
      set_active_modal
        .callback()
        .map_input(|_| Some(SettingsModal::Erase)),
    ),
  )
}

fn multiline_row_height(scale: FontScale) -> f32 {
  match scale {
    FontScale::Percent100 => 159.0,
    FontScale::Percent150 => 227.0,
    FontScale::Percent200 => 211.0,
  }
}

fn resolution_from_label(label: String) -> DisplayResolution {
  let (width, height) = label.split_once(" × ").expect("resolution option");
  DisplayResolution {
    width: width.parse().expect("resolution width"),
    height: height.parse().expect("resolution height"),
    refresh_numerator: 60,
    refresh_denominator: 1,
  }
}

fn language_label(value: &str) -> LocalizedString {
  // Autonyms stay recognizable in either interface language.
  ls(value)
}
