use crate::controls;
use crate::{Control, Interaction, design_system};
use battlement_reactant::prelude::*;
pub(crate) struct Composition {
  pub(crate) reversed: bool,
  pub(crate) interaction: Interaction,
  pub(crate) compact: bool,
}

struct Badge {
  pub(crate) text: &'static str,
}

pub(crate) struct Specimen<Heading = Missing, Child = Missing> {
  required: (Heading, Child),
  optional: (),
}

required_props!(Specimen, heading: String, child: Node);

impl Component for Composition {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .name("composition-canvas")
      .style(design_system::canvas(self.compact))
      .child(battlement_reactant::host::Label::new("COMPOSITION").style(design_system::eyebrow()))
      .child(
        battlement_reactant::host::Label::new("Build declaratively")
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
      .child(battlement_reactant::host::Label::new(self.text).style(design_system::badge_text()))
  }
}

impl Specimen<Missing, Missing> {
  pub(crate) fn new() -> Self {
    Self {
      required: (Missing, Missing),
      optional: (),
    }
  }
}

impl Component for Specimen<String, Node> {
  fn render(&self) -> impl Render {
    battlement_reactant::host::View::new()
      .name("composition-specimen")
      .style(design_system::specimen())
      .child(
        battlement_reactant::host::Label::new(self.required.0.clone())
          .name("specimen-heading")
          .style(design_system::specimen_title()),
      )
      .child(self.required.1.clone())
  }
}

fn composition_badges(reversed: bool) -> Node {
  let mut badges = vec![
    Badge {
      text: "01  Required props",
    },
    Badge {
      text: "02  Structural values",
    },
    Badge {
      text: "03  Primitive children",
    },
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
