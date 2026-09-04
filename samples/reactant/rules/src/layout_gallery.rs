use trox::{ls, tx};

use crate::{Game, design_system, layout_gallery_styles as styles};
use battlement::{
  Align, Color, GridAutoFlow, GridItem, GridTrack, PickingMode, ScrollViewMode, StackItem, Sticky,
};
use battlement_reactant::{control_behavior, hooks, prelude::*};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayoutGalleryState {
  pub(crate) active_tab: usize,
  pub(crate) collection_choice: usize,
  pub(crate) collection_page: usize,
  pub(crate) alternate_tracks: bool,
  pub(crate) captions_enabled: bool,
  pub(crate) disclosure_open: bool,
  pub(crate) inert_content: bool,
  pub(crate) large_text: bool,
  pub(crate) menu_open: bool,
  pub(crate) modal_open: bool,
  pub(crate) layers_reversed: bool,
  pub(crate) reconnects: u32,
  pub(crate) radio_selection: usize,
  pub(crate) spatial_audio: bool,
  pub(crate) trace: Vec<&'static str>,
  pub(crate) volume: u32,
}

#[builder]
pub(crate) struct LayoutGallery {
  pub(crate) state: LayoutGalleryState,
  pub(crate) compact: bool,
  #[builder(required)]
  pub(crate) overlay: PortalTarget,
}

#[builder]
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
    ScrollArea::new(
      Some(tx(
        "Layout Gallery",
        "User-facing product copy in the Reactant sample.",
      )),
      AccessibilityScrollAxis::Vertical,
      true,
      true,
    )
    .on_scroll(|game: &mut Game, direction| match direction {
      AccessibilityScrollDirection::Forward => game.layout_gallery.trace.push("SCROLL FORWARD"),
      AccessibilityScrollDirection::Backward => game.layout_gallery.trace.push("SCROLL BACKWARD"),
    })
    .host_name("layout-gallery-canvas")
    .configure_host(|host| {
      host
        .mode(ScrollViewMode::Vertical)
        .content_container_style(styles::content())
        .layout_scroll(true)
    })
    .style(design_system::canvas(self.compact).padding(0.0))
    .child(
      Label::new(tx(
        "PUBLIC LAYOUT · ONE COHERENT FLOW",
        "User-facing product copy in the Reactant sample.",
      ))
      .style(styles::eyebrow()),
    )
    .child(
      Label::new(tx(
        "Layout Gallery",
        "User-facing product copy in the Reactant sample.",
      ))
      .name("page-title")
      .semantic(control_behavior::heading(
        tx(
          "Layout Gallery",
          "User-facing product copy in the Reactant sample.",
        ),
        1,
      ))
      .style(styles::title(self.state.large_text)),
    )
    .child(self.controls(modal_trigger.clone()))
    .child(self.tabs())
    .child(
      View::new()
        .name("layout-gallery-inert-region")
        .inert(self.state.inert_content)
        .child(
          Button::new(tx(
            "INERT TARGET",
            "User-facing product copy in the Reactant sample.",
          ))
          .host_name("layout-gallery-inert-target")
          .on_press(|game: &mut Game| game.layout_gallery.trace.push("INERT")),
        ),
    )
    .child(self.settings())
    .child(self.accessible_settings())
    .child(
      crate::collection_settings::CollectionSettings::new()
        .choice(self.state.collection_choice)
        .page(self.state.collection_page),
    )
    .child(self.table())
    .child(self.dropdown(menu_anchor))
    .child(self.layers())
    .child(self.modal(modal_trigger, modal_initial))
    .child(
      Label::new(ls(format!(
        "TRACE {} · RECONNECTS {}",
        if self.state.trace.is_empty() {
          "READY".to_owned()
        } else {
          self.state.trace.join(" > ")
        },
        self.state.reconnects,
      )))
      .name("layout-gallery-status")
      .style(styles::status()),
    )
  }
}

impl LayoutGallery {
  fn controls(&self, modal_trigger: ElementRef) -> Flex {
    let app = use_app();
    let announce = use_announce();
    Flex::new()
      .name("layout-gallery-controls")
      .direction(battlement::FlexDirection::Row)
      .wrap(battlement::FlexWrap::Wrap)
      .gap(8.0)
      .style(styles::toolbar())
      .child(
        Button::new(tx(
          "RESPONSIVE TRACKS",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-tracks")
        .configure_host(|host| host.focus_props(FocusProps::new().auto_focus(true)))
        .on_press(|game: &mut Game| {
          game.layout_gallery.alternate_tracks = !game.layout_gallery.alternate_tracks;
        }),
      )
      .child(
        Button::new(tx(
          "LARGE TEXT",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-text")
        .on_press(|game: &mut Game| {
          game.layout_gallery.large_text = !game.layout_gallery.large_text;
        }),
      )
      .child(
        Button::new(tx(
          "TOGGLE INERT",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-inert-toggle")
        .on_press(|game: &mut Game| {
          game.layout_gallery.inert_content = !game.layout_gallery.inert_content;
        }),
      )
      .child(
        Button::new(tx(
          "OPEN MODAL",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-modal")
        .element_ref(modal_trigger)
        .on_press(|game: &mut Game| game.layout_gallery.modal_open = true),
      )
      .child(
        Button::new(tx(
          "RECONNECT",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-reconnect")
        .on_press(move |game: &mut Game| {
          game.layout_gallery.reconnects += 1;
          app.refresh_snapshot();
        }),
      )
      .child(
        Button::new(tx(
          "RESET",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-reset")
        .on_press(move |game: &mut Game| {
          game.layout_gallery = LayoutGalleryState::default();
          announce.send(tx(
            "Settings reset",
            "User-facing product copy in the Reactant sample.",
          ));
        }),
      )
  }
  fn tabs(&self) -> View {
    let tabs = ["GENERAL", "AUDIO", "ACCESS"];
    View::new()
      .style(styles::section())
      .child(
        Label::new(tx(
          "FIXED TAB GRID",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(styles::section_heading()),
      )
      .child(
        Tabs::new(
          tx(
            "Settings sections",
            "User-facing product copy in the Reactant sample.",
          ),
          self.state.active_tab as u32,
        )
        .on_select(|game: &mut Game, index| {
          game.layout_gallery.active_tab = index as usize;
        })
        .host_name("layout-gallery-tabs")
        .child(
          tabs
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
              let selected = index == self.state.active_tab;
              Tab::new(ls(label), index as u32)
                .host_name(format!("layout-tab-{index}"))
                .configure_host(|host| host.key(label).style(styles::tab(selected)))
                .child(TabPanel::new(
                  index as u32,
                  Label::new(ls(format!("{label} SETTINGS")))
                    .name(format!("layout-tab-panel-{index}"))
                    .style(styles::setting_label(self.state.large_text)),
                ))
            })
            .collect::<Vec<_>>(),
        ),
      )
  }
  fn accessible_settings(&self) -> View {
    let announce = use_announce();
    View::new().name("layout-gallery-components").child(
      Group::new(Some(tx(
        "Accessible settings",
        "User-facing product copy in the Reactant sample.",
      )))
      .style(styles::section())
      .child(
        Heading::new(
          tx(
            "Accessible settings",
            "User-facing product copy in the Reactant sample.",
          ),
          2,
        )
        .style(styles::section_heading()),
      )
      .child(
        Image::new(tx(
          "Sound wave preview",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-image"),
      )
      .child(Text::new(tx(
        "Changes apply immediately",
        "User-facing product copy in the Reactant sample.",
      )))
      .child(
        Checkbox::new(
          if self.state.captions_enabled {
            tx(
              "CAPTIONS ON",
              "User-facing product copy in the Reactant sample.",
            )
          } else {
            tx(
              "CAPTIONS OFF",
              "User-facing product copy in the Reactant sample.",
            )
          },
          self.state.captions_enabled,
        )
        .host_name("layout-gallery-checkbox")
        .on_change(|game: &mut Game, checked| {
          game.layout_gallery.captions_enabled = checked;
        }),
      )
      .child(
        Switch::new(
          if self.state.spatial_audio {
            tx(
              "SPATIAL AUDIO ON",
              "User-facing product copy in the Reactant sample.",
            )
          } else {
            tx(
              "SPATIAL AUDIO OFF",
              "User-facing product copy in the Reactant sample.",
            )
          },
          self.state.spatial_audio,
        )
        .host_name("layout-gallery-switch")
        .on_change(|game: &mut Game, checked| {
          game.layout_gallery.spatial_audio = checked;
        }),
      )
      .child(
        RadioGroup::new(tx(
          "Audio quality",
          "User-facing product copy in the Reactant sample.",
        ))
        .child(
          Radio::new(
            tx(
              "Standard quality",
              "User-facing product copy in the Reactant sample.",
            ),
            self.state.radio_selection == 0,
          )
          .host_name("layout-gallery-radio-standard")
          .on_select(|game: &mut Game| game.layout_gallery.radio_selection = 0),
        )
        .child(
          Radio::new(
            tx(
              "Studio quality",
              "User-facing product copy in the Reactant sample.",
            ),
            self.state.radio_selection == 1,
          )
          .host_name("layout-gallery-radio-studio")
          .on_select(|game: &mut Game| game.layout_gallery.radio_selection = 1),
        ),
      )
      .child(
        Slider::new(
          tx(
            "Music volume",
            "User-facing product copy in the Reactant sample.",
          ),
          f64::from(self.state.volume),
          0.0,
          100.0,
          10.0,
        )
        .host_name("layout-gallery-slider")
        .value_text(ls(format!("{} percent", self.state.volume)))
        .on_change(|game: &mut Game, value| game.layout_gallery.volume = value as u32),
      )
      .child(
        Progress::determinate(
          tx(
            "Audio loaded",
            "User-facing product copy in the Reactant sample.",
          ),
          SemanticRange {
            current: f64::from(self.state.volume),
            minimum: 0.0,
            maximum: 100.0,
            text: None,
          },
        )
        .host_name("layout-gallery-progress"),
      )
      .child(
        Disclosure::new(
          tx(
            "Advanced audio",
            "User-facing product copy in the Reactant sample.",
          ),
          self.state.disclosure_open,
        )
        .host_name("layout-gallery-disclosure")
        .on_press(|game: &mut Game| {
          game.layout_gallery.disclosure_open = !game.layout_gallery.disclosure_open;
        }),
      )
      .child(self.state.disclosure_open.then(|| {
        Text::new(tx(
          "Advanced audio controls",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-disclosure-content")
      }))
      .child(
        Button::new(tx(
          "SAVE SETTINGS",
          "User-facing product copy in the Reactant sample.",
        ))
        .host_name("layout-gallery-announce")
        .on_press(move |game: &mut Game| {
          game.layout_gallery.trace.push("SAVED");
          announce.send(tx(
            "Settings saved",
            "User-facing product copy in the Reactant sample.",
          ));
        }),
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
      .child(
        Label::new(tx(
          "RESPONSIVE SETTINGS",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(styles::section_heading()),
      )
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
                StatefulSetting::new()
                  .name(name)
                  .row(index as u32 + 1)
                  .large_text(self.state.large_text)
                  .key(name)
              })
              .collect::<Vec<_>>(),
          )
          .child(
            Label::new(if self.state.large_text {
              tx("LARGE", "User-facing product copy in the Reactant sample.")
            } else {
              tx(
                "STANDARD",
                "User-facing product copy in the Reactant sample.",
              )
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
      .child(
        Label::new(tx(
          "STICKY INPUT TABLE",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(styles::section_heading()),
      )
      .child(
        ScrollView::new()
          .name("layout-gallery-table")
          .mode(ScrollViewMode::Vertical)
          .style(styles::table())
          .content_container_style(styles::table_content())
          .child(
            Label::new(tx(
              "SETTING",
              "User-facing product copy in the Reactant sample.",
            ))
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
                      Label::new(ls(format!("INPUT {:02}", index + 1)))
                        .key((index, 0_u8))
                        .style(styles::table_cell(index % 2 == 0)),
                      Label::new(if index % 2 == 0 {
                        tx(
                          "ENABLED",
                          "User-facing product copy in the Reactant sample.",
                        )
                      } else {
                        tx("AUTO", "User-facing product copy in the Reactant sample.")
                      })
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
        .host_name("layout-gallery-menu")
        .placement(PopoverPlacement::bottom_start().offset(6.0))
        .style(styles::popover())
        .child(Label::new(tx(
          "PORTALED MENU",
          "User-facing product copy in the Reactant sample.",
        )))
        .child(
          Button::new(tx(
            "APPLY AND CLOSE",
            "User-facing product copy in the Reactant sample.",
          ))
          .host_name("layout-gallery-menu-action")
          .style(styles::popover_action())
          .on_press(|game: &mut Game| {
            game.layout_gallery.trace.push("TARGET");
            game.layout_gallery.menu_open = false;
          }),
        )
    });
    View::new()
      .style(styles::section())
      .child(
        Label::new(tx(
          "CLIPPED DROPDOWN",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(styles::section_heading()),
      )
      .child(
        View::new()
          .name("layout-gallery-clip")
          .style(styles::clipped_control())
          .child(
            Button::new(if self.state.menu_open {
              tx(
                "CLOSE MENU",
                "User-facing product copy in the Reactant sample.",
              )
            } else {
              tx(
                "OPEN MENU",
                "User-facing product copy in the Reactant sample.",
              )
            })
            .host_name("layout-gallery-menu-trigger")
            .element_ref(anchor)
            .on_press(|game: &mut Game| {
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
    let changed = control_behavior::static_text_props(tx(
      "Layer order changed",
      "User-facing product copy in the Reactant sample.",
    ));
    let foreground_order = if self.state.layers_reversed { -1 } else { 2 };
    View::new()
      .style(styles::section())
      .child(
        Label::new(tx(
          "ISOLATED STACK LAYERS",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(styles::section_heading()),
      )
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
            Label::new(tx(
              "EQUAL ORDER · SOURCE FIRST",
              "User-facing product copy in the Reactant sample.",
            ))
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
            Button::new(tx(
              "CHANGE LAYER ORDER",
              "User-facing product copy in the Reactant sample.",
            ))
            .host_name("layout-gallery-layer-action")
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
            .on_press(|game: &mut Game| {
              game.layout_gallery.layers_reversed = !game.layout_gallery.layers_reversed;
            }),
          ),
      )
      .child(
        AnimatePresence::new().child(self.state.layers_reversed.then(|| {
          Node::new(
            Label::new(tx(
              "LAYER ORDER CHANGED",
              "User-facing product copy in the Reactant sample.",
            ))
            .key("layout-gallery-layer-presence")
            .semantic(changed)
            .initial(StyleTarget::new().opacity(0.0).y(-8.0))
            .animate(StyleTarget::new().opacity(1.0).y(0.0))
            .exit(StyleTarget::new().opacity(0.0).y(8.0)),
          )
        })),
      )
  }
  fn modal(&self, trigger: ElementRef, initial: ElementRef) -> impl Render {
    self.state.modal_open.then(|| {
      Overlay::modal(
        self.overlay.clone(),
        tx(
          "Viewport modal",
          "User-facing product copy in the Reactant sample.",
        ),
      )
      .host_name("layout-gallery-modal-scope")
      .on_dismiss(|game: &mut Game| {
        game.layout_gallery.modal_open = false;
      })
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
            .child(
              Label::new(tx(
                "Viewport modal",
                "User-facing product copy in the Reactant sample.",
              ))
              .style(styles::modal_title()),
            )
            .child(
              Button::new(tx(
                "CLOSE MODAL",
                "User-facing product copy in the Reactant sample.",
              ))
              .host_name("layout-gallery-modal-close")
              .element_ref(initial)
              .while_focus_visible(StyleTarget::new().scale(1.06))
              .on_press(|game: &mut Game| {
                game.layout_gallery.modal_open = false;
              }),
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
      Label::new(ls(self.name))
        .name(format!("layout-setting-{}", self.name.to_ascii_lowercase()))
        .grid_item(GridItem::new().row(self.row).column(1))
        .style(styles::setting_label(self.large_text)),
      Button::new(ls(format!("VALUE {revision}")))
        .host_name(format!(
          "layout-setting-value-{}",
          self.name.to_ascii_lowercase()
        ))
        .grid_item(GridItem::new().row(self.row).column(2))
        .style(styles::setting_value())
        .on_press(move |_game: &mut Game| set_revision.update(|value| value + 1)),
    ))
  }
}
