use crate::{Game, design_system, layout_gallery_styles as styles};
use battlement::{
  Align, Color, GridAutoFlow, GridItem, GridTrack, PickingMode, ScrollViewMode, StackItem, Sticky,
};
use battlement_reactant::{hooks, prelude::*};

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
    let page = use_scroll_area(ScrollAreaOptions {
      name: Some(text("Layout Gallery")),
      axis: AccessibilityScrollAxis::Vertical,
      can_scroll_forward: true,
      can_scroll_backward: true,
      on_scroll: |game: &mut Game, direction| match direction {
        AccessibilityScrollDirection::Forward => game.layout_gallery.trace.push("SCROLL FORWARD"),
        AccessibilityScrollDirection::Backward => game.layout_gallery.trace.push("SCROLL BACKWARD"),
      },
    });
    ScrollView::new()
      .name("layout-gallery-canvas")
      .semantic(page.semantic)
      .interaction_props(page.interaction)
      .mode(ScrollViewMode::Vertical)
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(styles::content())
      .layout_scroll(true)
      .child(Label::new("PUBLIC LAYOUT · ONE COHERENT FLOW").style(styles::eyebrow()))
      .child(
        Label::new("Layout Gallery")
          .name("page-title")
          .semantic(use_heading(text("Layout Gallery"), 1))
          .style(styles::title(self.state.large_text)),
      )
      .child(self.controls(modal_trigger.clone()))
      .child(self.tabs())
      .child(
        View::new()
          .name("layout-gallery-inert-region")
          .inert(self.state.inert_content)
          .child(
            Button::new("INERT TARGET")
              .name("layout-gallery-inert-target")
              .on_click(|game: &mut Game| game.layout_gallery.trace.push("INERT")),
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
    let app = use_app();
    let announce = use_announce();
    let reset = use_button(ButtonOptions {
      name: text("Reset settings"),
      is_disabled: false,
      on_press: move |game: &mut Game| {
        game.layout_gallery = LayoutGalleryState::default();
        announce.send(text("Settings reset"));
      },
    });
    let tracks = use_button(ButtonOptions {
      name: text("Responsive tracks"),
      is_disabled: false,
      on_press: |game: &mut Game| {
        game.layout_gallery.alternate_tracks = !game.layout_gallery.alternate_tracks;
      },
    });
    let open_modal = use_button(ButtonOptions {
      name: text("Open modal"),
      is_disabled: false,
      on_press: |game: &mut Game| game.layout_gallery.modal_open = true,
    });
    Flex::new()
      .name("layout-gallery-controls")
      .direction(battlement::FlexDirection::Row)
      .wrap(battlement::FlexWrap::Wrap)
      .gap(8.0)
      .style(styles::toolbar())
      .child(
        Button::new("RESPONSIVE TRACKS")
          .name("layout-gallery-tracks")
          .semantic(tracks.semantic)
          .focus_props(tracks.focus.auto_focus(true))
          .interaction_props(tracks.interaction),
      )
      .child(
        Button::new("LARGE TEXT")
          .name("layout-gallery-text")
          .on_click(|game: &mut Game| {
            game.layout_gallery.large_text = !game.layout_gallery.large_text;
          }),
      )
      .child(
        Button::new("TOGGLE INERT")
          .name("layout-gallery-inert-toggle")
          .on_click(|game: &mut Game| {
            game.layout_gallery.inert_content = !game.layout_gallery.inert_content;
          }),
      )
      .child(
        Button::new("OPEN MODAL")
          .name("layout-gallery-modal")
          .element_ref(modal_trigger)
          .semantic(open_modal.semantic)
          .focus_props(open_modal.focus)
          .interaction_props(open_modal.interaction),
      )
      .child(
        Button::new("RECONNECT")
          .name("layout-gallery-reconnect")
          .on_click(move |game: &mut Game| {
            game.layout_gallery.reconnects += 1;
            app.refresh_snapshot();
          }),
      )
      .child(
        Button::new("RESET")
          .name("layout-gallery-reset")
          .semantic(reset.semantic)
          .focus_props(reset.focus)
          .interaction_props(reset.interaction),
      )
  }
  fn tabs(&self) -> View {
    let tabs = ["GENERAL", "AUDIO", "ACCESS"];
    let tab_list = use_tabs(text("Settings sections"));
    View::new()
      .style(styles::section())
      .child(Label::new("FIXED TAB GRID").style(styles::section_heading()))
      .child(
        Grid::new()
          .name("layout-gallery-tabs")
          .semantic(tab_list.semantic.clone())
          .element_ref(tab_list.element_ref.clone())
          .columns([
            GridTrack::px(132.0),
            GridTrack::px(132.0),
            GridTrack::px(132.0),
          ])
          .rows([GridTrack::auto(), GridTrack::auto()])
          .column_gap(8.0)
          .child(
            tabs
              .into_iter()
              .enumerate()
              .map(|(index, label)| {
                let tab = use_tab(
                  &tab_list,
                  ChoiceOptions {
                    name: text(label),
                    selected: index == self.state.active_tab,
                    is_disabled: false,
                    on_select: move |game: &mut Game| {
                      game.layout_gallery.active_tab = index;
                    },
                  },
                );
                Button::new(label)
                  .key(label)
                  .name(format!("layout-tab-{index}"))
                  .grid_item(GridItem::new().row(1).column(index as u32 + 1))
                  .semantic(tab.semantic)
                  .focus_props(tab.focus)
                  .interaction_props(tab.interaction)
                  .style(styles::tab(index == self.state.active_tab))
              })
              .collect::<Vec<_>>(),
          )
          .child(
            tabs
              .into_iter()
              .enumerate()
              .map(|(index, label)| {
                Label::new(format!("{label} SETTINGS"))
                  .key(format!("{label}-panel"))
                  .name(format!("layout-tab-panel-{index}"))
                  .semantic(use_tab_panel(&tab_list, index == self.state.active_tab))
                  .grid_item(GridItem::new().row(2).column(1))
                  .style(styles::setting_label(self.state.large_text))
              })
              .collect::<Vec<_>>(),
          ),
      )
  }
  fn accessible_settings(&self) -> View {
    let checkbox = use_checkbox(ToggleOptions {
      name: text("Captions"),
      checked: self.state.captions_enabled,
      is_disabled: false,
      on_change: |game: &mut Game, checked| {
        game.layout_gallery.captions_enabled = checked;
      },
    });
    let spatial_audio = use_switch(ToggleOptions {
      name: text("Spatial audio"),
      checked: self.state.spatial_audio,
      is_disabled: false,
      on_change: |game: &mut Game, checked| {
        game.layout_gallery.spatial_audio = checked;
      },
    });
    let radio_group = use_radio_group(text("Audio quality"));
    let standard = use_radio(
      &radio_group,
      ChoiceOptions {
        name: text("Standard quality"),
        selected: self.state.radio_selection == 0,
        is_disabled: false,
        on_select: |game: &mut Game| game.layout_gallery.radio_selection = 0,
      },
    );
    let studio = use_radio(
      &radio_group,
      ChoiceOptions {
        name: text("Studio quality"),
        selected: self.state.radio_selection == 1,
        is_disabled: false,
        on_select: |game: &mut Game| game.layout_gallery.radio_selection = 1,
      },
    );
    let slider = use_slider(SliderOptions {
      name: text("Music volume"),
      value: f64::from(self.state.volume),
      minimum: 0.0,
      maximum: 100.0,
      step: 10.0,
      value_text: Some(text(format!("{} percent", self.state.volume))),
      is_disabled: false,
      on_change: |game: &mut Game, value| game.layout_gallery.volume = value as u32,
    });
    let disclosure = use_disclosure(DisclosureOptions {
      name: text("Advanced audio"),
      expanded: self.state.disclosure_open,
      is_disabled: false,
      on_toggle: |game: &mut Game| {
        game.layout_gallery.disclosure_open = !game.layout_gallery.disclosure_open;
      },
    });
    let announce = use_announce();
    let save = use_button(ButtonOptions {
      name: text("Save settings"),
      is_disabled: false,
      on_press: move |game: &mut Game| {
        game.layout_gallery.trace.push("SAVED");
        announce.send(text("Settings saved"));
      },
    });
    View::new()
      .name("layout-gallery-accessibility")
      .semantic(use_group(Some(text("Accessible settings"))))
      .style(styles::section())
      .child(
        Label::new("ACCESSIBLE SETTINGS")
          .semantic(use_heading(text("Accessible settings"), 2))
          .style(styles::section_heading()),
      )
      .child(
        Label::new("Sound wave preview")
          .name("layout-gallery-image")
          .semantic(use_image(text("Sound wave preview"))),
      )
      .child(
        Label::new("Changes apply immediately")
          .semantic(use_static_text(text("Changes apply immediately"))),
      )
      .child(
        Button::new(if self.state.captions_enabled {
          "CAPTIONS ON"
        } else {
          "CAPTIONS OFF"
        })
        .name("layout-gallery-checkbox")
        .semantic(checkbox.semantic)
        .focus_props(checkbox.focus)
        .interaction_props(checkbox.interaction),
      )
      .child(
        Button::new(if self.state.spatial_audio {
          "SPATIAL AUDIO ON"
        } else {
          "SPATIAL AUDIO OFF"
        })
        .name("layout-gallery-switch")
        .semantic(spatial_audio.semantic)
        .focus_props(spatial_audio.focus)
        .interaction_props(spatial_audio.interaction),
      )
      .child(
        View::new()
          .name("layout-gallery-radio-group")
          .semantic(radio_group.semantic)
          .element_ref(radio_group.element_ref)
          .child(
            Button::new("STANDARD QUALITY")
              .name("layout-gallery-radio-standard")
              .semantic(standard.semantic)
              .focus_props(standard.focus)
              .interaction_props(standard.interaction),
          )
          .child(
            Button::new("STUDIO QUALITY")
              .name("layout-gallery-radio-studio")
              .semantic(studio.semantic)
              .focus_props(studio.focus)
              .interaction_props(studio.interaction),
          ),
      )
      .child(
        Button::new(format!("MUSIC VOLUME {}", self.state.volume))
          .name("layout-gallery-slider")
          .semantic(slider.semantic)
          .focus_props(slider.focus)
          .interaction_props(slider.interaction),
      )
      .child(
        Label::new(format!("LOADED {}%", self.state.volume))
          .name("layout-gallery-progress")
          .semantic(use_progress(
            text("Audio loaded"),
            AccessibilityRangeValue {
              current: f64::from(self.state.volume),
              minimum: 0.0,
              maximum: 100.0,
              text: None,
            },
          )),
      )
      .child(
        Button::new("ADVANCED AUDIO")
          .name("layout-gallery-disclosure")
          .semantic(disclosure.semantic)
          .focus_props(disclosure.focus)
          .interaction_props(disclosure.interaction),
      )
      .child(self.state.disclosure_open.then(|| {
        Label::new("Advanced audio controls")
          .name("layout-gallery-disclosure-content")
          .semantic(use_static_text(text("Advanced audio controls")))
      }))
      .child(
        Button::new("SAVE SETTINGS")
          .name("layout-gallery-announce")
          .semantic(save.semantic)
          .focus_props(save.focus)
          .interaction_props(save.interaction),
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
              .semantic(use_static_text(text("Layer order changed")))
              .initial(StyleTarget::new().opacity(0.0).y(-8.0))
              .animate(StyleTarget::new().opacity(1.0).y(0.0))
              .exit(StyleTarget::new().opacity(0.0).y(8.0)),
          )
        })),
      )
  }
  fn modal(&self, trigger: ElementRef, initial: ElementRef) -> impl Render {
    self.state.modal_open.then(|| {
      let dialog = use_dialog(DialogOptions {
        name: text("Viewport modal"),
        on_dismiss: Some(|game: &mut Game| {
          game.layout_gallery.modal_open = false;
        }),
      });
      Overlay::modal(self.overlay.clone())
        .name("layout-gallery-modal-scope")
        .semantic(dialog.semantic)
        .interaction_props(dialog.interaction)
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
                  .while_focus_visible(StyleTarget::new().scale(1.06))
                  .on_click(|game: &mut Game| {
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
