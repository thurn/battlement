//! Browse isolated examples of the chess design system.

use trox::{opaque, tx, tx_args, txa};

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
#[builder]
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
#[builder]
pub struct Demonstration;

impl Gallery {
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
      ReviewNavigation::new()
        .title(tx("CHESS UI", "Review page count label."))
        .caption(txa(
          "{page_count} review pages",
          tx_args![page_count => self.pages.len() as u32],
          "Review page count label.",
        ))
        .children(self.pages.iter().enumerate().map(|(index, page)| {
          ReviewButton::new()
            .label(txa(
              "{position}. {title}",
              tx_args![
                position => (index + 1) as u32,
                title => opaque(page.title_text()),
              ],
              "Numbered navigation label for a Chess UI review page.",
            ))
            .navigation(selection.index == index)
            .reveal_generation(selection.generation)
            .name(format!("review-page-{}", index + 1))
            .on_press(select.update_callback(move |old| Selection {
              index,
              generation: old.generation + 1,
            }))
            .key(index + 1)
        })),
      ReviewStage::new().child(self.pages.get(selection.index).map(|page| {
        page
          .clone()
          .eyebrow(txa(
            "REVIEW {review_index} / {page_count}",
            tx_args![
              review_index => format!("{:02}", selection.index + 1),
              page_count => self.pages.len() as u32,
            ],
            "Current review page indicator.",
          ))
          .key((selection.index, selection.generation))
      })),
    ))
  }
}

impl Component for Demonstration {
  fn render(&self) -> impl Render {
    let (count, set_count) = hooks::use_state(0_u32);
    ReviewPanel::new().children((
      ReviewText::new()
        .text(tx(
          "One page. A fresh start.",
          "Gallery introduction heading.",
        ))
        .kind(ReviewTextKind::Title),
      ReviewText::new().text(tx(
        "Select a page to explore it. Select it again to reset its demonstration.",
        "Gallery usage instructions.",
      )),
      ReviewText::new()
        .text(txa(
          "Changes: {count}",
          tx_args![count],
          "Demonstration change count.",
        ))
        .kind(ReviewTextKind::Title)
        .name("demonstration-count"),
      ReviewButton::new()
        .label(tx("Change demonstration", "Demonstration change button."))
        .on_press(set_count.update_callback(|value| value + 1)),
    ))
  }
}
