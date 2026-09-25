//! Confirmation and recovery UI for local game-progress deletion.

use reactant::{announcement, hooks, portal::PortalTarget, prelude::*};
use trox::{LocalizedString, tx};

use crate::{
  menu::{arcade_frame_pulse::ArcadeScreen, arcade_modal::ArcadeModal, arcade_route_transition},
  saved_progress::{self, EraseStatus},
};

#[builder]
pub struct EraseDialog {
  open: bool,
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  on_close: EventCallback<()>,
}

impl Component for EraseDialog {
  fn render(&self) -> impl Render {
    let progress = saved_progress::use_saved_progress();
    let navigation = arcade_route_transition::use_arcade_navigation();
    let announce = announcement::use_announce();
    let description = self::description(progress.status);
    hooks::use_effect(
      {
        let progress = progress.clone();
        let navigation = navigation.clone();
        let description = description.clone();
        move || match progress.status {
          EraseStatus::Complete => {
            navigation.navigate(ArcadeScreen::Main);
            progress.acknowledge();
          }
          EraseStatus::Pending | EraseStatus::Failed => announce.send(description),
          EraseStatus::Idle => {}
        }
      },
      progress.status,
    );
    let close = EventCallback::new({
      let progress = progress.clone();
      move |()| progress.cancel()
    })
    .then(self.on_close.clone())
    .filter_map_input({
      let progress = progress.clone();
      move |()| (!progress.pending()).then_some(())
    });
    ArcadeModal::new()
      .open(self.open && progress.status != EraseStatus::Complete)
      .title(tx("Erase Saved Data?", "Saved-data confirmation title."))
      .children(Text::new(description))
      .confirm_label(match progress.status {
        EraseStatus::Pending => tx("Erasing…", "Pending saved-progress deletion."),
        EraseStatus::Failed => tx("Retry", "Retry the failed storage operation."),
        _ => tx("Erase", "Saved-data confirmation action."),
      })
      .cancel_label(tx("Cancel", "Cancel the current dialog."))
      .danger(true)
      .busy(progress.status == EraseStatus::Pending)
      .close_on_escape(progress.status != EraseStatus::Pending)
      .reduce_motion(navigation.reduce_motion)
      .on_confirm(move |()| progress.erase())
      .on_close(close)
      .overlay(self.overlay.clone())
  }
}

fn description(status: EraseStatus) -> LocalizedString {
  match status {
    EraseStatus::Pending => tx(
      "Erasing saved game progress…",
      "Progress deletion awaiting durable storage acknowledgement.",
    ),
    EraseStatus::Failed => tx(
      "Saved game progress could not be erased. Your game is still available. Retry or cancel.",
      "Recoverable saved-progress deletion failure.",
    ),
    _ => tx(
      "Saved game progress will be permanently erased. Your preferences and input bindings will be kept. This cannot be undone.",
      "Saved-progress confirmation warning; preferences are preserved.",
    ),
  }
}
