//! Complete main-menu composition with host-owned settings navigation.

use battlement::{Align, FlexDirection, Position, Style};
use reactant::{control_behavior, hooks, prelude::*};
use trox::tx;

use crate::menu::{
  action_button::{ActionButton, ActionLabel},
  arcade_attract_mode::ArcadeAttractMode,
  arcade_exit_sequence::ArcadeExitStage,
  arcade_frame_pulse::ArcadeScreen,
  arcade_route_transition, background_music, font_scale,
  music_playback_indicator::MusicPlaybackIndicator,
  screen_header::{HeaderVariant, ScreenHeader},
};
use battlement::Overflow;

const MENU_TOP: f32 = 476.0;
const MENU_LEFT: f32 = 132.0;
const MENU_WIDTH: f32 = 760.0;
const MENU_BUTTON_HEIGHT: f32 = 140.0;
const MENU_GAP: f32 = 24.0;

/// The source main menu, including attract mode, music, and terminal exit behavior.
#[builder]
pub struct MainMenu {
  autofocus_heading: bool,
  #[builder(required)]
  on_play: EventCallback<()>,
}

impl Component for MainMenu {
  fn render(&self) -> impl Render {
    let (exiting, set_exiting) = hooks::use_state(false);
    let music = background_music::use_background_music();
    let navigation = arcade_route_transition::use_arcade_navigation();
    hooks::use_effect(
      {
        let music = music.clone();
        move || music.start_music()
      },
      (),
    );
    ArcadeExitStage::new()
      .active(exiting)
      .reduce_motion(navigation.reduce_motion)
      .children(
        MainMenuContent::new()
          .exiting(exiting)
          .reduce_motion(navigation.reduce_motion)
          .autofocus_heading(self.autofocus_heading)
          .on_play(self.on_play.clone())
          .on_settings(EventCallback::new(move |()| {
            navigation.navigate(ArcadeScreen::Settings)
          }))
          .on_exit(set_exiting.callback().map_input(|_| true)),
      )
  }
}

#[builder]
struct MainMenuContent {
  #[builder(required)]
  exiting: bool,
  #[builder(required)]
  reduce_motion: bool,
  autofocus_heading: bool,
  #[builder(required)]
  on_play: EventCallback<()>,
  #[builder(required)]
  on_settings: EventCallback<()>,
  #[builder(required)]
  on_exit: EventCallback<()>,
}

impl Component for MainMenuContent {
  fn render(&self) -> impl Render {
    let scale = font_scale::use_font_scale();
    Region::new(tx("Chess Chess Revolution main menu", "Main menu region."))
      .host_name("main-menu")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(-29)
          .top(-29)
          .width(1024)
          .height(1536)
          .overflow(Overflow::Hidden),
      )
      .child((
        ArcadeAttractMode::new().reduce_motion(self.reduce_motion),
        ScreenHeader::new()
          .variant(HeaderVariant::Game)
          .autofocus(self.autofocus_heading),
        ScrollRegion::new(tx("Main navigation", "Main menu navigation label."))
          .host_name("main-menu-actions")
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(if scale.factor() > 1.0 {
                8.0
              } else {
                MENU_LEFT - 24.0
              })
              .top(MENU_TOP - 24.0)
              .width(if scale.factor() > 1.0 {
                1008.0
              } else {
                MENU_WIDTH + 48.0
              })
              .height(if scale.factor() > 1.0 { 638.0 } else { 748.0 }),
          )
          .child(
            Flex::new()
              .direction(FlexDirection::Column)
              .gap(MENU_GAP * (1.0 + (scale.factor() - 1.0) * 0.1))
              .style(
                Style::new()
                  .width(if scale.factor() > 1.0 {
                    1008.0
                  } else {
                    MENU_WIDTH + 48.0
                  })
                  .padding(24)
                  .align_items(Align::Stretch),
              )
              .child((
                self::action(
                  self,
                  "PLAY",
                  ActionLabel::Play,
                  scale.factor(),
                  self.on_play.clone(),
                ),
                self::action(
                  self,
                  "SETTINGS",
                  ActionLabel::Settings,
                  scale.factor(),
                  self.on_settings.clone(),
                ),
                self::action(
                  self,
                  "ABOUT",
                  ActionLabel::About,
                  scale.factor(),
                  EventCallback::noop(),
                ),
                self::action(
                  self,
                  "QUIT",
                  ActionLabel::Quit,
                  scale.factor(),
                  self.on_exit.clone(),
                ),
              )),
          ),
        View::new()
          .name("main-menu-music-indicator")
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(80)
              .right(80)
              .bottom(if scale.factor() > 1.0 { 170 } else { 218 })
              .height(114.0 * scale.factor()),
          )
          .child(MusicPlaybackIndicator::new().reduced_motion(self.reduce_motion)),
      ))
  }
}

fn action(
  component: &MainMenuContent,
  label: &'static str,
  artwork: ActionLabel,
  scale: f32,
  on_press: EventCallback<()>,
) -> View {
  View::new()
    .name(format!("main-menu-action-{}", label.to_ascii_lowercase()))
    .style(
      Style::new()
        .width(if scale > 1.0 { 960.0 } else { MENU_WIDTH })
        .height(MENU_BUTTON_HEIGHT * scale),
    )
    .child(
      ActionButton::new()
        .artwork(artwork)
        .children(control_behavior::name_source_text(artwork.label()))
        .disabled(component.exiting)
        .reduced_motion(component.reduce_motion)
        .on_press(on_press),
    )
}
