use trox::{LocalizedString, ls, tx};

use crate::controls;
use crate::{Control, Interaction, design_system};
use battlement_reactant::prelude::*;

#[builder]
pub(crate) struct Composition {
  pub(crate) reversed: bool,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

#[builder]
struct Badge {
  #[builder(required)]
  pub(crate) text: LocalizedString,
}

#[builder]
pub(crate) struct Specimen {
  /// Heading above the specimen's contents.
  #[builder(required)]
  heading: String,
  /// Owned content shown inside the specimen.
  #[builder(required)]
  child: Node,
}

impl Component for Composition {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .name("composition-canvas")
      .style(design_system::canvas(self.compact))
      .child(
        battlement_reactant::host::Label::new(tx(
          "COMPOSITION",
          "Component composition section heading.",
        ))
        .style(design_system::eyebrow()),
      )
      .child(
        battlement_reactant::host::Label::new(tx(
          "Build declaratively",
          "Component composition interface label.",
        ))
        .name("page-title")
        .style(design_system::title()),
      )
      .child(controls::interactive_button(
        if self.reversed { "RESTORE" } else { "REORDER" },
        "composition-action",
        design_system::primary_action(controls::control_state(
          self.interaction,
          Control::CompositionAction,
        )),
        Control::CompositionAction,
        |game| game.reversed = !game.reversed,
      ))
      .child(Fragment::new(
        Specimen::new()
          .child(composition_badges(self.reversed))
          .heading("Owned components".to_owned()),
      ))
  }
}

impl Component for Badge {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .style(design_system::badge())
      .child(
        battlement_reactant::host::Label::new(self.text.clone()).style(design_system::badge_text()),
      )
  }
}

impl Component for Specimen {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .name("composition-specimen")
      .style(design_system::specimen())
      .child(
        battlement_reactant::host::Label::new(ls(self.heading.clone()))
          .name("specimen-heading")
          .style(design_system::specimen_title()),
      )
      .child(self.child.clone())
  }
}

fn composition_badges(reversed: bool) -> Node {
  let mut badges = vec![
    Badge::new().text(tx(
      "01  Required props",
      "Component composition interface label.",
    )),
    Badge::new().text(tx(
      "02  Structural values",
      "Component composition interface label.",
    )),
    Badge::new().text(tx(
      "03  Primitive children",
      "Component composition interface label.",
    )),
  ];
  if reversed {
    badges.reverse();
  }
  Node::new(
    battlement_reactant::host::View::new()
      .name("composition-badges")
      .style(design_system::badge_row())
      .child(Fragment::new(badges)),
  )
}
