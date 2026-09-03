//! Accessible behavior hooks for common settings controls.

use battlement::{
  AccessibilityAction, AccessibilityRangeValue, AccessibilityScrollAxis,
  AccessibilityScrollDirection, CheckedState, SemanticRole, SemanticState,
};

use crate::{
  activation,
  callback::{Callback, IntoCallback},
  element_ref::{ElementRef, use_element_ref},
  event_handler::Handler,
  focus::FocusProps,
  host::{Label, TextElement},
  motion::MotionProps,
  semantics::{
    self, AccessibleBehavior, AccessibleName, InteractionProps, LocalizedText, SemanticProps,
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
pub struct ButtonOptions<F, N = LocalizedText> {
  /// Accessible name.
  pub name: N,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Ordinary press callback.
  pub on_press: F,
}

/// Options for checkbox and switch patterns.
pub struct ToggleOptions<F, N = LocalizedText> {
  /// Accessible name.
  pub name: N,
  /// Current checked state.
  pub checked: bool,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Authoritative change callback.
  pub on_change: F,
}

/// Options for [`use_slider`].
pub struct SliderOptions<F, N = LocalizedText> {
  /// Accessible name.
  pub name: N,
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
pub struct DisclosureOptions<F, N = LocalizedText> {
  /// Accessible name.
  pub name: N,
  /// Current expansion state.
  pub expanded: bool,
  /// Whether activation is unavailable.
  pub is_disabled: bool,
  /// Ordinary toggle callback.
  pub on_toggle: F,
}

/// Options for a modal dialog wrapper.
pub struct DialogOptions<F, N = LocalizedText> {
  /// Accessible name.
  pub name: N,
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
pub struct ChoiceOptions<F, N = LocalizedText> {
  /// Accessible name.
  pub name: N,
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
  options: ButtonOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, PressState> {
  let callback = options.on_press.into_callback();
  let interaction = activation::interaction(options.is_disabled, callback);
  AccessibleBehavior {
    semantic: named(SemanticRole::Button, options.name)
      .state(disabled_state(options.is_disabled))
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(options.is_disabled),
    interaction,
    motion: MotionProps::new(),
    state: PressState {
      disabled: options.is_disabled,
    },
  }
}

/// Creates visible text with an exposed static-text semantic declaration.
#[must_use]
pub fn static_text(value: impl Into<String>) -> TextElement {
  let value = value.into();
  TextElement::new(value.clone()).semantic(self::use_static_text(semantics::text(value)))
}

/// Creates a native label with an exposed static-text semantic declaration.
#[must_use]
pub fn static_label(value: impl Into<String>) -> Label {
  let value = value.into();
  Label::new(value.clone()).semantic(self::use_static_text(semantics::text(value)))
}

/// Creates visible text that participates only in names derived from content.
#[must_use]
pub fn name_source_text(value: impl Into<String>) -> TextElement {
  let value = value.into();
  TextElement::new(value.clone()).semantic(
    self::use_static_text(semantics::text(value)).visibility(SemanticVisibility::NameSourceOnly),
  )
}

/// Returns checkbox semantics and unified Boolean activation.
pub fn use_checkbox<G: 'static>(
  options: ToggleOptions<impl IntoCallback<bool, G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, bool> {
  use_toggle(options, SemanticRole::Checkbox)
}

/// Returns switch semantics and unified Boolean activation.
pub fn use_switch<G: 'static>(
  options: ToggleOptions<impl IntoCallback<bool, G>, impl Into<AccessibleName>>,
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
  options: ChoiceOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
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
  options: ChoiceOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
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
  options: SliderOptions<impl IntoCallback<f64, G>, impl Into<AccessibleName>>,
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
  let on_change = options.on_change.into_callback();
  let value = options.value;
  let minimum = options.minimum;
  let maximum = options.maximum;
  let step = options.step;
  let disabled = options.is_disabled;
  let interaction = self::accessible(
    "slider",
    on_change.map(move |action| {
      if disabled {
        return None;
      }
      match action {
        AccessibilityAction::Increment => Some((value + step).min(maximum)),
        AccessibilityAction::Decrement => Some((value - step).max(minimum)),
        _ => None,
      }
    }),
  );
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
    motion: MotionProps::new(),
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
  options: DisclosureOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, bool> {
  let callback = options.on_toggle.into_callback();
  AccessibleBehavior {
    semantic: named(SemanticRole::Disclosure, options.name)
      .state(SemanticState {
        disabled: options.is_disabled,
        expanded: Some(options.expanded),
        ..SemanticState::default()
      })
      .action(AccessibilityAction::Activate),
    focus: ordinary_focus(options.is_disabled),
    interaction: activation::interaction(options.is_disabled, callback),
    motion: MotionProps::new(),
    state: options.expanded,
  }
}

/// Returns dialog semantics and optional dismiss behavior.
pub fn use_dialog<G: 'static>(
  options: DialogOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, ()> {
  let mut semantic = named(SemanticRole::Dialog, options.name);
  let mut interaction = InteractionProps::new();
  if let Some(on_dismiss) = options.on_dismiss {
    semantic = semantic.action(AccessibilityAction::Dismiss);
    let callback = on_dismiss.into_callback();
    interaction = self::accessible(
      "dialog-dismiss",
      callback.map(|action| (action == AccessibilityAction::Dismiss).then_some(())),
    );
  }
  AccessibleBehavior {
    semantic,
    focus: FocusProps::new(),
    interaction,
    motion: MotionProps::new(),
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
  options: ScrollAreaOptions<impl IntoCallback<AccessibilityScrollDirection, G>>,
) -> AccessibleBehavior<G, ()> {
  let callback = options.on_scroll.into_callback();
  let forward = options.can_scroll_forward;
  let backward = options.can_scroll_backward;
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
    motion: MotionProps::new(),
    state: (),
  }
}

fn use_toggle<G: 'static>(
  options: ToggleOptions<impl IntoCallback<bool, G>, impl Into<AccessibleName>>,
  role: SemanticRole,
) -> AccessibleBehavior<G, bool> {
  let callback = options.on_change.into_callback();
  let next = !options.checked;
  let disabled = options.is_disabled;
  let interaction = activation::interaction(disabled, callback.map(move |()| Some(next)));
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
    motion: MotionProps::new(),
    state: options.checked,
  }
}

fn use_choice<G: 'static>(
  role: SemanticRole,
  membership_ref: ElementRef,
  membership: fn(ElementRef) -> SemanticMembership,
  options: ChoiceOptions<impl IntoCallback<(), G>, impl Into<AccessibleName>>,
) -> AccessibleBehavior<G, bool> {
  let selected = options.selected;
  let disabled = options.is_disabled;
  let callback = options.on_select.into_callback();
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
    interaction: activation::interaction(disabled, callback),
    motion: MotionProps::new(),
    state: selected,
  }
}

fn accessible<G: 'static>(
  slot: &'static str,
  callback: Callback<AccessibilityAction>,
) -> InteractionProps<G> {
  let mut interaction = InteractionProps::new();
  interaction
    .handlers
    .push(Handler::accessibility_callback(slot, callback));
  interaction
}

fn named(role: SemanticRole, name: impl Into<AccessibleName>) -> SemanticProps {
  SemanticProps::new(role).name(name.into())
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
