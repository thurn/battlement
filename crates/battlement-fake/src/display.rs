use std::mem;

use battlement::{
  CommandId,
  display::{DisplayCommand, DisplayPreview, DisplayPreviewState},
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
    HostSettingsResult, SettingAvailability,
  },
};

struct Pending {
  id: CommandId,
  prior: DisplayConfiguration,
  target: DisplayConfiguration,
  deadline_ms: u64,
}

#[derive(Default)]
pub(crate) struct DisplayFake {
  now_ms: u64,
  pending: Option<Pending>,
  pub(crate) fail_next_save: bool,
}

impl DisplayFake {
  pub(crate) fn execute(
    &mut self,
    id: CommandId,
    command: DisplayCommand,
    host: &mut HostSettings,
  ) -> bool {
    match command {
      DisplayCommand::Preview(target) => {
        if !self::supported(target, host) || host.applied_display.is_none() {
          self::result(
            host,
            id,
            Some("The requested display configuration is unavailable."),
          );
          return true;
        }
        if self.take_failure() {
          self::result(host, id, Some("Could not save display recovery."));
          return true;
        }
        let prior = self
          .pending
          .as_ref()
          .map(|p| p.prior)
          .unwrap_or_else(|| host.applied_display.expect("checked display"));
        self.pending = Some(Pending {
          id,
          prior,
          target,
          deadline_ms: self
            .now_ms
            .checked_add(15_000)
            .expect("fake host time overflow"),
        });
        host.applied_display = Some(target);
        self::result(host, id, None);
      }
      DisplayCommand::Confirm(preview) => {
        if !self.matches(preview) {
          return false;
        }
        if self.take_failure() {
          self.revert(host, id, Some("Could not keep the display."));
        } else {
          self.pending = None;
          self::result(host, id, None);
        }
      }
      DisplayCommand::Cancel(preview) => {
        if !self.matches(preview) {
          return false;
        }
        self.revert(host, id, None);
      }
    }
    self.publish(host);
    true
  }

  pub(crate) fn observe(&mut self, host: &mut HostSettings) {
    if let Some(pending) = &self.pending
      && (host.applied_display != Some(pending.target) || !self::supported(pending.target, host))
    {
      self.revert(
        host,
        pending.id,
        Some("The display changed before confirmation."),
      );
    }
    self.publish(host);
  }

  pub(crate) fn advance(&mut self, milliseconds: u64, host: &mut HostSettings) {
    self.now_ms = self
      .now_ms
      .checked_add(milliseconds)
      .expect("fake host time overflow");
    if let Some(pending) = &self.pending
      && self.now_ms >= pending.deadline_ms
    {
      self.revert(host, pending.id, Some("Display confirmation timed out."));
    }
    self.publish(host);
  }

  pub(crate) fn lose_owner(&mut self, host: &mut HostSettings) {
    if let Some(pending) = &self.pending {
      self.revert(
        host,
        pending.id,
        Some("The display preview lost its owner."),
      );
    }
    self.publish(host);
  }

  fn matches(&self, id: CommandId) -> bool {
    self
      .pending
      .as_ref()
      .is_some_and(|pending| pending.id == id)
  }

  fn take_failure(&mut self) -> bool {
    mem::take(&mut self.fail_next_save)
  }

  fn revert(&mut self, host: &mut HostSettings, id: CommandId, error: Option<&str>) {
    let prior = self.pending.take().expect("active display preview").prior;
    let recoverable = if prior.mode == DisplayMode::Windowed {
      self::window_fits(prior.resolution, host)
    } else {
      self::supported(prior, host)
    };
    let target = if recoverable {
      Some(prior)
    } else {
      host.window_bounds.map(|bounds| DisplayConfiguration {
        mode: DisplayMode::Windowed,
        resolution: DisplayResolution {
          width: (bounds.width.min(1600) * 4 / 5).max(1),
          height: (bounds.height.min(900) * 4 / 5).max(1),
          refresh_numerator: 0,
          refresh_denominator: 1,
        },
      })
    };
    if let Some(target) = target {
      host.applied_display = Some(target);
      self::result(host, id, error);
    } else {
      self::result(
        host,
        id,
        Some("Current monitor bounds are unavailable for recovery."),
      );
    }
  }

  fn publish(&self, host: &mut HostSettings) {
    host.display_preview = self.pending.as_ref().map(|pending| DisplayPreview {
      request_id: pending.id,
      state: DisplayPreviewState::Confirmable,
      remaining_seconds: u32::try_from(
        pending
          .deadline_ms
          .saturating_sub(self.now_ms)
          .div_ceil(1000),
      )
      .expect("bounded display deadline"),
    });
  }
}

fn window_fits(value: DisplayResolution, host: &HostSettings) -> bool {
  host
    .window_bounds
    .is_some_and(|bounds| value.width <= bounds.width && value.height <= bounds.height)
}

fn supported(value: DisplayConfiguration, host: &HostSettings) -> bool {
  if !matches!(host.platform, HostPlatform::MacOs | HostPlatform::Windows) {
    return false;
  }
  if host.display != SettingAvailability::Available || !host.display_modes.contains(&value.mode) {
    return false;
  }
  if value.mode == DisplayMode::Fullscreen && host.platform != HostPlatform::Windows {
    return false;
  }
  if value.mode == DisplayMode::Windowed && !self::window_fits(value.resolution, host) {
    return false;
  }
  host.resolutions.iter().any(|resolution| {
    if resolution.width != value.resolution.width || resolution.height != value.resolution.height {
      return false;
    }
    if value.mode != DisplayMode::Fullscreen || value.resolution.refresh_numerator == 0 {
      return true;
    }
    u64::from(resolution.refresh_numerator) * u64::from(value.resolution.refresh_denominator)
      == u64::from(value.resolution.refresh_numerator) * u64::from(resolution.refresh_denominator)
  })
}

fn result(host: &mut HostSettings, id: CommandId, error: Option<&str>) {
  host.last_result = Some(HostSettingsResult {
    request_id: id,
    error: error.map(str::to_owned),
  });
}
