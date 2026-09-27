use battlement_hearts_rules::domain::{
  HeartsState, IgnorePresentation, Intention, Phase, Seat, transition,
};

#[test]
fn every_accepted_boundary_reconstructs_through_passes_tricks_and_scores() {
  for hand_index in 0..4 {
    let deal = HeartsState::new(73 + u64::from(hand_index));
    let mut state = HeartsState::from_deal(
      std::array::from_fn(|seat| deal.hand(Seat::ALL[seat]).to_vec()),
      hand_index,
      [99; 4],
      deal.random().clone(),
    );
    loop {
      state.validate_saved().unwrap();
      let action = match state.phase() {
        Phase::Passing => {
          let seat = Seat::ALL
            .into_iter()
            .find(|seat| state.observe(*seat).pending_pass.is_none())
            .unwrap();
          Intention::SubmitPass {
            seat,
            cards: state.hand(seat)[..3].to_vec(),
          }
        }
        Phase::Playing { turn } => Intention::PlayCard {
          seat: turn,
          card: state.observe(turn).legal_plays[0],
        },
        Phase::HandOver | Phase::MatchOver { .. } => break,
      };
      transition::apply(&mut state, action, &mut IgnorePresentation).unwrap();
    }
    assert!(matches!(state.phase(), Phase::MatchOver { .. }));
  }
}

#[test]
fn malformed_saved_state_is_rejected_without_panicking() {
  let state = HeartsState::new(43);
  for field in ["hands", "totals", "hearts_broken", "hand_index"] {
    let mut json = serde_json::to_value(&state).unwrap();
    match field {
      "hands" => json[field][0][0] = json[field][1][0].clone(),
      "totals" => json[field][0] = 100.into(),
      "hearts_broken" => json[field] = true.into(),
      "hand_index" => json[field] = u32::MAX.into(),
      _ => unreachable!(),
    }
    let malformed: HeartsState = serde_json::from_value(json).unwrap();
    assert!(malformed.validate_saved().is_err(), "{field}");
  }
}
