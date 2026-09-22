//! Finite synchronous scenarios share real input, rendering, and wire decoding.

use crate::Display;
use battlement::Connect;
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient, time::ManualClock};
use std::{sync::Arc, time::Duration};

/// Whether input admitted a rules action; display-only changes are not a move.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InlineActionResult {
  /// All admitted action output and finite presentation completed successfully.
  Completed,
  /// No game action was admitted; display-local input effects still completed.
  NoAction,
}

impl Display<reactant::ApplicationEngine> {
  /// Performs visible input and verifies admission separately from rendered assertions.
  pub fn action_inline(&mut self, input: impl FnOnce(&mut Self)) -> InlineActionResult {
    let before = self.client.engine_mut().inline_action_count();
    input(self);
    self.finish_inline();
    if self.client.engine_mut().inline_action_count() == before {
      InlineActionResult::NoAction
    } else {
      InlineActionResult::Completed
    }
  }

  /// Connects an inline application without calling the engine's polling API.
  pub fn connect_inline(
    factory: impl FnOnce(ManualClock) -> reactant::ApplicationEngine,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    let (client, _) = FakeClient::connect_with_clocked(factory, assets, connect);
    let mut display = Self { client };
    display.finish_inline();
    display
  }

  /// Applies all known output and finite presentation, without assertion predicates.
  pub fn finish_inline(&mut self) {
    for _ in 0..100_000 {
      if let Some(response) = self.client.engine_mut().take_inline_output() {
        self.client.receive(response);
        continue;
      }
      let Some(presentation) = self.client.next_presentation_in() else {
        self.client.engine_mut().assert_inline_complete();
        return;
      };
      let delay = self
        .client
        .engine_mut()
        .next_timer_due_in()
        .map_or(presentation, |timer| timer.min(presentation));
      self.client.advance_time(delay);
      self.client.engine_mut().fire_inline_timers();
    }
    panic!("inline scenario exceeded 100,000 concrete operations");
  }

  /// Reconciles an injected dependency change, such as one permitted opponent turn.
  pub fn refresh_inline(&mut self) {
    self.client.engine_mut().invalidate_inline();
    self.finish_inline();
  }

  /// Advances a chosen amount of virtual time, visiting intervening app timers.
  pub fn advance_inline(&mut self, duration: Duration) {
    let target = self.presentation_time() + duration;
    for _ in 0..100_000 {
      if let Some(response) = self.client.engine_mut().take_inline_output() {
        self.client.receive(response);
        continue;
      }
      let remaining = target.saturating_sub(self.presentation_time());
      if remaining.is_zero() {
        return;
      }
      let timer = self
        .client
        .engine_mut()
        .next_timer_due_in()
        .unwrap_or(remaining);
      let presentation = self.client.next_presentation_in().unwrap_or(remaining);
      self
        .client
        .advance_time(remaining.min(timer).min(presentation));
      self.client.engine_mut().fire_inline_timers();
    }
    panic!("inline clock exceeded 100,000 concrete operations");
  }
}
