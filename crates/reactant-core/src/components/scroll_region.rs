//! Measured scrolling with coherent native and accessibility positions.

use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, LengthUnits, ScrollEvent,
  ScrollerVisibility, Style, Vector,
};
use trox::LocalizedString;

use crate::{
  component::Component,
  components::ScrollArea,
  element_ref,
  event::ReactantEvent,
  geometry, hooks,
  host::{ScrollView, View},
  render::{Children, Render},
};

/// A vertical scroll area whose range follows the measured content.
/// Native wheel, touch, and scrollbar movement update the semantic position.
/// Semantic actions advance by a viewport page, clamped to the current range.
pub struct ScrollRegion {
  name: LocalizedString,
  host: ScrollView,
  children: Children,
}

impl ScrollRegion {
  /// Creates an empty region with an accessible name.
  pub fn new(name: LocalizedString) -> Self {
    Self {
      name,
      host: ScrollView::new(),
      children: Children::new(()),
    }
  }

  /// Sets the native query identity.
  pub fn host_name(mut self, name: impl Into<String>) -> Self {
    self.host = self.host.name(name.into());
    self
  }

  /// Supplies the bounded viewport dimensions and appearance.
  pub fn style(mut self, style: Style) -> Self {
    self.host = self.host.style(style);
    self
  }

  /// Sets content whose measured height determines the scroll range.
  pub fn child(mut self, children: impl Render) -> Self {
    self.children = Children::new(children);
    self
  }
}

impl Component for ScrollRegion {
  fn render(&self) -> impl Render {
    let viewport = element_ref::use_element_ref();
    let content = element_ref::use_element_ref();
    let measured = geometry::use_geometry([viewport.clone(), content.clone()]);
    let height = measured.measurements[0]
      .latest
      .map_or(0.0, |value| value.layout.height as f32);
    let extent = measured.measurements[1]
      .latest
      .map_or(0.0, |value| value.layout.height as f32);
    let maximum = (extent - height).max(0.0);
    let (offset, set_offset) = hooks::use_state(0.0_f32);
    let position = offset.clamp(0.0, maximum);
    let normalize = set_offset.clone();
    hooks::use_effect(move || normalize.set(position), position);
    let observed = set_offset.clone();
    let host = self.host.clone();
    ScrollArea::new(
      Some(self.name.clone()),
      AccessibilityScrollAxis::Vertical,
      position + 0.5 < maximum,
      position > 0.5,
    )
    .on_scroll(move |direction| {
      let delta = height * 0.8;
      set_offset.update(move |value| {
        (value
          + if direction == AccessibilityScrollDirection::Forward {
            delta
          } else {
            -delta
          })
        .clamp(0.0, maximum)
      });
    })
    .configure_host(move |_| {
      host
        .element_ref(viewport)
        .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
        .vertical_scroller_visibility(ScrollerVisibility::Auto)
        .scroll_offset(Vector::new(0.0, position))
        .on_scroll_changed_event(move |event: ReactantEvent<ScrollEvent>| {
          observed.set(event.payload().offset.y)
        })
    })
    .child(
      View::new()
        .element_ref(content)
        .style(Style::new().width(100.pct()).flex_shrink(0))
        .child(self.children.render()),
    )
  }
}
