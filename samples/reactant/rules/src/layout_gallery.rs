use battlement::{
  Align, Color, GridAutoFlow, GridItem, GridTrack, PickingMode, ScrollViewMode, StackItem, Sticky,
};
use battlement_reactant::{hooks, prelude::*};

use crate::{Game, design_system, layout_gallery_styles as styles};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayoutGalleryState {
  pub(crate) active_tab: usize,
  pub(crate) alternate_tracks: bool,
  pub(crate) large_text: bool,
  pub(crate) menu_open: bool,
  pub(crate) modal_open: bool,
  pub(crate) layers_reversed: bool,
  pub(crate) reconnects: u32,
  pub(crate) trace: Vec<&'static str>,
  reconnect_requested: bool,
}

impl LayoutGalleryState {
  pub(crate) fn take_reconnect_request(&mut self) -> bool {
    std::mem::take(&mut self.reconnect_requested)
  }
}

pub(crate) struct LayoutGallery {
  pub(crate) state: LayoutGalleryState,
  pub(crate) compact: bool,
  pub(crate) overlay: PortalTarget,
}

struct StatefulSetting {
  name: &'static str,
  row: u32,
  large_text: bool,
}

impl Component for LayoutGallery {
  fn render(&self) -> impl Render {
    let menu_anchor = use_element_ref();
    let modal_trigger = use_element_ref();
    let modal_initial = use_element_ref();
    ScrollView::new()
      .name("layout-gallery-canvas")
      .mode(ScrollViewMode::Vertical)
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(styles::content())
      .layout_scroll(true)
      .child(Label::new("PUBLIC LAYOUT · ONE COHERENT FLOW").style(styles::eyebrow()))
      .child(
        Label::new("Layout Gallery")
          .name("page-title")
          .style(styles::title(self.state.large_text)),
      )
      .child(self.controls(modal_trigger.clone()))
      .child(self.tabs())
      .child(self.settings())
      .child(self.table())
      .child(self.dropdown(menu_anchor))
      .child(self.layers())
      .child(self.modal(modal_trigger, modal_initial))
      .child(
        Label::new(format!(
          "TRACE {} · RECONNECTS {}",
          if self.state.trace.is_empty() {
            "READY".to_owned()
          } else {
            self.state.trace.join(" > ")
          },
          self.state.reconnects,
        ))
        .name("layout-gallery-status")
        .style(styles::status()),
      )
  }
}

impl LayoutGallery {
  fn controls(&self, modal_trigger: ElementRef) -> Flex {
    Flex::new()
      .name("layout-gallery-controls")
      .direction(battlement::FlexDirection::Row)
      .wrap(battlement::FlexWrap::Wrap)
      .gap(8.0)
      .style(styles::toolbar())
      .child(
        Button::new("RESPONSIVE TRACKS")
          .name("layout-gallery-tracks")
          .on_click(|game: &mut Game| {
            game.layout_gallery.alternate_tracks = !game.layout_gallery.alternate_tracks;
          }),
      )
      .child(
        Button::new("LARGE TEXT")
          .name("layout-gallery-text")
          .on_click(|game: &mut Game| {
            game.layout_gallery.large_text = !game.layout_gallery.large_text;
          }),
      )
      .child(
        Button::new("OPEN MODAL")
          .name("layout-gallery-modal")
          .element_ref(modal_trigger)
          .on_click(|game: &mut Game| game.layout_gallery.modal_open = true),
      )
      .child(
        Button::new("RECONNECT")
          .name("layout-gallery-reconnect")
          .on_click(|game: &mut Game| {
            game.layout_gallery.reconnects += 1;
            game.layout_gallery.reconnect_requested = true;
          }),
      )
      .child(
        Button::new("RESET")
          .name("layout-gallery-reset")
          .on_click(|game: &mut Game| game.layout_gallery = LayoutGalleryState::default()),
      )
  }

  fn tabs(&self) -> View {
    let tabs = ["GENERAL", "AUDIO", "ACCESS"];
    View::new()
      .style(styles::section())
      .child(Label::new("FIXED TAB GRID").style(styles::section_heading()))
      .child(
        Grid::new()
          .name("layout-gallery-tabs")
          .columns([
            GridTrack::px(132.0),
            GridTrack::px(132.0),
            GridTrack::px(132.0),
          ])
          .rows([GridTrack::auto()])
          .column_gap(8.0)
          .child(
            tabs
              .into_iter()
              .enumerate()
              .map(|(index, label)| {
                Button::new(label)
                  .key(label)
                  .name(format!("layout-tab-{index}"))
                  .grid_item(GridItem::new().row(1).column(index as u32 + 1))
                  .style(styles::tab(index == self.state.active_tab))
                  .on_click(move |game: &mut Game| game.layout_gallery.active_tab = index)
              })
              .collect::<Vec<_>>(),
          ),
      )
  }

  fn settings(&self) -> View {
    let compact_tracks = self.state.alternate_tracks || self.compact;
    let columns = if compact_tracks {
      vec![GridTrack::fr(1.0), GridTrack::px(150.0)]
    } else {
      vec![
        GridTrack::px(180.0),
        GridTrack::fr(1.0),
        GridTrack::px(96.0),
      ]
    };
    View::new()
      .style(styles::section())
      .child(Label::new("RESPONSIVE SETTINGS").style(styles::section_heading()))
      .child(
        Grid::new()
          .name("layout-gallery-settings")
          .columns(columns)
          .rows([
            GridTrack::auto(),
            GridTrack::auto(),
            GridTrack::auto(),
            GridTrack::auto(),
          ])
          .auto_flow(GridAutoFlow::Row)
          .row_gap(8.0)
          .column_gap(12.0)
          .align_items(Align::Center)
          .layout(Layout::Both)
          .child(
            ["MUSIC", "EFFECTS", "VOICE"]
              .into_iter()
              .enumerate()
              .map(|(index, name)| {
                StatefulSetting {
                  name,
                  row: index as u32 + 1,
                  large_text: self.state.large_text,
                }
                .key(name)
              })
              .collect::<Vec<_>>(),
          )
          .child(
            Label::new(if self.state.large_text {
              "LARGE"
            } else {
              "STANDARD"
            })
            .name("layout-gallery-text-mode")
            .grid_item(
              GridItem::new()
                .row(if compact_tracks { 4 } else { 1 })
                .column(if compact_tracks { 2 } else { 3 }),
            )
            .style(styles::setting_label(self.state.large_text)),
          ),
      )
  }

  fn table(&self) -> View {
    View::new()
      .style(styles::section())
      .child(Label::new("STICKY INPUT TABLE").style(styles::section_heading()))
      .child(
        ScrollView::new()
          .name("layout-gallery-table")
          .mode(ScrollViewMode::Vertical)
          .style(styles::table())
          .content_container_style(styles::table_content())
          .child(
            Label::new("SETTING")
              .name("layout-gallery-table-header")
              .sticky(Sticky::top(0.0).order(4))
              .style(styles::table_header()),
          )
          .child(
            Grid::new()
              .name("layout-gallery-table-grid")
              .columns([GridTrack::fr(1.0), GridTrack::px(120.0)])
              .auto_rows(GridTrack::auto())
              .child(
                (0..12)
                  .flat_map(|index| {
                    [
                      Label::new(format!("INPUT {:02}", index + 1))
                        .key((index, 0_u8))
                        .style(styles::table_cell(index % 2 == 0)),
                      Label::new(if index % 2 == 0 { "ENABLED" } else { "AUTO" })
                        .key((index, 1_u8))
                        .style(styles::table_cell(index % 2 == 0)),
                    ]
                  })
                  .collect::<Vec<_>>(),
              ),
          ),
      )
  }

  fn dropdown(&self, anchor: ElementRef) -> View {
    let popover = self.state.menu_open.then(|| {
      Overlay::popover(self.overlay.clone(), anchor.clone())
        .name("layout-gallery-menu")
        .placement(PopoverPlacement::bottom_start().offset(6.0))
        .style(styles::popover())
        .child(Label::new("PORTALED MENU"))
        .child(
          Button::new("APPLY AND CLOSE")
            .name("layout-gallery-menu-action")
            .style(styles::popover_action())
            .on_click(|game: &mut Game| {
              game.layout_gallery.trace.push("TARGET");
              game.layout_gallery.menu_open = false;
            }),
        )
    });
    View::new()
      .style(styles::section())
      .child(Label::new("CLIPPED DROPDOWN").style(styles::section_heading()))
      .child(
        View::new()
          .name("layout-gallery-clip")
          .style(styles::clipped_control())
          .child(
            Button::new(if self.state.menu_open {
              "CLOSE MENU"
            } else {
              "OPEN MENU"
            })
            .name("layout-gallery-menu-trigger")
            .element_ref(anchor)
            .on_click(|game: &mut Game| {
              game.layout_gallery.trace.push("ANCHOR");
              game.layout_gallery.menu_open = !game.layout_gallery.menu_open;
            }),
          )
          .child(popover)
          .on_click_capture(|game: &mut Game| {
            game.layout_gallery.trace.clear();
            game.layout_gallery.trace.push("CAPTURE");
          })
          .on_click(|game: &mut Game| game.layout_gallery.trace.push("BUBBLE")),
      )
  }

  fn layers(&self) -> View {
    let foreground_order = if self.state.layers_reversed { -1 } else { 2 };
    View::new()
      .style(styles::section())
      .child(Label::new("ISOLATED STACK LAYERS").style(styles::section_heading()))
      .child(
        Stack::new()
          .name("layout-gallery-layers")
          .style(styles::layer_stage())
          .child(
            View::new()
              .picking_mode(PickingMode::Ignore)
              .style(styles::layer(Color::rgba(0.09, 0.2, 0.34, 1.0)))
              .stack_item(StackItem::new().order(-2).contributes_to_size(false)),
          )
          .child(
            Label::new("EQUAL ORDER · SOURCE FIRST")
              .style(styles::layer(Color::rgba(0.1, 0.35, 0.4, 0.85)))
              .stack_item(
                StackItem::new()
                  .order(0)
                  .top(24.0)
                  .left(24.0)
                  .contributes_to_size(false),
              ),
          )
          .child(
            Button::new("CHANGE LAYER ORDER")
              .name("layout-gallery-layer-action")
              .style(styles::layer(Color::rgba(0.65, 0.24, 0.3, 0.95)))
              .stack_item(
                StackItem::new()
                  .order(foreground_order)
                  .right(18.0)
                  .bottom(18.0)
                  .align_self(Align::FlexEnd)
                  .justify_self(Align::FlexEnd)
                  .contributes_to_size(false),
              )
              .layout(Layout::Position)
              .on_click(|game: &mut Game| {
                game.layout_gallery.layers_reversed = !game.layout_gallery.layers_reversed;
              }),
          ),
      )
      .child(
        AnimatePresence::new().child(self.state.layers_reversed.then(|| {
          Node::new(
            Label::new("LAYER ORDER CHANGED")
              .key("layout-gallery-layer-presence")
              .initial(MotionStyle::new().opacity(0.0).y(-8.0))
              .animate(MotionStyle::new().opacity(1.0).y(0.0))
              .exit(MotionStyle::new().opacity(0.0).y(8.0)),
          )
        })),
      )
  }

  fn modal(&self, trigger: ElementRef, initial: ElementRef) -> impl Render {
    self.state.modal_open.then(|| {
      Overlay::modal(self.overlay.clone())
        .name("layout-gallery-modal-scope")
        .initial_focus(initial.clone())
        .restore_focus(trigger)
        .style(styles::modal_overlay())
        .child(
          Stack::new().style(styles::modal()).child(
            View::new()
              .style(styles::modal_card())
              .stack_item(
                StackItem::new()
                  .align_self(Align::Center)
                  .justify_self(Align::Center)
                  .contributes_to_size(false),
              )
              .child(Label::new("Viewport modal").style(styles::modal_title()))
              .child(
                Button::new("CLOSE MODAL")
                  .name("layout-gallery-modal-close")
                  .element_ref(initial)
                  .on_click(|game: &mut Game| game.layout_gallery.modal_open = false),
              ),
          ),
        )
    })
  }
}

impl Component for StatefulSetting {
  fn render(&self) -> impl Render {
    let (revision, set_revision) = hooks::use_state(0_u32);
    Fragment::new((
      Label::new(self.name)
        .name(format!("layout-setting-{}", self.name.to_ascii_lowercase()))
        .grid_item(GridItem::new().row(self.row).column(1))
        .style(styles::setting_label(self.large_text)),
      Button::new(format!("VALUE {revision}"))
        .name(format!(
          "layout-setting-value-{}",
          self.name.to_ascii_lowercase()
        ))
        .grid_item(GridItem::new().row(self.row).column(2))
        .style(styles::setting_value())
        .on_click(move |_game: &mut Game| set_revision.update(|value| value + 1)),
    ))
  }
}
