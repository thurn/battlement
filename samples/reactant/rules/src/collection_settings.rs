use battlement::{CurrentPage, FlexDirection};
use battlement_reactant::{accessibility_collections as collections, prelude::*};

use crate::{Game, layout_gallery_styles as styles};

pub(crate) struct CollectionSettings {
  pub(crate) choice: usize,
  pub(crate) page: usize,
}

impl Component for CollectionSettings {
  fn render(&self) -> impl Render {
    let link = collections::use_link(ButtonOptions {
      name: text("Documentation link"),
      is_disabled: false,
      on_press: |game: &mut Game| game.layout_gallery.trace.push("LINK ACTIVATED"),
    });
    View::new().style(styles::section()).child((
      Label::new("COLLECTION SEMANTICS").style(styles::section_heading()),
      Flex::new()
        .direction(FlexDirection::Row)
        .gap(8.0)
        .semantic(collections::use_navigation(text("Settings pages")))
        .child(
          ["Controls page", "Bindings page"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
              let mut page = use_button(ButtonOptions {
                name: text(name),
                is_disabled: false,
                on_press: move |game: &mut Game| game.layout_gallery.collection_page = index,
              });
              page.semantic.state.current = (self.page == index).then_some(CurrentPage::Page);
              Button::new(name)
                .semantic(page.semantic)
                .focus_props(page.focus)
                .interaction_props(page.interaction)
            })
            .collect::<Vec<_>>(),
        ),
      View::new()
        .semantic(collections::use_region(text("Collection settings")))
        .child((
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(8.0)
            .semantic(collections::use_listbox(text("Display quality")))
            .child(
              ["Standard", "High", "Unavailable"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                  let option = collections::use_option(ChoiceOptions {
                    name: text(name),
                    selected: self.choice == index,
                    is_disabled: index == 2,
                    on_select: move |game: &mut Game| game.layout_gallery.collection_choice = index,
                  });
                  Button::new(name)
                    .semantic(option.semantic)
                    .focus_props(option.focus)
                    .interaction_props(option.interaction)
                })
                .collect::<Vec<_>>(),
            ),
          View::new()
            .semantic(collections::use_table(text("Keyboard bindings")))
            .child((
              Flex::new()
                .direction(FlexDirection::Row)
                .gap(24.0)
                .semantic(collections::use_row())
                .child((
                  Label::new("ACTION").semantic(collections::use_column_header(text("Action"))),
                  Label::new("KEYBOARD").semantic(collections::use_column_header(text("Keyboard"))),
                )),
              Flex::new()
                .direction(FlexDirection::Row)
                .gap(24.0)
                .semantic(collections::use_row())
                .child((
                  Label::new("MOVE").semantic(collections::use_row_header(text("Move"))),
                  Label::new("W").semantic(collections::use_cell(text("W"))),
                )),
            )),
          Button::new("DOCUMENTATION LINK")
            .semantic(link.semantic)
            .focus_props(link.focus)
            .interaction_props(link.interaction),
        )),
    ))
  }
}
