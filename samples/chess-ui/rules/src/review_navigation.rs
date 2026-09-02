use battlement::{Display, LengthUnits, ScrollViewMode, ScrollerVisibility, Style};
use battlement_reactant::{
  component::Component,
  host::{Label, ScrollView, View},
  render::{Node, Render},
};

use crate::{
  review_text::{ReviewText, ReviewTextKind},
  review_theme,
};

/// A titled navigation column with a themed vertical scroll area.
pub struct ReviewNavigation {
  pub title: String,
  pub caption: String,
  pub scroll: ScrollView,
  pub children: Node,
}

impl Component for ReviewNavigation {
  fn render(&self) -> impl Render {
    let thumb = Style::new()
      .background_color(review_theme::MUTED)
      .border_width(0)
      .border_radius(5);
    View::new()
      .style(
        Style::new()
          .width(320)
          .margin_right(24)
          .height(100.pct())
          .flex_shrink(0),
      )
      .child((
        ReviewText::new(Label::new(self.title.clone()), ReviewTextKind::Brand),
        ReviewText::new(Label::new(self.caption.clone()), ReviewTextKind::Caption),
        self
          .scroll
          .clone()
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
          .content_container_style(Style::new().padding_right(12))
          .child(self.children.clone()),
      ))
  }
}
