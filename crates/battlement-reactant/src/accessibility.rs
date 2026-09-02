//! Accessible behavior hooks for common settings controls.

use std::rc::Rc;

use battlement::{
  AccessibilityAction, AccessibilityRangeValue, AccessibilityScrollAxis,
  AccessibilityScrollDirection, CheckedState, SemanticRole, SemanticState, UiEventBody,
  UiEventKind,
};

use crate::{
  element_ref::{ElementRef, use_element_ref},
  event_handler::{Handler, HandlerPhase},
  focus::FocusProps,
  semantics::{
    AccessibleBehavior, AccessibleName, ActionDisposition, InteractionProps, LocalizedText,
    SemanticProps,
  },
};

use crate::semantics::{SemanticMembership, SemanticVisibility};

/// Styling state shared by pressable patterns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PressState {
  /// Whether the control is unavailable.
  pub disabled: bool,
}

/// Options for [`use_button`].
pub struct ButtonOptions<F> {
  /// Accessible name.
  pub name: LocalizedText,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Ordinary press callback.
  pub on_press: F,
}

/// Options for checkbox and switch patterns.
pub struct ToggleOptions<F> {
  /// Accessible name.
  pub name: LocalizedText,
  /// Current checked state.
  pub checked: bool,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Authoritative change callback.
  pub on_change: F,
}

/// Options for [`use_slider`].
pub struct SliderOptions<F> {
  /// Accessible name.
  pub name: LocalizedText,
  /// Current value.
  pub value: f64,
  /// Inclusive minimum.
  pub minimum: f64,
  /// Inclusive maximum.
  pub maximum: f64,
  /// Positive increment/decrement step.
  pub step: f64,
  /// Optional localized value text.
  pub value_text: Option<LocalizedText>,
  /// Whether changes are unavailable.
  pub is_disabled: bool,
  /// Authoritative change callback.
  pub on_change: F,
}

/// Options for a disclosure trigger.
pub struct DisclosureOptions<F> {
  /// Accessible name.
  pub name: LocalizedText,
  /// Current expansion state.
  pub expanded: bool,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Ordinary toggle callback.
  pub on_toggle: F,
}

/// Options for a modal dialog wrapper.
pub struct DialogOptions<F> {
  /// Accessible name.
  pub name: LocalizedText,
  /// Optional ordinary dismiss callback.
  pub on_dismiss: Option<F>,
}

/// Options for a semantic scroll area.
pub struct ScrollAreaOptions<F> {
  /// Optional accessible name.
  pub name: Option<LocalizedText>,
  /// Owned logical axis.
  pub axis: AccessibilityScrollAxis,
  /// Whether forward movement is available.
  pub can_scroll_forward: bool,
  /// Whether backward movement is available.
  pub can_scroll_backward: bool,
  /// Ordinary logical scroll callback.
  pub on_scroll: F,
}

/// Radio-group semantics and the ref used by member declarations.
#[derive(Clone)]
pub struct RadioGroupBehavior {
  /// Group-host semantics.
  pub semantic: SemanticProps,
  /// Ref that must be attached to the same group host.
  pub element_ref: ElementRef,
}

/// Tab-list semantics and the ref used by tabs and panels.
#[derive(Clone)]
pub struct TabsBehavior {
  /// Tab-list host semantics.
  pub semantic: SemanticProps,
  /// Ref that must be attached to the same tab-list host.
  pub element_ref: ElementRef,
}

/// Options for one radio or tab choice.
pub struct ChoiceOptions<F> {
  /// Accessible name.
  pub name: LocalizedText,
  /// Current selected state.
  pub selected: bool,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Ordinary selection callback.
  pub on_select: F,
}

/// State supplied to slider visuals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderState {
  /// Current authoritative value.
  pub value: f64,
  /// Whether changes are unavailable.
  pub disabled: bool,
}

/// Returns button semantics, focus, and unified activation behavior.
pub fn use_button<G: 'static>(
  options: ButtonOptions<impl Fn(&mut G) + 'static>,
) -> AccessibleBehavior<G, PressState> {
  let callback: Rc<dyn Fn(&mut G)> = Rc::new(options.on_press);
  let interaction = activation_interaction(options.is_disabled, callback);
  AccessibleBehavior {
    semantic: named(SemanticRole::Button, options.name)
      .state(disabled_state(options.is_disabled))
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(options.is_disabled),
    interaction,
    state: PressState {
      disabled: options.is_disabled,
    },
  }
}

/// Returns checkbox semantics and unified Boolean activation.
pub fn use_checkbox<G: 'static>(
  options: ToggleOptions<impl Fn(&mut G, bool) + 'static>,
) -> AccessibleBehavior<G, bool> {
  use_toggle(options, SemanticRole::Checkbox)
}

/// Returns switch semantics and unified Boolean activation.
pub fn use_switch<G: 'static>(
  options: ToggleOptions<impl Fn(&mut G, bool) + 'static>,
) -> AccessibleBehavior<G, bool> {
  use_toggle(options, SemanticRole::Switch)
}

/// Returns a semantic radio group and its runtime-local membership handle.
pub fn use_radio_group(name: LocalizedText) -> RadioGroupBehavior {
  RadioGroupBehavior {
    semantic: named(SemanticRole::RadioGroup, name),
    element_ref: use_element_ref(),
  }
}

/// Returns radio semantics, ordinary focus, and unified selection behavior.
pub fn use_radio<G: 'static>(
  group: &RadioGroupBehavior,
  options: ChoiceOptions<impl Fn(&mut G) + 'static>,
) -> AccessibleBehavior<G, bool> {
  use_choice(
    SemanticRole::Radio,
    group.element_ref.clone(),
    SemanticMembership::Radio,
    options,
  )
}

/// Returns a semantic tab list and its runtime-local membership handle.
pub fn use_tabs(name: LocalizedText) -> TabsBehavior {
  TabsBehavior {
    semantic: named(SemanticRole::TabList, name),
    element_ref: use_element_ref(),
  }
}

/// Returns tab semantics, ordinary focus, and unified selection behavior.
pub fn use_tab<G: 'static>(
  tabs: &TabsBehavior,
  options: ChoiceOptions<impl Fn(&mut G) + 'static>,
) -> AccessibleBehavior<G, bool> {
  use_choice(
    SemanticRole::Tab,
    tabs.element_ref.clone(),
    SemanticMembership::Tab,
    options,
  )
}

/// Returns selected-panel semantics, hidden before a deselection exit begins.
pub fn use_tab_panel(tabs: &TabsBehavior, selected: bool) -> SemanticProps {
  SemanticProps::new(SemanticRole::TabPanel)
    .visibility(if selected {
      SemanticVisibility::Exposed
    } else {
      SemanticVisibility::Hidden
    })
    .membership(SemanticMembership::TabPanel(tabs.element_ref.clone()))
}

/// Returns single-thumb slider semantics and direct range actions.
pub fn use_slider<G: 'static>(
  options: SliderOptions<impl Fn(&mut G, f64) + 'static>,
) -> AccessibleBehavior<G, SliderState> {
  assert!(
    options.minimum.is_finite()
      && options.maximum.is_finite()
      && options.minimum <= options.maximum,
    "slider bounds must be finite and ordered"
  );
  assert!(
    options.step.is_finite() && options.step > 0.0,
    "slider step must be positive"
  );
  assert!(
    options.value >= options.minimum && options.value <= options.maximum,
    "slider value must be within its bounds"
  );
  let on_change: ValueCallback<G> = Rc::new(options.on_change);
  let value = options.value;
  let minimum = options.minimum;
  let maximum = options.maximum;
  let step = options.step;
  let disabled = options.is_disabled;
  let interaction = InteractionProps::new().accessibility("slider", move |game, action| {
    if disabled {
      return ActionDisposition::Unhandled;
    }
    let next = match action {
      AccessibilityAction::Increment => (value + step).min(maximum),
      AccessibilityAction::Decrement => (value - step).max(minimum),
      _ => return ActionDisposition::Unhandled,
    };
    on_change(game, next);
    ActionDisposition::Handled
  });
  AccessibleBehavior {
    semantic: named(SemanticRole::Slider, options.name)
      .state(disabled_state(disabled))
      .value(AccessibilityRangeValue {
        current: value,
        minimum,
        maximum,
        text: options.value_text.map(|text| text.resolved()),
      })
      .action(AccessibilityAction::Increment)
      .action(AccessibilityAction::Decrement),
    focus: ordinary_focus(disabled),
    interaction,
    state: SliderState { value, disabled },
  }
}

/// Returns determinate progress semantics.
pub fn use_progress(name: LocalizedText, value: AccessibilityRangeValue) -> SemanticProps {
  named(SemanticRole::Progress, name).value(value)
}

/// Returns indeterminate progress semantics.
pub fn use_busy_progress(name: LocalizedText) -> SemanticProps {
  named(SemanticRole::Progress, name).state(SemanticState {
    busy: true,
    ..SemanticState::default()
  })
}

/// Returns disclosure semantics and activation behavior.
pub fn use_disclosure<G: 'static>(
  options: DisclosureOptions<impl Fn(&mut G) + 'static>,
) -> AccessibleBehavior<G, bool> {
  let callback: Rc<dyn Fn(&mut G)> = Rc::new(options.on_toggle);
  AccessibleBehavior {
    semantic: named(SemanticRole::Disclosure, options.name)
      .state(SemanticState {
        disabled: options.is_disabled,
        expanded: Some(options.expanded),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(options.is_disabled),
    interaction: activation_interaction(options.is_disabled, callback),
    state: options.expanded,
  }
}

/// Returns dialog semantics and optional dismiss behavior.
pub fn use_dialog<G: 'static>(
  options: DialogOptions<impl Fn(&mut G) + 'static>,
) -> AccessibleBehavior<G, ()> {
  let mut semantic = named(SemanticRole::Dialog, options.name);
  let mut interaction = InteractionProps::new();
  if let Some(on_dismiss) = options.on_dismiss {
    semantic = semantic.action(AccessibilityAction::Dismiss);
    let callback: Rc<dyn Fn(&mut G)> = Rc::new(on_dismiss);
    interaction = interaction.accessibility("dialog-dismiss", move |game, action| {
      if action != AccessibilityAction::Dismiss {
        return ActionDisposition::Unhandled;
      }
      callback(game);
      ActionDisposition::Handled
    });
  }
  AccessibleBehavior {
    semantic,
    focus: FocusProps::new(),
    interaction,
    state: (),
  }
}

/// Returns heading semantics.
pub fn use_heading(name: LocalizedText, level: u8) -> SemanticProps {
  named(SemanticRole::Heading, name).heading_level(level)
}

/// Returns informative-image semantics.
pub fn use_image(name: LocalizedText) -> SemanticProps {
  named(SemanticRole::Image, name)
}

/// Returns static-text semantics.
pub fn use_static_text(value: LocalizedText) -> SemanticProps {
  named(SemanticRole::StaticText, value)
}

/// Returns optional named group semantics.
pub fn use_group(name: Option<LocalizedText>) -> SemanticProps {
  optionally_named(SemanticRole::Group, name)
}

/// Returns semantic scroll behavior using ordinary application state.
pub fn use_scroll_area<G: 'static>(
  options: ScrollAreaOptions<impl Fn(&mut G, AccessibilityScrollDirection) + 'static>,
) -> AccessibleBehavior<G, ()> {
  let callback: ScrollCallback<G> = Rc::new(options.on_scroll);
  let forward = options.can_scroll_forward;
  let backward = options.can_scroll_backward;
  let interaction = InteractionProps::new().accessibility("scroll-area", move |game, action| {
    let AccessibilityAction::Scroll(direction) = action else {
      return ActionDisposition::Unhandled;
    };
    if direction == AccessibilityScrollDirection::Forward && !forward {
      return ActionDisposition::Unhandled;
    }
    if direction == AccessibilityScrollDirection::Backward && !backward {
      return ActionDisposition::Unhandled;
    }
    callback(game, direction);
    ActionDisposition::Handled
  });
  let mut semantic =
    optionally_named(SemanticRole::ScrollArea, options.name).scroll_axis(options.axis);
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
  AccessibleBehavior {
    semantic,
    focus: FocusProps::new(),
    interaction,
    state: (),
  }
}

fn use_toggle<G: 'static>(
  options: ToggleOptions<impl Fn(&mut G, bool) + 'static>,
  role: SemanticRole,
) -> AccessibleBehavior<G, bool> {
  let callback: ToggleCallback<G> = Rc::new(options.on_change);
  let next = !options.checked;
  let disabled = options.is_disabled;
  let click_callback = Rc::clone(&callback);
  let mut interaction = InteractionProps::new();
  if !disabled {
    interaction.handlers.push(Handler::brief(
      "toggle-click",
      UiEventKind::Click,
      HandlerPhase::Default,
      click_event,
      move |game| click_callback(game, next),
    ));
  }
  interaction = interaction.accessibility("toggle-accessibility", move |game, action| {
    if disabled || action != AccessibilityAction::Activate {
      return ActionDisposition::Unhandled;
    }
    callback(game, next);
    ActionDisposition::Handled
  });
  AccessibleBehavior {
    semantic: named(role, options.name)
      .state(SemanticState {
        disabled,
        checked: Some(if options.checked {
          CheckedState::True
        } else {
          CheckedState::False
        }),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(disabled),
    interaction,
    state: options.checked,
  }
}

fn use_choice<G: 'static>(
  role: SemanticRole,
  membership_ref: ElementRef,
  membership: fn(ElementRef) -> SemanticMembership,
  options: ChoiceOptions<impl Fn(&mut G) + 'static>,
) -> AccessibleBehavior<G, bool> {
  let selected = options.selected;
  let disabled = options.is_disabled;
  let callback: Rc<dyn Fn(&mut G)> = Rc::new(options.on_select);
  AccessibleBehavior {
    semantic: named(role, options.name)
      .state(SemanticState {
        disabled,
        selected: Some(selected),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate)
      .membership(membership(membership_ref)),
    focus: ordinary_focus(disabled),
    interaction: activation_interaction(disabled, callback),
    state: selected,
  }
}

fn activation_interaction<G: 'static>(
  disabled: bool,
  callback: Rc<dyn Fn(&mut G)>,
) -> InteractionProps<G> {
  let click_callback = Rc::clone(&callback);
  let mut interaction = InteractionProps::new();
  if !disabled {
    interaction.handlers.push(Handler::brief(
      "press-click",
      UiEventKind::Click,
      HandlerPhase::Default,
      click_event,
      move |game| click_callback(game),
    ));
  }
  interaction.accessibility("press-accessibility", move |game, action| {
    if disabled || action != AccessibilityAction::Activate {
      return ActionDisposition::Unhandled;
    }
    callback(game);
    ActionDisposition::Handled
  })
}

fn click_event(body: &UiEventBody) -> &battlement::ClickEvent {
  match body {
    UiEventBody::Click(value) => value,
    _ => panic!("Reactant activation handler received another event kind"),
  }
}

fn named(role: SemanticRole, name: LocalizedText) -> SemanticProps {
  SemanticProps::new(role).name(AccessibleName::Text(name))
}

fn optionally_named(role: SemanticRole, name: Option<LocalizedText>) -> SemanticProps {
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

type ScrollCallback<G> = Rc<dyn Fn(&mut G, AccessibilityScrollDirection)>;
type ToggleCallback<G> = Rc<dyn Fn(&mut G, bool)>;
type ValueCallback<G> = Rc<dyn Fn(&mut G, f64)>;
