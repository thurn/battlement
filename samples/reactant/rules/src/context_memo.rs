use battlement_reactant::prelude::*;

use crate::{Control, Game, design_system};

static THEME: Context<Theme> = Context::new(|| Theme::Outer);

pub(crate) struct ContextMemo {
  pub(crate) overridden: bool,
  pub(crate) unrelated: u8,
  pub(crate) interaction: design_system::ControlState,
  pub(crate) unrelated_interaction: design_system::ControlState,
  pub(crate) compact: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Theme {
  Outer,
  Overridden,
}

#[derive(PartialEq)]
struct ThemeCard {
  scope: &'static str,
}

impl Component for ContextMemo {
  fn render(&self) -> impl Render {
    let override_action = use_callback(
      |game: &mut Game| game.context_overridden = !game.context_overridden,
      (),
    );
    let unrelated_action = use_callback(|game: &mut Game| game.context_unrelated ^= 1, ());
    let unrelated = use_memo(|| format!("VALUE  {}", self.unrelated), self.unrelated);
    let nested = if self.overridden {
      Node::new(
        THEME
          .provider(Theme::Overridden)
          .child(memo(ThemeCard { scope: "NESTED" })),
      )
    } else {
      Node::new(memo(ThemeCard { scope: "NESTED" }))
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
      .child(
        VisualElement::new()
          .name("context-specimen")
          .style(design_system::context_specimen())
          .child(
            VisualElement::new()
              .name("context-control")
              .style(design_system::context_control())
              .child(
                Label::new("CONTEXT  Nearest provider wins")
                  .style(design_system::experiment_title()),
              )
              .child(crate::interactive_button(
                if self.overridden {
                  "RESTORE DEFAULT"
                } else {
                  "OVERRIDE NESTED"
                },
                "context-action",
                design_system::context_action(self.interaction),
                Control::ContextAction,
                move |game: &mut Game| override_action(game),
              )),
          )
          .child(
            VisualElement::new()
              .name("context-cards")
              .style(design_system::context_row())
              .child(memo(ThemeCard { scope: "OUTER" }))
              .child(nested),
          ),
      )
      .child(
        VisualElement::new()
          .name("memo-experiment")
          .style(design_system::memo_experiment())
          .child(Label::new("MEMO  Unrelated value").style(design_system::experiment_title()))
          .child(crate::interactive_button(
            if self.unrelated == 0 {
              "CHANGE VALUE"
            } else {
              "RESET VALUE"
            },
            "context-unrelated-action",
            design_system::memo_action(self.unrelated_interaction),
            Control::ContextUnrelatedAction,
            move |game: &mut Game| unrelated_action(game),
          ))
          .child(
            Label::new(unrelated)
              .name("context-unrelated-value")
              .style(design_system::context_counter()),
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
