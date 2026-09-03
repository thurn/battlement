//! A themed navigation landmark whose entries share a scroll container.

use battlement::{Display, LengthUnits, ScrollViewMode, ScrollerVisibility, Style};
use battlement_reactant::{
  component::Component,
  host::{ScrollView, View},
  render::Render,
};

use battlement_reactant::{
  accessibility_collections,
  context::Context,
  element_ref::{self, ElementRef},
  semantics,
};

use crate::{
  review_text::{ReviewText, ReviewTextKind},
  review_theme,
};

/// Scroll context shared by navigation entries so callers need no host references.
pub static SCROLL: Context<Option<ElementRef>> = Context::new(|| None);

/// A titled navigation column with a themed vertical scroll area.
pub struct ReviewNavigation {
  title: String,
  caption: String,
  scroll: ScrollView,
}

impl ReviewNavigation {
  /// Creates a navigation column with an application title.
  pub fn new(title: impl Into<String>) -> Self {
    Self {
      title: title.into(),
      caption: String::new(),
      scroll: ScrollView::new().name("review-navigation"),
    }
  }

  /// Sets the supporting text below the navigation title.
  pub fn caption(mut self, caption: impl Into<String>) -> Self {
    self.caption = caption.into();
    self
  }

  /// Appends entries in navigation order; key entries by their stable identity.
  pub fn children<R: Render>(mut self, children: impl IntoIterator<Item = R>) -> Self {
    self.scroll = self.scroll.children(children);
    self
  }
}

impl Component for ReviewNavigation {
  fn render(&self) -> impl Render {
    let scroll = element_ref::use_element_ref();
    let thumb = Style::new()
      .background_color(review_theme::MUTED)
      .border_width(0)
      .border_radius(5);
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
          ReviewText::new(self.title.clone()).kind(ReviewTextKind::Brand),
          ReviewText::new(self.caption.clone()).kind(ReviewTextKind::Caption),
          self
            .scroll
            .clone()
            .element_ref(scroll)
            .semantic(accessibility_collections::use_navigation(semantics::text(
              "Chess UI review pages",
            )))
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
            .vertical_dragger_style(thumb.clone())
            .vertical_dragger_border_style(thumb)
            .style(Style::new().flex_grow(1).min_height(0))
            .content_container_style(Style::new().padding_right(12)),
        )),
    )
  }
}
