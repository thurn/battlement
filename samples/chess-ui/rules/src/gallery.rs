use battlement::CurrentPage;
use battlement_reactant::{accessibility_collections as collections, prelude::*};

use crate::{
  pages::{self, Page, Registration},
  review_button::{ReviewButton, ReviewButtonKind},
  review_navigation::ReviewNavigation,
  review_page::ReviewPage,
  review_panel::ReviewPanel,
  review_stage::ReviewStage,
  review_surface::ReviewSurface,
  review_text::{ReviewText, ReviewTextKind},
};

pub(crate) struct Gallery;

#[derive(Clone, Copy, PartialEq)]
struct Selection {
  index: usize,
  generation: u64,
}

struct NavigationItem {
  page: &'static Page,
  selection: Selection,
  select: StateSetter<Selection>,
  scroll: ElementRef,
}

struct PageHarness(Registration);

struct Demonstration;

impl Component for Gallery {
  fn render(&self) -> impl Render {
    let (selection, select) = use_state(Selection {
      index: 0,
      generation: 0,
    });
    let scroll = use_element_ref();
    let viewport = use_viewport_size();
    let reference = use_element_ref();
    let measured = use_geometry(reference.clone()).measurements.latest;
    let width = measured.map_or(viewport.width as f32, |value| value.layout.width as f32);
    let height = measured.map_or(viewport.height as f32, |value| value.layout.height as f32);
    let scale = ((width - 392.0) / 1024.0)
      .min((height - 48.0) / 1536.0)
      .clamp(0.0, 1.0);
    ReviewSurface {
      view: View::new().name("gallery").element_ref(reference).child((
        ReviewNavigation {
          title: "CHESS UI".to_owned(),
          caption: format!("{} review pages", pages::ALL.len()),
          scroll: ScrollView::new()
            .name("review-navigation")
            .element_ref(scroll.clone())
            .semantic(collections::use_navigation(text("Chess UI review pages"))),
          children: Node::new(
            pages::ALL
              .iter()
              .map(|page| {
                NavigationItem {
                  page,
                  selection,
                  select: select.clone(),
                  scroll: scroll.clone(),
                }
                .key(page.number)
              })
              .collect::<Vec<_>>(),
          ),
        },
        ReviewStage {
          scale,
          children: Node::new(
            PageHarness(Registration {
              page: &pages::ALL[selection.index],
              reset_generation: selection.generation,
            })
            .key((selection.index, selection.generation)),
          ),
        },
      )),
    }
  }
}

impl Component for NavigationItem {
  fn render(&self) -> impl Render {
    let reference = use_element_ref();
    let reveal = reference.clone();
    let scroll = self.scroll.clone();
    let selected = self.selection.index + 1 == self.page.number;
    use_effect(
      move || {
        if selected {
          scroll.scroll_to(&reveal);
        }
      },
      (selected, self.selection.generation),
    );
    let select = self.select.clone();
    let index = self.page.number - 1;
    let mut button = use_button(ButtonOptions {
      name: text(self.page.semantic_target),
      is_disabled: false,
      on_press: move || {
        select.update(move |old| Selection {
          index,
          generation: old.generation + 1,
        })
      },
    });
    button.semantic.state.current = selected.then_some(CurrentPage::Page);
    ReviewButton::new(
      Button::new(self.page.semantic_target)
        .element_ref(reference)
        .name(format!("review-page-{}", self.page.number))
        .semantic(button.semantic)
        .focus_props(button.focus)
        .interaction_props(button.interaction),
      ReviewButtonKind::Navigation { selected },
    )
  }
}

impl Component for PageHarness {
  fn render(&self) -> impl Render {
    let heading = use_element_ref();
    let focus = heading.clone();
    use_effect(move || focus.focus(), self.0.reset_generation);
    let page = self.0.page;
    ReviewPage {
      eyebrow: format!("REVIEW {:02} / {}", page.number, pages::ALL.len()),
      title: page.title.to_owned(),
      description: page.description.to_owned(),
      heading,
      children: (page.render_harness)(),
    }
  }
}

pub(crate) fn demonstration() -> Node {
  Node::new(Demonstration)
}

pub(crate) fn empty_harness() -> Node {
  Node::new(())
}

impl Component for Demonstration {
  fn render(&self) -> impl Render {
    let (count, set_count) = use_state(0_u32);
    let button = use_button(ButtonOptions {
      name: text("Change demonstration"),
      is_disabled: false,
      on_press: move || set_count.update(|value| value + 1),
    });
    ReviewPanel {
      children: Node::new((
        ReviewText::new(
          Label::new("One page. A fresh start."),
          ReviewTextKind::Title,
        ),
        ReviewText::new(
          Label::new("Select a page to explore it. Select it again to reset its demonstration."),
          ReviewTextKind::Description,
        ),
        ReviewText::new(
          Label::new(format!("Changes: {count}"))
            .name("demonstration-count")
            .semantic(use_static_text(text(format!("Changes: {count}")))),
          ReviewTextKind::Title,
        ),
        ReviewButton::new(
          Button::new("Change demonstration")
            .semantic(button.semantic)
            .focus_props(button.focus)
            .interaction_props(button.interaction),
          ReviewButtonKind::Action,
        ),
      )),
    }
  }
}
