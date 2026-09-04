use trox::{assert_localized, tx};

use crate::controls;
use crate::{Control, Game, Interaction, Screen, design_system, sample_navigation};
use battlement_reactant::prelude::*;

#[builder]
pub(crate) struct Navigation {
  #[builder(required)]
  pub(crate) screen: Screen,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
  pub(crate) phone: bool,
}

impl Component for Navigation {
  fn render(&self) -> impl Render {
    if self.phone {
      return Node::new(
        battlement_reactant::host::View::new()
          .name("navigation")
          .style(design_system::phone_navigation())
          .child(
            battlement_reactant::host::Label::new(tx(
              "R",
              "User-facing product copy in the Reactant sample.",
            ))
            .style(design_system::phone_brand()),
          )
          .child(controls::interactive_button(
            "<",
            "previous-navigation",
            design_system::phone_navigation_action(controls::control_state(
              self.interaction,
              Control::PreviousNavigation,
            )),
            Control::PreviousNavigation,
            |game| game.screen = sample_navigation::previous(game.screen),
          ))
          .child(
            battlement_reactant::host::Label::new(assert_localized(sample_navigation::phone_name(
              self.screen,
            )))
            .name("phone-current-screen")
            .style(design_system::phone_navigation_label()),
          )
          .child(controls::interactive_button(
            ">",
            "next-navigation",
            design_system::phone_navigation_action(controls::control_state(
              self.interaction,
              Control::NextNavigation,
            )),
            Control::NextNavigation,
            |game| game.screen = sample_navigation::next(game.screen),
          )),
      );
    }
    Node::new(
      battlement_reactant::host::View::new()
        .name("navigation")
        .style(design_system::navigation(self.compact))
        .child(
          battlement_reactant::host::Label::new(if self.screen == Screen::TargetsTimelines {
            tx(
              "VALUES & TIME",
              "User-facing product copy in the Reactant sample.",
            )
          } else {
            tx(
              "REACTANT",
              "User-facing product copy in the Reactant sample.",
            )
          })
          .name(match self.screen {
            Screen::TargetsTimelines => "values-navigation",
            Screen::ValuesTimeControls => "gestures-navigation",
            Screen::GesturesDrag => "layout-gallery-navigation",
            Screen::LayoutGallery => "layout-reorder-navigation",
            Screen::LayoutReorder => "composed-effects-navigation",
            Screen::ComposedEffects => "layout-performance-navigation",
            Screen::LayoutPerformance => "motion-performance-navigation",
            _ => "targets-timelines-navigation",
          })
          .style(design_system::brand(self.compact))
          .on_click(|game: &mut Game| {
            game.screen = match game.screen {
              Screen::TargetsTimelines => Screen::ValuesTimeControls,
              Screen::ValuesTimeControls => Screen::GesturesDrag,
              Screen::GesturesDrag => Screen::LayoutGallery,
              Screen::LayoutGallery => Screen::LayoutReorder,
              Screen::LayoutReorder => Screen::ComposedEffects,
              Screen::ComposedEffects => Screen::LayoutPerformance,
              Screen::LayoutPerformance => Screen::MotionPerformance,
              Screen::MotionPerformance => Screen::TargetsTimelines,
              _ => Screen::TargetsTimelines,
            };
          }),
        )
        .child(
          battlement_reactant::host::View::new()
            .name("navigation-items")
            .style(design_system::navigation_items(self.compact))
            .child(controls::interactive_button(
              if self.compact {
                "01  Build"
              } else {
                "01  COMPOSITION"
              },
              "composition-navigation",
              design_system::navigation_item(
                self.screen == Screen::Composition,
                controls::control_state(self.interaction, Control::CompositionNavigation),
                self.compact,
              ),
              Control::CompositionNavigation,
              |game| game.screen = Screen::Composition,
            ))
            .child(controls::interactive_button(
              if self.compact {
                "02  Events"
              } else {
                "02  EVENTS & PORTALS"
              },
              "events-navigation",
              design_system::navigation_item(
                self.screen == Screen::EventsPortals,
                controls::control_state(self.interaction, Control::EventsNavigation),
                self.compact,
              ),
              Control::EventsNavigation,
              |game| game.screen = Screen::EventsPortals,
            ))
            .child(controls::interactive_button(
              if self.compact {
                "03  State"
              } else {
                "03  STATE & IDENTITY"
              },
              "state-navigation",
              design_system::navigation_item(
                self.screen == Screen::StateIdentity,
                controls::control_state(self.interaction, Control::StateNavigation),
                self.compact,
              ),
              Control::StateNavigation,
              |game| game.screen = Screen::StateIdentity,
            ))
            .child(controls::interactive_button(
              if self.compact {
                "04  Context"
              } else {
                "04  CONTEXT & MEMO"
              },
              "context-navigation",
              design_system::navigation_item(
                self.screen == Screen::ContextMemo,
                controls::control_state(self.interaction, Control::ContextNavigation),
                self.compact,
              ),
              Control::ContextNavigation,
              |game| game.screen = Screen::ContextMemo,
            ))
            .child(controls::interactive_button(
              if self.compact {
                "05  Effects"
              } else {
                "05  EFFECTS & STORES"
              },
              "effects-navigation",
              design_system::navigation_item(
                self.screen == Screen::EffectsStores,
                controls::control_state(self.interaction, Control::EffectsNavigation),
                self.compact,
              ),
              Control::EffectsNavigation,
              |game| game.screen = Screen::EffectsStores,
            ))
            .child(controls::interactive_button(
              "06  RESOURCES",
              "resources-navigation",
              design_system::navigation_item(
                self.screen == Screen::ResourcesBoundaries,
                controls::control_state(self.interaction, Control::ResourcesNavigation),
                self.compact,
              ),
              Control::ResourcesNavigation,
              |game| game.screen = Screen::ResourcesBoundaries,
            ))
            .child(controls::interactive_button(
              if self.compact {
                "07  Refs"
              } else {
                "07  REFS & GEOMETRY"
              },
              "refs-navigation",
              design_system::navigation_item(
                self.screen == Screen::RefsGeometry,
                controls::control_state(self.interaction, Control::RefsNavigation),
                self.compact,
              ),
              Control::RefsNavigation,
              |game| game.screen = Screen::RefsGeometry,
            ))
            .child(controls::interactive_button(
              "08  ASSETS",
              "assets-navigation",
              design_system::navigation_item(
                self.screen == Screen::Assets,
                controls::control_state(self.interaction, Control::AssetsNavigation),
                self.compact,
              ),
              Control::AssetsNavigation,
              |game| game.screen = Screen::Assets,
            )),
        ),
    )
  }
}
