use reactant_rules::CancellationToken;

use crate::ai::{
  decision::Decision,
  policy,
  sampling::{self, SampleDeal},
};
use crate::domain::{
  AiObservation, HandResult, IgnorePresentation, Intention, Phase, RandomStream, Seat, simulation,
  transition,
};

/// Evaluates canonical candidates on common deals/seeds by mean end-of-hand penalty.
pub fn decide(
  observation: AiObservation,
  stream: RandomStream,
  token: CancellationToken,
) -> Decision {
  self::evaluate(observation, stream, || token.checkpoint())
}

/// Pure search boundary with cooperative checkpoints, also used by deterministic probes.
pub fn evaluate(
  observation: AiObservation,
  stream: RandomStream,
  mut checkpoint: impl FnMut(),
) -> Decision {
  let batch = sampling::sample_deals(&observation, stream, &mut checkpoint);
  let candidates = match observation.table.phase {
    Phase::Passing => {
      assert!(observation.pending_pass.is_none());
      let mut passes = policy::passes(&observation);
      passes.sort();
      passes
        .into_iter()
        .map(|cards| Intention::SubmitPass {
          seat: observation.seat,
          cards,
        })
        .collect::<Vec<_>>()
    }
    Phase::Playing { turn } => {
      assert_eq!(turn, observation.seat);
      let mut cards = observation.legal_plays.clone();
      cards.sort_unstable();
      cards
        .into_iter()
        .map(|card| Intention::PlayCard {
          seat: observation.seat,
          card,
        })
        .collect()
    }
    _ => panic!("search requested outside a decision phase"),
  };
  let intention = candidates
    .into_iter()
    .min_by_key(|candidate| {
      checkpoint();
      batch
        .deals
        .iter()
        .map(|deal| {
          u32::from(
            self::rollout(&observation, deal, candidate.clone(), &mut checkpoint).points
              [observation.seat.index()],
          )
        })
        .sum::<u32>()
    })
    .expect("actor has a legal candidate");
  checkpoint();
  Decision {
    observation,
    previous_stream: stream,
    next_stream: batch.next_stream,
    intention,
  }
}

/// Runs the same transitions and moon scoring as play; policy sees only each actor's view.
pub fn rollout(
  observation: &AiObservation,
  deal: &SampleDeal,
  candidate: Intention,
  mut checkpoint: impl FnMut(),
) -> HandResult {
  let mut state = simulation::from_observation(observation, deal.hands.clone());
  let mut stream = deal.rollout_stream;
  if state.phase() == Phase::Passing {
    for seat in Seat::ALL {
      checkpoint();
      let intention = if seat == observation.seat {
        candidate.clone()
      } else {
        Intention::SubmitPass {
          seat,
          cards: policy::passes(&state.observe(seat)).remove(0),
        }
      };
      transition::apply(&mut state, intention, &mut IgnorePresentation)
        .expect("sampled legal pass");
    }
  } else {
    checkpoint();
    transition::apply(&mut state, candidate, &mut IgnorePresentation)
      .expect("sampled legal candidate");
  }
  for _ in 0..52 {
    checkpoint();
    let Phase::Playing { turn } = state.phase() else {
      return state
        .public_table()
        .result
        .expect("rollout scored the hand");
    };
    let card = policy::play(&state.observe(turn), &mut stream);
    transition::apply(
      &mut state,
      Intention::PlayCard { seat: turn, card },
      &mut IgnorePresentation,
    )
    .expect("sampled legal play");
  }
  state
    .public_table()
    .result
    .expect("bounded rollout completed")
}
