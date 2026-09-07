//! Gallery launcher for the complete full-screen application.

use battlement_reactant::{element_ref, hooks, portal::PortalTarget, prelude::*};
use trox::tx;

use crate::{
  arcade_screen_router::application_layer, background_music, gallery, review_button::ReviewButton,
};

/// Opens the complete app as an unanchored layer and restores launcher focus.
#[builder]
pub struct ArcadeScreenRouterHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for ArcadeScreenRouterHarness {
  fn render(&self) -> impl Render {
    let (open, set_open) = hooks::use_state(false);
    let launcher = element_ref::use_element_ref();
    let gallery_layer = gallery::use_review_app_layer();
    let music = background_music::use_background_music();
    self::harness(self, open, set_open, launcher, gallery_layer, music)
  }
}

fn harness(
  component: &ArcadeScreenRouterHarness,
  open: bool,
  set_open: StateSetter<bool>,
  launcher: ElementRef,
  gallery_layer: gallery::ReviewAppLayerContext,
  music: background_music::BackgroundMusicContext,
) -> impl Render {
  let close = EventCallback::new({
    let launcher = launcher.clone();
    let gallery_layer = gallery_layer.clone();
    let set_open = set_open.clone();
    move |()| {
      set_open.set(false);
      gallery_layer.set_open(false);
      launcher.focus();
    }
  });
  (
    ReviewButton::new()
      .label(tx(
        "Launch Chess UI",
        "Open the complete Chess Chess Revolution interface.",
      ))
      .name("arcade-app-launcher")
      .element_ref(launcher)
      .on_press(EventCallback::new({
        let gallery_layer = gallery_layer.clone();
        let music = music.clone();
        move |()| {
          music.set_master_volume(80);
          music.set_music_volume(65);
          music.set_mute_in_background(false);
          music.set_sound_muted(false);
          music.start_music();
          set_open.set(true);
          gallery_layer.set_open(true);
        }
      })),
    open.then(|| application_layer(component.overlay.clone(), close)),
  )
}
