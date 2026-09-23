//! Completion follows queued output, finite presentation, and worker notifications.
//! Assertions never control progress. Inline applications reject engine polling.
use crate::Display;
use battlement::Connect;
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient, time::ManualClock};
use reactant::{ApplicationEngine, GameStatus, rules::Game};
use std::{
  sync::Arc,
  time::{Duration, Instant},
};

/// The rules boundary reached after input and its finite presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionResult {
  /// All admitted output and finite presentation completed.
  Completed,
  /// The real worker is blocked on a displayed human choice.
  AwaitingInput,
  /// Input did not admit a rules action.
  NoAction,
}

impl Display<ApplicationEngine> {
  /// Connects either rules executor to the same input and observation API.
  /// Inline rules run entirely on the caller. Real workers wait on notifications;
  /// their five-second deadline diagnoses a stuck worker, never drives simulation.
  pub fn connect_application<G: Game>(
    factory: impl FnOnce(ManualClock) -> ApplicationEngine,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    let (client, _) = FakeClient::connect_with_clocked(factory, assets, connect);
    let mut display = Self::from_client(client);
    display.drive = self::drive_application::<G>;
    display.settle();
    display
  }

  /// Performs input and completes its presentation before returning admission status.
  pub fn action(&mut self, input: impl FnOnce(&mut Self)) -> ActionResult {
    let before = self.client.engine_mut().action_count();
    let answering = self.boundary == ActionResult::AwaitingInput;
    input(self);
    let result = self.settle();
    let admitted = self.client.engine_mut().action_count() != before;
    if result == ActionResult::Completed && !(answering || admitted) {
      return ActionResult::NoAction;
    }
    result
  }

  /// Reconciles an explicitly changed injected dependency and completes its output.
  pub fn refresh(&mut self) {
    self.client.engine_mut().invalidate_inline();
    self.settle();
  }
}

fn drive_application<G: Game>(display: &mut Display, target: Option<Duration>) -> ActionResult {
  let inline = display.client.engine_mut().rules_are_inline();
  let deadline = (!inline).then(|| Instant::now() + Duration::from_secs(5));
  loop {
    if inline {
      if let Some(response) = display.client.engine_mut().take_inline_output() {
        display.client.receive(response);
        continue;
      }
    } else {
      display.flush();
    }
    let presentation = display.client.next_presentation_in();
    // A worker publication belongs to the current instant. Receive it before
    // jumping to a future timer; otherwise thread scheduling changes virtual time.
    let boundary = if !inline && presentation.is_none() {
      let Some(boundary) = self::worker_boundary::<G>(display, deadline.unwrap()) else {
        continue;
      };
      boundary
    } else {
      ActionResult::Completed
    };
    let remaining = target.map(|t| t.saturating_sub(display.presentation_time()));
    if remaining == Some(Duration::ZERO) {
      return boundary;
    }
    if let Some(delay) = remaining.or(presentation) {
      let timer = display
        .client
        .engine_mut()
        .next_timer_due_in()
        .unwrap_or(delay);
      display
        .client
        .advance_time(delay.min(timer).min(presentation.unwrap_or(delay)));
      if inline {
        display.client.engine_mut().fire_inline_timers();
      }
      continue;
    }
    if inline {
      display.client.engine_mut().assert_inline_complete();
      return ActionResult::Completed;
    }
    return boundary;
  }
}

fn worker_boundary<G: Game>(display: &mut Display, deadline: Instant) -> Option<ActionResult> {
  let Some(game) = display.client.engine_mut().game::<G>() else {
    return Some(ActionResult::Completed);
  };
  if game.waiting_for_input() {
    return Some(ActionResult::AwaitingInput);
  }
  match game.status() {
    GameStatus::Ready => Some(ActionResult::Completed),
    GameStatus::Failed => panic!("game failed: {:?}", game.diagnostic()),
    GameStatus::Stopped => panic!("game stopped before completing its action"),
    GameStatus::Busy => {
      assert!(
        display.wait_for_game_output::<G>(deadline.saturating_duration_since(Instant::now())),
        "rules worker did not publish output within five seconds"
      );
      None
    }
  }
}
