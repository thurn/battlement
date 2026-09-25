use crate::{GameStatus, GameVersion, game_session::SessionData};
use reactant_rules::Game;

/// Native completion at one accepted session/revision boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationReceipt {
  /// Accepted session and revision whose presentation is observed.
  pub version: GameVersion,
  /// Native readiness of this owner.
  pub status: PresentationStatus,
}

/// Submission alone never produces Settled; cancelled/failed ownership stays terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationStatus {
  /// Rules, output submission or blocking native work remains.
  Pending,
  /// Accepted and presented revisions match and all blocking work completed.
  Settled,
  /// The current owner failed and requires recovery.
  Failed,
  /// The owner ended; late receipts cannot revive it.
  Cancelled,
}

impl PresentationReceipt {
  /// Whether gameplay may proceed at this receipt version.
  pub fn is_settled(self) -> bool {
    self.status == PresentationStatus::Settled
  }
}

pub(crate) fn observation<G: Game>(session: u64, data: &SessionData<G>) -> PresentationReceipt {
  let status = match data.status {
    GameStatus::Failed => PresentationStatus::Failed,
    GameStatus::Stopped => PresentationStatus::Cancelled,
    GameStatus::Busy => PresentationStatus::Pending,
    GameStatus::Ready => {
      let pending = data.pending_presentations != 0 || !data.blocking_motion.is_empty();
      if pending || data.rendered_revision != data.completed_actions {
        PresentationStatus::Pending
      } else {
        PresentationStatus::Settled
      }
    }
  };
  PresentationReceipt {
    version: GameVersion {
      session,
      revision: data.completed_actions,
    },
    status,
  }
}
