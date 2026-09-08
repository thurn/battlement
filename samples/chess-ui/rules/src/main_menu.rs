//! Complete main-menu composition with host-owned settings navigation.

use battlement::{Align, FlexDirection, Position, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};
use trox::{ls, tx};

use crate::{
  action_button::{ActionButton, ActionLabel},
  arcade_attract_mode::ArcadeAttractMode,
  arcade_exit_sequence::ArcadeExitStage,
  arcade_frame_pulse::ArcadeScreen,
  arcade_route_transition, background_music, font_scale,
  music_playback_indicator::MusicPlaybackIndicator,
  screen_header::{HeaderVariant, ScreenHeader},
};

const MENU_TOP: f32 = 476.0;
const MENU_LEFT: f32 = 132.0;
const MENU_WIDTH: f32 = 760.0;
const MENU_BUTTON_HEIGHT: f32 = 140.0;
const MENU_GAP: f32 = 24.0;

/// The source main menu, including attract mode, music, and terminal exit behavior.
#[builder]
pub struct MainMenu {
  autofocus_heading: bool,
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
  on_settings: EventCallback<()>,
  #[builder(required)]
  on_exit: EventCallback<()>,
}

impl Component for MainMenuContent {
  fn render(&self) -> impl Render {
    let scale = font_scale::use_font_scale();
    Region::new(ls("Chess Chess Revolution main menu"))
      .host_name("main-menu")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(-29)
          .top(-29)
          .width(1024)
          .height(1536)
          .overflow(battlement::Overflow::Hidden),
      )
      .child((
        ArcadeAttractMode::new().reduce_motion(self.reduce_motion),
        ScreenHeader::new()
          .variant(HeaderVariant::Game)
          .autofocus(self.autofocus_heading),
        Region::new(tx("Main navigation", "Main menu navigation label."))
          .host_name("main-menu-actions")
          .style(
            Style::new()
              .position(Position::Absolute)
              .left(MENU_LEFT)
              .top(MENU_TOP + (scale.factor() - 1.0) * 64.0)
              .width(MENU_WIDTH),
          )
          .child(
            Flex::new()
              .direction(FlexDirection::Column)
              .gap(MENU_GAP * (1.0 + (scale.factor() - 1.0) * 0.1))
              .style(Style::new().width(MENU_WIDTH).align_items(Align::Stretch))
              .child((
                self::action(
                  self,
                  "PLAY",
                  ActionLabel::Play,
                  1.0 + (scale.factor() - 1.0) * 0.12,
                  self.on_exit.clone(),
                ),
                self::action(
                  self,
                  "SETTINGS",
                  ActionLabel::Settings,
                  1.0 + (scale.factor() - 1.0) * 0.12,
                  self.on_settings.clone(),
                ),
                self::action(
                  self,
                  "ABOUT",
                  ActionLabel::About,
                  1.0 + (scale.factor() - 1.0) * 0.12,
                  EventCallback::noop(),
                ),
                self::action(
                  self,
                  "QUIT",
                  ActionLabel::Quit,
                  1.0 + (scale.factor() - 1.0) * 0.12,
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
              .bottom(218)
              .height(114),
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
        .width(MENU_WIDTH)
        .height(MENU_BUTTON_HEIGHT * scale),
    )
    .child(
      ActionButton::new()
        .artwork(artwork)
        .children(control_behavior::name_source_text(ls(label)))
        .max_text_scale(1.2)
        .disabled(component.exiting)
        .reduced_motion(component.reduce_motion)
        .on_press(on_press),
    )
}
