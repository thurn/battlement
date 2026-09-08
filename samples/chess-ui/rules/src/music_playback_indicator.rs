//! Source-shaped music recommendation and sound toggle.

use crate::{
  background_music::{BackgroundMusicContext, BackgroundMusicStatus, use_background_music},
  music_heartbeat,
  setting_row::DISPLAY_FONT,
};
use battlement::{
  Align, Color, FlexDirection, Length, LengthUnits, Position, Rotate, Style, TextAnchor, TextShadow,
};
use battlement_reactant::{
  control_behavior,
  paint::PaintStyle,
  prelude::{PaintDropShadow, PaintFilterList, *},
};
use trox::ls;

/// Two-line recommendation that toggles the shared music output.
#[builder]
pub struct MusicPlaybackIndicator {
  reduced_motion: bool,
}

impl Component for MusicPlaybackIndicator {
  fn render(&self) -> impl Render {
    let music = use_background_music();
    let heartbeat = music_heartbeat::use_control_heartbeat(self.reduced_motion);
    self::button(&music, &heartbeat)
  }
}

fn button(
  music: &BackgroundMusicContext,
  heartbeat: &music_heartbeat::ControlHeartbeat,
) -> impl Render + use<> {
  Button::content(
    View::new()
      .style(
        Style::new()
          .position(Position::Relative)
          .full_size()
          .flex_direction(FlexDirection::Column)
          .align_items(Align::Center)
          .center_content(),
      )
      .child((
        self::recommendation_line("Playing with sound"),
        self::recommendation_line("is recommended!"),
        (!self::sound_enabled(music)).then(self::speaker_slash),
      )),
  )
  .host_name("music-playback-indicator")
  .animate(heartbeat.apply(StyleTarget::new()))
  .semantic_name(SemanticName::Text(ls(if self::sound_enabled(music) {
    "Mute background music"
  } else {
    "Enable background music"
  })))
  .on_press({
    let music = music.clone();
    move || self::toggle_sound(&music)
  })
  .style(
    Style::new()
      .position(Position::Relative)
      .full_size()
      .padding(0)
      .border_width(0)
      .background_color(Color::TRANSPARENT),
  )
}

fn sound_enabled(music: &BackgroundMusicContext) -> bool {
  music.status == BackgroundMusicStatus::Playing
    && !music.muted
    && music.master_volume > 0
    && music.music_volume > 0
}

fn toggle_sound(music: &BackgroundMusicContext) {
  if self::sound_enabled(music) {
    music.set_sound_muted(true);
    return;
  }
  if music.master_volume == 0 {
    music.set_master_volume(80);
  }
  if music.music_volume == 0 {
    music.set_music_volume(65);
  }
  music.set_sound_muted(false);
  music.start_music();
}

fn recommendation_line(text: &'static str) -> impl Render {
  control_behavior::static_label(ls(text)).key(text).style(
    Style::new()
      .color(Color::WHITE)
      .unity_font_definition(DISPLAY_FONT)
      .font_size(56)
      .unity_font_style_and_weight(battlement::FontStyle::Bold)
      .height(57)
      .letter_spacing(0.3)
      .unity_text_align(TextAnchor::MiddleCenter)
      .text_shadow(TextShadow::new(0.0, 3.0, 8.0, Color::BLACK)),
  )
}

fn speaker_slash() -> View {
  View::decorative()
    .key("music-speaker-slash")
    .name("music-speaker-slash")
    .style(
      Style::new()
        .position(Position::Absolute)
        .left(50.pct())
        .top(132)
        .width(54)
        .height(54)
        .margin_left(-27),
    )
    .paint(
      PaintStyle::new()
        .background(Color::TRANSPARENT)
        .paint_filter(PaintFilterList::default().drop_shadow(PaintDropShadow::new(
          0.0,
          0.0,
          8.0,
          0.0,
          Color::BLACK,
        ))),
    )
    .child((
      View::decorative().style(
        Style::new()
          .position(Position::Absolute)
          .left(3)
          .top(19)
          .width(15)
          .height(18)
          .background_color(Color::rgb8(150, 155, 169)),
      ),
      View::decorative()
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(14)
            .top(11)
            .width(24)
            .height(34),
        )
        .paint(
          PaintStyle::new()
            .background(Color::rgb8(150, 155, 169))
            .clip_polygon(
              [[0.0, 50.0], [100.0, 0.0], [100.0, 100.0]].map(|point| point.map(Length::percent)),
            ),
        ),
      View::decorative().style(
        Style::new()
          .position(Position::Absolute)
          .left(3)
          .top(25)
          .width(50)
          .height(6)
          .border_radius(3)
          .background_color(Color::rgb8(150, 155, 169))
          .rotate(Rotate::degrees(45.0)),
      ),
    ))
}
