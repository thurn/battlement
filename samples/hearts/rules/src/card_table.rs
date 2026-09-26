use std::collections::BTreeMap;

use battlement::{LocalTransform, Quaternion, Vector3};
use reactant::{hooks, native_host, prelude::*, world};
use uuid::Uuid;

use crate::{
  assets, card_assets, card_gesture,
  card_input::{self, CardInput},
  domain::Seat,
  projection::{CardToken, HumanView, VisibleCard},
};

const CARD_WIDTH: f64 = 4.31462;
const CARD_HEIGHT: f64 = 6.000022 / CARD_WIDTH;

type Destinations = BTreeMap<CardToken, world::LayoutDestination>;

pub(crate) struct CardTable {
  view: HumanView,
  aspect: f64,
  inspection: Option<VisibleCard>,
}

struct CardSurface(VisibleCard, Option<world::LayoutDestination>);

impl CardTable {
  pub(crate) fn new(view: &HumanView, aspect: f64) -> Self {
    Self {
      view: view.clone(),
      aspect,
      inspection: None,
    }
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
    let portrait = self.aspect < 1.0;
    let half_width = 5.7 * self.aspect;
    let hands: Vec<_> = Seat::ALL
      .into_iter()
      .map(|seat| {
        self::hand(
          &self.view.hands[seat.index()],
          seat,
          half_width,
          portrait,
          &destinations,
        )
      })
      .collect();
    let trick: Vec<_> = self
      .view
      .trick
      .iter()
      .map(|(seat, card)| {
        let spacing = if portrait { 0.65 } else { 1.25 };
        let (x, z) = match seat {
          Seat::South => (0.0, -0.45),
          Seat::West => (-spacing, 0.35),
          Seat::North => (0.0, 1.15),
          Seat::East => (spacing, 0.35),
        };
        self::fan(
          &[*card],
          &destinations,
          if portrait { 0.63 } else { 1.3 },
          0.0,
          0.0,
          0.0,
        )
        .plane(self::plane(x - 0.5, z - 0.5))
      })
      .collect();
    let piles: Vec<_> = Seat::ALL
      .into_iter()
      .map(|seat| {
        let (x, z) = match seat {
          Seat::South => (-half_width * 0.83, -1.3),
          Seat::West => (-half_width * 0.83, 3.15),
          Seat::North => (half_width * 0.83, 4.8),
          Seat::East => (half_width * 0.83, -1.3),
        };
        let width = if portrait { 0.42 } else { 0.65 };
        world::Pile::new()
          .extent((1.0, 1.0))
          .plane(self::plane(x - 0.5, z - 0.5))
          .step(0.012, 0.012)
          .children(
            self.view.captured[seat.index()]
              .iter()
              .enumerate()
              .map(|(index, card)| self::child(*card, &destinations, width, index)),
          )
      })
      .collect();
    let inspection = self.inspection.map(|card| {
      world::Group::new()
        .id(inspection_id)
        .position(Vector3::new(0.0, 1.0, 0.1))
        .rotation(self::pitch(90.0))
        .scale(Vector3::new(2.0, 2.0, 2.0))
        .child(CardSurface(card, None))
    });
    (hands, trick, piles, inspection)
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
    let selected = interactive
      .as_ref()
      .is_some_and(|input| input.selected(self.0.token));
    let mut hit = world::BoxHitRegion::new().size(Vector3::new(1.0, CARD_HEIGHT, 0.06));
    if let Some(input) = interactive {
      let token = self.0.token;
      let enabled = input.inspection().is_none();
      let activate = EventCallback::new(move |_| input.activate(token));
      hit = hit
        .capture_on_press(enabled)
        .events(handlers)
        .accessible_button(
          trox::ls(card_input::name(self.0.face.expect("owned face"))),
          activate.clone(),
        )
        .on_click(activate);
    }
    world::Group::new()
      .reference(reference)
      .position(Vector3::new(
        offset.x,
        offset.y + if selected { 0.18 } else { 0.0 },
        offset.z - if selected { 0.05 } else { 0.0 },
      ))
      .child((
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
        }),
        hit,
      ))
  }
}

fn hand(
  cards: &[VisibleCard],
  seat: Seat,
  half_width: f64,
  portrait: bool,
  destinations: &Destinations,
) -> Node {
  if seat == Seat::South {
    if portrait {
      let spacing = (half_width * 2.0 - 0.65) / 7.0;
      return Node::new(
        cards
          .chunks(7)
          .enumerate()
          .map(|(row, cards)| {
            self::fan(cards, destinations, spacing * 1.12, spacing, 0.0, 0.0)
              .plane(self::plane(-0.5, -3.0 - row as f64 * 1.55))
          })
          .collect::<Vec<_>>(),
      );
    }
    let spacing = ((half_width * 2.0 - 3.0) / 13.0).min(1.16);
    return Node::new(
      self::fan(cards, destinations, spacing * 2.0, spacing, 0.01, -2.3)
        .plane(self::plane(-0.5, -4.0 - self::rise(cards.len(), 0.01))),
    );
  }
  let (x, z, angle, width, spacing) = match seat {
    Seat::North => (
      0.0,
      3.6,
      180.0,
      if portrait { 0.72 } else { 1.65 },
      if portrait { 0.22 } else { 0.56 },
    ),
    Seat::West => (
      -half_width * if portrait { 0.7 } else { 0.60 },
      0.6,
      90.0,
      if portrait { 0.72 } else { 1.5 },
      if portrait { 0.17 } else { 0.32 },
    ),
    Seat::East => (
      half_width * if portrait { 0.7 } else { 0.60 },
      0.6,
      -90.0,
      if portrait { 0.72 } else { 1.5 },
      if portrait { 0.17 } else { 0.32 },
    ),
    Seat::South => unreachable!(),
  };
  Node::new(
    world::Group::new()
      .position(Vector3::new(x, 0.0, z))
      .rotation(self::yaw(angle))
      .child(
        self::fan(cards, destinations, width, spacing, 0.01, 2.5)
          .plane(self::plane(-0.5, -0.5 - self::rise(cards.len(), 0.01))),
      ),
  )
}

fn fan(
  cards: &[VisibleCard],
  destinations: &Destinations,
  width: f64,
  spacing: f64,
  curvature: f64,
  angle: f64,
) -> world::Fan {
  let spread = cards.len().saturating_sub(1) as f64;
  world::Fan::new()
    .extent((1.0, 1.0))
    .curve(spread * spacing, self::rise(cards.len(), curvature))
    .angle((spread * angle).to_radians())
    .children(
      cards
        .iter()
        .enumerate()
        .map(|(index, card)| self::child(*card, destinations, width, index)),
    )
}

fn child(
  card: VisibleCard,
  destinations: &Destinations,
  width: f64,
  index: usize,
) -> world::LayoutChild {
  let rest = world::LayoutBox::new(1.0, CARD_HEIGHT);
  world::LayoutChild::new(
    destinations[&card.token].clone(),
    rest,
    CardSurface(card, Some(destinations[&card.token].clone())),
  )
  .item(
    world::LayoutItem::new(card.token.id(), rest)
      .orientation(world::LayoutOrientation::Arrangement)
      .depth(-0.12 - index as f64 * 0.008)
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

fn rise(count: usize, curvature: f64) -> f64 {
  (count.saturating_sub(1) as f64 / 2.0).powi(2) * curvature
}

fn pitch(degrees: f64) -> Quaternion {
  let half = degrees.to_radians() / 2.0;
  Quaternion::new(half.sin(), 0.0, 0.0, half.cos())
}

fn yaw(degrees: f64) -> Quaternion {
  let half = degrees.to_radians() / 2.0;
  Quaternion::new(0.0, half.sin(), 0.0, half.cos())
}
