//! Complete settings-screen composition with host-owned navigation boundaries.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Position, ScrollerVisibility, Style,
  Vector,
};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};
use trox::{ls, tx};

use crate::{
  arcade_modal::ArcadeModal,
  arcade_tab_transition::ArcadeTabTransition,
  background_music,
  erase_control::EraseControl,
  font_scale::{self, FontScale},
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsModal {
  Erase,
  Privacy,
}

/// The source settings screen; the host owns routing and external URL requests.
#[builder]
pub struct SettingsScreen {
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(default = EventCallback::noop())]
  on_return: EventCallback<()>,
  #[builder(required)]
  on_open_url: EventCallback<String>,
  autofocus_heading: bool,
}

impl Component for SettingsScreen {
  fn render(&self) -> impl Render {
    let (active_tab, set_active_tab) = hooks::use_state(SettingsTab::Gameplay);
    let (tab_direction, set_tab_direction) = hooks::use_state(1_i32);
    let (font_scale, set_font_scale) = font_scale::use_font_scale_state();
    let (language, set_language) = hooks::use_state(String::from("English"));
    let navigation = crate::arcade_route_transition::use_arcade_navigation();
    let (increase_move_duration, set_increase_move_duration) = hooks::use_state(true);
    let (upload_crash_reports, set_upload_crash_reports) = hooks::use_state(true);
    let (resolution, set_resolution) = hooks::use_state(String::from("1920 × 1080"));
    let (max_framerate, set_max_framerate) = hooks::use_state(String::from("144 FPS"));
    let (display_mode, set_display_mode) = hooks::use_state(String::from("Borderless"));
    let (screenshake, set_screenshake) = hooks::use_state(true);
    let (vsync, set_vsync) = hooks::use_state(true);
    let (effects_volume, set_effects_volume) = hooks::use_state(75_u32);
    let (panel_scrolled, set_panel_scrolled) = hooks::use_state([false; 3]);
    let (active_modal, set_active_modal) = hooks::use_state(None::<SettingsModal>);
    let music = background_music::use_background_music();

    font_scale::provider(
      font_scale,
      Region::new(ls(format!("{} settings", active_tab.label_text())))
        .host_name("settings-screen")
        .style(
          Style::new()
            .position(Position::Absolute)
            .inset(0)
            .overflow(battlement::Overflow::Hidden),
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
                    font_scale,
                    &set_font_scale,
                    &language,
                    &set_language,
                    navigation.reduce_motion,
                    &navigation.reduce_motion_callback(),
                    increase_move_duration,
                    &set_increase_move_duration,
                    upload_crash_reports,
                    &set_upload_crash_reports,
                    &resolution,
                    &set_resolution,
                    &max_framerate,
                    &set_max_framerate,
                    &display_mode,
                    &set_display_mode,
                    screenshake,
                    &set_screenshake,
                    vsync,
                    &set_vsync,
                    effects_volume,
                    &set_effects_volume,
                    panel_scrolled,
                    &set_panel_scrolled,
                    &music,
                    &set_active_modal,
                    self.overlay.clone(),
                  )),
              ),
            )),
          ReturnButton::new().on_press(self.on_return.clone()),
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
        )),
    )
  }
}

#[allow(clippy::too_many_arguments)]
fn panel(
  active_tab: SettingsTab,
  font_scale: FontScale,
  set_font_scale: &StateSetter<FontScale>,
  language: &str,
  set_language: &StateSetter<String>,
  reduce_motion: bool,
  set_reduce_motion: &EventCallback<bool>,
  increase_move_duration: bool,
  set_increase_move_duration: &StateSetter<bool>,
  upload_crash_reports: bool,
  set_upload_crash_reports: &StateSetter<bool>,
  resolution: &str,
  set_resolution: &StateSetter<String>,
  max_framerate: &str,
  set_max_framerate: &StateSetter<String>,
  display_mode: &str,
  set_display_mode: &StateSetter<String>,
  screenshake: bool,
  set_screenshake: &StateSetter<bool>,
  vsync: bool,
  set_vsync: &StateSetter<bool>,
  effects_volume: u32,
  set_effects_volume: &StateSetter<u32>,
  panel_scrolled: [bool; 3],
  set_panel_scrolled: &StateSetter<[bool; 3]>,
  music: &background_music::BackgroundMusicContext,
  set_active_modal: &StateSetter<Option<SettingsModal>>,
  overlay: PortalTarget,
) -> impl Render {
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
        font_scale,
        set_font_scale,
        language,
        set_language,
        reduce_motion,
        set_reduce_motion,
        increase_move_duration,
        set_increase_move_duration,
        upload_crash_reports,
        set_upload_crash_reports,
        set_panel_scrolled,
        set_active_modal,
        overlay,
      )),
      SettingsTab::Graphics => Either::Right(Either::Left(
        GraphicsSettings::new()
          .resolution(resolution.to_owned())
          .max_framerate(max_framerate.to_owned())
          .display_mode(display_mode.to_owned())
          .screenshake(screenshake)
          .vsync(vsync)
          .overlay(overlay)
          .on_resolution_change(set_resolution.clone())
          .on_max_framerate_change(set_max_framerate.clone())
          .on_display_mode_change(set_display_mode.clone())
          .on_screenshake_change(set_screenshake.clone())
          .on_vsync_change(set_vsync.clone()),
      )),
      SettingsTab::Sound => Either::Right(Either::Right(
        SoundSettings::new()
          .master_volume(music.master_volume)
          .music_volume(music.music_volume)
          .effects_volume(effects_volume)
          .mute_in_background(music.mute_in_background)
          .on_master_volume_change({
            let music = music.clone();
            move |value| music.set_master_volume(value)
          })
          .on_music_volume_change({
            let music = music.clone();
            move |value| music.set_music_volume(value)
          })
          .on_effects_volume_change(set_effects_volume.clone())
          .on_mute_in_background_change({
            let music = music.clone();
            move |value| music.set_mute_in_background(value)
          }),
      )),
      SettingsTab::Input => unreachable!(),
    }),
  )
}

#[allow(clippy::too_many_arguments)]
fn gameplay(
  font_scale: FontScale,
  set_font_scale: &StateSetter<FontScale>,
  language: &str,
  set_language: &StateSetter<String>,
  reduce_motion: bool,
  set_reduce_motion: &EventCallback<bool>,
  increase_move_duration: bool,
  set_increase_move_duration: &StateSetter<bool>,
  upload_crash_reports: bool,
  set_upload_crash_reports: &StateSetter<bool>,
  set_panel_scrolled: &StateSetter<[bool; 3]>,
  set_active_modal: &StateSetter<Option<SettingsModal>>,
  overlay: PortalTarget,
) -> impl Render {
  (
    SelectControl::new()
      .label(control_behavior::name_source_text(tx(
        "Language",
        "Gameplay language setting label.",
      )))
      .value(language.to_owned())
      .options(
        ["English", "Español", "Français", "Deutsch"]
          .map(String::from)
          .to_vec(),
      )
      .overlay(overlay.clone())
      .on_change(set_language.clone())
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
        set_font_scale
          .callback()
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
      .checked(reduce_motion)
      .on_change(set_reduce_motion.clone()),
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
      .checked(increase_move_duration)
      .on_change(set_increase_move_duration.clone()),
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
      .checked(upload_crash_reports)
      .with_info(true)
      .on_info_click(
        set_active_modal
          .callback()
          .map_input(|_| Some(SettingsModal::Privacy)),
      )
      .on_change(set_upload_crash_reports.clone()),
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
