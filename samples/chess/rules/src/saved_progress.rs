//! Session quiescence and recoverable, durably acknowledged progress erasure.

use std::rc::Rc;

use cozy_chess::Board;
use reactant::{GameHandle, PersistentState, hooks};

use crate::{chess_ui_state::ChessUiController, persistence::SavedGame, reactant_game::ChessGame};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EraseStatus {
  Idle,
  Pending,
  Failed,
  Complete,
}

#[derive(Clone, PartialEq)]
pub(crate) struct SavedProgress {
  pub status: EraseStatus,
  current: hooks::Ref<EraseStatus>,
  active: hooks::Ref<Option<(u64, GameHandle<ChessGame>)>>,
  latest: hooks::Ref<Option<Board>>,
  recovery: Option<(u64, Board)>,
  erase: hooks::Callback<Rc<dyn Fn()>>,
  cancel: hooks::Callback<Rc<dyn Fn()>>,
  acknowledge: hooks::Callback<Rc<dyn Fn()>>,
}

#[derive(Clone)]
struct Phase {
  current: hooks::Ref<EraseStatus>,
  setter: hooks::StateSetter<EraseStatus>,
}

pub(crate) fn use_saved_progress() -> SavedProgress {
  hooks::use_required_context::<SavedProgress>()
}

pub(crate) fn use_progress_owner(
  persistence: Option<PersistentState<SavedGame>>,
  control: ChessUiController,
  initial: Option<Board>,
) -> SavedProgress {
  let hydrated = persistence.as_ref().is_none_or(PersistentState::hydrated);
  let (status, setter) = hooks::use_state(EraseStatus::Idle);
  let phase = Phase {
    current: hooks::use_ref(EraseStatus::Idle),
    setter,
  };
  let active = hooks::use_ref(None::<(u64, GameHandle<ChessGame>)>);
  let latest = hooks::use_ref(initial);
  let (recovery, set_recovery) = hooks::use_state(None::<(u64, Board)>);
  let erase: Rc<dyn Fn()> = Rc::new({
    let phase = phase.clone();
    let active = active.clone();
    let latest = latest.clone();
    let set_recovery = set_recovery.clone();
    let persistence = persistence.clone();
    let control = control.clone();
    move || {
      if phase.current.get() == EraseStatus::Pending {
        return;
      }
      if phase.current.get() != EraseStatus::Failed {
        control.begin_erasure();
        set_recovery.set(
          latest
            .get()
            .map(|board| (control.current().opening_generation, board)),
        );
        if let Some((_, game)) = active.get() {
          game.stop();
        }
      }
      phase.set(EraseStatus::Pending);
      if let Some(saved) = &persistence {
        saved.clear();
        if !saved.hydrated() {
          saved.retry();
        }
      } else {
        latest.replace(None);
        set_recovery.set(None);
        control.finish_erasure();
        phase.set(EraseStatus::Complete);
      }
    }
  });
  let cancel: Rc<dyn Fn()> = Rc::new({
    let phase = phase.clone();
    let recovery = recovery.clone();
    let persistence = persistence.clone();
    let control = control.clone();
    move || {
      if phase.current.get() != EraseStatus::Failed {
        return;
      }
      if let Some(saved) = &persistence
        && let Some((_, board)) = &recovery
      {
        saved.update(SavedGame::new(board));
      }
      control.cancel_erasure();
      phase.set(EraseStatus::Idle);
    }
  });
  let acknowledge: Rc<dyn Fn()> = Rc::new({
    let phase = phase.clone();
    move || {
      if phase.current.get() == EraseStatus::Complete {
        phase.set(EraseStatus::Idle);
      }
    }
  });
  let current = phase.current.clone();
  let completion = persistence.as_ref().map(|saved| {
    (
      saved.pending(),
      saved.desired().cloned(),
      saved.value().cloned(),
      saved.error().map(str::to_owned),
    )
  });
  hooks::use_effect(
    {
      let recovery = recovery.clone();
      let set_recovery = set_recovery.clone();
      let latest = latest.clone();
      let completion = completion.clone();
      move || {
        if phase.current.get() != EraseStatus::Pending {
          return;
        }
        let Some((pending, desired, durable, error)) = completion else {
          return;
        };
        if pending.is_some() || desired.is_some() {
          return;
        }
        if error.is_some() {
          if recovery.is_none() {
            set_recovery.set(
              durable
                .and_then(|saved| saved.board())
                .map(|board| (control.current().opening_generation, board)),
            );
          }
          phase.set(EraseStatus::Failed);
        } else if durable.is_none() {
          latest.replace(None);
          set_recovery.set(None);
          control.finish_erasure();
          phase.set(EraseStatus::Complete);
        }
      }
    },
    (status, completion.clone()),
  );
  SavedProgress {
    status,
    current,
    active,
    latest,
    recovery: recovery.clone(),
    erase: hooks::use_callback(erase, hydrated),
    cancel: hooks::use_callback(cancel, recovery),
    acknowledge: hooks::use_callback(acknowledge, ()),
  }
}

impl SavedProgress {
  pub fn pending(&self) -> bool {
    self.current.get() == EraseStatus::Pending
  }

  pub fn erase(&self) {
    (self.erase)();
  }

  pub fn cancel(&self) {
    (self.cancel)();
  }

  pub fn acknowledge(&self) {
    (self.acknowledge)();
  }

  pub fn recovery(&self, generation: u64) -> Option<Board> {
    self
      .recovery
      .as_ref()
      .and_then(|(owner, board)| (*owner == generation).then(|| board.clone()))
  }

  pub fn record(&self, board: Board) {
    self.latest.replace(Some(board));
  }

  pub fn use_session(&self, generation: u64, game: GameHandle<ChessGame>) {
    let active = self.active.clone();
    hooks::use_effect(
      move || {
        active.replace(Some((generation, game)));
        move || {
          if active.get().is_some_and(|(owner, _)| owner == generation) {
            active.replace(None);
          }
        }
      },
      generation,
    );
  }
}

impl Phase {
  fn set(&self, status: EraseStatus) {
    self.current.replace(status);
    self.setter.set(status);
  }
}
