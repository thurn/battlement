use trox::{ls, tx};

use crate::{Game, layout_gallery_styles as styles};
use battlement::{Command, FlexDirection};
use battlement_reactant::{application, prelude::*};

#[builder]
pub(crate) struct CollectionSettings {
  pub(crate) choice: usize,
  pub(crate) page: usize,
}

impl Component for CollectionSettings {
  fn render(&self) -> impl Render {
    let application_state = application::use_application_state();
    let status = if application_state.is_active() {
      "APPLICATION ACTIVE"
    } else {
      "APPLICATION INACTIVE"
    };
    let app = use_app();
    View::new().style(styles::section()).child((
      Label::new(tx(
        "COLLECTION SEMANTICS",
        "User-facing product copy in the Reactant sample.",
      ))
      .style(styles::section_heading()),
      Text::new(ls(status)),
      Navigation::new(tx(
          "Settings pages",
          "User-facing product copy in the Reactant sample.",
        ))
        .child(Flex::new().direction(FlexDirection::Row).gap(8.0).child(
          ["Controls page", "Bindings page"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
              Button::new(ls(name))
                .current_page(self.page == index)
                .on_press(move |game: &mut Game| {
                  game.layout_gallery.collection_page = index;
                })
            })
            .collect::<Vec<_>>(),
        )),
      Region::new(tx(
          "Collection settings",
          "User-facing product copy in the Reactant sample.",
        ))
        .child((
          ListBox::new(tx(
              "Display quality",
              "User-facing product copy in the Reactant sample.",
            ))
            .child(Flex::new().direction(FlexDirection::Row).gap(8.0).child(
              ["Standard", "High", "Unavailable"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                  ListBoxOption::new(ls(name), self.choice == index)
                    .disabled(index == 2)
                    .on_press(move |game: &mut Game| {
                        game.layout_gallery.collection_choice = index;
                      })
                })
                .collect::<Vec<_>>(),
            )),
          Table::new(tx(
            "Keyboard bindings",
            "User-facing product copy in the Reactant sample.",
          ))
          .child((
            TableRow::new().child(
              Flex::new().direction(FlexDirection::Row).gap(24.0).child((
                ColumnHeader::new(tx(
                  "ACTION",
                  "User-facing product copy in the Reactant sample.",
                )),
                ColumnHeader::new(tx(
                  "KEYBOARD",
                  "User-facing product copy in the Reactant sample.",
                )),
              )),
            ),
            TableRow::new().child(
              Flex::new().direction(FlexDirection::Row).gap(24.0).child((
                RowHeader::new(tx(
                  "MOVE",
                  "User-facing product copy in the Reactant sample.",
                )),
                TableCell::new(tx(
                  "W",
                  "User-facing product copy in the Reactant sample.",
                )),
              )),
            ),
          )),
          Link::new(tx(
            "Open Unity documentation",
            "User-facing product copy in the Reactant sample.",
          ))
          .on_press(move || {
            app.send(Command::open_external_url(
              "https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Application.OpenURL.html",
            ))
          }),
          Link::new(tx(
            "Documentation link",
            "User-facing product copy in the Reactant sample.",
          ))
          .on_press(|game: &mut Game| game.layout_gallery.trace.push("LINK ACTIVATED")),
        )),
    ))
  }
}
