use battlement::{CurrentPage, FlexDirection, ScrollViewMode, ScrollerVisibility};
use battlement_reactant::{accessibility_collections as collections, prelude::*};

use crate::{
  engine::Game,
  pages::{self, Page, Registration},
  styles,
};

pub(crate) struct Gallery {
  pub(crate) width: f32,
  pub(crate) height: f32,
}

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
    let scale = ((self.width - 392.0) / 1024.0)
      .min((self.height - 48.0) / 1536.0)
      .clamp(0.0, 1.0);
    View::new()
      .name("gallery")
      .style(styles::gallery())
      .on_geometry_changed_event(|game: &mut Game, event| {
        game.width = event.payload().current.width as f32;
        game.height = event.payload().current.height as f32;
      })
      .child((
        View::new().style(styles::navigation_column()).child((
          Label::new("CHESS UI").style(styles::brand()),
          Label::new("40 review pages").style(styles::navigation_caption()),
          ScrollView::new()
            .name("review-navigation")
            .element_ref(scroll.clone())
            .mode(ScrollViewMode::Vertical)
            .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
            .vertical_scroller_visibility(ScrollerVisibility::Auto)
            .vertical_scroller_style(styles::scrollbar())
            .vertical_low_button_style(styles::scroll_button())
            .vertical_high_button_style(styles::scroll_button())
            .vertical_track_style(styles::scroll_track())
            .vertical_dragger_style(styles::scroll_thumb())
            .vertical_dragger_border_style(styles::scroll_thumb())
            .semantic(collections::use_navigation(text("Chess UI review pages")))
            .style(styles::navigation_scroll())
            .content_container_style(styles::navigation_content())
            .child(
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
        )),
        Flex::new()
          .direction(FlexDirection::Column)
          .style(styles::stage_area())
          .child(
            View::new()
              .name("design-stage-bounds")
              .style(styles::stage_bounds(scale))
              .child(
                View::new()
                  .name("design-stage")
                  .style(styles::stage(scale))
                  .child(
                    PageHarness(Registration {
                      page: &pages::ALL[selection.index],
                      reset_generation: selection.generation,
                    })
                    .key((selection.index, selection.generation)),
                  ),
              ),
          ),
      ))
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
      on_press: move |_: &mut Game| {
        select.update(move |old| Selection {
          index,
          generation: old.generation + 1,
        })
      },
    });
    button.semantic.state.current = selected.then_some(CurrentPage::Page);
    Button::new(self.page.semantic_target)
      .element_ref(reference)
      .name(format!("review-page-{}", self.page.number))
      .semantic(button.semantic)
      .focus_props(button.focus)
      .interaction_props(button.interaction)
      .style(styles::navigation_item(selected))
      .while_focus_visible(styles::focus_visible())
  }
}

impl Component for PageHarness {
  fn render(&self) -> impl Render {
    let heading = use_element_ref();
    let focus = heading.clone();
    use_effect(move || focus.focus(), self.0.reset_generation);
    let page = self.0.page;
    let mut region = collections::use_region(text(page.title));
    region.name = Some(AccessibleName::LabelledBy(heading.clone()));
    View::new()
      .name("page-content")
      .style(styles::page())
      .semantic(region)
      .child((
        Label::new(format!("REVIEW {:02} / 40", page.number)).style(styles::eyebrow()),
        Label::new(page.title)
          .name("page-heading")
          .element_ref(heading)
          .semantic(use_heading(text(page.title), 1))
          .focus_props(FocusProps::new().focusable(true).tab_index(-1))
          .style(styles::heading())
          .while_focus_visible(
            MotionStyle::new()
              .background_color(battlement::MotionColor::new(0.12, 0.23, 0.28, 1.0)),
          ),
        Label::new(page.description)
          .name("page-description")
          .semantic(use_static_text(text(page.description)))
          .style(styles::description()),
        (page.render_harness)(),
      ))
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
      on_press: move |_: &mut Game| set_count.update(|value| value + 1),
    });
    View::new().style(styles::demonstration()).child((
      Label::new("One page. A fresh start.").style(styles::demonstration_title()),
      Label::new("Select a page to explore it. Select it again to reset its demonstration.")
        .style(styles::description()),
      Label::new(format!("Changes: {count}"))
        .name("demonstration-count")
        .semantic(use_static_text(text(format!("Changes: {count}"))))
        .style(styles::demonstration_title()),
      Button::new("Change demonstration")
        .semantic(button.semantic)
        .focus_props(button.focus)
        .interaction_props(button.interaction)
        .style(styles::action())
        .while_focus_visible(styles::focus_visible()),
    ))
  }
}
