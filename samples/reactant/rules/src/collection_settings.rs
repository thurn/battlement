use trox::{ls, tx};

use crate::{Game, layout_gallery_styles as styles};
use battlement::{Command, CurrentPage, FlexDirection};
use battlement_reactant::{accessibility_collections as collections, application, prelude::*};

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
    let link = collections::use_link(
      ButtonOptions::new()
        .name(tx(
          "Documentation link",
          "User-facing product copy in the Reactant sample.",
        ))
        .on_press(|game: &mut Game| game.layout_gallery.trace.push("LINK ACTIVATED")),
    );
    let app = use_app();
    let external_link = collections::use_link(
      ButtonOptions::new()
        .name(tx(
          "Open Unity documentation",
          "User-facing product copy in the Reactant sample.",
        ))
        .on_press(move || {
          app.send(Command::open_external_url(
          "https://docs.unity3d.com/6000.5/Documentation/ScriptReference/Application.OpenURL.html",
        ))
        }),
    );
    View::new().style(styles::section()).child((
      Label::new(tx(
        "COLLECTION SEMANTICS",
        "User-facing product copy in the Reactant sample.",
      ))
      .style(styles::section_heading()),
      Label::new(ls(status)).semantic(use_static_text(ls(status))),
      Flex::new()
        .direction(FlexDirection::Row)
        .gap(8.0)
        .semantic(collections::use_navigation(tx(
          "Settings pages",
          "User-facing product copy in the Reactant sample.",
        )))
        .child(
          ["Controls page", "Bindings page"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
              let mut page = use_button(ButtonOptions::new().name(ls(name)).on_press(
                move |game: &mut Game| {
                  game.layout_gallery.collection_page = index;
                },
              ));
              page.semantic.state.current = (self.page == index).then_some(CurrentPage::Page);
              Button::new(ls(name)).behavior(page)
            })
            .collect::<Vec<_>>(),
        ),
      View::new()
        .semantic(collections::use_region(tx(
          "Collection settings",
          "User-facing product copy in the Reactant sample.",
        )))
        .child((
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(8.0)
            .semantic(collections::use_listbox(tx(
              "Display quality",
              "User-facing product copy in the Reactant sample.",
            )))
            .child(
              ["Standard", "High", "Unavailable"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                  let option = collections::use_option(
                    ChoiceOptions::new()
                      .name(ls(name))
                      .selected(self.choice == index)
                      .is_disabled(index == 2)
                      .on_select(move |game: &mut Game| {
                        game.layout_gallery.collection_choice = index;
                      }),
                  );
                  Button::new(ls(name)).behavior(option)
                })
                .collect::<Vec<_>>(),
            ),
          View::new()
            .semantic(collections::use_table(tx(
              "Keyboard bindings",
              "User-facing product copy in the Reactant sample.",
            )))
            .child((
              Flex::new()
                .direction(FlexDirection::Row)
                .gap(24.0)
                .semantic(collections::use_row())
                .child((
                  Label::new(tx(
                    "ACTION",
                    "User-facing product copy in the Reactant sample.",
                  ))
                  .semantic(collections::use_column_header(tx(
                    "Action",
                    "User-facing product copy in the Reactant sample.",
                  ))),
                  Label::new(tx(
                    "KEYBOARD",
                    "User-facing product copy in the Reactant sample.",
                  ))
                  .semantic(collections::use_column_header(tx(
                    "Keyboard",
                    "User-facing product copy in the Reactant sample.",
                  ))),
                )),
              Flex::new()
                .direction(FlexDirection::Row)
                .gap(24.0)
                .semantic(collections::use_row())
                .child((
                  Label::new(tx(
                    "MOVE",
                    "User-facing product copy in the Reactant sample.",
                  ))
                  .semantic(collections::use_row_header(tx(
                    "Move",
                    "User-facing product copy in the Reactant sample.",
                  ))),
                  Label::new(tx("W", "User-facing product copy in the Reactant sample.")).semantic(
                    collections::use_cell(tx(
                      "W",
                      "User-facing product copy in the Reactant sample.",
                    )),
                  ),
                )),
            )),
          Button::new(tx(
            "OPEN UNITY DOCUMENTATION",
            "User-facing product copy in the Reactant sample.",
          ))
          .behavior(external_link),
          Button::new(tx(
            "DOCUMENTATION LINK",
            "User-facing product copy in the Reactant sample.",
          ))
          .behavior(link),
        )),
    ))
  }
}
