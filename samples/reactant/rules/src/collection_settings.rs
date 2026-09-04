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
        "Collection settings section heading.",
      ))
      .style(styles::section_heading()),
      Text::new(ls(status)),
      Navigation::new(tx(
          "Settings pages",
          "Collection settings interface label.",
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
          "Collection settings interface label.",
        ))
        .child((
          ListBox::new(tx(
              "Display quality",
              "Collection settings interface label.",
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
            "Collection settings interface label.",
          ))
          .child((
            TableRow::new().child(
              Flex::new().direction(FlexDirection::Row).gap(24.0).child((
                ColumnHeader::new(tx(
                  "ACTION",
                  "Collection settings section heading.",
                )),
                ColumnHeader::new(tx(
                  "KEYBOARD",
                  "Collection settings section heading.",
                )),
              )),
            ),
            TableRow::new().child(
              Flex::new().direction(FlexDirection::Row).gap(24.0).child((
                RowHeader::new(tx(
                  "MOVE",
                  "Collection settings section heading.",
                )),
                TableCell::new(tx(
                  "W",
                  "Collection settings section heading.",
                )),
              )),
            ),
          )),
          Link::new(tx(
            "Open Unity documentation",
            "Collection settings interface label.",
          ))
          .on_press(move || {
            app.send(Command::open_external_url(
              "https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Application.OpenURL.html",
            ))
          }),
          Link::new(tx(
            "Documentation link",
            "Collection settings interface label.",
          ))
          .on_press(|game: &mut Game| game.layout_gallery.trace.push("LINK ACTIVATED")),
        )),
    ))
  }
}
