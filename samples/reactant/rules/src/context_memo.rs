use trox::{ls, tx};

use crate::{Control, Game, design_system};
use battlement_reactant::prelude::*;

static THEME: Context<Theme> = Context::new(|| Theme::Outer);

#[builder]
pub(crate) struct ContextMemo {
  pub(crate) overridden: bool,
  pub(crate) unrelated: u8,
  #[builder(required)]
  pub(crate) interaction: design_system::ControlState,
  #[builder(required)]
  pub(crate) unrelated_interaction: design_system::ControlState,
  pub(crate) compact: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Theme {
  Outer,
  Overridden,
}

#[builder]
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
          .child(memo(ThemeCard::new().scope("NESTED"))),
      )
    } else {
      Node::new(memo(ThemeCard::new().scope("NESTED")))
    };
    battlement_reactant::host::View::new()
      .name("context-canvas")
      .style(design_system::canvas(self.compact))
      .child(
        battlement_reactant::host::Label::new(tx(
          "CONTEXT & MEMO",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(design_system::eyebrow()),
      )
      .child(
        battlement_reactant::host::Label::new(tx(
          "Values follow ancestry",
          "User-facing product copy in the Reactant sample.",
        ))
        .name("context-title")
        .style(design_system::title()),
      )
      .child(
        battlement_reactant::host::View::new()
          .name("context-specimen")
          .style(design_system::context_specimen())
          .child(
            battlement_reactant::host::View::new()
              .name("context-control")
              .style(design_system::context_control())
              .child(
                battlement_reactant::host::Label::new(tx(
                  "CONTEXT  Nearest provider wins",
                  "User-facing product copy in the Reactant sample.",
                ))
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
            battlement_reactant::host::View::new()
              .name("context-cards")
              .style(design_system::context_row())
              .child(memo(ThemeCard::new().scope("OUTER")))
              .child(nested),
          ),
      )
      .child(
        battlement_reactant::host::View::new()
          .name("memo-experiment")
          .style(design_system::memo_experiment())
          .child(
            battlement_reactant::host::Label::new(tx(
              "MEMO  Unrelated value",
              "User-facing product copy in the Reactant sample.",
            ))
            .style(design_system::experiment_title()),
          )
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
            battlement_reactant::host::Label::new(ls(unrelated))
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
    battlement_reactant::host::View::new()
      .name(format!("context-{}", self.scope.to_ascii_lowercase()))
      .style(design_system::context_card(color))
      .child(
        battlement_reactant::host::Label::new(ls(self.scope)).style(design_system::context_scope()),
      )
      .child(
        battlement_reactant::host::Label::new(ls(name))
          .name("context-theme")
          .style(design_system::context_theme(color)),
      )
  }
}
