//! Complete settings-screen composition with host-owned navigation boundaries.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Position, ScrollerVisibility, Style,
  Vector,
};
use reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};
use trox::{ls, tx};

use crate::menu::arcade_route_transition;
use crate::menu::{
  arcade_modal::ArcadeModal,
  arcade_tab_transition::ArcadeTabTransition,
  erase_control::EraseControl,
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
    let (panel_scrolled, set_panel_scrolled) = hooks::use_state([false; 3]);
    let (active_modal, set_active_modal) = hooks::use_state(None::<SettingsModal>);

    Region::new(ls(format!("{} settings", active_tab.label_text())))
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
              .top(233)
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
                  panel_scrolled,
                  &set_panel_scrolled,
                  &set_active_modal,
                  self.overlay.clone(),
                )),
            ),
          )),
        SettingsSaveStatus,
        ReturnButton::new()
          .reduced_motion(navigation.reduce_motion)
          .on_press(self.on_return.clone()),
        ArcadeModal::new()
          .open(active_modal == Some(SettingsModal::Erase))
          .title(tx("Erase Saved Data?", "Saved-data confirmation title."))
          .children(Text::new(tx(
            "All saved data will be permanently erased. This cannot be undone.",
            "Saved-data confirmation warning.",
          )))
          .confirm_label(tx("Erase", "Saved-data confirmation action."))
          .cancel_label(tx("Cancel", "Saved-data cancellation action."))
          .danger(true)
          .reduce_motion(navigation.reduce_motion)
          .on_confirm(set_active_modal.callback().map_input(|_| None))
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
  panel_scrolled: [bool; 3],
  set_panel_scrolled: &StateSetter<[bool; 3]>,
  set_active_modal: &StateSetter<Option<SettingsModal>>,
  overlay: PortalTarget,
) -> impl Render {
  let value = settings.desired;
  let font_scale = value.text_size;
  if active_tab == SettingsTab::Input {
    return Either::Left(InputSettings::new().overlay(overlay));
  }

  let index = active_tab.index();
  let scrolled = panel_scrolled[index];
  Either::Right(
    ScrollArea::new(
      Some(ls(format!("{} settings controls", active_tab.label_text()))),
      AccessibilityScrollAxis::Vertical,
      font_scale.factor() > 1.0 && !scrolled,
      scrolled,
    )
    .host_name(format!("{}-settings-scroll", active_tab.slug()))
    .on_scroll({
      let set_panel_scrolled = set_panel_scrolled.clone();
      move |direction| {
        set_panel_scrolled.update(move |mut values| {
          values[index] = direction == AccessibilityScrollDirection::Forward;
          values
        })
      }
    })
    .configure_host(move |host| {
      host
        .scroll_offset(Vector::new(
          0.0,
          if scrolled {
            self::content_height(active_tab, font_scale).max(971.0) - 971.0
          } else {
            0.0
          },
        ))
        .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
        .vertical_scroller_visibility(if font_scale.factor() > 1.0 {
          ScrollerVisibility::Auto
        } else {
          ScrollerVisibility::Hidden
        })
    })
    .style(Style::new().width(839).height(971))
    .child(match active_tab {
      SettingsTab::Gameplay => Either::Left(self::gameplay(
        settings,
        set_panel_scrolled,
        set_active_modal,
        overlay,
      )),
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
  set_panel_scrolled: &StateSetter<[bool; 3]>,
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
          })
          .then(set_panel_scrolled.callback().map_input(|_| [false; 3])),
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

fn content_height(tab: SettingsTab, scale: FontScale) -> f32 {
  match tab {
    SettingsTab::Gameplay => {
      (4.0 * 159.0 + 2.0 * self::multiline_row_height(scale)) * scale.factor()
    }
    SettingsTab::Graphics => 5.0 * 159.0 * scale.factor(),
    SettingsTab::Sound => 971.0 * scale.factor(),
    SettingsTab::Input => 971.0,
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
