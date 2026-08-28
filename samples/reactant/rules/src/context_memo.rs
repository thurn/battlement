use battlement_reactant::prelude::*;

use crate::{Control, Game, design_system};

static THEME: Context<Theme> = Context::new(|| Theme::Outer);

pub(crate) struct ContextMemo {
  pub(crate) overridden: bool,
  pub(crate) interaction: design_system::ControlState,
  pub(crate) compact: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Theme {
  Outer,
  Overridden,
}

struct ThemeCard {
  scope: &'static str,
}

impl Component for ContextMemo {
  fn render(&self) -> impl Render {
    let nested = if self.overridden {
      Node::new(
        THEME
          .provider(Theme::Overridden)
          .child(ThemeCard { scope: "NESTED" }),
      )
    } else {
      Node::new(ThemeCard { scope: "NESTED" })
    };
    VisualElement::new()
      .name("context-canvas")
      .style(design_system::canvas(self.compact))
      .child(Label::new("CONTEXT & MEMO").style(design_system::eyebrow()))
      .child(
        Label::new("Values follow ancestry")
          .name("context-title")
          .style(design_system::title()),
      )
      .child(crate::interactive_button(
        if self.overridden {
          "RESTORE"
        } else {
          "OVERRIDE"
        },
        "context-action",
        design_system::primary_action(self.interaction),
        Control::ContextAction,
        |game: &mut Game| game.context_overridden = !game.context_overridden,
      ))
      .child(
        VisualElement::new()
          .name("context-specimen")
          .style(design_system::context_specimen())
          .child(Label::new("Nearest provider wins").style(design_system::specimen_title()))
          .child(
            VisualElement::new()
              .name("context-cards")
              .style(design_system::context_row())
              .child(ThemeCard { scope: "OUTER" })
              .child(nested),
          ),
      )
  }
}

impl Component for ThemeCard {
  fn render(&self) -> impl Render {
    let theme = use_context(&THEME);
    let (name, color) = match theme {
      Theme::Outer => ("DEFAULT", design_system::CYAN),
      Theme::Overridden => ("OVERRIDDEN", design_system::CONTEXT_OVERRIDE),
    };
    VisualElement::new()
      .name(format!("context-{}", self.scope.to_ascii_lowercase()))
      .style(design_system::context_card(color))
      .child(Label::new(self.scope).style(design_system::context_scope()))
      .child(
        Label::new(name)
          .name("context-theme")
          .style(design_system::context_theme(color)),
      )
  }
}
