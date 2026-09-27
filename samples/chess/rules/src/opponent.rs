//! Computer move selection with explicit scripted admission.
use crate::ai;
use cozy_chess::{Board, Move};
use reactant::rules::CancellationToken;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Selects computer moves independently of rules-action execution.
#[derive(Clone)]
pub struct Opponent(Kind);

type Search = dyn Fn(&Board, &CancellationToken) -> Move + Send + Sync;

#[derive(Clone)]
enum Kind {
  Search(Arc<Search>),
  Scripted(Arc<Mutex<Script>>),
}

#[derive(Default)]
struct Script {
  reply: Option<Move>,
  revision: u64,
}

impl Opponent {
  /// Automatically searches whenever the computer has a turn.
  pub fn search(budget: Duration) -> Self {
    Self::with_search(move |board, token| {
      ai::choose_move(board, budget, token).expect("ongoing computer turn")
    })
  }

  /// Supplies an off-thread search policy with cooperative cancellation.
  pub fn with_search(
    search: impl Fn(&Board, &CancellationToken) -> Move + Send + Sync + 'static,
  ) -> Self {
    Self(Kind::Search(Arc::new(search)))
  }

  pub(crate) fn searches(&self) -> bool {
    matches!(self.0, Kind::Search(_))
  }

  /// Holds computer turns until a caller supplies exactly one legal reply.
  pub fn scripted() -> Self {
    Self(Kind::Scripted(Arc::default()))
  }

  /// Permits one reply. The caller must reconcile the application afterward.
  pub fn reply_with(&self, reply: Move) {
    let Kind::Scripted(script) = &self.0 else {
      panic!("search opponent cannot accept a script")
    };
    let mut script = script.lock().unwrap();
    assert!(script.reply.is_none(), "unused computer reply");
    script.reply = Some(reply);
    script.revision += 1;
  }

  /// Verifies that the turn coordinator consumed the supplied reply.
  pub fn assert_reply_consumed(&self) {
    if let Kind::Scripted(s) = &self.0 {
      assert!(s.lock().unwrap().reply.is_none(), "unused computer reply");
    }
  }

  pub(crate) fn revision(&self) -> u64 {
    match &self.0 {
      Kind::Search(_) => 0,
      Kind::Scripted(s) => s.lock().unwrap().revision,
    }
  }

  pub(crate) fn permitted(&self) -> bool {
    match &self.0 {
      Kind::Search(_) => true,
      Kind::Scripted(s) => s.lock().unwrap().reply.is_some(),
    }
  }

  pub(crate) fn reset(&self) {
    if let Kind::Scripted(s) = &self.0 {
      s.lock().unwrap().reply = None;
    }
  }

  pub(crate) fn search_move(&self, board: &Board, token: &CancellationToken) -> Move {
    let Kind::Search(search) = &self.0 else {
      panic!("scripted replies do not search")
    };
    search(board, token)
  }

  pub(crate) fn scripted_reply(&self, board: &Board) -> Move {
    let Kind::Scripted(script) = &self.0 else {
      panic!("search opponent has no scripted reply")
    };
    let reply = script
      .lock()
      .unwrap()
      .reply
      .take()
      .expect("missing computer reply");
    assert!(
      board.is_legal(reply),
      "illegal scripted computer reply: {reply}"
    );
    reply
  }
}
