//! Complete two-screen application composition and review-layer dismissal.

use battlement::{Command, KeyEvent, PhysicalKey, Position, Style};
use battlement_reactant::{portal::PortalTarget, prelude::*};

use crate::{
  arcade_frame_pulse::ArcadeScreen,
  arcade_menu_transition::ArcadeMenuTransition,
  arcade_route_transition::{self, ArcadeRouteTransition},
  font_scale::FontScaleProvider,
  main_menu::MainMenu,
  portrait_viewport::PortraitViewport,
  screen_frame::ExitAwareScreenFrame,
  settings_screen::SettingsScreen,
};

/// Selects the complete main or settings screen through one keyed transition.
#[builder]
pub struct ArcadeScreenRouter {
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  on_close: EventCallback<()>,
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
  let dismiss = self::dismiss_action(navigation.clone(), component.on_close.clone());
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
  on_close: EventCallback<()>,
) -> EventCallback<()> {
  if navigation.active_screen == ArcadeScreen::Settings {
    EventCallback::new(move |()| navigation.navigate(ArcadeScreen::Main))
  } else {
    on_close
  }
}

fn escape(event: ReactantEvent<KeyEvent>) -> Option<()> {
  (event.payload().physical_key == Some(PhysicalKey::Escape)).then(|| {
    event.prevent_default();
    event.stop_propagation();
  })
}

/// Creates the unanchored review layer and complete application provider tree.
pub fn application_layer(overlay: PortalTarget, on_close: EventCallback<()>) -> Overlay {
  let application_overlay = overlay.clone();
  Overlay::layer(overlay)
    .host_name("chess-ui-application-layer")
    .style(Style::new().background_color(battlement::Color::BLACK))
    .child(
      FontScaleProvider::new().children(
        PortraitViewport::new().child(
          ArcadeRouteTransition::new().children(
            ArcadeScreenRouter::new()
              .overlay(application_overlay)
              .on_close(on_close),
          ),
        ),
      ),
    )
}
