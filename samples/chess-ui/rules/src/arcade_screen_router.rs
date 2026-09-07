//! Complete two-screen application composition.

use battlement::{Command, KeyEvent, PhysicalKey, Position, Style};
use battlement_reactant::{portal::PortalTarget, prelude::*};

use crate::{
  arcade_frame_pulse::ArcadeScreen, arcade_menu_transition::ArcadeMenuTransition,
  arcade_route_transition, main_menu::MainMenu, screen_frame::ExitAwareScreenFrame,
  settings_screen::SettingsScreen,
};

/// Selects the complete main or settings screen through one keyed transition.
#[builder]
pub struct ArcadeScreenRouter {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for ArcadeScreenRouter {
  fn render(&self) -> impl Render {
    let navigation = arcade_route_transition::use_arcade_navigation();
    let app = use_app();
    self::router(self, navigation, app)
  }
}

fn router(
  component: &ArcadeScreenRouter,
  navigation: arcade_route_transition::ArcadeNavigationContext,
  app: AppHandle,
) -> View {
  let dismiss = self::dismiss_action(navigation.clone());
  View::new()
    .name("arcade-application-input-boundary")
    .style(Style::new().position(Position::Absolute).inset(0))
    .on_key_down_event_callback(dismiss.clone().filter_map_input(self::escape))
    .on_navigation_cancel(dismiss)
    .child(
      ExitAwareScreenFrame::new()
        .frame_pulse(navigation.active_screen)
        .reduce_motion(navigation.reduce_motion)
        .children(
          ArcadeMenuTransition::new()
            .screen_key(navigation.active_screen)
            .play_transition(navigation.has_navigated)
            .reduce_motion(navigation.reduce_motion)
            .children(if navigation.active_screen == ArcadeScreen::Settings {
              Either::left(
                SettingsScreen::new()
                  .overlay(component.overlay.clone())
                  .on_return(navigation.navigate_callback(ArcadeScreen::Main))
                  .on_open_url(move |url| app.send(Command::open_external_url(url)))
                  .autofocus_heading(true),
              )
            } else {
              Either::right(MainMenu::new().autofocus_heading(true))
            }),
        ),
    )
}

fn dismiss_action(
  navigation: arcade_route_transition::ArcadeNavigationContext,
) -> EventCallback<()> {
  EventCallback::new(move |()| {
    if navigation.active_screen == ArcadeScreen::Settings {
      navigation.navigate(ArcadeScreen::Main);
    }
  })
}

fn escape(event: ReactantEvent<KeyEvent>) -> Option<()> {
  (event.payload().physical_key == Some(PhysicalKey::Escape)).then(|| {
    event.prevent_default();
    event.stop_propagation();
  })
}
