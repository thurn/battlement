//! A synchronous computer player with explicit turn admission.
use cozy_chess::{Board, Move};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Selects computer moves without changing the game's synchronous execution model.
#[derive(Clone)]
pub struct Opponent(Kind);

#[derive(Clone)]
enum Kind {
  Search(Duration),
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
    Self(Kind::Search(budget))
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
  pub(crate) fn choose(&self, board: &Board) -> Move {
    match &self.0 {
      Kind::Search(budget) => {
        crate::ai::choose_move(board, *budget).expect("ongoing computer turn")
      }
      Kind::Scripted(s) => {
        let reply = s
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
  }
}
