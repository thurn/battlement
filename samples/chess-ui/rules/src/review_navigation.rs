//! A themed navigation landmark whose entries share a scroll container.

use trox::{LocalizedString, tx};

use crate::{
  review_text::{ReviewText, ReviewTextKind},
  review_theme,
};
use battlement::{Display, LengthUnits, ScrollViewMode, ScrollerVisibility, Style};
use battlement_reactant::prelude::builder;
use battlement_reactant::{
  component::Component,
  host::{ScrollView, View},
  render::Render,
};
use battlement_reactant::{
  components::Navigation,
  context::Context,
  element_ref::{self, ElementRef},
};

/// Scroll context shared by navigation entries so callers need no host references.
pub static SCROLL: Context<Option<ElementRef>> = Context::new(|| None);

/// A titled navigation column with a themed vertical scroll area.
#[builder]
pub struct ReviewNavigation {
  #[builder(required)]
  title: LocalizedString,
  /// Sets the supporting text below the navigation title.
  #[builder(required)]
  caption: LocalizedString,
  #[builder(default = ScrollView::new().name("review-navigation"))]
  scroll: ScrollView,
}

fn thumb_style() -> Style {
  Style::new()
    .background_color(review_theme::MUTED)
    .border_width(0)
    .border_radius(5)
}

impl ReviewNavigation {
  /// Appends entries in navigation order; key entries by their stable identity.
  pub fn children<R: Render>(mut self, children: impl IntoIterator<Item = R>) -> Self {
    self.scroll = self.scroll.children(children);
    self
  }
}

impl Component for ReviewNavigation {
  fn render(&self) -> impl Render {
    let scroll = element_ref::use_element_ref();
    SCROLL.provider(Some(scroll.clone())).child(
      View::new()
        .style(
          Style::new()
            .width(320)
            .margin_right(24)
            .height(100.pct())
            .flex_shrink(0),
        )
        .child((
          ReviewText::new()
            .text(self.title.clone())
            .kind(ReviewTextKind::Brand),
          ReviewText::new()
            .text(self.caption.clone())
            .kind(ReviewTextKind::Caption),
          Navigation::new(tx(
            "Chess UI review pages",
            "User-facing product copy in the Chess UI sample.",
          ))
          .child(
            self
              .scroll
              .clone()
              .element_ref(scroll)
              .mode(ScrollViewMode::Vertical)
              .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
              .vertical_scroller_visibility(ScrollerVisibility::Auto)
              .vertical_scroller_style(
                Style::new()
                  .width(10)
                  .background_color(review_theme::BACKGROUND),
              )
              .vertical_low_button_style(Style::new().display(Display::None))
              .vertical_high_button_style(Style::new().display(Display::None))
              .vertical_track_style(Style::new().background_color(review_theme::SURFACE))
              .vertical_dragger_style(self::thumb_style())
              .vertical_dragger_border_style(self::thumb_style())
              .style(Style::new().flex_grow(1).min_height(0))
              .content_container_style(Style::new().padding_right(12)),
          ),
        )),
    )
  }
}
