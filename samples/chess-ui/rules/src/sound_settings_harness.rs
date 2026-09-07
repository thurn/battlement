//! Shared-audio review harness for the Sound settings composition.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Align, Color, FlexDirection,
  ScrollerVisibility, Style, TextAnchor, Vector,
};
use battlement_reactant::{application, control_behavior, hooks, prelude::*};
use trox::ls;

use crate::{
  background_music::{
    self, BackgroundMusicContext, BackgroundMusicProvider, BackgroundMusicStatus,
  },
  font_scale::{self, FontScale},
  settings_panel::SettingsPanel,
  sound_settings::SoundSettings,
};

/// Exercises Sound values, background lifecycle, large text, scrolling, and reset.
#[builder]
pub struct SoundSettingsHarness;

impl Component for SoundSettingsHarness {
  fn render(&self) -> impl Render {
    let (hidden, set_hidden) = hooks::use_state(false);
    application::provider(battlement::application::ApplicationState {
      focused: true,
      paused: hidden,
    })
    .child(
      BackgroundMusicProvider::new().children(
        SoundSettingsSpecimen::new()
          .hidden(hidden)
          .set_hidden(set_hidden),
      ),
    )
  }
}

#[builder]
struct SoundSettingsSpecimen {
  #[builder(required)]
  hidden: bool,
  #[builder(required)]
  set_hidden: StateSetter<bool>,
}

impl Component for SoundSettingsSpecimen {
  fn render(&self) -> impl Render {
    let music = background_music::use_background_music();
    let (effects_volume, set_effects_volume) = hooks::use_state(75_u32);
    let (font_scale, set_font_scale) = hooks::use_state(FontScale::Percent100);
    let (scrolled, set_scrolled) = hooks::use_state(false);
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);

    View::new()
      .name("sound-settings-harness")
      .style(Style::new().width(887).margin_top(8))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(10.0)
          .style(Style::new().min_height(64).align_items(Align::Center))
          .child((
            self::button(
              "START",
              "sound-settings-start",
              EventCallback::new({
                let music = music.clone();
                move |()| music.start_music()
              }),
            ),
            self::button(
              if self.hidden { "HIDDEN" } else { "VISIBLE" },
              "sound-settings-visibility",
              self.set_hidden.update_callback(|hidden| !hidden),
            ),
            self::button(
              font_scale.label(),
              "sound-settings-text-size",
              set_font_scale
                .update_callback(|scale| match scale {
                  FontScale::Percent100 => FontScale::Percent200,
                  _ => FontScale::Percent100,
                })
                .then(set_scrolled.callback().map_input(|_| false)),
            ),
            self::button(
              "RESET",
              "sound-settings-reset",
              EventCallback::new({
                let music = music.clone();
                move |()| music.reset()
              })
              .then(set_effects_volume.callback().map_input(|_| 75))
              .then(
                set_font_scale
                  .callback()
                  .map_input(|_| FontScale::Percent100),
              )
              .then(self.set_hidden.callback().map_input(|_| false))
              .then(set_scrolled.callback().map_input(|_| false))
              .then(set_reset_generation.update_callback(|value| value.wrapping_add(1))),
            ),
          )),
        self::status(&music, effects_volume),
        font_scale::provider(
          font_scale,
          SettingsPanel::new().children(
            ScrollArea::new(
              Some(ls("Sound settings controls")),
              AccessibilityScrollAxis::Vertical,
              font_scale.factor() > 1.0 && !scrolled,
              scrolled,
            )
            .on_scroll({
              let set_scrolled = set_scrolled.clone();
              move |direction| set_scrolled.set(direction == AccessibilityScrollDirection::Forward)
            })
            .host_name("sound-settings-scroll")
            .configure_host(|host| {
              host
                .scroll_offset(Vector::new(
                  0.0,
                  if scrolled {
                    971.0 * font_scale.factor()
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
            .child(
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
                .on_effects_volume_change(set_effects_volume)
                .on_mute_in_background_change({
                  let music = music.clone();
                  move |value| music.set_mute_in_background(value)
                })
                .key(reset_generation),
            ),
          ),
        ),
      ))
  }
}

fn button(label: &'static str, name: &'static str, on_press: EventCallback<()>) -> impl Render {
  Button::new(ls(label))
    .host_name(name)
    .on_press(on_press)
    .style(
      Style::new()
        .width(160)
        .height(54)
        .padding((8, 12))
        .border_width(1)
        .border_color(Color::hex(0x31455d))
        .border_radius(6)
        .background_color(Color::hex(0x101a28))
        .color(Color::hex(0xd4e4f1))
        .font_size(24)
        .unity_text_align(TextAnchor::MiddleCenter),
    )
}

fn status(music: &BackgroundMusicContext, effects_volume: u32) -> Label {
  let playback = match music.status {
    BackgroundMusicStatus::Stopped => "stopped",
    BackgroundMusicStatus::Playing => "playing",
    BackgroundMusicStatus::Unavailable => "unavailable",
  };
  control_behavior::static_label(ls(format!(
    "Playback {playback} · Output {} · Effective {:.0}% · Effects {effects_volume}%",
    if music.muted { "muted" } else { "audible" },
    music.effective_volume * 100.0,
  )))
  .name("sound-settings-status")
  .style(Style::new().width(839).height(42).font_size(20))
}
