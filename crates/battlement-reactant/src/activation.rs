use battlement::{AccessibilityAction, ClickEvent, UiEventBody, UiEventKind};

use crate::{
  callback::Callback,
  element_ref::ElementRef,
  event::ReactantEvent,
  event_handler::{Handler, HandlerPhase},
  semantics::InteractionProps,
};

#[derive(Clone)]
pub(crate) struct Activation {
  disabled: bool,
  callback: Callback<()>,
}

pub(crate) fn interaction<G: 'static>(
  disabled: bool,
  callback: Callback<()>,
) -> InteractionProps<G> {
  let activation = Activation { disabled, callback };
  let mut interaction = InteractionProps::new();
  interaction.activation = Some(activation.clone());
  interaction.handlers.push(Handler::event_callback(
    "press-click",
    UiEventKind::Click,
    HandlerPhase::Default,
    self::click_event,
    activation
      .callback
      .clone()
      .map(move |event: ReactantEvent<ClickEvent>| {
        event.mark_activation_handled();
        (!disabled && !event.default_prevented()).then_some(())
      }),
  ));
  interaction.handlers.push(Handler::accessibility_callback(
    "press-accessibility",
    activation.callback.map(move |requested| {
      (!disabled && requested == AccessibilityAction::Activate).then_some(())
    }),
  ));
  interaction
}

impl Activation {
  pub(crate) fn label_interaction<G: 'static>(&self, control: &ElementRef) -> InteractionProps<G> {
    let disabled = self.disabled;
    let control = control.clone();
    let mut interaction = InteractionProps::new();
    interaction.handlers.push(Handler::event_callback(
      "associated-label-click",
      UiEventKind::Click,
      HandlerPhase::Default,
      self::click_event,
      self
        .callback
        .clone()
        .map(move |event: ReactantEvent<ClickEvent>| {
          if event.activation_handled() || event.default_prevented() {
            return None;
          }
          if disabled || !control.is_attached() {
            return None;
          }
          event.mark_activation_handled();
          control.focus();
          Some(())
        }),
    ));
    interaction
  }
}

fn click_event(body: &UiEventBody) -> &ClickEvent {
  match body {
    UiEventBody::Click(value) => value,
    _ => panic!("Reactant activation handler received another event kind"),
  }
}
