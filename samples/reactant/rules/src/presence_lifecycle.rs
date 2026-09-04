use trox::{assert_localized, tx};

use crate::{Game, design_system};
use battlement::{
  Align, Color, FlexDirection, FlexWrap, LengthUnits, Overflow, ScrollViewMode, ScrollerVisibility,
  Style, WhiteSpace,
};
use battlement_reactant::prelude::*;
use std::{
  cell::{Cell, RefCell},
  collections::BTreeMap,
  fmt,
  rc::Rc,
};

#[derive(Clone)]
pub(crate) struct PresenceLifecycleState {
  open: bool,
  route: u32,
  mode: PresenceMode,
  manual_hold: Rc<Cell<bool>>,
  exit_waves: u32,
  reconnects: u32,
  events: Vec<String>,
  holds: Rc<RefCell<BTreeMap<u32, Presence>>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct MountRecord {
  mounts: u32,
  unmounts: u32,
  events: Vec<String>,
}

#[builder]
pub(crate) struct PresenceLifecycle {
  pub(crate) state: PresenceLifecycleState,
  pub(crate) compact: bool,
}

#[builder]
#[derive(Clone)]
struct RetainedPanel {
  route: u32,
  manual_hold: Rc<Cell<bool>>,
  holds: Rc<RefCell<BTreeMap<u32, Presence>>>,
  #[builder(required)]
  record: StateSetter<MountRecord>,
}

impl Default for PresenceLifecycleState {
  fn default() -> Self {
    Self {
      open: true,
      route: 1,
      mode: PresenceMode::Sync,
      manual_hold: Rc::new(Cell::new(false)),
      exit_waves: 0,
      reconnects: 0,
      events: vec!["boundary mounted".to_owned()],
      holds: Rc::new(RefCell::new(BTreeMap::new())),
    }
  }
}

impl fmt::Debug for PresenceLifecycleState {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("PresenceLifecycleState")
      .field("open", &self.open)
      .field("route", &self.route)
      .field("mode", &self.mode)
      .field("manual_hold", &self.manual_hold.get())
      .field("exit_waves", &self.exit_waves)
      .field("reconnects", &self.reconnects)
      .field("events", &self.events)
      .finish()
  }
}

impl PartialEq for PresenceLifecycleState {
  fn eq(&self, other: &Self) -> bool {
    self.open == other.open
      && self.route == other.route
      && self.mode == other.mode
      && self.manual_hold.get() == other.manual_hold.get()
      && self.exit_waves == other.exit_waves
      && self.reconnects == other.reconnects
      && self.events == other.events
  }
}

impl PresenceLifecycleState {
  fn toggle_open(&mut self) {
    self.open = !self.open;
    self.events.push(if self.open {
      "modal reopened".to_owned()
    } else {
      "modal exit requested".to_owned()
    });
  }

  fn route(&mut self) {
    self.route = self.route.wrapping_add(1);
    self.open = true;
    self.events.push(format!("route {} selected", self.route));
  }

  fn toggle_mode(&mut self) {
    self.mode = match self.mode {
      PresenceMode::Sync => PresenceMode::Wait,
      PresenceMode::Wait => PresenceMode::Sync,
      PresenceMode::PopLayout => unreachable!(),
    };
    self.events.push(format!("mode {:?}", self.mode));
  }

  fn toggle_hold(&mut self) {
    self.manual_hold.set(!self.manual_hold.get());
    self.events.push(if self.manual_hold.get() {
      "manual hold armed".to_owned()
    } else {
      "automatic release armed".to_owned()
    });
  }

  fn release(&mut self) {
    if let Some(presence) = self.holds.borrow().get(&self.route).cloned() {
      presence.safe_to_remove();
      self.events.push("manual hold released".to_owned());
    }
  }

  fn reconnect(&mut self) {
    self.reconnects = self.reconnects.wrapping_add(1);
    self.events.push("reconnect snapshot requested".to_owned());
  }

  fn reset(&mut self) {
    *self = Self::default();
  }
}

impl Component for PresenceLifecycle {
  fn render(&self) -> impl Render {
    let (record, set_record) = use_state(MountRecord::default());
    let mode_name = match self.state.mode {
      PresenceMode::Sync => "SYNC",
      PresenceMode::Wait => "WAIT",
      PresenceMode::PopLayout => unreachable!(),
    };
    ScrollView::new()
      .name("presence-lifecycle-canvas")
      .mode(ScrollViewMode::Vertical)
      .horizontal_scroller_visibility(ScrollerVisibility::Hidden)
      .vertical_scroller_visibility(ScrollerVisibility::Auto)
      .style(design_system::canvas(self.compact).padding(0.0))
      .content_container_style(content())
      .child(
        Label::new(tx(
          "RETENTION & TERMINAL ORDERING",
          "User-facing product copy in the Reactant sample.",
        ))
        .style(eyebrow()),
      )
      .child(
        Label::new(tx(
          "Presence & Lifecycle",
          "User-facing product copy in the Reactant sample.",
        ))
        .name("page-title")
        .style(title()),
      )
      .child(
        Label::new(assert_localized(format!(
          "{mode_name}  ·  ROUTE {}  ·  {}  ·  EXIT WAVES {}  ·  RECONNECTS {}",
          self.state.route,
          if self.state.manual_hold.get() {
            "MANUAL HOLD"
          } else {
            "AUTOMATIC"
          },
          self.state.exit_waves,
          self.state.reconnects,
        )))
        .name("presence-status")
        .style(status()),
      )
      .child(controls())
      .child(
        View::new().name("presence-stage").style(stage()).child(
          AnimatePresence::new()
            .initial(false)
            .mode(self.state.mode)
            .custom(self.state.route as i32)
            .on_exit_complete(|game: &mut Game| {
              game.presence_lifecycle.exit_waves += 1;
              game
                .presence_lifecycle
                .events
                .push("presence exit complete".to_owned());
            })
            .child(self.state.open.then(|| {
              Node::new(
                RetainedPanel::new()
                  .route(self.state.route)
                  .manual_hold(Rc::clone(&self.state.manual_hold))
                  .holds(Rc::clone(&self.state.holds))
                  .record(set_record.clone())
                  .key(self.state.route),
              )
            })),
        ),
      )
      .child(
        Label::new(assert_localized(format!(
          "MOUNTS {}  ·  UNMOUNTS {}  ·  {}",
          record.mounts,
          record.unmounts,
          if self.state.open {
            "source present"
          } else {
            "source absent"
          },
        )))
        .name("presence-mount-record")
        .style(record_style()),
      )
      .child(
        Label::new(assert_localized(format!(
          "ORDERED EVENTS\n{}\n{}",
          record.events.join("  ›  "),
          self.state.events.join("  ›  "),
        )))
        .name("presence-event-record")
        .style(event_record()),
      )
  }
}

impl Component for RetainedPanel {
  fn render(&self) -> impl Render {
    let (counter, set_counter) = use_state(7_u32);
    let presence = use_presence();
    self.holds.borrow_mut().insert(self.route, presence.clone());
    if !presence.is_present() && !self.manual_hold.get() {
      presence.safe_to_remove();
    }
    let route = self.route;
    let mounted_record = self.record.clone();
    let holds = Rc::clone(&self.holds);
    use_effect(
      move || {
        mounted_record.update(move |mut record| {
          record.mounts += 1;
          record.events.push(format!("route {route} mounted"));
          record
        });
        let unmounted_record = mounted_record.clone();
        move || {
          holds.borrow_mut().remove(&route);
          unmounted_record.update(move |mut record| {
            record.unmounts += 1;
            record.events.push(format!("route {route} unmounted"));
            record
          });
        }
      },
      (),
    );
    let increment = set_counter.clone();
    View::new()
      .name(format!("presence-panel-{}", self.route))
      .style(panel())
      .animate(StyleTarget::new().opacity(1.0))
      .exit(
        MotionTarget::new(StyleTarget::new().opacity(0.0))
          .transition(
            Transition::tween()
              .duration_secs(0.32)
              .ease(Easing::EaseInOut),
          )
          .on_complete(|game: &mut Game| {
            game
              .presence_lifecycle
              .events
              .push("panel animation completed".to_owned());
          })
          .on_cancel(|game: &mut Game| {
            game
              .presence_lifecycle
              .events
              .push("panel animation cancelled".to_owned());
          }),
      )
      .child(
        Label::new(assert_localized(format!("ROUTED PANEL {}", self.route))).style(panel_title()),
      )
      .child(
        Label::new(assert_localized(format!(
          "RETAINED STATE {counter}  ·  {}",
          if presence.is_present() {
            "PRESENT"
          } else {
            "EXITING"
          }
        )))
        .name("presence-retained-state")
        .style(panel_state()),
      )
      .child(
        Button::new(tx(
          "COUNTER +1",
          "User-facing product copy in the Reactant sample.",
        ))
        .name("presence-counter")
        .style(action_style())
        .on_click(move |_game: &mut Game| increment.update(|value| value + 1)),
      )
      .child(
        View::new()
          .name("presence-nested-exit")
          .style(nested())
          .animate(StyleTarget::new().opacity(1.0).scale(1.0))
          .exit(
            MotionTarget::new(StyleTarget::new().opacity(0.0).scale(0.55))
              .transition(Transition::tween().duration_secs(0.46)),
          ),
      )
  }
}

fn controls() -> View {
  let app = use_app();
  View::new()
    .style(control_row())
    .child(action("OPEN / CLOSE", "presence-toggle", |game| {
      game.presence_lifecycle.toggle_open()
    }))
    .child(action("ROUTE", "presence-route", |game| {
      game.presence_lifecycle.route()
    }))
    .child(action("SYNC / WAIT", "presence-mode", |game| {
      game.presence_lifecycle.toggle_mode()
    }))
    .child(action("MANUAL HOLD", "presence-hold", |game| {
      game.presence_lifecycle.toggle_hold()
    }))
    .child(action("RELEASE", "presence-release", |game| {
      game.presence_lifecycle.release()
    }))
    .child(action("RECONNECT", "presence-reconnect", move |game| {
      game.presence_lifecycle.reconnect();
      app.refresh_snapshot();
    }))
    .child(action("RESET", "presence-reset", |game| {
      game.presence_lifecycle.reset()
    }))
    .child(action("VARIANTS", "presence-variants", |game| {
      game.screen = crate::Screen::VariantsOrchestration;
    }))
}

fn action(
  text: &'static str,
  name: &'static str,
  callback: impl Fn(&mut Game) + 'static,
) -> Button {
  Button::new(assert_localized(text))
    .name(name)
    .style(action_style())
    .on_click(callback)
}

fn content() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .padding(28.0)
    .align_items(Align::FlexStart)
}

fn eyebrow() -> Style {
  Style::new()
    .font_size(20.0)
    .color(Color::rgb(0.98, 0.4, 0.16))
}

fn title() -> Style {
  Style::new()
    .font_size(40.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
    .margin((6, 0, 12, 0))
}

fn status() -> Style {
  Style::new()
    .font_size(17.0)
    .color(Color::rgb(0.68, 0.76, 0.78))
}

fn control_row() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((10, 0))
}

fn action_style() -> Style {
  Style::new()
    .height(40.0)
    .min_width(104.0)
    .background_color(Color::rgb(0.035, 0.09, 0.115))
    .color(Color::rgb(0.94, 0.98, 0.99))
    .border_color(Color::rgb(0.32, 0.92, 0.96))
    .border_width(1.0)
    .font_size(14.0)
    .margin((0, 7, 7, 0))
}

fn stage() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .min_height(260.0)
    .padding(18.0)
    .background_color(Color::rgb(0.018, 0.045, 0.06))
    .border_color(Color::rgb(0.15, 0.28, 0.32))
    .border_width(1.0)
    .overflow(Overflow::Hidden)
}

fn panel() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .min_height(210.0)
    .padding(20.0)
    .background_color(Color::rgb(0.045, 0.11, 0.14))
    .border_color(Color::rgb(0.32, 0.92, 0.96))
    .border_width(2.0)
}

fn panel_title() -> Style {
  Style::new()
    .font_size(23.0)
    .color(Color::rgb(0.94, 0.98, 0.99))
}

fn panel_state() -> Style {
  Style::new()
    .font_size(16.0)
    .color(Color::rgb(0.48, 0.9, 0.7))
    .margin((10, 0))
}

fn nested() -> Style {
  Style::new()
    .width(140.0)
    .height(14.0)
    .margin((18, 0, 0, 0))
    .border_radius(7.0)
    .background_color(Color::rgb(0.65, 0.28, 0.95))
}

fn record_style() -> Style {
  Style::new()
    .font_size(16.0)
    .color(Color::rgb(0.32, 0.92, 0.96))
    .margin((14, 0, 4, 0))
}

fn event_record() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .font_size(13.0)
    .white_space(WhiteSpace::Normal)
    .color(Color::rgb(0.68, 0.76, 0.78))
}
