//! Resettable review surface for the three finished arcade-modal variants.

use trox::{ls, tx};

use crate::{arcade_modal::ArcadeModal, review_button::ReviewButton, select_control::VALUE_FONT};
use battlement::{Color, FlexDirection, Style, TextAnchor, WhiteSpace};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModalKind {
  Erase,
  Help,
  Rebinding,
}

/// Exercises entrance, exit, interruption, shine, and reduced motion.
#[builder]
pub struct ArcadeModalAnimationHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for ArcadeModalAnimationHarness {
  fn render(&self) -> impl Render {
    let (open, set_open) = hooks::use_state(None::<ModalKind>);
    let (reduced_motion, set_reduced_motion) = hooks::use_state(false);
    View::new()
      .name("arcade-modal-animation-specimen")
      .style(
        Style::new()
          .width(839)
          .height(650)
          .margin_top(48)
          .background_color(Color::rgb(0.01, 0.035, 0.08)),
      )
      .child(
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(10.0)
          .style(Style::new().height(105).padding_left(10))
          .child(self::opener(
            "ERASE",
            "modal-animation-erase",
            ModalKind::Erase,
            &set_open,
          ))
          .child(self::opener(
            "HELP",
            "modal-animation-help",
            ModalKind::Help,
            &set_open,
          ))
          .child(self::opener(
            "REBIND",
            "modal-animation-rebinding",
            ModalKind::Rebinding,
            &set_open,
          )),
      )
      .child(
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(10.0)
          .style(Style::new().height(105).padding_left(10))
          .child(
            ReviewButton::new()
              .label(ls(if reduced_motion {
                "REDUCED"
              } else {
                "FULL MOTION"
              }))
              .name("modal-animation-motion-policy")
              .on_press(set_reduced_motion.update_callback(|value| !value)),
          )
          .child(
            ReviewButton::new()
              .label(ls("RESET"))
              .name("modal-animation-reset")
              .on_press(EventCallback::new({
                let set_open = set_open.clone();
                move |()| {
                  set_open.set(None);
                  set_reduced_motion.set(false);
                }
              })),
          ),
      )
      .child(
        control_behavior::static_label(ls(format!(
          "Modal animation: {} · {}",
          self::status(open),
          if reduced_motion { "REDUCED" } else { "FULL" }
        )))
        .name("modal-animation-status")
        .style(
          Style::new()
            .margin_top(22)
            .font_size(30)
            .color(Color::rgb(0.75, 0.86, 0.97)),
        ),
      )
      .child((
        self::erase_modal(open, reduced_motion, set_open.clone(), self.overlay.clone()),
        self::help_modal(open, reduced_motion, set_open.clone(), self.overlay.clone()),
        self::rebinding_modal(open, reduced_motion, set_open, self.overlay.clone()),
      ))
  }
}

fn status(open: Option<ModalKind>) -> &'static str {
  match open {
    Some(ModalKind::Erase) => "ERASE OPEN",
    Some(ModalKind::Help) => "HELP OPEN",
    Some(ModalKind::Rebinding) => "REBINDING OPEN",
    None => "CLOSED",
  }
}

fn opener(
  label: &'static str,
  name: &'static str,
  kind: ModalKind,
  set_open: &StateSetter<Option<ModalKind>>,
) -> ReviewButton {
  ReviewButton::new()
    .label(ls(label))
    .name(name)
    .on_press(set_open.callback().map_input(move |_| Some(kind)))
}

fn erase_modal(
  open: Option<ModalKind>,
  reduced_motion: bool,
  set_open: StateSetter<Option<ModalKind>>,
  overlay: PortalTarget,
) -> ArcadeModal {
  ArcadeModal::new()
    .open(open == Some(ModalKind::Erase))
    .title(tx("Erase Saved Data?", "Erase confirmation title."))
    .children(self::copy(
      "All saved data will be permanently erased. This cannot be undone.",
    ))
    .confirm_label(tx("Erase", "Erase confirmation action."))
    .cancel_label(tx("Cancel", "Erase confirmation cancellation."))
    .danger(true)
    .reduce_motion(reduced_motion)
    .on_confirm(set_open.callback().map_input(|_| None))
    .on_close(set_open.callback().map_input(|_| None))
    .overlay(overlay)
}

fn help_modal(
  open: Option<ModalKind>,
  reduced_motion: bool,
  set_open: StateSetter<Option<ModalKind>>,
  overlay: PortalTarget,
) -> ArcadeModal {
  ArcadeModal::new()
    .open(open == Some(ModalKind::Help))
    .aria_label(tx(
      "Crash report upload information",
      "Crash report help dialog label.",
    ))
    .children(self::copy("We upload crash reports to Unity Diagnostics."))
    .confirm_label(tx("OK", "Close help action."))
    .reduce_motion(reduced_motion)
    .on_confirm(set_open.callback().map_input(|_| None))
    .on_close(set_open.callback().map_input(|_| None))
    .overlay(overlay)
}

fn rebinding_modal(
  open: Option<ModalKind>,
  reduced_motion: bool,
  set_open: StateSetter<Option<ModalKind>>,
  overlay: PortalTarget,
) -> ArcadeModal {
  ArcadeModal::new()
    .open(open == Some(ModalKind::Rebinding))
    .title(tx("Rebind Move Left", "Keyboard rebinding title."))
    .children(self::copy("Press a key to replace the current binding."))
    .confirm_label(tx("Cancel", "Cancel keyboard rebinding."))
    .reduce_motion(reduced_motion)
    .on_confirm(set_open.callback().map_input(|_| None))
    .on_close(set_open.callback().map_input(|_| None))
    .overlay(overlay)
}

fn copy(value: &'static str) -> impl Render {
  control_behavior::static_label(ls(value)).style(
    Style::new()
      .width(620)
      .font_size(47)
      .white_space(WhiteSpace::Normal)
      .unity_font_definition(VALUE_FONT)
      .unity_text_align(TextAnchor::MiddleCenter),
  )
}
