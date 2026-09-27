use std::{array, collections::BTreeSet, iter, mem};

use crate::domain::{AiObservation, CardId, Hands, Phase, RandomStream, Seat, cards};

pub const SAMPLE_COUNT: usize = 32;

/// One plausible assignment and the common rollout stream used by every candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleDeal {
  pub hands: Hands,
  pub rollout_stream: RandomStream,
}

/// Detached search input; only an accepted decision may commit the resulting stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleBatch {
  pub deals: [SampleDeal; SAMPLE_COUNT],
  pub next_stream: RandomStream,
}

struct Constraints {
  fixed: Hands,
  unknown: Vec<CardId>,
  slots: Vec<Seat>,
  voids: [[bool; 4]; 4],
}

/// Samples from public constraints only. Matching is finite, not claimed uniform.
/// Checkpoint runs at every augmenting-path assignment boundary and may cancel work.
pub fn sample_deals(
  observation: &AiObservation,
  mut stream: RandomStream,
  mut checkpoint: impl FnMut(),
) -> SampleBatch {
  checkpoint();
  let constraints = Constraints::new(observation);
  let deals = array::from_fn(|_| {
    checkpoint();
    SampleDeal {
      hands: constraints.sample(&mut stream, &mut checkpoint),
      rollout_stream: stream.fork(),
    }
  });
  checkpoint();
  SampleBatch {
    deals,
    next_stream: stream,
  }
}

impl Constraints {
  fn new(observation: &AiObservation) -> Self {
    assert!(matches!(
      observation.table.phase,
      Phase::Passing | Phase::Playing { .. }
    ));
    let mut unseen: BTreeSet<_> = cards::deck().into_iter().collect();
    for played in &observation.table.history {
      assert!(unseen.remove(&played.card), "duplicate public card");
    }
    assert!(
      observation
        .table
        .history
        .ends_with(&observation.table.trick),
      "current trick must be the public history suffix"
    );
    assert_eq!(
      observation.table.hand_counts.iter().sum::<usize>(),
      unseen.len(),
      "public counts must account for every unplayed card"
    );
    assert_eq!(
      observation.hand.len(),
      observation.table.hand_counts[observation.seat.index()],
      "the acting hand must match its public count"
    );
    let mut fixed: Hands = array::from_fn(|_| Vec::new());
    for &card in &observation.hand {
      assert!(
        !fixed[observation.seat.index()].contains(&card),
        "duplicate owned card"
      );
      self::fix(card, observation.seat, &mut fixed, &mut unseen);
    }
    if let Some(known) = &observation.known_pass {
      let unique: BTreeSet<_> = known.cards.iter().copied().collect();
      assert_eq!(unique.len(), known.cards.len(), "duplicate known pass");
      for &card in &known.cards {
        self::fix(card, known.recipient, &mut fixed, &mut unseen);
      }
    }
    if observation.table.phase != Phase::Passing && observation.table.history.is_empty() {
      self::fix(
        CardId::TWO_OF_CLUBS,
        observation.table.leader.expect("opening leader is public"),
        &mut fixed,
        &mut unseen,
      );
    }
    let mut slots = Vec::new();
    for seat in Seat::ALL {
      let hand = &mut fixed[seat.index()];
      let count = observation.table.hand_counts[seat.index()];
      assert!(
        count <= 13 && hand.len() <= count,
        "invalid known hand capacity"
      );
      assert!(
        hand
          .iter()
          .all(|card| !observation.table.voids[seat.index()][card.suit.index()]),
        "known card contradicts a public void"
      );
      hand.sort_unstable();
      slots.extend(iter::repeat_n(seat, count - hand.len()));
    }
    assert_eq!(
      unseen.len(),
      slots.len(),
      "unknown cards must fill the remaining slots"
    );
    Self {
      fixed,
      unknown: unseen.into_iter().collect(),
      slots,
      voids: observation.table.voids,
    }
  }

  fn sample(&self, stream: &mut RandomStream, checkpoint: &mut impl FnMut()) -> Hands {
    let mut cards = self.unknown.clone();
    let mut slots = self.slots.clone();
    stream.shuffle(&mut cards);
    stream.shuffle(&mut slots);
    let allowed: Vec<Vec<_>> = cards
      .iter()
      .map(|card| {
        slots
          .iter()
          .enumerate()
          .filter_map(|(index, seat)| {
            (!self.voids[seat.index()][card.suit.index()]).then_some(index)
          })
          .collect()
      })
      .collect();
    let mut assigned = vec![None; slots.len()];
    for card in 0..cards.len() {
      let mut visited = vec![false; slots.len()];
      assert!(
        self::assign(card, &allowed, &mut assigned, &mut visited, checkpoint),
        "observation has no feasible hidden deal"
      );
    }
    let mut hands = self.fixed.clone();
    for (slot, card) in assigned.into_iter().enumerate() {
      hands[slots[slot].index()].push(cards[card.expect("every slot is assigned")]);
    }
    for hand in &mut hands {
      hand.sort_unstable();
    }
    hands
  }
}

fn fix(card: CardId, seat: Seat, fixed: &mut Hands, unseen: &mut BTreeSet<CardId>) {
  if let Some(owner) = Seat::ALL
    .into_iter()
    .find(|owner| fixed[owner.index()].contains(&card))
  {
    assert_eq!(owner, seat, "known owners conflict");
    return;
  }
  assert!(
    unseen.remove(&card),
    "known card was already publicly played"
  );
  fixed[seat.index()].push(card);
}

fn assign(
  card: usize,
  allowed: &[Vec<usize>],
  assigned: &mut [Option<usize>],
  visited: &mut [bool],
  checkpoint: &mut impl FnMut(),
) -> bool {
  checkpoint();
  for &slot in &allowed[card] {
    if mem::replace(&mut visited[slot], true) {
      continue;
    }
    let previous = assigned[slot];
    if previous.is_none_or(|owner| self::assign(owner, allowed, assigned, visited, checkpoint)) {
      assigned[slot] = Some(card);
      return true;
    }
  }
  false
}
