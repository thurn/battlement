//! Complete semantic and interaction behavior for advanced custom controls.

use battlement::{
  AccessibilityAction, AccessibilityScrollAxis, AccessibilityScrollDirection, CheckedState,
  SemanticRole, SemanticState,
};
use trox::LocalizedString;

use crate::{
  activation,
  callback::{Callback as EventCallback, IntoCallback},
  element_ref::ElementRef,
  event_handler::Handler,
  focus::FocusProps,
  host::{Label, TextElement},
  motion::MotionProps,
  semantics::{
    ControlBehavior, InteractionProps, SemanticDescription, SemanticName, SemanticProps,
    SemanticRange,
  },
};

use crate::semantics::{SemanticMembership, SemanticVisibility};

/// Returns complete button semantics, focus, and unified activation behavior.
pub fn button<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  disabled: bool,
  on_press: impl IntoCallback<(), G>,
) -> ControlBehavior<G> {
  let callback = on_press.into_callback();
  let interaction = activation::interaction(disabled, callback);
  ControlBehavior {
    semantic: described(named(SemanticRole::Button, name), description)
      .state(disabled_state(disabled))
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(disabled),
    interaction,
    motion: MotionProps::new(),
  }
}

/// Creates visible text with an exposed static-text semantic declaration.
#[must_use]
pub fn static_text(value: LocalizedString) -> TextElement {
  TextElement::new(value.clone()).semantic(self::static_text_semantic(value))
}

/// Creates a native label with an exposed static-text semantic declaration.
#[must_use]
pub fn static_label(value: LocalizedString) -> Label {
  Label::new(value.clone()).semantic(self::static_text_semantic(value))
}

/// Creates visible text that participates only in names derived from content.
#[must_use]
pub fn name_source_text(value: LocalizedString) -> TextElement {
  TextElement::new(value.clone())
    .semantic(self::static_text_semantic(value).visibility(SemanticVisibility::NameSourceOnly))
}

/// Returns checkbox semantics and unified Boolean activation.
pub fn checkbox<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  checked: bool,
  disabled: bool,
  on_change: impl IntoCallback<bool, G>,
) -> ControlBehavior<G> {
  toggle(
    name,
    description,
    checked,
    disabled,
    on_change,
    SemanticRole::Checkbox,
  )
}

/// Returns switch semantics and unified Boolean activation.
pub fn switch<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  checked: bool,
  disabled: bool,
  on_change: impl IntoCallback<bool, G>,
) -> ControlBehavior<G> {
  toggle(
    name,
    description,
    checked,
    disabled,
    on_change,
    SemanticRole::Switch,
  )
}

pub(crate) fn checkbox_native<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  checked: bool,
  disabled: bool,
  on_change: impl IntoCallback<bool, G>,
) -> ControlBehavior<G> {
  native_toggle(
    name,
    description,
    checked,
    disabled,
    on_change,
    SemanticRole::Checkbox,
  )
}

pub(crate) fn switch_native<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  checked: bool,
  disabled: bool,
  on_change: impl IntoCallback<bool, G>,
) -> ControlBehavior<G> {
  native_toggle(
    name,
    description,
    checked,
    disabled,
    on_change,
    SemanticRole::Switch,
  )
}

pub(crate) fn radio_member<G: 'static>(
  group: ElementRef,
  name: impl Into<SemanticName>,
  selected: bool,
  disabled: bool,
  on_select: impl IntoCallback<(), G>,
) -> ControlBehavior<G> {
  native_choice(
    SemanticRole::Radio,
    group,
    SemanticMembership::Radio,
    name,
    selected,
    disabled,
    on_select,
  )
}

pub(crate) fn tab_member<G: 'static>(
  tabs: ElementRef,
  name: impl Into<SemanticName>,
  selected: bool,
  disabled: bool,
  on_select: impl IntoCallback<(), G>,
) -> ControlBehavior<G> {
  native_choice(
    SemanticRole::Tab,
    tabs,
    SemanticMembership::Tab,
    name,
    selected,
    disabled,
    on_select,
  )
}

pub(crate) fn tab_panel_for(tabs: ElementRef, selected: bool) -> SemanticProps {
  SemanticProps::new(SemanticRole::TabPanel)
    .visibility(if selected {
      SemanticVisibility::Exposed
    } else {
      SemanticVisibility::Hidden
    })
    .membership(SemanticMembership::TabPanel(tabs))
}

/// Returns single-thumb slider semantics and direct range actions.
pub fn slider<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  value: SemanticRange,
  step: f64,
  disabled: bool,
  on_change: impl IntoCallback<f64, G>,
) -> ControlBehavior<G> {
  assert!(
    value.minimum.is_finite() && value.maximum.is_finite() && value.minimum <= value.maximum,
    "slider bounds must be finite and ordered"
  );
  assert!(
    step.is_finite() && step > 0.0,
    "slider step must be positive"
  );
  assert!(
    value.current >= value.minimum && value.current <= value.maximum,
    "slider value must be within its bounds"
  );
  let on_change = on_change.into_callback();
  let current = value.current;
  let minimum = value.minimum;
  let maximum = value.maximum;
  let interaction = self::accessible(
    "slider",
    on_change.map(move |action| {
      if disabled {
        return None;
      }
      match action {
        AccessibilityAction::Increment => Some((current + step).min(maximum)),
        AccessibilityAction::Decrement => Some((current - step).max(minimum)),
        _ => None,
      }
    }),
  );
  ControlBehavior {
    semantic: described(named(SemanticRole::Slider, name), description)
      .state(disabled_state(disabled))
      .value(value)
      .action(AccessibilityAction::Increment)
      .action(AccessibilityAction::Decrement),
    focus: ordinary_focus(disabled),
    interaction,
    motion: MotionProps::new(),
  }
}

/// Returns determinate progress semantics.
pub fn progress(name: LocalizedString, value: SemanticRange) -> SemanticProps {
  named(SemanticRole::Progress, name).value(value)
}

/// Returns indeterminate progress semantics.
pub fn busy_progress(name: LocalizedString) -> SemanticProps {
  named(SemanticRole::Progress, name).state(SemanticState {
    busy: true,
    ..SemanticState::default()
  })
}

/// Returns disclosure semantics and activation behavior.
pub fn disclosure<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  expanded: bool,
  disabled: bool,
  on_toggle: impl IntoCallback<(), G>,
) -> ControlBehavior<G> {
  let callback = on_toggle.into_callback();
  ControlBehavior {
    semantic: described(named(SemanticRole::Disclosure, name), description)
      .state(SemanticState {
        disabled,
        expanded: Some(expanded),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(disabled),
    interaction: activation::interaction(disabled, callback),
    motion: MotionProps::new(),
  }
}

/// Returns dialog semantics and optional dismiss behavior.
pub fn dialog(
  name: impl Into<SemanticName>,
  on_dismiss: Option<EventCallback<()>>,
) -> ControlBehavior<()> {
  let mut semantic = named(SemanticRole::Dialog, name);
  let mut interaction = InteractionProps::new();
  if let Some(on_dismiss) = on_dismiss {
    semantic = semantic.action(AccessibilityAction::Dismiss);
    interaction = self::accessible(
      "dialog-dismiss",
      on_dismiss.map(|action| (action == AccessibilityAction::Dismiss).then_some(())),
    );
  }
  ControlBehavior {
    semantic,
    focus: FocusProps::new(),
    interaction,
    motion: MotionProps::new(),
  }
}

/// Returns heading semantics.
pub fn heading(name: LocalizedString, level: u8) -> SemanticProps {
  named(SemanticRole::Heading, name).heading_level(level)
}

/// Returns informative-image semantics.
pub fn image(name: LocalizedString) -> SemanticProps {
  named(SemanticRole::Image, name)
}

/// Returns static-text semantics.
pub fn static_text_props(value: LocalizedString) -> SemanticProps {
  self::static_text_semantic(value)
}

/// Returns optional named group semantics.
pub fn group(name: Option<LocalizedString>) -> SemanticProps {
  optionally_named(SemanticRole::Group, name)
}

/// Returns semantic scroll behavior using ordinary application state.
pub fn scroll_area<G: 'static>(
  name: Option<LocalizedString>,
  axis: AccessibilityScrollAxis,
  forward: bool,
  backward: bool,
  on_scroll: impl IntoCallback<AccessibilityScrollDirection, G>,
) -> ControlBehavior<G> {
  let callback = on_scroll.into_callback();
  let interaction = self::accessible(
    "scroll-area",
    callback.map(move |action| {
      let AccessibilityAction::Scroll(direction) = action else {
        return None;
      };
      if direction == AccessibilityScrollDirection::Forward && !forward {
        return None;
      }
      if direction == AccessibilityScrollDirection::Backward && !backward {
        return None;
      }
      Some(direction)
    }),
  );
  let mut semantic = optionally_named(SemanticRole::ScrollArea, name).scroll_axis(axis);
  if forward {
    semantic = semantic.action(AccessibilityAction::Scroll(
      AccessibilityScrollDirection::Forward,
    ));
  }
  if backward {
    semantic = semantic.action(AccessibilityAction::Scroll(
      AccessibilityScrollDirection::Backward,
    ));
  }
  ControlBehavior {
    semantic,
    focus: FocusProps::new(),
    interaction,
    motion: MotionProps::new(),
  }
}

fn toggle<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  checked: bool,
  disabled: bool,
  on_change: impl IntoCallback<bool, G>,
  role: SemanticRole,
) -> ControlBehavior<G> {
  let callback = on_change.into_callback();
  let next = !checked;
  let interaction = activation::interaction(disabled, callback.map(move |()| Some(next)));
  ControlBehavior {
    semantic: described(named(role, name), description)
      .state(SemanticState {
        disabled,
        checked: Some(if checked {
          CheckedState::True
        } else {
          CheckedState::False
        }),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(disabled),
    interaction,
    motion: MotionProps::new(),
  }
}

fn native_toggle<G: 'static>(
  name: impl Into<SemanticName>,
  description: Option<SemanticDescription>,
  checked: bool,
  disabled: bool,
  on_change: impl IntoCallback<bool, G>,
  role: SemanticRole,
) -> ControlBehavior<G> {
  let next = !checked;
  let callback = on_change.into_callback();
  ControlBehavior {
    semantic: described(named(role, name), description)
      .state(SemanticState {
        disabled,
        checked: Some(if checked {
          CheckedState::True
        } else {
          CheckedState::False
        }),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(disabled),
    interaction: self::accessible(
      "native-toggle",
      callback
        .map(move |action| (!disabled && action == AccessibilityAction::Activate).then_some(next)),
    ),
    motion: MotionProps::new(),
  }
}

fn native_choice<G: 'static>(
  role: SemanticRole,
  membership_ref: ElementRef,
  membership: fn(ElementRef) -> SemanticMembership,
  name: impl Into<SemanticName>,
  selected: bool,
  disabled: bool,
  on_select: impl IntoCallback<(), G>,
) -> ControlBehavior<G> {
  let callback = on_select.into_callback();
  ControlBehavior {
    semantic: named(role, name)
      .state(SemanticState {
        disabled,
        selected: Some(selected),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate)
      .membership(membership(membership_ref)),
    focus: ordinary_focus(disabled),
    interaction: self::accessible(
      "native-choice",
      callback
        .map(move |action| (!disabled && action == AccessibilityAction::Activate).then_some(())),
    ),
    motion: MotionProps::new(),
  }
}

fn accessible<G: 'static>(
  slot: &'static str,
  callback: EventCallback<AccessibilityAction>,
) -> InteractionProps<G> {
  let mut interaction = InteractionProps::new();
  interaction
    .handlers
    .push(Handler::accessibility_callback(slot, callback));
  interaction
}

fn static_text_semantic(value: LocalizedString) -> SemanticProps {
  named(SemanticRole::StaticText, value)
}

fn named(role: SemanticRole, name: impl Into<SemanticName>) -> SemanticProps {
  SemanticProps::new(role).name(name.into())
}

fn described(semantic: SemanticProps, description: Option<SemanticDescription>) -> SemanticProps {
  match description {
    Some(description) => semantic.description(description),
    None => semantic,
  }
}

fn optionally_named(role: SemanticRole, name: Option<LocalizedString>) -> SemanticProps {
  name.map_or_else(|| SemanticProps::new(role), |name| named(role, name))
}

fn disabled_state(disabled: bool) -> SemanticState {
  SemanticState {
    disabled,
    ..SemanticState::default()
  }
}

fn ordinary_focus(disabled: bool) -> FocusProps {
  FocusProps::new()
    .focusable(!disabled)
    .tab_index(if disabled { -1 } else { 0 })
}
