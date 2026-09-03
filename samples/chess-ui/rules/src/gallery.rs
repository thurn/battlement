//! Browse isolated examples of the chess design system.

use crate::{
  review_button::ReviewButton,
  review_navigation::ReviewNavigation,
  review_page::ReviewPage,
  review_panel::ReviewPanel,
  review_stage::ReviewStage,
  review_surface::ReviewSurface,
  review_text::{ReviewText, ReviewTextKind},
};
use battlement_reactant::{hooks, prelude::*};

/// The demo application's root component: navigation beside one live example.
///
/// Each selection mounts a fresh page, including selecting the current page
/// again. This makes experiments repeatable and restores focus to the heading.
/// Add pages with [`Self::page`], then mount the gallery with `App::ui`.
/// Page contents are ordinary component values, including configured builders.
#[derive(Default)]
pub struct Gallery {
  pages: Vec<ReviewPage>,
}

/// The chosen page and the identity of this visit.
///
/// `generation` changes even when the index stays the same, so Reactant keys
/// discard the previous demonstration's local state on every selection.
#[derive(Clone, Copy, Default, PartialEq)]
struct Selection {
  index: usize,
  generation: u64,
}

/// A small interactive example explaining how selection resets local state.
pub struct Demonstration;

impl Gallery {
  /// Creates an empty gallery ready to receive pages in navigation order.
  pub fn new() -> Self {
    Self::default()
  }

  /// Registers a page and its configured content without rendering either.
  /// Re-selecting a page remounts its content from these same immutable props.
  pub fn page(mut self, page: ReviewPage) -> Self {
    self.pages.push(page);
    self
  }
}

impl Component for Gallery {
  fn render(&self) -> impl Render {
    let (selection, select) = hooks::use_state(Selection::default());
    ReviewSurface::new().child((
      ReviewNavigation::new("CHESS UI")
        .caption(format!("{} review pages", self.pages.len()))
        .children(self.pages.iter().enumerate().map(|(index, page)| {
          let select = select.clone();
          ReviewButton::new(format!("{}. {}", index + 1, page.title()))
            .navigation(selection.index == index)
            .reveal_on(selection.generation)
            .name(format!("review-page-{}", index + 1))
            .on_press(move || {
              select.update(move |old| Selection {
                index,
                generation: old.generation + 1,
              })
            })
            .key(index + 1)
        })),
      ReviewStage::new().child(self.pages.get(selection.index).map(|page| {
        page
          .clone()
          .eyebrow(format!(
            "REVIEW {:02} / {}",
            selection.index + 1,
            self.pages.len()
          ))
          .key((selection.index, selection.generation))
      })),
    ))
  }
}

impl Component for Demonstration {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    ReviewPanel::new((
      ReviewText::new("One page. A fresh start.").kind(ReviewTextKind::Title),
      ReviewText::new("Select a page to explore it. Select it again to reset its demonstration."),
      ReviewText::new(format!("Changes: {count}"))
        .kind(ReviewTextKind::Title)
        .name("demonstration-count"),
      ReviewButton::new("Change demonstration")
        .on_press(move || set_count.update(|value| value + 1)),
    ))
  }
}
