use std::{cell::RefCell, rc::Rc};

use reactant_core::hooks;
use reactant_rules::{GameReducer, ReducerContext, ReducerGame};

use crate::{
  DispatchResult, GameHandle, GameStatus, PresentationReceipt, game_app, game_hooks,
  game_presentation,
};

/// Identifies an accepted revision within one application-owned game session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameVersion {
  /// Application-owned session identity; replacement creates a new identity.
  pub session: u64,
  /// Number of final action outputs accepted for this session.
  pub revision: u64,
}

/// Guarded admission; no result except Started admits any work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReducerDispatch<E> {
  /// One action entered the existing rules worker.
  Started,
  /// Rules, blocking presentation or cleanup still owns admission.
  Busy,
  /// The callback names an old session/revision or a stopped session.
  Stale,
  /// The ready accepted state rejected this intention.
  Rejected(E),
  /// The current session needs explicit recovery after a failure.
  Failed,
}

/// Immutable rules snapshot with an explicit session/revision boundary.
pub struct ReducerSnapshot<S> {
  /// Revision represented by this snapshot.
  pub version: GameVersion,
  /// Read-only state; callers must not mutate interior logical data.
  pub state: Rc<S>,
}

/// A stable handle to one reducer session, safe for delayed input callbacks.
pub struct ReducerHandle<R: GameReducer> {
  game: GameHandle<ReducerGame<R>>,
  recover: Rc<dyn Fn() -> bool>,
}

impl<R: GameReducer> Clone for ReducerHandle<R> {
  fn clone(&self) -> Self {
    Self {
      game: self.game.clone(),
      recover: self.recover.clone(),
    }
  }
}

impl<R: GameReducer> PartialEq for ReducerHandle<R> {
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.game.session, &other.game.session)
  }
}

impl<R: GameReducer> Eq for ReducerHandle<R> {}

impl<R: GameReducer> ReducerHandle<R> {
  /// Returns accepted logical state, including after presentation failure.
  pub fn accepted(&self) -> ReducerSnapshot<R::State> {
    let data = self.game.session.data.borrow();
    ReducerSnapshot {
      version: GameVersion {
        session: self.game.session.id,
        revision: data.completed_actions,
      },
      state: Rc::clone(&data.accepted_view),
    }
  }

  /// Returns the current rendering snapshot; submission is not native completion.
  pub fn presented(&self) -> ReducerSnapshot<R::State> {
    let data = self.game.session.data.borrow();
    ReducerSnapshot {
      version: GameVersion {
        session: self.game.session.id,
        revision: data.rendered_revision,
      },
      state: Rc::clone(&data.rendered),
    }
  }

  /// Reports rules readiness independently of native presentation completion.
  pub fn status(&self) -> GameStatus {
    self.game.status()
  }

  /// Observes successful completion of this revision's blocking native presentation.
  pub fn presentation(&self) -> PresentationReceipt {
    self.game.session.refresh();
    game_presentation::observation(self.game.session.id, &self.game.session.data.borrow())
  }

  /// Replaces a failed current session from accepted state without replaying events.
  /// Returns false for stale handles, duplicate recovery or a healthy session.
  pub fn recover(&self) -> bool {
    (self.recover)()
  }

  /// Returns diagnostic detail when explicit recovery is needed.
  pub fn diagnostic(&self) -> Option<String> {
    self.game.diagnostic()
  }

  /// Rejects stale, busy and illegal UI intentions without panicking or queueing.
  pub fn dispatch(
    &self,
    expected: GameVersion,
    action: R::Action,
  ) -> ReducerDispatch<R::Rejection> {
    self.game.session.refresh();
    let data = self.game.session.data.borrow();
    let current = GameVersion {
      session: self.game.session.id,
      revision: data.completed_actions,
    };
    if expected != current || data.status == GameStatus::Stopped {
      return ReducerDispatch::Stale;
    }
    match data.status {
      GameStatus::Busy => return ReducerDispatch::Busy,
      GameStatus::Failed => return ReducerDispatch::Failed,
      GameStatus::Ready => {}
      GameStatus::Stopped => unreachable!(),
    }
    if !game_presentation::observation(self.game.session.id, &data).is_settled() {
      return ReducerDispatch::Busy;
    }
    if let Err(reason) = R::validate(&data.accepted, &action) {
      return ReducerDispatch::Rejected(reason);
    }
    drop(data);
    match self.game.dispatch(action) {
      DispatchResult::Started => ReducerDispatch::Started,
      DispatchResult::Busy => ReducerDispatch::Busy,
    }
  }
}

/// Mounts a reducer with initialization once per session key, using existing workers.
/// The handle remains stable across renders; changing the key invalidates old callbacks.
pub fn use_game_reducer<R, K>(
  session_key: K,
  initialize: impl FnOnce() -> R::State + 'static,
  reducer: R,
) -> ReducerHandle<R>
where
  R: GameReducer,
  K: hooks::Dependencies,
{
  let recovery = hooks::use_memo(
    || Rc::new(RefCell::new(None::<(R::State, u64)>)),
    session_key.clone(),
  );
  let (generation, restart) = hooks::use_state(0_u64);
  let revision = recovery
    .borrow()
    .as_ref()
    .map_or(0, |(_, revision)| *revision);
  let initial = recovery.clone();
  let game = game_app::use_game_versioned(
    (session_key, generation),
    move || {
      initial
        .borrow()
        .as_ref()
        .map_or_else(initialize, |(state, _)| state.clone())
    },
    move |connection| ReducerContext::new(reducer, connection),
    revision,
  );
  let session = Rc::downgrade(&game.session);
  let recover = hooks::use_memo(
    move || {
      Rc::new(move || {
        let Some(session) = session.upgrade() else {
          return false;
        };
        let current = session
          .app
          .upgrade()
          .and_then(|app| app.game::<ReducerGame<R>>());
        if !current.is_some_and(|game| game.session.id == session.id) {
          return false;
        }
        let data = session.data.borrow();
        if data.status != GameStatus::Failed {
          return false;
        }
        *recovery.borrow_mut() = Some((data.accepted.clone(), data.completed_actions));
        drop(data);
        session.stop();
        restart.update(|generation| {
          generation
            .checked_add(1)
            .expect("recovery generation overflow")
        });
        true
      }) as Rc<dyn Fn() -> bool>
    },
    game.session.id,
  );
  ReducerHandle { game, recover }
}

/// Selects the presented snapshot inside the attached reducer's game subtree.
pub fn use_reducer_selector<R: GameReducer, V: Clone + PartialEq + 'static>(
  select: impl Fn(&R::State) -> V + 'static,
) -> V {
  game_hooks::use_game_selector::<ReducerGame<R>, V>(select)
}
