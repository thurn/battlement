use reactant::{hooks, overlay::Overlay, portal::PortalTarget, prelude::*};
use trox::ls;

use crate::{
  card_input::{self, CardInput},
  domain::Phase,
  match_ui,
  settings::Preferences,
};

pub(crate) struct Inspection(pub PortalTarget);

impl Component for Inspection {
  fn render(&self) -> impl Render {
    let input = hooks::use_required_context::<CardInput>();
    let card = input.inspection().expect("inspection is open");
    let font = hooks::use_context::<Preferences>().font();
    let close = input.clone();
    let dismiss = input.clone();
    Overlay::modal(self.0.clone(), ls("Card inspection"))
      .on_dismiss(move || dismiss.dismiss())
      .style(match_ui::backdrop())
      .child(
        match_ui::panel().child((
          Heading::new(
            ls(card_input::name(card.face.expect("owned inspection face"))),
            2,
          )
          .style(match_ui::text_style(font + 8.0)),
          Text::new(ls(match input.phase() {
            Phase::Passing => "Choose three cards to pass.".into(),
            _ => input
              .reason(card.token)
              .unwrap_or_else(|| "This card is available to play.".into()),
          }))
          .style(match_ui::text_style(font)),
          Button::new(ls("Close inspection"))
            .style(match_ui::button_style(font))
            .on_press(move || close.dismiss()),
        )),
      )
  }
}
