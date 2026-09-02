use std::rc::Rc;

use battlement::{AccessibilityAction, ClickEvent, UiEventBody, UiEventKind};

use crate::{
  element_ref::ElementRef,
  event_handler::{Handler, HandlerPhase},
  semantics::{ActionDisposition, InteractionProps},
};

pub(crate) struct Activation<G> {
  disabled: bool,
  callback: Rc<dyn Fn(&mut G)>,
}

pub(crate) fn interaction<G: 'static>(
  disabled: bool,
  callback: Rc<dyn Fn(&mut G)>,
) -> InteractionProps<G> {
  let activation = Activation { disabled, callback };
  let click = activation.clone();
  let action = activation.clone();
  let mut interaction = InteractionProps::new();
  interaction.activation = Some(activation);
  interaction.handlers.push(Handler::event(
    "press-click",
    UiEventKind::Click,
    HandlerPhase::Default,
    self::click_event,
    move |game, event| {
      event.mark_activation_handled();
      if !click.disabled && !event.default_prevented() {
        (click.callback)(game);
      }
    },
  ));
  interaction.accessibility("press-accessibility", move |game, requested| {
    if action.disabled || requested != AccessibilityAction::Activate {
      return ActionDisposition::Unhandled;
    }
    (action.callback)(game);
    ActionDisposition::Handled
  })
}

impl<G: 'static> Activation<G> {
  pub(crate) fn label_interaction(&self, control: &ElementRef) -> InteractionProps<G> {
    let activation = self.clone();
    let control = control.clone();
    let mut interaction = InteractionProps::new();
    interaction.handlers.push(Handler::event(
      "associated-label-click",
      UiEventKind::Click,
      HandlerPhase::Default,
      self::click_event,
      move |game, event| {
        if event.activation_handled() || event.default_prevented() {
          return;
        }
        if activation.disabled || !control.is_attached() {
          return;
        }
        event.mark_activation_handled();
        control.focus();
        (activation.callback)(game);
      },
    ));
    interaction
  }
}

impl<G> Clone for Activation<G> {
  fn clone(&self) -> Self {
    Self {
      disabled: self.disabled,
      callback: Rc::clone(&self.callback),
    }
  }
}

fn click_event(body: &UiEventBody) -> &ClickEvent {
  match body {
    UiEventBody::Click(value) => value,
    _ => panic!("Reactant activation handler received another event kind"),
  }
}
