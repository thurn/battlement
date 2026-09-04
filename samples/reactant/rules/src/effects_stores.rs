use trox::{ls, tx};

use crate::{Control, Game, design_system};
use battlement::{ScrollViewMode, ScrollerVisibility};
use battlement_reactant::prelude::*;
use std::{
  collections::HashMap,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
};

#[builder]
pub(crate) struct EffectsStores {
  pub(crate) enabled: bool,
  #[builder(required)]
  pub(crate) effect_interaction: design_system::ControlState,
  #[builder(required)]
  pub(crate) store: SampleStore,
  #[builder(required)]
  pub(crate) store_phase: StorePhase,
  #[builder(required)]
  pub(crate) store_interaction: design_system::ControlState,
  pub(crate) compact: bool,
}

#[derive(Clone)]
pub(crate) struct SampleStore {
  name: &'static str,
  state: Arc<StoreState>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum StorePhase {
  Primary,
  Secondary,
  Updated,
}

impl PartialEq for SampleStore {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.state, &other.state)
  }
}

impl ExternalStore for SampleStore {
  type Snapshot = usize;

  fn snapshot(&self) -> Self::Snapshot {
    self.state.value.load(Ordering::Acquire)
  }

  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    let listener = self.state.next_listener.fetch_add(1, Ordering::Relaxed);
    self
      .state
      .listeners
      .lock()
      .expect("sample store listeners are available")
      .insert(listener, notify);
    let state = Arc::clone(&self.state);
    Subscription::new(move || {
      state
        .listeners
        .lock()
        .expect("sample store listeners are available")
        .remove(&listener);
    })
  }
}

impl SampleStore {
  pub(crate) fn new(name: &'static str, value: usize) -> Self {
    Self {
      name,
      state: Arc::new(StoreState {
        value: AtomicUsize::new(value),
        next_listener: AtomicUsize::new(0),
        listeners: Mutex::new(HashMap::new()),
      }),
    }
  }

  pub(crate) fn publish(&self, value: usize) {
    self.state.value.store(value, Ordering::Release);
    let listeners = self
      .state
      .listeners
      .lock()
      .expect("sample store listeners are available")
      .values()
      .cloned()
      .collect::<Vec<_>>();
    for listener in listeners {
      listener.notify();
    }
  }
}

impl Component for EffectsStores {
  fn render(&self) -> impl Render {
    let (connected, set_connected) = use_state(false);
    let enabled = self.enabled;
    use_effect(
      move || {
        set_connected.set(enabled);
        let cleanup = set_connected.clone();
        move || cleanup.set(false)
      },
      enabled,
    );
    let snapshot = use_external_store(self.store.clone());
    battlement_reactant::host::ScrollView::new()
      .name("effects-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .vertical_scroller_style(design_system::effects_scroller())
      .vertical_low_button_style(design_system::effects_scroll_button())
      .vertical_high_button_style(design_system::effects_scroll_button())
      .vertical_track_style(design_system::effects_scroll_track())
      .vertical_dragger_style(design_system::effects_scroll_dragger())
      .vertical_dragger_border_style(design_system::effects_scroll_dragger())
      .style(design_system::canvas(self.compact))
      .child(
        battlement_reactant::host::View::new()
          .name("effects-content")
          .style(design_system::effects_content())
          .child(
            battlement_reactant::host::Label::new(tx(
              "EFFECTS & STORES",
              "Effects and stores section heading.",
            ))
            .style(design_system::eyebrow()),
          )
          .child(
            battlement_reactant::host::Label::new(tx(
              "Synchronize after commit",
              "Effects and stores interface label.",
            ))
            .name("effects-title")
            .style(design_system::effects_title(self.compact)),
          )
          .child(
            battlement_reactant::host::View::new()
              .name("effects-specimen")
              .style(design_system::effects_specimen(self.compact))
              .child(
                battlement_reactant::host::View::new()
                  .name("effect-card")
                  .style(design_system::effect_card(self.compact))
                  .child(
                    battlement_reactant::host::Label::new(tx(
                      "Connection",
                      "Effects and stores interface label.",
                    ))
                    .style(design_system::effect_heading()),
                  )
                  .child(
                    battlement_reactant::host::Label::new(if connected {
                      tx("CONNECTED", "Effects and stores status message.")
                    } else {
                      tx("DISCONNECTED", "Effects and stores status message.")
                    })
                    .name("effect-status")
                    .style(design_system::effect_status()),
                  )
                  .child(crate::interactive_button(
                    if self.enabled { "RESTORE" } else { "CONNECT" },
                    "effects-action",
                    design_system::effect_action(self.effect_interaction, !self.enabled),
                    Control::EffectsAction,
                    |game: &mut Game| {
                      game.effects_enabled = !game.effects_enabled;
                    },
                  )),
              )
              .child(
                battlement_reactant::host::View::new()
                  .name("store-card")
                  .style(design_system::effect_card(self.compact))
                  .child(
                    battlement_reactant::host::Label::new(tx(
                      "External snapshot",
                      "Effects and stores interface label.",
                    ))
                    .style(design_system::effect_heading()),
                  )
                  .child(
                    battlement_reactant::host::Label::new(ls(format!(
                      "{}  {snapshot}",
                      self.store.name
                    )))
                    .name("store-status")
                    .style(design_system::effect_status()),
                  )
                  .child(crate::interactive_button(
                    self.store_phase.action(),
                    "store-action",
                    design_system::effect_action(
                      self.store_interaction,
                      self.store_phase != StorePhase::Updated,
                    ),
                    Control::StoreAction,
                    |game: &mut Game| match game.store_phase {
                      StorePhase::Primary => {
                        game.store_phase = StorePhase::Secondary;
                      }
                      StorePhase::Secondary => {
                        game.secondary_store.publish(41);
                        game.store_phase = StorePhase::Updated;
                      }
                      StorePhase::Updated => {
                        game.store_phase = StorePhase::Primary;
                      }
                    },
                  )),
              ),
          ),
      )
  }
}

impl StorePhase {
  fn action(self) -> &'static str {
    match self {
      Self::Primary => "SWAP SOURCE",
      Self::Secondary => "PUBLISH UPDATE",
      Self::Updated => "RESTORE",
    }
  }
}

struct StoreState {
  value: AtomicUsize,
  next_listener: AtomicUsize,
  listeners: Mutex<HashMap<usize, StoreNotify>>,
}

impl Eq for SampleStore {}
