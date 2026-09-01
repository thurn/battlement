use battlement::{Color, FlexDirection, FlexWrap, ScrollViewMode, Style};
use battlement_reactant::prelude::*;

use crate::{Game, design_system};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutReorderState {
  pub(crate) expanded: bool,
  pub(crate) alternate: bool,
  pub(crate) reversed: bool,
  pub(crate) show_pop: bool,
}

impl Default for LayoutReorderState {
  fn default() -> Self {
    Self {
      expanded: false,
      alternate: false,
      reversed: false,
      show_pop: true,
    }
  }
}

pub(crate) struct LayoutReorder {
  pub(crate) state: LayoutReorderState,
  pub(crate) compact: bool,
}

impl Component for LayoutReorder {
  fn render(&self) -> impl Render {
    let mut order = vec!["ALPHA", "BRAVO", "CHARLIE"];
    if self.state.reversed {
      order.reverse();
    }
    ScrollView::new()
      .name("layout-reorder-canvas")
      .mode(ScrollViewMode::Vertical)
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .layout_scroll(true)
      .child(Label::new("ONE NATIVE LAYOUT PASS · INVERSE PROJECTION").style(eyebrow()))
      .child(
        Label::new("Layout & Reorder")
          .name("page-title")
          .style(title()),
      )
      .child(
        View::new()
          .style(toolbar())
          .child(Button::new("TOGGLE GEOMETRY").on_click(|game: &mut Game| {
            game.layout_reorder.expanded = !game.layout_reorder.expanded;
          }))
          .child(Button::new("SHARED HANDOFF").on_click(|game: &mut Game| {
            game.layout_reorder.alternate = !game.layout_reorder.alternate;
          }))
          .child(Button::new("REORDER").on_click(|game: &mut Game| {
            game.layout_reorder.reversed = !game.layout_reorder.reversed;
          }))
          .child(Button::new("POP ITEM").on_click(|game: &mut Game| {
            game.layout_reorder.show_pop = !game.layout_reorder.show_pop;
          })),
      )
      .child(
        View::new()
          .style(gallery())
          .child(expander(self.state.expanded))
          .child(grid(self.state.expanded))
          .child(shared_indicator(self.state.alternate))
          .child(shared_handoff(self.state.alternate))
          .child(scroll_root())
          .child(reorder_list(order))
          .child(pop_layout(self.state.show_pop)),
      )
  }
}

fn expander(expanded: bool) -> Node {
  Node::new(specimen(
    "POSITION + SIZE",
    View::new()
      .style(if expanded {
        expanded_box()
      } else {
        box_style()
      })
      .layout(Layout::Both)
      .child(Label::new(if expanded { "240 × 92" } else { "132 × 54" })),
  ))
}

fn grid(expanded: bool) -> Node {
  Node::new(specimen(
    "NESTED SCALE CORRECTION",
    View::new()
      .style(if expanded { wide_grid() } else { narrow_grid() })
      .layout(Layout::Both)
      .child(
        (0..4)
          .map(|index| {
            View::new()
              .key(index)
              .style(tile())
              .layout(Layout::Position)
              .child(Label::new(format!("0{}", index + 1)))
          })
          .collect::<Vec<_>>(),
      ),
  ))
}

fn shared_indicator(alternate: bool) -> Node {
  Node::new(specimen(
    "SHARED TAB INDICATOR",
    LayoutGroup::new("tabs").child(
      View::new().style(row()).child(
        (0..2)
          .map(|index| {
            View::new()
              .key(index)
              .style(tab())
              .child(Label::new(if index == 0 { "GENERAL" } else { "AUDIO" }))
              .child((index == usize::from(alternate)).then(|| {
                View::new()
                  .layout_id("active-tab")
                  .layout(Layout::Both)
                  .style(indicator())
              }))
          })
          .collect::<Vec<_>>(),
      ),
    ),
  ))
}

fn shared_handoff(alternate: bool) -> Node {
  Node::new(specimen(
    "SHARED ELEMENT HANDOFF",
    LayoutGroup::new("handoff").child(
      View::new().style(row()).child(
        (0..2)
          .map(|index| {
            View::new().key(index).style(handoff_slot()).child(
              (index == usize::from(alternate)).then(|| {
                View::new()
                  .layout_id(7_u32)
                  .layout(Layout::Both)
                  .style(shared_box())
              }),
            )
          })
          .collect::<Vec<_>>(),
      ),
    ),
  ))
}

fn scroll_root() -> Node {
  Node::new(specimen(
    "SCROLL + LAYOUT ROOT",
    ScrollView::new()
      .mode(ScrollViewMode::Horizontal)
      .style(scroll())
      .layout_scroll(true)
      .layout_root(true)
      .child(
        View::new()
          .style(scroll_content())
          .child(View::new().style(shared_box()).layout(Layout::Position)),
      ),
  ))
}

fn reorder_list(values: Vec<&'static str>) -> Node {
  Node::new(specimen(
    "DRAG REORDER",
    View::new().style(list()).child(
      values
        .into_iter()
        .map(|value| {
          View::new()
            .key(value)
            .name(format!("reorder-{value}").to_ascii_lowercase())
            .style(item())
            .reorder_item(ReorderAxis::Y)
            .on_drag_end(|game: &mut Game, event| {
              if event.offset.y.abs() > 18.0 {
                game.layout_reorder.reversed = !game.layout_reorder.reversed;
              }
            })
            .child(Label::new(value))
        })
        .collect::<Vec<_>>(),
    ),
  ))
}

fn pop_layout(show: bool) -> Node {
  Node::new(specimen(
    "POP LAYOUT REMOVAL",
    View::new().style(row()).child(
      AnimatePresence::new().mode(PresenceMode::PopLayout).child(
        (0..3)
          .filter(|index| show || *index != 1)
          .map(|index| {
            View::new()
              .key(index)
              .layout(Layout::Both)
              .style(pop_item())
              .exit(MotionStyle::new().opacity(0.0).scale(0.7))
              .child(Label::new(format!("P{}", index + 1)))
          })
          .collect::<Vec<_>>(),
      ),
    ),
  ))
}

fn specimen(label: &'static str, child: impl Render) -> View {
  View::new()
    .style(card())
    .child(Label::new(label).style(caption()))
    .child(child)
}

fn content() -> Style {
  Style::new().padding(30.0)
}
fn eyebrow() -> Style {
  Style::new()
    .font_size(11.0)
    .letter_spacing(1.6)
    .color(Color::rgba(0.45, 0.7, 1.0, 1.0))
}
fn title() -> Style {
  Style::new()
    .font_size(34.0)
    .color(Color::rgb(1.0, 1.0, 1.0))
}
fn toolbar() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}
fn gallery() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}
fn card() -> Style {
  Style::new()
    .width(310.0)
    .min_height(150.0)
    .padding(16.0)
    .margin(7.0)
    .background_color(Color::rgba(0.05, 0.08, 0.14, 1.0))
    .border_radius(10.0)
}
fn caption() -> Style {
  Style::new()
    .font_size(10.0)
    .letter_spacing(1.2)
    .color(Color::rgba(0.55, 0.65, 0.8, 1.0))
}
fn box_style() -> Style {
  Style::new()
    .width(132.0)
    .height(54.0)
    .background_color(Color::rgba(0.2, 0.55, 1.0, 1.0))
    .padding(12.0)
    .border_radius(8.0)
}
fn expanded_box() -> Style {
  box_style().width(240.0).height(92.0)
}
fn narrow_grid() -> Style {
  Style::new()
    .width(120.0)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
}
fn wide_grid() -> Style {
  narrow_grid().width(250.0)
}
fn tile() -> Style {
  Style::new()
    .width(54.0)
    .height(36.0)
    .background_color(Color::rgba(0.15, 0.25, 0.4, 1.0))
    .padding(8.0)
}
fn row() -> Style {
  Style::new().flex_direction(FlexDirection::Row)
}
fn tab() -> Style {
  Style::new().width(110.0).padding(10.0)
}
fn indicator() -> Style {
  Style::new()
    .height(3.0)
    .width(90.0)
    .background_color(Color::rgba(0.2, 0.7, 1.0, 1.0))
}
fn handoff_slot() -> Style {
  Style::new().width(130.0).height(60.0).padding(8.0)
}
fn shared_box() -> Style {
  Style::new()
    .width(44.0)
    .height(44.0)
    .background_color(Color::rgba(0.65, 0.25, 1.0, 1.0))
    .border_radius(8.0)
}
fn scroll() -> Style {
  Style::new().width(275.0).height(68.0)
}
fn scroll_content() -> Style {
  Style::new().width(520.0).height(52.0).padding(4.0)
}
fn list() -> Style {
  Style::new()
}
fn item() -> Style {
  Style::new()
    .height(34.0)
    .padding(8.0)
    .background_color(Color::rgba(0.12, 0.18, 0.28, 1.0))
    .border_radius(5.0)
}
fn pop_item() -> Style {
  Style::new()
    .width(66.0)
    .height(46.0)
    .padding(12.0)
    .background_color(Color::rgba(1.0, 0.35, 0.25, 1.0))
    .border_radius(6.0)
}
