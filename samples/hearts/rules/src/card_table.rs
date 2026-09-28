use std::collections::BTreeMap;

use battlement::{LocalTransform, Quaternion, RgbColor, Vector3};
use reactant::{hooks, native_host, prelude::*, world};
use uuid::Uuid;

use crate::{
  assets, card_assets, card_gesture,
  card_input::{self, CardInput},
  choreography::CardTiming,
  domain::Seat,
  hand_pan::{self, HandPan},
  layout,
  projection::{CardToken, HumanView, VisibleCard},
  settings::Preferences,
};

const CARD_WIDTH: f64 = 4.31462;
const CARD_HEIGHT: f64 = 6.000022 / CARD_WIDTH;
const CARD_STEP: f64 = 0.02;
const INSPECTION_ORDER: i16 = 100;

type Destinations = BTreeMap<CardToken, world::LayoutDestination>;

pub(crate) struct CardTable {
  view: HumanView,
  aspect: f64,
  inspection: Option<VisibleCard>,
  timing: CardTiming,
}

// A card with its layout destination, stacking order among overlapping backs and
// the scale its layout applies.
struct CardSurface(VisibleCard, Option<world::LayoutDestination>, i16, f64);

#[derive(Clone, Copy, Default)]
struct FanStyle {
  spacing: f64,
  curvature: f64,
  angle: f64,
  timing: CardTiming,
  seat: Option<Seat>,
}

impl CardTable {
  pub(crate) fn new(view: &HumanView, aspect: f64) -> Self {
    Self {
      view: view.clone(),
      aspect,
      inspection: None,
      timing: CardTiming::default(),
    }
  }

  pub(crate) fn timing(mut self, timing: CardTiming) -> Self {
    self.timing = timing;
    self
  }

  pub(crate) fn inspect(mut self, card: Option<VisibleCard>) -> Self {
    self.inspection = card;
    self
  }
}

impl Component for CardTable {
  fn render(&self) -> impl Render {
    let mut tokens: Vec<_> = self
      .view
      .hands
      .iter()
      .flatten()
      .chain(self.view.trick.iter().map(|(_, card)| card))
      .chain(self.view.captured.iter().flatten())
      .map(|card| card.token)
      .collect();
    tokens.sort_unstable();
    assert_eq!(
      tokens.len(),
      52,
      "the table must own every card exactly once"
    );
    assert!(
      tokens.windows(2).all(|pair| pair[0] != pair[1]),
      "duplicate table card"
    );
    let identities = tokens.clone();
    let destinations = hooks::use_memo(
      move || {
        identities
          .into_iter()
          .map(|token| (token, world::LayoutDestination::new(token.id())))
          .collect::<Destinations>()
      },
      tokens,
    );
    let inspection_id = hooks::use_memo(Uuid::new_v4, self.inspection.map(|card| card.token));
    let pan = hand_pan::use_hand_pan(&self.view.hands[Seat::South.index()]);
    let portrait = self.aspect < 1.0;
    let half_width = 5.7 * self.aspect;
    let larger_text = hooks::use_context::<Preferences>().larger_text;
    let footer_inset = if larger_text { 1.8 } else { 0.0 };
    let hands: Vec<_> = Seat::ALL
      .into_iter()
      .map(|seat| {
        if portrait {
          return self::portrait_hand(
            &self.view.hands[seat.index()],
            seat,
            &destinations,
            self.timing,
            &pan,
          );
        }
        self::hand(
          &self.view.hands[seat.index()],
          seat,
          half_width,
          &destinations,
          self.timing,
          footer_inset,
        )
      })
      .collect();
    let trick: Vec<_> = self
      .view
      .trick
      .iter()
      .map(|(seat, card)| {
        let (x, z, tilt) = if portrait {
          match seat {
            Seat::South => (0.0, -0.45, 0.0),
            Seat::West => (-0.65, 0.35, 0.0),
            Seat::North => (0.0, 1.15, 0.0),
            Seat::East => (0.65, 0.35, 0.0),
          }
        } else {
          match seat {
            Seat::South => (0.0, -0.7, -2.0),
            Seat::West => (-1.85, 0.45, -3.0),
            Seat::North => (0.0, 1.75, 0.0),
            Seat::East => (1.85, 0.45, 5.0),
          }
        };
        self::fan(
          &[*card],
          &destinations,
          if portrait { 0.63 } else { 1.5 },
          FanStyle {
            timing: self.timing,
            ..FanStyle::default()
          },
        )
        .plane(self::oriented_plane(x, z, tilt, 0.0))
      })
      .collect();
    let piles: Vec<_> = Seat::ALL
      .into_iter()
      .map(|seat| {
        let (x, z) = if portrait {
          match seat {
            Seat::South => (-half_width * 0.83, -1.3),
            Seat::West => (-half_width * 0.83, 3.15),
            Seat::North => (half_width * 0.83, 4.8),
            Seat::East => (half_width * 0.83, -1.3),
          }
        } else {
          match seat {
            Seat::South => (-3.0, -1.6 + footer_inset),
            Seat::West => (-3.3, 3.2),
            Seat::North => (3.3, 3.2),
            Seat::East => (3.0, -1.6 + footer_inset),
          }
        };
        let width = if portrait { 0.42 } else { 0.5 };
        world::Pile::new()
          .extent((1.0, 1.0))
          .plane(self::plane(x - 0.5, z - 0.5))
          .step(0.012, 0.012)
          .children(
            self.view.captured[seat.index()]
              .iter()
              .enumerate()
              .map(|(index, card)| {
                self.timing.child(
                  self::child(*card, &destinations, width, index, index),
                  None,
                  index,
                )
              }),
          )
      })
      .collect();
    let inspection = self.inspection.map(|card| {
      world::Group::new()
        .id(inspection_id)
        .position(Vector3::new(0.0, 1.0, 0.1))
        .rotation(self::pitch(90.0))
        .scale(Vector3::new(2.0, 2.0, 2.0))
        .child(CardSurface(card, None, INSPECTION_ORDER, 2.0))
    });
    ContextProvider::new()
      .context(pan)
      .child((hands, trick, piles, inspection))
  }
}

impl Component for CardSurface {
  fn render(&self) -> impl Render {
    let reference = native_host::use_object_ref();
    let input = hooks::use_optional_context::<CardInput>();
    let interactive = input
      .as_ref()
      .filter(|input| self.1.is_some() && input.owns(self.0.token))
      .cloned();
    let (offset, handlers) =
      card_gesture::use_gesture(self.0.token, self.1.clone(), interactive.clone());
    let (focused, set_focused) = hooks::use_state(false);
    let (hovered, set_hovered) = hooks::use_state(false);
    let focused = focused && interactive.is_some();
    let hovered = hovered && interactive.is_some();
    let focus = set_focused.clone();
    let pan = hooks::use_optional_context::<HandPan>();
    let focused_token = self.0.token;
    let hover = set_hovered.clone();
    let handlers = handlers
      .on_pointer_enter(
        move |_: reactant::event::ReactantEvent<battlement::PointerBoundaryEvent>| hover.set(true),
      )
      .on_pointer_leave(
        move |_: reactant::event::ReactantEvent<battlement::PointerBoundaryEvent>| {
          set_hovered.set(false)
        },
      );
    let selected = interactive
      .as_ref()
      .is_some_and(|input| input.selected(self.0.token));
    let mut hit = world::BoxHitRegion::new().size(Vector3::new(1.0, CARD_HEIGHT, 0.06));
    if let Some(input) = interactive {
      let token = self.0.token;
      let enabled = input.inspection().is_none();
      let cancel = input.clone();
      let activate = EventCallback::new(move |_| input.activate(token));
      hit = hit
        .focusable(true)
        .navigation(
          world::NavigationHandlers::new()
            .on_focus(EventCallback::new(move |_| {
              focus.set(true);
              if let Some(pan) = &pan {
                pan.focus(focused_token);
              }
            }))
            .on_blur(EventCallback::new(move |_| set_focused.set(false)))
            .on_cancel(EventCallback::new(move |_| cancel.cancel())),
        )
        .capture_on_press(enabled)
        .events(handlers)
        .accessible_button(
          trox::ls(card_input::name(self.0.face.expect("owned face"))),
          activate.clone(),
        )
        .on_click(activate);
    }
    let lift = if focused {
      0.12
    } else if hovered {
      0.07
    } else {
      0.0
    };
    let scale = if focused { 1.08 } else { 1.0 };
    world::Group::new()
      .reference(reference)
      .scale(Vector3::new(scale, scale, scale))
      .position(Vector3::new(
        offset.x,
        offset.y + lift + if selected { 0.3 } else { 0.0 },
        // A selected card lifts but keeps its stacking so its neighbor's index stays visible.
        offset.z - if focused { 0.07 } else { 0.0 },
      ))
      .child((
        self::shadow(self.2, self.3),
        self.0.face.map(|card| {
          world::Prefab::at(card_assets::model(card))
            .rotation(Quaternion::new(0.0, 1.0, 0.0, 0.0))
            .scale(Vector3::new(
              1.0 / CARD_WIDTH,
              1.0 / CARD_WIDTH,
              1.0 / CARD_WIDTH,
            ))
        }),
        self.0.face.is_none().then(|| {
          world::Sprite::new()
            .texture(assets::hearts::cards::BACK)
            .size(1.0, CARD_HEIGHT)
            .layer(self.2 * 2 + 1)
        }),
        hit,
      ))
  }
}

// Contact shadow: a translucent card silhouette just behind the card, sorted
// between the card and the one it overlaps.
fn shadow(order: i16, scale: f64) -> world::Sprite {
  world::Sprite::new()
    .texture(assets::hearts::cards::BACK)
    .size(1.0, CARD_HEIGHT)
    .tint(RgbColor::rgb(0.05, 0.08, 0.03))
    .opacity(0.4)
    // Half a card step behind in world units, so it stays in front of the card beneath.
    .position(Vector3::new(-0.015, -0.035, CARD_STEP / 2.0 / scale))
    .layer(order * 2)
}

fn portrait_hand(
  cards: &[VisibleCard],
  seat: Seat,
  destinations: &Destinations,
  timing: CardTiming,
  pan: &HandPan,
) -> Node {
  let style = FanStyle {
    timing,
    seat: Some(seat),
    ..FanStyle::default()
  };
  let units = pan.layout.units_per_pixel();
  let (center_x, z, angle, width, spacing) = if seat == Seat::South {
    let top = pan.layout.hand_top(pan.layout.large);
    (
      pan.world_center(),
      pan
        .layout
        .ground_point(0.0, top + pan.width * CARD_HEIGHT / 2.0)
        .1,
      0.0,
      pan.width,
      pan.spacing,
    )
  } else {
    let (x, y) = pan.layout.opponent_center(seat, pan.layout.large);
    let (x, z) = pan.layout.ground_point(x, y);
    let (width, spacing) = layout::opponent_fan(seat);
    (
      x,
      z,
      match seat {
        Seat::North => 180.0,
        Seat::West => 90.0,
        Seat::East => -90.0,
        Seat::South => unreachable!(),
      },
      width,
      spacing,
    )
  };
  Node::new(
    self::fan(
      cards,
      destinations,
      width * units,
      FanStyle {
        spacing: spacing * units,
        ..style
      },
    )
    .plane(self::oriented_plane(center_x, z, angle, 0.0)),
  )
}

fn hand(
  cards: &[VisibleCard],
  seat: Seat,
  half_width: f64,
  destinations: &Destinations,
  timing: CardTiming,
  footer_inset: f64,
) -> Node {
  let style = FanStyle {
    timing,
    seat: Some(seat),
    ..FanStyle::default()
  };
  if seat == Seat::South {
    let spacing = ((half_width * 2.0 - 3.0) / 13.0).min(1.16);
    // Extra curvature offsets the tilted camera's flattening of the arc. The
    // raised larger-text hand keeps a shallower arc so its end ranks clear the footer.
    let raised = footer_inset > 0.0;
    let curvature = self::arc(spacing, 3.6) * if raised { 1.3 } else { 1.25 };
    let drop = if raised { 0.0 } else { 0.62 };
    return Node::new(
      self::fan(
        cards,
        destinations,
        spacing * if raised { 1.75 } else { 2.1 },
        FanStyle {
          spacing,
          curvature,
          angle: -3.6,
          ..style
        },
      )
      .plane(self::plane(
        -0.5,
        -4.4 - drop - self::rise(cards.len(), curvature) + footer_inset,
      )),
    );
  }
  let raised = footer_inset > 0.0;
  let (side_z, side_spacing) = if raised { (1.75, 0.31) } else { (0.9, 0.45) };
  let (x, z, angle, width, spacing, fan_angle) = match seat {
    Seat::North => (0.0, 4.42, 180.0, 1.25, 0.58, -3.7),
    Seat::West => (-half_width * 0.55, side_z, 90.0, 1.4, side_spacing, -5.0),
    Seat::East => (half_width * 0.55, side_z, -90.0, 1.4, side_spacing, -5.0),
    Seat::South => unreachable!(),
  };
  // Opponent fans bow beyond the tangent arc, which the tilted camera flattens.
  let curvature = self::arc(spacing, f64::abs(fan_angle)) * 1.3;
  Node::new(
    self::fan(
      cards,
      destinations,
      width,
      FanStyle {
        spacing,
        curvature,
        angle: fan_angle,
        ..style
      },
    )
    .plane(self::oriented_plane(
      x,
      z,
      angle,
      self::rise(cards.len(), curvature),
    )),
  )
}

fn fan(
  cards: &[VisibleCard],
  destinations: &Destinations,
  width: f64,
  style: FanStyle,
) -> world::Fan {
  let spread = cards.len().saturating_sub(1) as f64;
  world::Fan::new()
    .extent((1.0, 1.0))
    .curve(
      spread * style.spacing,
      self::rise(cards.len(), style.curvature),
    )
    .angle((spread * style.angle).to_radians())
    .children(cards.iter().enumerate().map(|(index, card)| {
      // The mirrored East plane runs its cards bottom to top, so its backs stack in reverse.
      let order = if style.seat == Some(Seat::East) {
        cards.len() - 1 - index
      } else {
        index
      };
      style.timing.child(
        self::child(*card, destinations, width, index, order),
        style.seat,
        index,
      )
    }))
}

fn child(
  card: VisibleCard,
  destinations: &Destinations,
  width: f64,
  index: usize,
  order: usize,
) -> world::LayoutChild {
  let rest = world::LayoutBox::new(1.0, CARD_HEIGHT);
  world::LayoutChild::new(
    destinations[&card.token].clone(),
    rest,
    CardSurface(
      card,
      Some(destinations[&card.token].clone()),
      order as i16,
      width,
    ),
  )
  .item(
    world::LayoutItem::new(card.token.id(), rest)
      .orientation(world::LayoutOrientation::Arrangement)
      .depth(-0.12 - index as f64 * CARD_STEP)
      .authored(LocalTransform {
        scale: Vector3::new(width, width, width),
        ..LocalTransform::default()
      }),
  )
}

fn plane(x: f64, z: f64) -> world::LayoutPlane {
  world::LayoutPlane::new(
    Vector3::new(x, 0.0, z),
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, 0.0, 1.0),
  )
}

fn oriented_plane(x: f64, z: f64, angle: f64, rise: f64) -> world::LayoutPlane {
  let (sin, cos) = angle.to_radians().sin_cos();
  let x_axis = Vector3::new(cos, 0.0, -sin);
  let y_axis = Vector3::new(sin, 0.0, cos);
  world::LayoutPlane::new(
    Vector3::new(
      x - 0.5 * cos - (0.5 + rise) * sin,
      0.0,
      z + 0.5 * sin - (0.5 + rise) * cos,
    ),
    x_axis,
    y_axis,
  )
}

// Curvature that keeps cards tangent to a circular fan turning `degrees` per card.
fn arc(spacing: f64, degrees: f64) -> f64 {
  spacing * degrees.to_radians() / 2.0
}

fn rise(count: usize, curvature: f64) -> f64 {
  (count.saturating_sub(1) as f64 / 2.0).powi(2) * curvature
}

fn pitch(degrees: f64) -> Quaternion {
  let half = degrees.to_radians() / 2.0;
  Quaternion::new(half.sin(), 0.0, 0.0, half.cos())
}
