use trox::{ls, tx};

use crate::Game;
use battlement::{
  Color, FlexDirection, GridTrack, LengthUnits, ScrollViewMode, StackItem, Sticky, Style,
};
use battlement_reactant::prelude::*;

const GRID_CHILDREN: usize = 1_000;

const STICKY_ROWS: usize = 100;

const NESTED_STACKS: usize = 12;

const ANCHORED_OVERLAYS: usize = 10;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LayoutPerformanceState {
  phase: u32,
}

#[builder]
pub(crate) struct LayoutPerformance {
  pub(crate) state: LayoutPerformanceState,
  #[builder(required)]
  pub(crate) overlay: PortalTarget,
}

impl Component for LayoutPerformance {
  fn render(&self) -> impl Render {
    let anchors = (0..ANCHORED_OVERLAYS)
      .map(|_| use_element_ref())
      .collect::<Vec<_>>();
    let overlays = anchors
      .iter()
      .enumerate()
      .map(|(index, anchor)| {
        Overlay::popover(self.overlay.clone(), anchor.clone())
          .host_name(format!("layout-performance-overlay-{index}"))
          .style(popover())
          .child(Label::new(ls(format!("OVERLAY {index:02}"))))
      })
      .collect::<Vec<_>>();
    Fragment::new((
      ScrollView::new()
        .name("layout-performance-canvas")
        .mode(ScrollViewMode::Vertical)
        .style(canvas())
        .content_container_style(content())
        .child(
          Label::new(tx(
            "LAYOUT PERFORMANCE · 1,000 MIXED CHILDREN",
            "Layout performance explanatory message.",
          ))
          .style(title()),
        )
        .child(
          Button::new(ls(format!("DIRTY PHASE {}", self.state.phase)))
            .host_name("layout-performance-dirty")
            .on_press(|game: &mut Game| {
              game.layout_performance.phase = game.layout_performance.phase.wrapping_add(1);
            }),
        )
        .child(
          Grid::new()
            .name("layout-performance-grid")
            .columns((0..25).map(|_| GridTrack::fr(1.0)))
            .auto_rows(GridTrack::px(14.0))
            .gap(1.0)
            .style(grid())
            .child(
              (0..GRID_CHILDREN)
                .map(|index| View::new().key(index).style(cell(index, self.state.phase)))
                .collect::<Vec<_>>(),
            ),
        )
        .child(
          Flex::new()
            .name("layout-performance-stacks")
            .direction(FlexDirection::Row)
            .gap(3.0)
            .child(
              (0..NESTED_STACKS)
                .map(|index| {
                  Stack::new()
                    .key(index)
                    .style(stack())
                    .child(
                      View::new()
                        .style(stack_layer(index))
                        .stack_item(StackItem::new().order(-1)),
                    )
                    .child(
                      View::new().style(stack_layer(index + 1)).stack_item(
                        StackItem::new()
                          .order(1)
                          .top(2.0)
                          .left(2.0)
                          .contributes_to_size(false),
                      ),
                    )
                })
                .collect::<Vec<_>>(),
            ),
        )
        .child(
          Flex::new()
            .name("layout-performance-anchors")
            .direction(FlexDirection::Row)
            .gap(4.0)
            .child(
              anchors
                .into_iter()
                .enumerate()
                .map(|(index, anchor)| {
                  View::new()
                    .key(index)
                    .element_ref(anchor)
                    .style(anchor_style())
                })
                .collect::<Vec<_>>(),
            ),
        )
        .child(
          ScrollView::new()
            .name("layout-performance-sticky-scroll")
            .mode(ScrollViewMode::Vertical)
            .style(sticky_scroll())
            .child(
              Flex::new().direction(FlexDirection::Column).child(
                (0..STICKY_ROWS)
                  .map(|index| {
                    Label::new(ls(format!("STICKY {index:03}")))
                      .key(index)
                      .sticky(Sticky::top((index % 4) as f32).order(index as i32))
                      .style(sticky_row(index))
                  })
                  .collect::<Vec<_>>(),
              ),
            ),
        ),
      Fragment::new(overlays),
    ))
  }
}

fn canvas() -> Style {
  Style::new().width(100.0_f32.pct()).height(100.0_f32.pct())
}

fn content() -> Style {
  Style::new().padding(18.0)
}

fn title() -> Style {
  Style::new()
    .font_size(24.0)
    .color(Color::rgb(1.0, 1.0, 1.0))
}

fn grid() -> Style {
  Style::new().width(1_000.0).margin_top(8.0)
}

fn cell(index: usize, phase: u32) -> Style {
  let active = (index as u32 + phase).is_multiple_of(3);
  Style::new().height(12.0).background_color(if active {
    Color::rgba(0.08, 0.5, 0.72, 1.0)
  } else {
    Color::rgba(0.08, 0.13, 0.2, 1.0)
  })
}

fn stack() -> Style {
  Style::new().width(34.0).height(34.0)
}

fn stack_layer(index: usize) -> Style {
  Style::new()
    .width(30.0)
    .height(30.0)
    .background_color(if index.is_multiple_of(2) {
      Color::rgba(0.45, 0.2, 0.65, 1.0)
    } else {
      Color::rgba(0.1, 0.55, 0.55, 1.0)
    })
}

fn anchor_style() -> Style {
  Style::new()
    .width(28.0)
    .height(20.0)
    .background_color(Color::rgba(0.75, 0.3, 0.2, 1.0))
}

fn popover() -> Style {
  Style::new()
    .width(90.0)
    .height(28.0)
    .background_color(Color::rgba(0.04, 0.08, 0.13, 1.0))
}

fn sticky_scroll() -> Style {
  Style::new().height(240.0).margin_top(8.0)
}

fn sticky_row(index: usize) -> Style {
  Style::new()
    .height(22.0)
    .background_color(if index.is_multiple_of(2) {
      Color::rgba(0.07, 0.12, 0.18, 1.0)
    } else {
      Color::rgba(0.05, 0.09, 0.14, 1.0)
    })
}
