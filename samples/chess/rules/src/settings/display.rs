//! Correlates a menu-owned display preview with host readback and confirmation.

use battlement::{
  CommandId,
  display::DisplayPreviewState,
  host_settings::{DisplayConfiguration, HostSettings},
};
use reactant::{app_context, hooks, prelude::*};

use crate::settings::{self, SettingsChange};

#[derive(Clone)]
pub struct DisplayControl {
  pub pending: bool,
  pub confirmable: bool,
  pub finishing: bool,
  pub remaining_seconds: u32,
  pub failed: bool,
  pub preview: EventCallback<DisplayConfiguration>,
  pub confirm: EventCallback<()>,
  pub cancel: EventCallback<()>,
}

pub fn use_display_control(host: &HostSettings) -> DisplayControl {
  let app = app_context::use_app();
  let settings = settings::use_settings();
  let (pending, set_pending) = hooks::use_state(None::<Pending>);
  let (failed, set_failed) = hooks::use_state(false);
  let active = hooks::use_ref(None::<CommandId>);
  hooks::use_effect(
    {
      let app = app.clone();
      let active = active.clone();
      move || {
        move || {
          if let Some(id) = active.get() {
            app.send(reactant::cancel_display(id));
          }
        }
      }
    },
    (),
  );
  hooks::use_effect(
    {
      let host = host.clone();
      let active = active.clone();
      let set_pending = set_pending.clone();
      let set_failed = set_failed.clone();
      move || {
        let Some(mut request) = pending else {
          return;
        };
        if let Some((operation, keeping)) = request.operation
          && let Some(result) = host
            .last_result
            .as_ref()
            .filter(|value| value.request_id == operation)
        {
          if host
            .display_preview
            .is_some_and(|preview| preview.request_id == operation)
          {
            return;
          }
          let applied = host
            .applied_display
            .filter(|value| self::matches(*value, request.target));
          let accepted = result.error.is_none() && host.display_preview.is_none();
          let accepted = accepted && (!keeping || applied.is_some());
          if keeping && accepted {
            settings.change(SettingsChange::Display(
              applied.expect("confirmed readback"),
            ));
          }
          set_failed.set(!accepted);
          active.with_mut(|value| *value = None);
          set_pending.set(None);
          return;
        }
        if let Some(preview) = host
          .display_preview
          .filter(|value| value.request_id == request.preview)
        {
          if !request.observed {
            request.observed = true;
            set_pending.set(Some(request));
          }
          if preview.state == DisplayPreviewState::Reverting {
            set_failed.set(true);
          }
        } else if request.observed
          || host
            .last_result
            .as_ref()
            .is_some_and(|result| result.request_id == request.preview && result.error.is_some())
        {
          active.with_mut(|value| *value = None);
          set_pending.set(None);
          set_failed.set(true);
        }
      }
    },
    (pending, host.clone()),
  );
  let preview = host
    .display_preview
    .filter(|preview| pending.is_some_and(|value| value.preview == preview.request_id));
  DisplayControl {
    pending: pending.is_some(),
    confirmable: preview.is_some_and(|value| value.state == DisplayPreviewState::Confirmable)
      && pending.is_some_and(|value| value.operation.is_none()),
    finishing: pending.is_some_and(|value| value.operation.is_some()),
    remaining_seconds: preview.map_or(15, |value| value.remaining_seconds),
    failed,
    preview: EventCallback::new({
      let app = app.clone();
      let active = active.clone();
      let set_pending = set_pending.clone();
      let set_failed = set_failed.clone();
      move |target| {
        let command = reactant::preview_display(target);
        active.with_mut(|value| *value = Some(command.command_id));
        set_pending.set(Some(Pending {
          preview: command.command_id,
          target,
          operation: None,
          observed: false,
        }));
        set_failed.set(false);
        app.send(command);
      }
    }),
    confirm: EventCallback::new({
      let app = app.clone();
      let set_pending = set_pending.clone();
      move |()| {
        if let Some(mut request) = pending.filter(|value| value.operation.is_none())
          && preview.is_some_and(|value| value.state == DisplayPreviewState::Confirmable)
        {
          let command = reactant::confirm_display(request.preview);
          request.operation = Some((command.command_id, true));
          set_pending.set(Some(request));
          app.send(command);
        }
      }
    }),
    cancel: EventCallback::new(move |()| {
      if let Some(mut request) = pending.filter(|value| value.operation.is_none()) {
        let command = reactant::cancel_display(request.preview);
        request.operation = Some((command.command_id, false));
        set_pending.set(Some(request));
        app.send(command);
      }
    }),
  }
}

#[derive(Clone, Copy, PartialEq)]
struct Pending {
  preview: CommandId,
  target: DisplayConfiguration,
  operation: Option<(CommandId, bool)>,
  observed: bool,
}

fn matches(actual: DisplayConfiguration, requested: DisplayConfiguration) -> bool {
  actual.mode == requested.mode
    && (actual.resolution.width, actual.resolution.height)
      == (requested.resolution.width, requested.resolution.height)
}
