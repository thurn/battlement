//! Controlled application routing and shared reduced-motion state.

use battlement_reactant::{hooks, prelude::*};

use crate::arcade_frame_pulse::ArcadeScreen;

/// Values and actions exposed by [`use_arcade_navigation`].
#[derive(Clone, PartialEq)]
pub struct ArcadeNavigationContext {
  /// The route currently selected by the application.
  pub active_screen: ArcadeScreen,
  /// Whether at least one route replacement has occurred.
  pub has_navigated: bool,
  /// The effective player-selected reduced-motion policy.
  pub reduce_motion: bool,
  set_active_screen: StateSetter<ArcadeScreen>,
  set_has_navigated: StateSetter<bool>,
  set_reduce_motion: StateSetter<bool>,
}

impl ArcadeNavigationContext {
  /// Selects a different route and leaves current-route requests inert.
  pub fn navigate(&self, screen: ArcadeScreen) {
    if screen != self.active_screen {
      self.set_has_navigated.set(true);
      self.set_active_screen.set(screen);
    }
  }

  /// Builds an activation callback for a fixed destination route.
  pub fn navigate_callback(&self, screen: ArcadeScreen) -> EventCallback<()> {
    let navigation = self.clone();
    EventCallback::new(move |()| navigation.navigate(screen))
  }

  /// Replaces the player-selected reduced-motion policy.
  pub fn set_reduce_motion(&self, reduce_motion: bool) {
    self.set_reduce_motion.set(reduce_motion);
  }

  /// Builds a controlled callback for the reduced-motion setting.
  pub fn reduce_motion_callback(&self) -> EventCallback<bool> {
    self.set_reduce_motion.callback()
  }
}

/// Provides the two-route application state to the complete screen tree.
#[builder]
pub struct ArcadeRouteTransition {
  #[builder(required, into)]
  children: Children,
  #[builder(default = ArcadeScreen::Main)]
  initial_screen: ArcadeScreen,
}

impl Component for ArcadeRouteTransition {
  fn render(&self) -> impl Render {
    let (active_screen, set_active_screen) = hooks::use_state(self.initial_screen);
    let (has_navigated, set_has_navigated) = hooks::use_state(false);
    let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
    ContextProvider::new()
      .context(Some(ArcadeNavigationContext {
        active_screen,
        has_navigated,
        reduce_motion,
        set_active_screen,
        set_has_navigated,
        set_reduce_motion,
      }))
      .child(self.children.render())
  }
}

/// Reads the nearest complete-app route and motion policy.
pub fn use_arcade_navigation() -> ArcadeNavigationContext {
  let provided = hooks::use_context::<Option<ArcadeNavigationContext>>();
  let (active_screen, set_active_screen) = hooks::use_state(ArcadeScreen::Main);
  let (has_navigated, set_has_navigated) = hooks::use_state(false);
  let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
  provided.unwrap_or(ArcadeNavigationContext {
    active_screen,
    has_navigated,
    reduce_motion,
    set_active_screen,
    set_has_navigated,
    set_reduce_motion,
  })
}
