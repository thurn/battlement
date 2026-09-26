//! Conflict-safe binding capture with a pointer-accessible cancel action.
use crate::menu::{arcade_modal::ArcadeModal, input_labels, input_styles};
use crate::settings::{
  self, SettingsChange, SettingsContext,
  bindings::{self, Bindings, ControllerBinding},
};
use battlement::{
  Align, AnimationDirection, AnimationIterations, ControllerButton, FlexDirection,
  InputCaptureDevice, InputCaptureResult, PhysicalKey, SemanticRole, Style,
};
use reactant::{
  announcement::{self, Announce},
  hooks, motion_config,
  portal::PortalTarget,
  prelude::*,
  semantics::{SemanticName, SemanticProps},
};
use std::rc::Rc;
use trox::{LocalizedString, opaque, tx, tx_args, txa};

#[derive(Clone, Copy, PartialEq)]
pub struct CaptureTarget {
  pub index: usize,
  pub device: InputCaptureDevice,
}

/// Captures one binding while leaving Reset and Cancel reachable by pointer.
#[builder]
pub struct InputCaptureDialog {
  #[builder(required)]
  target: CaptureTarget,
  #[builder(required)]
  overlay: PortalTarget,
  #[builder(required)]
  restore_focus: ElementRef,
  #[builder(required)]
  set_capture: hooks::StateSetter<Option<CaptureTarget>>,
}

#[builder]
struct CaptureListener {
  #[builder(required)]
  device: InputCaptureDevice,
  #[builder(required)]
  on_result: Rc<dyn Fn(InputCaptureResult)>,
}

impl Component for CaptureListener {
  fn render(&self) -> impl Render {
    let on_result = self.on_result.clone();
    reactant::use_input_capture(self.device, move |result| on_result(result));
  }
}

impl Component for InputCaptureDialog {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings();
    let reduced = motion_config::use_reduced_motion();
    let (status, set_status) = hooks::use_state(None::<LocalizedString>);
    let (attempt, set_attempt) = hooks::use_state(0_u64);
    let focus = use_element_ref();
    let announce = announcement::use_announce();
    let scale = settings.desired.text_size.factor();
    let target = self.target;
    let session = CaptureSession {
      target,
      settings: settings.clone(),
      announce,
      set_capture: self.set_capture.clone(),
      set_status,
    };
    (
      CaptureListener::new()
        .device(target.device)
        .on_result({
          let session = session.clone();
          move |result| {
            let outcome = self::apply(result, session.target, &session.settings, session.announce);
            let retry = outcome.is_err();
            session.finish(outcome);
            if retry {
              set_attempt.update(|value| value + 1);
            }
          }
        })
        .key(attempt),
      ArcadeModal::new()
        .open(true)
        .title(tx("Change Shortcut", "Keyboard shortcut dialog title."))
        .children(
          View::new()
            .style(
              Style::new()
                .width(650)
                .flex_direction(FlexDirection::Column)
                .align_items(Align::Center),
            )
            .child((
              Text::new(match target.device {
                InputCaptureDevice::Keyboard => txa(
                  "Press a key for {action}",
                  tx_args![action => opaque(input_labels::action(target.index))],
                  "Keyboard capture prompt.",
                ),
                InputCaptureDevice::Controller => txa(
                  "Press a controller button for {action}",
                  tx_args![action => opaque(input_labels::action(target.index))],
                  "Controller capture prompt.",
                ),
              })
              .style(input_styles::capture_prompt_style(scale)),
              Label::new(trox::ls("●"))
                .name("shortcut-waiting-marker")
                .element_ref(focus.clone())
                .focusable(true)
                .tab_index(0)
                .semantic(
                  SemanticProps::new(SemanticRole::StaticText).name(SemanticName::Text(
                    match target.device {
                      InputCaptureDevice::Keyboard => tx(
                        "Waiting for keyboard input",
                        "Keyboard shortcut capture status.",
                      ),
                      InputCaptureDevice::Controller => tx(
                        "Waiting for controller input",
                        "Controller shortcut capture status.",
                      ),
                    },
                  )),
                )
                .style(input_styles::waiting_marker_style(scale))
                .animations((!reduced).then(|| {
                  Animation::new(Keyframes::new([
                    StyleTarget::new().opacity(1.0),
                    StyleTarget::new().opacity(0.22),
                  ]))
                  .duration_secs(0.72)
                  .iterations(AnimationIterations::Forever)
                  .direction(AnimationDirection::Alternate)
                  .animation_key("shortcut-waiting-blink")
                })),
              status.map(|message| {
                Text::new(message)
                  .host_name("shortcut-status")
                  .style(input_styles::status_style(scale))
              }),
            )),
        )
        .confirm_label(tx("Reset", "Reset keyboard shortcut action."))
        .cancel_label(tx("Cancel", "Cancel the current dialog."))
        .close_on_escape(false)
        .reduce_motion(reduced)
        .initial_focus(focus)
        .restore_focus(self.restore_focus.clone())
        .on_confirm(EventCallback::new(move |()| {
          let outcome = match target.device {
            InputCaptureDevice::Keyboard => self::apply(
              InputCaptureResult::Key {
                device_id: 0,
                key: Bindings::<PhysicalKey>::default().values()[target.index],
              },
              target,
              &session.settings,
              announce,
            ),
            InputCaptureDevice::Controller => self::assign_controller(
              Bindings::<ControllerBinding>::default().values()[target.index],
              target.index,
              &session.settings,
              announce,
            ),
          };
          session.finish(outcome);
        }))
        .on_close(self.set_capture.callback().map_input(|_| None))
        .overlay(self.overlay.clone()),
    )
  }
}

#[derive(Clone)]
struct CaptureSession {
  target: CaptureTarget,
  settings: SettingsContext,
  announce: Announce,
  set_capture: hooks::StateSetter<Option<CaptureTarget>>,
  set_status: hooks::StateSetter<Option<LocalizedString>>,
}

impl CaptureSession {
  fn finish(&self, result: Result<(), LocalizedString>) {
    match result {
      Ok(()) => self.set_capture.set(None),
      Err(message) => {
        self.announce.send(message.clone());
        self.set_status.set(Some(message));
      }
    }
  }
}

fn apply(
  result: InputCaptureResult,
  target: CaptureTarget,
  settings: &SettingsContext,
  announce: Announce,
) -> Result<(), LocalizedString> {
  match result {
    InputCaptureResult::Key {
      key: PhysicalKey::Escape,
      ..
    } if target.index != 5 => Ok(()),
    InputCaptureResult::Key { key, .. } => {
      let mut values = settings.desired.keyboard.values();
      self::check_conflict(&values, key, target.index)?;
      values[target.index] = key;
      let updated = Bindings::from_values(values);
      if !bindings::valid_keyboard(updated) {
        return Err(tx(
          "Escape is reserved for Pause",
          "Reserved keyboard shortcut error.",
        ));
      }
      settings.change(SettingsChange::Keyboard(updated));
      self::accepted(target.index, input_labels::key(key), announce);
      Ok(())
    }
    InputCaptureResult::Button(input) if input.button == ControllerButton::East => Ok(()),
    InputCaptureResult::Button(input) => {
      let binding = ControllerBinding::from_button(input.button).ok_or_else(|| {
        tx(
          "Choose a button or D-pad direction",
          "Unsupported controller binding error.",
        )
      })?;
      self::assign_controller(binding, target.index, settings, announce)
    }
    InputCaptureResult::Direction(input) => self::assign_controller(
      ControllerBinding::from_direction(input.direction),
      target.index,
      settings,
      announce,
    ),
    InputCaptureResult::Cancelled(_) => Ok(()),
  }
}

fn assign_controller(
  binding: ControllerBinding,
  index: usize,
  settings: &SettingsContext,
  announce: Announce,
) -> Result<(), LocalizedString> {
  let mut values = settings.desired.controller.values();
  self::check_conflict(&values, binding, index)?;
  values[index] = binding;
  settings.change(SettingsChange::Controller(Bindings::from_values(values)));
  self::accepted(index, input_labels::controller(binding), announce);
  Ok(())
}

fn check_conflict<T: PartialEq>(
  values: &[T; 7],
  binding: T,
  index: usize,
) -> Result<(), LocalizedString> {
  if let Some(conflict) = values
    .iter()
    .enumerate()
    .find_map(|(other, value)| (*value == binding && other != index).then_some(other))
  {
    Err(txa(
      "Already used by {action}",
      tx_args![action => opaque(input_labels::action(conflict))],
      "Duplicate shortcut error.",
    ))
  } else {
    Ok(())
  }
}

fn accepted(index: usize, key: LocalizedString, announce: Announce) {
  announce.send(txa(
    "{action} assigned to {key}",
    tx_args![action => opaque(input_labels::action(index)), key => opaque(key)],
    "Accepted keyboard shortcut announcement.",
  ));
}
