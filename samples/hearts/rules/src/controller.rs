use std::rc::Rc;

use reactant::{GameStatus, GameVersion, ReducerDispatch, ReducerHandle, Task, TaskState, hooks};
use reactant_rules::CancellationToken;

use crate::{
  ai::{decision::Decision, search},
  domain::{AiObservation, HeartsState, Intention, Phase, RandomStream, Rejection, Seat},
  projection::{CardToken, HumanView, PresentationSeed, Projection},
  reducer::{HeartsAction, HeartsReducer},
};

/// Application-owned controller; visual children receive its projected view and callbacks.
#[derive(Clone)]
pub struct HeartsController {
  pub game: ReducerHandle<HeartsReducer>,
  pub view: HumanView,
  pub computer: Task<Decision>,
  projection: Rc<Projection>,
  version: GameVersion,
  human: Seat,
  paused: bool,
}

#[derive(Clone, PartialEq)]
struct SearchKey {
  session: u64,
  observation: AiObservation,
  stream: RandomStream,
}

impl HeartsController {
  pub fn is_paused(&self) -> bool {
    self.paused
  }

  pub fn play(&self, token: CardToken) -> ReducerDispatch<Rejection> {
    let Some(card) = self
      .projection
      .resolve_owned(&self.game.presented().state, self.human, token)
    else {
      return ReducerDispatch::Rejected(Rejection::CardNotHeld);
    };
    self.send(Intention::PlayCard {
      seat: self.human,
      card,
    })
  }

  pub fn pass(&self, tokens: &[CardToken]) -> ReducerDispatch<Rejection> {
    let state = self.game.presented();
    let cards: Option<Vec<_>> = tokens
      .iter()
      .map(|token| {
        self
          .projection
          .resolve_owned(&state.state, self.human, *token)
      })
      .collect();
    let Some(cards) = cards else {
      return ReducerDispatch::Rejected(Rejection::CardNotHeld);
    };
    self.send(Intention::SubmitPass {
      seat: self.human,
      cards,
    })
  }

  pub fn next_hand(&self) -> ReducerDispatch<Rejection> {
    self.send(Intention::NextHand)
  }

  fn send(&self, intention: Intention) -> ReducerDispatch<Rejection> {
    if self.paused {
      return ReducerDispatch::Busy;
    }
    self
      .game
      .dispatch(self.version, HeartsAction::Human(intention))
  }
}

/// Composes reducer, private view and one observation-keyed deterministic opponent task.
pub fn use_hearts<K: hooks::Dependencies>(
  session_key: K,
  initialize: impl FnOnce() -> HeartsState + 'static,
  human: Seat,
  paused: bool,
) -> HeartsController {
  self::use_hearts_with_policy(session_key, initialize, human, paused, search::decide)
}

/// Runs an injected opponent with only its permitted observation and cancellation token.
pub fn use_hearts_with_policy<K: hooks::Dependencies>(
  session_key: K,
  initialize: impl FnOnce() -> HeartsState + 'static,
  human: Seat,
  paused: bool,
  policy: impl FnOnce(AiObservation, RandomStream, CancellationToken) -> Decision + Send + 'static,
) -> HeartsController {
  let game = reactant::use_game_reducer(session_key, initialize, HeartsReducer);
  let accepted = game.accepted();
  let decision = if paused || matches!(game.status(), GameStatus::Failed | GameStatus::Stopped) {
    None
  } else {
    self::next_observation(&accepted.state, human).map(|observation| SearchKey {
      session: accepted.version.session,
      stream: accepted.state.random().ai[observation.seat.index()],
      observation,
    })
  };
  let input = decision.clone();
  let computer = reactant::use_task(decision.clone(), move |token| {
    let input = input.expect("enabled decision");
    policy(input.observation, input.stream, token)
  });
  let result = computer.state();
  let dispatch = game.clone();
  let current = decision.clone();
  let completion = result.clone();
  hooks::use_effect(
    move || {
      let (Some(decision), TaskState::Ready(result)) = (current, completion) else {
        return;
      };
      let accepted = dispatch.accepted();
      if accepted.version.session != decision.session {
        return;
      }
      if accepted.state.observe(decision.observation.seat) != decision.observation {
        return;
      }
      if dispatch.presentation().is_settled() {
        dispatch.dispatch(
          accepted.version,
          HeartsAction::Computer(Box::new((*result).clone())),
        );
      }
    },
    (decision, result, game.presentation(), game.status()),
  );
  let presented = game.presented();
  let seed = hooks::use_optional_context::<PresentationSeed>();
  let projection = hooks::use_memo(
    move || {
      Rc::new(seed.map_or_else(Projection::default, |seed| {
        Projection::with_order_seed(seed.0)
      }))
    },
    presented.version.session,
  );
  HeartsController {
    view: projection.view(&presented.state, human),
    version: presented.version,
    game,
    computer,
    projection,
    human,
    paused,
  }
}

fn next_observation(state: &HeartsState, human: Seat) -> Option<AiObservation> {
  match state.phase() {
    Phase::Passing => Seat::ALL
      .into_iter()
      .filter(|seat| *seat != human)
      .map(|seat| state.observe(seat))
      .find(|observation| observation.pending_pass.is_none()),
    Phase::Playing { turn } if turn != human => Some(state.observe(turn)),
    _ => None,
  }
}
