//! Opaque Reactant host façades.
//!
//! Native Battlement UI values are protocol data and cannot render through
//! Reactant directly.
//!
//! ```compile_fail
//! use battlement::UiButton;
//! use battlement_reactant::render::Render;
//!
//! fn require_render(_: impl Render) {}
//! require_render(UiButton::new("Save"));
//! ```
//!
//! Leaf façades do not expose child builders.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! let _ = Label::new(trox::tx(
//!   "Caption",
//!   "Caption in the host facade example.",
//! )).child(Label::new(trox::tx(
//!   "Invalid",
//!   "Invalid child in the host facade example.",
//! )));
//! ```
//!
//! Product copy does not accept unlocalized string literals or owned strings.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! let _ = Label::new("Caption");
//! ```
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! let copy = String::from("Save");
//! let _ = Button::new(copy);
//! ```
//!
//! Façades expose no conversion from native protocol hosts.
//!
//! ```compile_fail
//! use battlement::UiButton;
//! use battlement_reactant::host::Button;
//!
//! let _: Button = UiButton::new("Save").into();
//! ```

#![allow(private_interfaces)]

use std::{any::TypeId, boxed::Box as Boxed, hash::Hash, num::NonZeroU32, rc::Rc};

use battlement::{
  GridItem, OverlayPlacement, PickingMode, Prop, StackItem, Sticky, Style, UiBox, UiButton,
  UiDropdownField, UiFlex, UiGrid, UiGroupBox, UiImage, UiLabel, UiMinMaxSlider, UiPopupWindow,
  UiProgressBar, UiRadioButton, UiRadioButtonGroup, UiRepeatButton, UiScrollView, UiScroller,
  UiSlider, UiSliderInt, UiStack, UiTab, UiTabView, UiTextElement, UiTextField, UiToggle,
  UiToggleButtonGroup, UiVisualElement, UiVisualElementProperties,
};
use trox::LocalizedString;

use crate::{
  animation_controls::{AnimationControls, AnimationScope},
  builder_support::IntoOption,
  element_ref::ElementRef,
  event_handler::Handler,
  focus::FocusProps,
  host_facade::{self, HostState},
  key::ErasedKey,
  label_binding::{AssociatedControl, AssociatedLabel},
  motion::{InitialValue, MotionProps, MotionTarget, Transition},
  motion_css::{Animation, Decoration, IntoPseudoStyle, StyleTransition},
  paint::PaintStyle,
  portal::PortalTarget,
  render::{Node, Render, RenderSink},
  render_value::Sealed,
  semantics::{AccessibleBehavior, InteractionProps, SemanticProps},
  variant_map::{VariantData, VariantKey, Variants},
};

/// An optional dropdown selection whose display value remains localized until presentation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocalizedChoice {
  /// Zero-based choice index, or `None` when the selection is empty.
  pub index: Option<u32>,
  /// Localized display value at `index`, or `None` when the selection is empty.
  pub value: Option<LocalizedString>,
}

impl LocalizedChoice {
  /// Creates a populated selection.
  #[must_use]
  pub fn selected(index: u32, value: LocalizedString) -> Self {
    Self {
      index: Some(index),
      value: Some(value),
    }
  }

  /// Creates an explicit empty selection.
  #[must_use]
  pub const fn none() -> Self {
    Self {
      index: None,
      value: None,
    }
  }
}

macro_rules! gesture_methods {
  () => {
    /// Sets the locally activated hover target.
    #[must_use]
    pub fn while_hover(self, value: impl Into<MotionTarget>) -> Self {
      self.motion(MotionProps::new().while_hover(value))
    }

    /// Sets the locally activated exact-focus target.
    #[must_use]
    pub fn while_focus(self, value: impl Into<MotionTarget>) -> Self {
      self.motion(MotionProps::new().while_focus(value))
    }

    /// Sets the target activated by keyboard- or controller-visible focus.
    #[must_use]
    pub fn while_focus_visible(self, value: impl Into<MotionTarget>) -> Self {
      self.motion(MotionProps::new().while_focus_visible(value))
    }

    /// Sets the locally activated tap target.
    #[must_use]
    pub fn while_tap(self, value: impl Into<MotionTarget>) -> Self {
      self.motion(MotionProps::new().while_tap(value))
    }

    /// Sets the locally activated drag target.
    #[must_use]
    pub fn while_drag(self, value: impl Into<MotionTarget>) -> Self {
      self.motion(MotionProps::new().while_drag(value))
    }

    /// Sets the viewport-entry target.
    #[must_use]
    pub fn while_in_view(self, value: impl Into<MotionTarget>) -> Self {
      self.motion(MotionProps::new().while_in_view(value))
    }

    /// Replaces local gesture thresholds.
    #[must_use]
    pub fn gesture_config(self, value: crate::gesture::GestureConfig) -> Self {
      self.motion(MotionProps::new().gesture_config(value))
    }

    /// Enables pan recognition.
    #[must_use]
    pub fn pan(self, value: bool) -> Self {
      self.motion(MotionProps::new().pan(value))
    }

    /// Enables drag ownership on selected axes.
    #[must_use]
    pub fn drag(mut self, value: crate::gesture::DragAxis) -> Self {
      self.state.motion = self.state.motion.drag(value);
      self
    }

    /// Replaces drag constraints.
    #[must_use]
    pub fn drag_constraints(mut self, value: crate::gesture::DragConstraints) -> Self {
      self.state.motion = self.state.motion.drag_constraints(value);
      self
    }

    /// Replaces elastic overshoot.
    #[must_use]
    pub fn drag_elastic(mut self, value: crate::gesture::DragElastic) -> Self {
      self.state.motion = self.state.motion.drag_elastic(value);
      self
    }

    /// Enables or disables release momentum.
    #[must_use]
    pub fn drag_momentum(mut self, value: bool) -> Self {
      self.state.motion = self.state.motion.drag_momentum(value);
      self
    }

    /// Enables or disables drag direction locking.
    #[must_use]
    pub fn drag_direction_lock(mut self, value: bool) -> Self {
      self.state.motion = self.state.motion.drag_direction_lock(value);
      self
    }

    /// Enables or disables pointer initiation on this host.
    #[must_use]
    pub fn drag_listener(mut self, value: bool) -> Self {
      self.state.motion = self.state.motion.drag_listener(value);
      self
    }

    /// Selects axes returned to their origin after release.
    #[must_use]
    pub fn drag_snap_to_origin(mut self, value: crate::gesture::DragAxis) -> Self {
      self.state.motion = self.state.motion.drag_snap_to_origin(value);
      self
    }

    /// Allows an eligible ancestor to recognize the same pointer drag.
    #[must_use]
    pub fn drag_propagation(mut self, value: bool) -> Self {
      self.state.motion = self.state.motion.drag_propagation(value);
      self
    }

    /// Replaces release inertia and boundary spring behavior.
    #[must_use]
    pub fn drag_transition(mut self, value: crate::gesture::DragTransition) -> Self {
      self.state.motion = self.state.motion.drag_transition(value);
      self
    }

    /// Binds native drag offsets to mutable motion values.
    #[must_use]
    pub fn drag_motion_values(
      mut self,
      x: crate::motion_value::MotionValue<f32>,
      y: crate::motion_value::MotionValue<f32>,
    ) -> Self {
      self.state.motion = self.state.motion.drag_motion_values(x, y);
      self
    }

    /// Binds stable external drag controls.
    #[must_use]
    pub fn drag_controls(mut self, value: crate::gesture::DragControls) -> Self {
      self.state.motion = self.state.motion.drag_controls(value);
      self
    }

    /// Enables native scroll observation.
    #[must_use]
    pub fn observe_scroll(self, value: bool) -> Self {
      self.motion(MotionProps::new().observe_scroll(value))
    }

    /// Binds native scroll offsets to mutable motion values.
    #[must_use]
    pub fn scroll_motion_values(
      self,
      x: crate::motion_value::MotionValue<f32>,
      y: crate::motion_value::MotionValue<f32>,
    ) -> Self {
      self.motion(MotionProps::new().scroll_motion_values(x, y))
    }

    /// Binds viewport membership to a mutable zero-or-one motion value.
    #[must_use]
    pub fn in_view_motion_value(self, value: crate::motion_value::MotionValue<f32>) -> Self {
      self.motion(MotionProps::new().in_view_motion_value(value))
    }

    gesture_callback_methods! {
      on_hover_start;
      on_hover_end;
      on_tap_start;
      on_tap;
      on_tap_cancel;
      on_focus_start;
      on_focus_end;
      on_focus_visible_start;
      on_focus_visible_end;
      on_pan_session_start;
      on_pan_start;
      on_pan;
      on_pan_end;
      on_pan_cancel;
      on_drag_start;
      on_drag_direction_lock;
      on_drag;
      on_drag_end;
      on_drag_cancel;
      on_drag_momentum_complete;
      on_drag_constraints_measured;
      on_scroll_motion;
      on_viewport_enter;
      on_viewport_leave;
    }
  };
}

macro_rules! gesture_callback_methods {
  ($($event:ident;)+) => {
    $(
      #[doc = concat!("Runs native gesture callback `", stringify!($event), "` with event data.")]
      #[must_use]
      pub fn $event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionGestureEvent) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().$event(callback))
      }
    )+
  };
}

macro_rules! facade {
  ($name:ident, $native:ty, $docs:literal) => {
    #[doc = $docs]
    #[derive(Clone)]
    pub struct $name {
      pub(crate) state: Boxed<HostState<$native>>,
    }

    impl Default for $name {
      fn default() -> Self {
        Self::from_native(<$native>::default())
      }
    }

    impl $name {
      pub(crate) fn from_native(host: $native) -> Self {
        Self {
          state: Boxed::new(HostState {
            host,
            localizers: Vec::new(),
            children: Vec::new(),
            handlers: Vec::new(),
            key: None,
            element_ref: None,
            portal_target: None,
            motion: MotionProps::new(),
            semantic: None,
            overlay_reference: None,
          }),
        }
      }

      /// Sets the name used by Unity queries and `#name` USS selectors.
      #[must_use]
      pub fn name(mut self, value: impl Into<Prop<String>>) -> Self {
        self.state.host.visual_element_mut().name = value.into();
        self
      }

      /// Sets whether this element is locally enabled for interaction.
      #[must_use]
      pub fn enabled(mut self, value: impl Into<Prop<bool>>) -> Self {
        self.state.host.visual_element_mut().enabled = value.into();
        self
      }

      /// Sets whether pointer hit testing may select this element.
      #[must_use]
      pub fn picking_mode(mut self, value: impl Into<Prop<battlement::PickingMode>>) -> Self {
        self.state.host.visual_element_mut().picking_mode = value.into();
        self
      }

      /// Sets text directionality for this element's inheriting subtree.
      #[must_use]
      pub fn language_direction(
        mut self,
        value: impl Into<Prop<battlement::LanguageDirection>>,
      ) -> Self {
        self.state.host.visual_element_mut().language_direction = value.into();
        self
      }

      /// Sets whether this element may receive focus.
      #[must_use]
      pub fn focusable(mut self, value: impl Into<Prop<bool>>) -> Self {
        self.state.host.visual_element_mut().focusable = value.into();
        self
      }

      /// Sets this element's position in Unity's keyboard focus ring.
      #[must_use]
      pub fn tab_index(mut self, value: impl Into<Prop<i32>>) -> Self {
        self.state.host.visual_element_mut().tab_index = value.into();
        self
      }

      /// Sets whether focus requested here transfers to an eligible descendant.
      #[must_use]
      pub fn delegates_focus(mut self, value: impl Into<Prop<bool>>) -> Self {
        self.state.host.visual_element_mut().delegates_focus = value.into();
        self
      }

      /// Merges one composable focus declaration bundle into this host.
      #[must_use]
      pub fn focus_props(mut self, value: FocusProps) -> Self {
        value.apply(self.state.host.visual_element_mut());
        self
      }

      /// Requests focus once when this keyed host is mounted.
      #[must_use]
      pub fn auto_focus(mut self, value: bool) -> Self {
        self.state.host.visual_element_mut().auto_focus = Prop::Set(value);
        self
      }

      /// Sets whether this logical subtree is unavailable to user interaction.
      #[must_use]
      pub fn inert(mut self, value: bool) -> Self {
        self.state.host.visual_element_mut().inert = Prop::Set(value);
        self
      }

      /// Attaches this host's single semantic declaration.
      #[must_use]
      pub fn semantic(mut self, value: impl IntoOption<SemanticProps>) -> Self {
        if let Some(value) = value.into_option() {
          assert!(
            self.state.semantic.replace(value).is_none(),
            "a Reactant host accepts at most one SemanticProps bundle"
          );
        }
        self
      }

      /// Attaches an associated visible label's reference, semantics, and interaction.
      #[must_use]
      pub fn associated_label(mut self, value: impl IntoOption<AssociatedLabel>) -> Self {
        if let Some(value) = value.into_option() {
          self = self
            .element_ref(value.reference)
            .semantic(value.semantic)
            .interaction_props(value.interaction);
        }
        self
      }

      /// Attaches an associated control's reference and accessible behavior.
      #[must_use]
      pub fn associated_control<G: 'static, S>(self, value: AssociatedControl<G, S>) -> Self {
        self.element_ref(value.reference).behavior(value.behavior)
      }

      /// Merges ordinary callbacks returned by an accessible behavior hook.
      #[must_use]
      pub fn interaction_props<G: 'static>(mut self, value: InteractionProps<G>) -> Self {
        for handler in value.handlers {
          assert!(
            !self
              .state
              .handlers
              .iter()
              .any(|candidate| candidate.same_slot(&handler)),
            "duplicate Reactant interaction callback slot"
          );
          self.state.handlers.push(handler);
        }
        self
      }

      /// Attaches one accessible behavior's semantics, focus, and interaction atomically.
      #[must_use]
      pub fn behavior<G: 'static, S>(self, value: AccessibleBehavior<G, S>) -> Self {
        self
          .semantic(value.semantic)
          .focus_props(value.focus)
          .interaction_props(value.interaction)
          .motion(value.motion)
      }

      /// Appends one USS class name.
      #[must_use]
      pub fn class(mut self, value: impl Into<String>) -> Self {
        self.state.host = self.state.host.clone().class(value);
        self
      }

      /// Adds create-time rendering optimization hints.
      #[must_use]
      pub fn usage_hints(
        mut self,
        values: impl IntoIterator<Item = battlement::UsageHint>,
      ) -> Self {
        self.state.host = self.state.host.clone().usage_hints(values);
        self
      }

      /// Replaces this host's inline style declarations.
      #[must_use]
      pub fn style(mut self, value: Style) -> Self {
        self.state.host.visual_element_mut().style = value;
        self
      }

      /// Places this host when it is a Grid placement child.
      #[must_use]
      pub fn grid_item(mut self, value: impl Into<Prop<GridItem>>) -> Self {
        self.state.host.visual_element_mut().grid_item = value.into();
        self
      }

      /// Places and orders this host when it is a Stack placement child.
      #[must_use]
      pub fn stack_item(mut self, value: impl Into<Prop<StackItem>>) -> Self {
        self.state.host.visual_element_mut().stack_item = value.into();
        self
      }

      /// Makes this host sticky within its nearest supported scroll container.
      #[must_use]
      pub fn sticky(mut self, value: impl Into<Prop<Sticky>>) -> Self {
        self.state.host.visual_element_mut().sticky = value.into();
        self
      }

      /// Places this host through a target-owned overlay presentation slot.
      #[must_use]
      pub fn overlay_placement(mut self, value: impl Into<Prop<OverlayPlacement>>) -> Self {
        self.state.host.visual_element_mut().overlay_placement = value.into();
        self
      }

      /// Assigns typed identity within the sibling list.
      #[must_use]
      pub fn key<K: Clone + Eq + Hash + 'static>(mut self, key: K) -> Self {
        self.state.key = Some(ErasedKey::from_value(key));
        self
      }

      /// Attaches one exclusive element ref to this host.
      #[must_use]
      pub fn element_ref(mut self, element_ref: impl IntoOption<ElementRef>) -> Self {
        if let Some(element_ref) = element_ref.into_option() {
          self.state.element_ref = Some(element_ref);
        }
        self
      }

      /// Makes this host the unique container for `target`.
      #[must_use]
      pub fn portal_target(mut self, target: PortalTarget) -> Self {
        self.state.portal_target = Some(target);
        self
      }

      pub(crate) fn with_handler(mut self, handler: Handler) -> Self {
        self
          .state
          .handlers
          .retain(|existing| !existing.same_slot(&handler));
        self.state.handlers.push(handler);
        self
      }

      /// Paints a static clipped background without animation slots.
      #[must_use]
      pub fn paint(mut self, value: PaintStyle) -> Self {
        self.state.host.visual_element_mut().paint = Prop::Set(value);
        self
      }

      /// Applies a complete Motion authoring value.
      #[must_use]
      pub fn motion(mut self, value: MotionProps) -> Self {
        self.state.motion = self.state.motion.merge(value);
        self
      }

      /// Enables state-driven native layout projection.
      #[must_use]
      pub fn layout(self, value: crate::layout::Layout) -> Self {
        self.motion(MotionProps::new().layout(value))
      }

      /// Assigns typed identity for a shared-layout handoff.
      #[must_use]
      pub fn layout_id<K: Hash + 'static>(self, value: K) -> Self {
        self.motion(MotionProps::new().layout_id(value))
      }

      /// Marks this host as a projection-aware scroll boundary.
      #[must_use]
      pub fn layout_scroll(self, value: bool) -> Self {
        self.motion(MotionProps::new().layout_scroll(value))
      }

      /// Establishes a fixed projection root for descendants.
      #[must_use]
      pub fn layout_root(self, value: bool) -> Self {
        self.motion(MotionProps::new().layout_root(value))
      }

      /// Enables position projection and drag behavior for reordering.
      #[must_use]
      pub fn reorder_item(self, axis: crate::layout::ReorderAxis) -> Self {
        self.motion(MotionProps::new().reorder_item(axis))
      }

      /// Binds this host to stable typed animation controls.
      #[must_use]
      pub fn animation_controls<Name: VariantKey>(self, value: AnimationControls<Name>) -> Self {
        self.motion(MotionProps::new().animation_controls(value))
      }

      /// Marks this host as an animation-scope root.
      #[must_use]
      pub fn animation_scope(self, value: AnimationScope) -> Self {
        self.motion(MotionProps::new().animation_scope(value))
      }

      /// Assigns a stable name for closed scope selectors.
      #[must_use]
      pub fn motion_name(self, value: impl Into<String>) -> Self {
        self.motion(MotionProps::new().motion_name(value))
      }

      gesture_methods!();

      /// Selects the mount origin.
      #[must_use]
      pub fn initial(self, value: impl InitialValue) -> Self {
        self.motion(MotionProps::new().initial(value))
      }

      /// Selects the base animation target.
      #[must_use]
      pub fn animate(self, value: impl Into<MotionTarget>) -> Self {
        self.motion(MotionProps::new().animate(value))
      }

      /// Selects the presence-exit target.
      #[must_use]
      pub fn exit(self, value: impl Into<MotionTarget>) -> Self {
        self.motion(MotionProps::new().exit(value))
      }

      /// Replaces the default transition.
      #[must_use]
      pub fn transition(self, value: Transition) -> Self {
        self.motion(MotionProps::new().transition(value))
      }

      /// Runs when the direct Motion slot leaves its delay.
      #[must_use]
      pub fn on_animation_start<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.motion(MotionProps::new().on_start(callback))
      }

      /// Runs with the native boundary when the direct Motion slot starts.
      #[must_use]
      pub fn on_animation_start_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionLifecycleEvent) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().on_start_event(callback))
      }

      /// Runs for coalesced rendered-frame samples from the direct Motion slot.
      #[must_use]
      pub fn on_animation_update<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.motion(MotionProps::new().on_update(callback))
      }

      /// Runs with each coalesced direct Motion presentation sample.
      #[must_use]
      pub fn on_animation_update_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionPresentationSample) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().on_update_event(callback))
      }

      /// Runs when the direct Motion slot crosses a repeat boundary.
      #[must_use]
      pub fn on_animation_repeat<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.motion(MotionProps::new().on_repeat(callback))
      }

      /// Runs with the native boundary when the direct Motion slot repeats.
      #[must_use]
      pub fn on_animation_repeat_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionLifecycleEvent) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().on_repeat_event(callback))
      }

      /// Runs after the direct finite Motion slot completes.
      #[must_use]
      pub fn on_animation_complete<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.motion(MotionProps::new().on_complete(callback))
      }

      /// Runs with the native boundary after direct Motion completion.
      #[must_use]
      pub fn on_animation_complete_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionLifecycleEvent) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().on_complete_event(callback))
      }

      /// Runs if imperative playback stops the direct Motion slot.
      #[must_use]
      pub fn on_animation_stop<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.motion(MotionProps::new().on_stop(callback))
      }

      /// Runs with the native boundary when direct Motion stops.
      #[must_use]
      pub fn on_animation_stop_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionLifecycleEvent) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().on_stop_event(callback))
      }

      /// Runs when the direct Motion slot is cancelled or superseded.
      #[must_use]
      pub fn on_animation_cancel<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.motion(MotionProps::new().on_cancel(callback))
      }

      /// Runs with the native boundary when direct Motion is cancelled.
      #[must_use]
      pub fn on_animation_cancel_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, &battlement::MotionLifecycleEvent) + 'static,
      ) -> Self {
        self.motion(MotionProps::new().on_cancel_event(callback))
      }

      /// Replaces the named variant definitions available to this host.
      #[must_use]
      pub fn variants<Name, Custom>(self, value: Variants<Name, Custom>) -> Self
      where
        Name: VariantKey,
        Custom: VariantData,
      {
        self.motion(MotionProps::new().variants(value))
      }

      /// Selects one named variant target.
      #[must_use]
      pub fn animate_variant<Name: VariantKey>(self, value: Name) -> Self {
        self.motion(MotionProps::new().animate_variant(value))
      }

      /// Selects one named mount origin.
      #[must_use]
      pub fn initial_variant<Name: VariantKey>(self, value: Name) -> Self {
        self.motion(MotionProps::new().initial_variant(value))
      }

      /// Selects an ordered named mount-origin list.
      #[must_use]
      pub fn initial_variants<Name: VariantKey>(
        self,
        values: impl IntoIterator<Item = Name>,
      ) -> Self {
        self.motion(MotionProps::new().initial_variants(values))
      }

      /// Selects one named presence-exit target.
      #[must_use]
      pub fn exit_variant<Name: VariantKey>(self, value: Name) -> Self {
        self.motion(MotionProps::new().exit_variant(value))
      }

      /// Selects an ordered named presence-exit list.
      #[must_use]
      pub fn exit_variants<Name: VariantKey>(self, values: impl IntoIterator<Item = Name>) -> Self {
        self.motion(MotionProps::new().exit_variants(values))
      }

      /// Selects an ordered variant list.
      #[must_use]
      pub fn animate_variants<Name: VariantKey>(
        self,
        values: impl IntoIterator<Item = Name>,
      ) -> Self {
        self.motion(MotionProps::new().animate_variants(values))
      }

      /// Supplies custom data to computed variants.
      #[must_use]
      pub fn custom<T: VariantData>(self, value: T) -> Self {
        self.motion(MotionProps::new().custom(value))
      }

      /// Enables or disables logical parent variant propagation.
      #[must_use]
      pub fn inherit_variants(self, value: bool) -> Self {
        self.motion(MotionProps::new().inherit_variants(value))
      }

      /// Merges a typed hover-state style.
      #[must_use]
      pub fn hover_style(mut self, value: impl IntoPseudoStyle) -> Self {
        let current = self.state.motion.css.hover.take().unwrap_or_default();
        self.state.motion.css.hover = Some(current.merge(value.into_pseudo_style()));
        self
      }

      /// Merges a typed focus-state style.
      #[must_use]
      pub fn focus_style(mut self, value: impl IntoPseudoStyle) -> Self {
        let current = self.state.motion.css.focus.take().unwrap_or_default();
        self.state.motion.css.focus = Some(current.merge(value.into_pseudo_style()));
        self
      }

      /// Merges a typed active-state style.
      #[must_use]
      pub fn active_style(mut self, value: impl IntoPseudoStyle) -> Self {
        let current = self.state.motion.css.active.take().unwrap_or_default();
        self.state.motion.css.active = Some(current.merge(value.into_pseudo_style()));
        self
      }

      /// Merges a typed disabled-state style.
      #[must_use]
      pub fn disabled_style(mut self, value: impl IntoPseudoStyle) -> Self {
        let current = self.state.motion.css.disabled.take().unwrap_or_default();
        self.state.motion.css.disabled = Some(current.merge(value.into_pseudo_style()));
        self
      }

      /// Installs typed CSS transition rules.
      #[must_use]
      pub fn style_transition(mut self, value: StyleTransition) -> Self {
        self.state.motion.css.transition = value;
        self
      }

      /// Appends one reusable CSS-style animation.
      #[must_use]
      pub fn animation(mut self, value: Animation) -> Self {
        self.state.motion.css.animations.push(value);
        self
      }

      /// Replaces the ordered CSS-style animation list.
      #[must_use]
      pub fn animations(mut self, values: impl IntoIterator<Item = Animation>) -> Self {
        self.state.motion.css.animations = values.into_iter().collect();
        self
      }

      /// Appends a decoration behind host content.
      #[must_use]
      pub fn before(mut self, value: Decoration) -> Self {
        self.state.motion.css.before.push(value);
        self
      }

      /// Replaces decorations behind host content.
      #[must_use]
      pub fn before_all(mut self, values: impl IntoIterator<Item = Decoration>) -> Self {
        self.state.motion.css.before = values.into_iter().collect();
        self
      }

      /// Appends a decoration above host content.
      #[must_use]
      pub fn after(mut self, value: Decoration) -> Self {
        self.state.motion.css.after.push(value);
        self
      }

      /// Replaces decorations above host content.
      #[must_use]
      pub fn after_all(mut self, values: impl IntoIterator<Item = Decoration>) -> Self {
        self.state.motion.css.after = values.into_iter().collect();
        self
      }
    }

    impl Render for $name {}

    impl Sealed for $name {
      fn descriptor(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
      }

      fn render_into(&self, sink: &mut RenderSink<'_>) {
        host_facade::lower::<Self, $native>(self.state.as_ref(), None, sink);
      }

      fn render_owned(self, sink: &mut RenderSink<'_>) {
        Rc::new(self).render_shared(sink);
      }

      fn render_shared(self: Rc<Self>, sink: &mut RenderSink<'_>) {
        let retained_render = self.state.key.as_ref().map(|_| Node {
          render: Rc::clone(&self) as Rc<dyn crate::render_value::ErasedRender>,
          descriptor: TypeId::of::<Self>(),
        });
        host_facade::lower::<Self, $native>(self.state.as_ref(), retained_render, sink);
      }
    }
  };
}

facade!(
  View,
  UiVisualElement,
  "Unity UI Toolkit's neutral, general-purpose layout and hierarchy element.\n\nUse a `View` to group children, apply shared style, or create a structural region without control behavior. It lowers to one [`UiVisualElement`] and adds logical children directly to that host's content container. Unlike [`Box`], it has no themed box treatment.\n\nSee Unity's [VisualElement manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-VisualElement.html)."
);
facade!(
  Flex,
  UiFlex,
  "A native Flex layout container with independent row and column gaps.\n\nLogical child order remains stable while direction and wrapping affect presentation."
);
facade!(
  Grid,
  UiGrid,
  "A deterministic track-based layout container with explicit and implicit rows and columns.\n\nGrid uses stable native slots and preserves logical child order while tracks determine presentation."
);
facade!(
  Stack,
  UiStack,
  "An isolated overlapping layout container with explicit layer order.\n\nStack uses stable native slots for placement while preserving logical hierarchy and host identity."
);
facade!(
  Box,
  UiBox,
  "A themed Unity UI Toolkit container with a visible box treatment.\n\n`Box` has the hierarchy and layout role of [`View`], while Unity's `.unity-box` USS class supplies the themed background and border. Use it to visually group related content.\n\nSee Unity's [Box manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Box.html)."
);
facade!(
  Label,
  UiLabel,
  "A Unity UI Toolkit text leaf for titles, captions, and descriptions.\n\nText styles affect the rendered text and layout styles affect its box. Use [`Button`] when the text should activate an action.\n\nSee Unity's [Label manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Label.html)."
);
facade!(
  TextElement,
  UiTextElement,
  "A leaf Unity UI Toolkit text element for styled, rich, or selectable text.\n\nUnlike [`Label`], this maps directly to Unity's `TextElement` base class. Selection permits copying but not editing; use [`TextField`] for input. Rich-text link regions can be observed through the `on_link_*` handlers.\n\nSee Unity's [TextElement manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-TextElement.html)."
);
facade!(
  TextField,
  UiTextField,
  "A controlled editable text input with a native local draft.\n\nTyping emits `Input` proposals without changing Rust's authoritative value. Single-line Enter and focus loss emit a committed proposal; Escape restores the latest authored value. Cursor and selection indices use UTF-16 code units.\n\nSee Unity's [TextField manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-TextField.html)."
);
facade!(
  Toggle,
  UiToggle,
  "A controlled Boolean field rendered as a checkbox-style toggle.\n\nUse it for an independent on/off setting. Interaction proposes a value through `ValueCommitted`; Rust remains authoritative until the next render accepts it.\n\nSee Unity's [Toggle manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Toggle.html)."
);
facade!(
  RadioButton,
  UiRadioButton,
  "A controlled Boolean option with Unity's radio-button appearance.\n\nThe nearest ancestor [`GroupBox`] defines mutual-exclusion scope. User activation proposes a committed value, while Rust's authored value remains authoritative.\n\nSee Unity's [RadioButton manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RadioButton.html)."
);
facade!(
  RadioButtonGroup,
  UiRadioButtonGroup,
  "A controlled single-choice field that keeps every option visible.\n\nChoices are native radio controls rather than logical children. Activation proposes a zero-based index, and Rust remains authoritative until `selected_index` changes.\n\nSee Unity's [RadioButtonGroup manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RadioButtonGroup.html)."
);
facade!(
  ToggleButtonGroup,
  UiToggleButtonGroup,
  "A controlled group that presents direct [`Button`] children as toggles.\n\nIt selects one button by default; multiple and empty selection are separately configurable. Selected indices address direct children in visual order and interaction emits committed proposals.\n\nSee Unity's [ToggleButtonGroup manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ToggleButtonGroup.html)."
);
facade!(
  DropdownField,
  UiDropdownField,
  "A controlled single-choice field that opens its options in a popup.\n\nUse it when a permanently visible option list would consume too much space. Selection is provisional until Rust authors the accepted [`LocalizedChoice`].\n\nSee Unity's [DropdownField manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-DropdownField.html)."
);
facade!(
  Button,
  UiButton,
  "A Unity UI Toolkit control for a discrete pointer or navigation-submit command.\n\nUnity supplies standard button appearance and interaction states. Reactant forwards activations only when an `on_click` handler is authored. Logical children can supply styled labels and decorative content.\n\nSee Unity's [Button manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Button.html)."
);
facade!(
  RepeatButton,
  UiRepeatButton,
  "A leaf button that repeatedly activates while held.\n\nUnity invokes the action after the initial delay and then at each positive interval until release. Timed activations arrive through `on_click` without a Rust-side timer.\n\nSee Unity's [RepeatButton manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RepeatButton.html)."
);
facade!(
  GroupBox,
  UiGroupBox,
  "A Unity UI Toolkit container that groups related controls under an optional title.\n\nAn empty title omits the native title label. Group boxes establish native radio-button scope without imposing [`Box`]'s themed border and background.\n\nSee Unity's [GroupBox manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-GroupBox.html)."
);
facade!(
  PopupWindow,
  UiPopupWindow,
  "A popup-styled text container with a public logical content container.\n\nIt supplies popup card structure, not positioning, modality, dismissal, or lifecycle behavior. The application owns when and where it renders; the content-container part can be styled independently.\n\nSee Unity's [PopupWindow manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-PopupWindow.html)."
);
facade!(
  ScrollView,
  UiScrollView,
  "A viewport that displays arbitrary child content through a scrollable frame.\n\nChildren enter Unity's unbounded content container. Axis mode, scroller visibility, nested interaction, touch deceleration, elasticity, and authored panel-pixel offset mirror the native control.\n\nSee Unity's [ScrollView manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ScrollView.html) and [scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.ScrollView.html)."
);
facade!(
  Scroller,
  UiScroller,
  "A controlled scrollbar that proposes floating-point values within a range.\n\nInteraction emits changing and committed proposals, then restores Rust's latest authored value. A scroller includes decrement and increment buttons around its internal slider.\n\nSee Unity's [Scroller manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Scroller.html)."
);
facade!(
  Slider,
  UiSlider,
  "A controlled floating-point field for approximate adjustment within a range.\n\nDragging, track clicks, and keyboard input produce provisional changing and committed proposals. A positive page size is a percentage of the complete range; zero moves directly to a track-click position.\n\nSee Unity's [Slider manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Slider.html)."
);
facade!(
  SliderInt,
  UiSliderInt,
  "A controlled integer field for approximate adjustment within a range.\n\nIt shares [`Slider`]'s interaction model while proposing integral values. Rust remains authoritative until a render accepts the changing or committed proposal.\n\nSee Unity's [SliderInt manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-SliderInt.html)."
);
facade!(
  MinMaxSlider,
  UiMinMaxSlider,
  "A controlled floating-point interval selector with two draggable thumbs.\n\nThe authored limits constrain the track and `min_value`/`max_value` select its ordered interval. Thumb and range dragging produce changing and committed range proposals.\n\nSee Unity's [MinMaxSlider manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-MinMaxSlider.html)."
);
facade!(
  ProgressBar,
  UiProgressBar,
  "A read-only indicator that visualizes progress through a numeric range.\n\nThe low and high values define the range, `value` controls the filled proportion, and `title` draws explanatory text over the track. Unity clamps out-of-range display values.\n\nSee Unity's [ProgressBar manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ProgressBar.html)."
);
facade!(
  Tab,
  UiTab,
  "One labeled, optionally icon-bearing page inside a [`TabView`].\n\nThe text and icon form its header while logical children form page content. Closing is a proposal handled by the parent tab view; it never destroys the tab automatically.\n\nSee Unity's [Tab manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Tab.html)."
);
facade!(
  TabView,
  UiTabView,
  "A controlled collection of [`Tab`] pages with native headers.\n\nOnly tabs are valid direct children. Selection, close, and reorder gestures are proposals; accept them by changing the authored selected index or logical child collection.\n\nSee Unity's [TabView manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-TabView.html)."
);
facade!(
  Image,
  UiImage,
  "A Unity UI Toolkit image for raster, sprite, vector, or rendered content.\n\nUse it when graphics participate in layout or require direct fit, crop, tint, or sampled-region control. Images are logical leaves and source leases live until replacement or destruction.\n\nSee Unity's [Image manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Image.html) and [scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.Image.html)."
);

macro_rules! empty_constructor {
  ($($name:ident => $native:ty),+ $(,)?) => {
    $(
      impl $name {
        #[doc = concat!("Creates an empty [`", stringify!($name), "`] façade.")]
        #[must_use]
        pub fn new() -> Self {
          Self::from_native(<$native>::new())
        }
      }
    )+
  };
}

empty_constructor!(
  View => UiVisualElement,
  Flex => UiFlex,
  Grid => UiGrid,
  Stack => UiStack,
  Box => UiBox,
  TextField => UiTextField,
  Toggle => UiToggle,
  RadioButton => UiRadioButton,
  RadioButtonGroup => UiRadioButtonGroup,
  ToggleButtonGroup => UiToggleButtonGroup,
  DropdownField => UiDropdownField,
  GroupBox => UiGroupBox,
  PopupWindow => UiPopupWindow,
  ScrollView => UiScrollView,
  Scroller => UiScroller,
  Slider => UiSlider,
  SliderInt => UiSliderInt,
  MinMaxSlider => UiMinMaxSlider,
  ProgressBar => UiProgressBar,
  TabView => UiTabView,
  Image => UiImage,
);

macro_rules! text_constructor {
  ($($name:ident => $native:ty),+ $(,)?) => {
    $(
      impl $name {
        #[doc = concat!("Creates a [`", stringify!($name), "`] with authored text.")]
        #[must_use]
        pub fn new(text: LocalizedString) -> Self {
          Self::from_native(<$native>::new("")).text(text)
        }
      }
    )+
  };
}

text_constructor!(
  Label => UiLabel,
  TextElement => UiTextElement,
  Button => UiButton,
  Tab => UiTab,
);

impl View {
  /// Creates a visual layer that does not participate in pointer picking.
  #[must_use]
  pub fn decorative() -> Self {
    Self::new().picking_mode(PickingMode::Ignore)
  }
}

impl RepeatButton {
  /// Creates a repeat button with its initial timing contract.
  #[must_use]
  pub fn new(text: LocalizedString, delay_ms: u32, interval_ms: NonZeroU32) -> Self {
    Self::from_native(UiRepeatButton::new("", delay_ms, interval_ms)).text(text)
  }
}

macro_rules! container {
  ($($name:ident),+ $(,)?) => {
    $(
      impl $name {
        /// Appends one logical child.
        #[must_use]
        pub fn child(mut self, child: impl Render) -> Self {
          self.state.children.push(Node::new(child));
          self
        }

        /// Appends logical children in iterator order.
        #[must_use]
        pub fn children<R: Render>(
          mut self,
          children: impl IntoIterator<Item = R>,
        ) -> Self {
          self.state.children.extend(children.into_iter().map(Node::new));
          self
        }
      }
    )+
  };
}

container!(
  Button,
  View,
  Flex,
  Grid,
  Stack,
  Box,
  ToggleButtonGroup,
  GroupBox,
  PopupWindow,
  ScrollView,
  Tab,
  TabView
);
