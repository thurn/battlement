use std::rc::Rc;

use battlement::{PanelPoint, ScreenSize};
use reactant::{GameStatus, GameVersion, ReducerDispatch, TaskState, hooks};

use crate::{
  controller::HeartsController,
  domain::{CardId, Phase},
  projection::{CardToken, VisibleCard},
};

#[derive(Clone, PartialEq)]
struct Selection {
  epoch: (u64, u32, bool),
  cards: Vec<CardToken>,
  inspection: Option<CardToken>,
}

#[derive(Clone)]
pub(crate) struct CardInput(Rc<Input>);

struct Input {
  game: HeartsController,
  selection: Selection,
  setter: hooks::StateSetter<Selection>,
  viewport: ScreenSize,
  version: GameVersion,
}

impl PartialEq for CardInput {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.0, &other.0)
  }
}

pub(crate) fn use_card_input(game: HeartsController, viewport: ScreenSize) -> CardInput {
  let epoch = (
    game.game.presented().version.session,
    game.view.table.hand_index,
    game.view.table.phase == Phase::Passing,
  );
  let empty = Selection {
    epoch,
    cards: Vec::new(),
    inspection: None,
  };
  let (stored, setter) = hooks::use_state(empty.clone());
  let mut selection = if stored.epoch == epoch {
    stored.clone()
  } else {
    empty
  };
  let owns = |token: &CardToken| {
    game.view.hands[game.view.seat.index()]
      .iter()
      .any(|card| card.token == *token)
  };
  selection.cards.retain(owns);
  selection.inspection = selection.inspection.filter(owns);
  let clean = selection.clone();
  let update = setter.clone();
  hooks::use_effect(
    move || {
      if stored != clean {
        update.set(clean);
      }
    },
    (epoch, game.view.hands[game.view.seat.index()].clone()),
  );
  let version = game.game.presented().version;
  CardInput(Rc::new(Input {
    game,
    selection,
    setter,
    viewport,
    version,
  }))
}

pub(crate) fn name(card: CardId) -> String {
  format!("{:?} of {:?}", card.rank, card.suit)
}

impl CardInput {
  pub(crate) fn version(&self) -> GameVersion {
    self.0.version
  }
  pub(crate) fn owns(&self, token: CardToken) -> bool {
    self.0.game.view.hands[self.0.game.view.seat.index()]
      .iter()
      .any(|card| card.token == token)
  }

  pub(crate) fn selected(&self, token: CardToken) -> bool {
    self.0.selection.cards.contains(&token)
  }

  pub(crate) fn selected_cards(&self) -> &[CardToken] {
    &self.0.selection.cards
  }

  pub(crate) fn inspection(&self) -> Option<VisibleCard> {
    self
      .0
      .selection
      .inspection
      .and_then(|token| self.card(token))
  }

  pub(crate) fn card(&self, token: CardToken) -> Option<VisibleCard> {
    self.0.game.view.hands[self.0.game.view.seat.index()]
      .iter()
      .find(|card| card.token == token)
      .copied()
  }

  pub(crate) fn reason(&self, token: CardToken) -> Option<String> {
    self
      .0
      .game
      .view
      .rejections
      .iter()
      .find(|(card, _)| *card == token)
      .map(|(_, reason)| reason.to_string())
  }

  pub(crate) fn phase(&self) -> Phase {
    self.0.game.view.table.phase
  }

  pub(crate) fn passing(&self) -> bool {
    self.0.game.view.table.phase == Phase::Passing
  }

  pub(crate) fn enabled(&self) -> bool {
    if !matches!(self.0.game.computer.state(), TaskState::Idle) {
      return false;
    }
    self.0.game.game.status() == GameStatus::Ready && self.0.game.game.presentation().is_settled()
  }

  pub(crate) fn table_enabled(&self) -> bool {
    self.enabled() && self.0.selection.inspection.is_none()
  }

  pub(crate) fn can_pass(&self) -> bool {
    if !self.passing() || self.0.game.view.pending_pass.is_some() {
      return false;
    }
    self.0.selection.cards.len() == 3 && self.table_enabled()
  }

  pub(crate) fn can_play(&self, token: CardToken) -> bool {
    self.table_enabled() && self.0.game.view.legal_plays.contains(&token)
  }

  pub(crate) fn activate(&self, token: CardToken) {
    if !self.table_enabled() || !self.owns(token) {
      return;
    }
    let mut next = self.0.selection.clone();
    if self.passing() {
      if self.0.game.view.pending_pass.is_some() {
        return;
      }
      if next.cards.contains(&token) {
        next.cards.retain(|card| *card != token);
      } else if next.cards.len() < 3 {
        next.cards.push(token);
      }
    } else if next.cards == [token] && self.can_play(token) {
      self.play(token);
      return;
    } else {
      next.cards = vec![token];
    }
    self.0.setter.set(next);
  }

  pub(crate) fn pass(&self) {
    if self.can_pass() && self.0.game.pass(&self.0.selection.cards) == ReducerDispatch::Started {
      self.clear();
    }
  }

  pub(crate) fn play(&self, token: CardToken) {
    if self.can_play(token) && self.0.game.play(token) == ReducerDispatch::Started {
      self.clear();
    }
  }

  pub(crate) fn drop_card(&self, token: CardToken, point: PanelPoint) {
    if !self.table_enabled() {
      return;
    }
    let x = point.x / f64::from(self.0.viewport.width);
    let y = point.y / f64::from(self.0.viewport.height);
    if (0.32..=0.68).contains(&x) && (0.32..=0.57).contains(&y) {
      self.play(token);
    }
  }

  pub(crate) fn inspect(&self) {
    if let Some(&token) = self.0.selection.cards.last() {
      let mut next = self.0.selection.clone();
      next.inspection = Some(token);
      self.0.setter.set(next);
    }
  }

  pub(crate) fn dismiss(&self) {
    let mut next = self.0.selection.clone();
    next.inspection = None;
    self.0.setter.set(next);
  }

  fn clear(&self) {
    self.0.setter.set(Selection {
      cards: Vec::new(),
      inspection: None,
      ..self.0.selection.clone()
    });
  }
}
