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
//! let _ = Button::new("Save").child(Label::new("invalid"));
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

use std::{any::TypeId, hash::Hash, num::NonZeroU32};

use battlement::{
  MotionCallbackSubscriptions, MotionClockSource, MotionDescriptor, MotionEasing, MotionGeneration,
  MotionLayer, MotionProperty, MotionPropertyTrack, MotionPropertyValue, MotionRepeat,
  MotionRepeatType, MotionSlotDescriptor, MotionSlotId, MotionTargetDescriptor, MotionValue, Prop,
  ReducedMotionPolicy, Style, TransitionDefinition, TransitionGenerator, UiBox, UiButton,
  UiDropdownField, UiElement, UiGroupBox, UiImage, UiLabel, UiMinMaxSlider, UiPopupWindow,
  UiProgressBar, UiRadioButton, UiRadioButtonGroup, UiRepeatButton, UiScrollView, UiScroller,
  UiSlider, UiSliderInt, UiTab, UiTabView, UiTextElement, UiTextField, UiToggle,
  UiToggleButtonGroup, UiVisualElement, UiVisualElementProperties,
};

use crate::{
  element_ref::ElementRef,
  event_handler::Handler,
  key::ErasedKey,
  portal::PortalTarget,
  render::{FacadeMetadata, Node, Render, RenderSink},
  render_value::Sealed,
};

#[derive(Clone)]
pub(crate) struct HostState<H> {
  pub(crate) host: H,
  pub(crate) children: Vec<Node>,
  pub(crate) handlers: Vec<Handler>,
  pub(crate) key: Option<ErasedKey>,
  pub(crate) element_ref: Option<ElementRef>,
  pub(crate) portal_target: Option<PortalTarget>,
  pub(crate) protocol_motion: Option<ProtocolMotion>,
}

#[derive(Clone, Copy)]
pub(crate) struct ProtocolMotion {
  elapsed_micros: u64,
  generation: u32,
}

impl ProtocolMotion {
  pub(crate) fn descriptor(self, host_id: battlement::ObjectId) -> MotionDescriptor {
    let immediate = TransitionDefinition {
      generator: TransitionGenerator::Immediate,
      delay_micros: 0,
      repeat: MotionRepeat::None,
      repeat_delay_micros: 0,
      repeat_type: MotionRepeatType::Loop,
    };
    MotionDescriptor {
      descriptor_id: host_id,
      host_id,
      generation: MotionGeneration(self.generation),
      static_baseline: vec![MotionPropertyValue {
        property: MotionProperty::Opacity,
        value: MotionValue::Scalar(1.0),
      }],
      initial: Some(MotionTargetDescriptor {
        tracks: vec![MotionPropertyTrack {
          property: MotionProperty::Opacity,
          values: vec![MotionValue::Scalar(0.0)],
          times: None,
          transition: immediate,
        }],
        transition_end: Vec::new(),
      }),
      initial_disabled: false,
      slots: vec![MotionSlotDescriptor {
        slot: MotionSlotId(1),
        generation: MotionGeneration(self.generation),
        layer: MotionLayer::Animate,
        target: MotionTargetDescriptor {
          tracks: vec![MotionPropertyTrack {
            property: MotionProperty::Opacity,
            values: vec![MotionValue::Scalar(1.0)],
            times: None,
            transition: TransitionDefinition {
              generator: TransitionGenerator::Tween {
                duration_micros: 1_000_000,
                easings: vec![MotionEasing::Linear],
                times: None,
              },
              delay_micros: -i64::try_from(self.elapsed_micros)
                .expect("motion checkpoint must fit signed microseconds"),
              repeat: MotionRepeat::None,
              repeat_delay_micros: 0,
              repeat_type: MotionRepeatType::Loop,
            },
          }],
          transition_end: Vec::new(),
        },
        callbacks: MotionCallbackSubscriptions::default(),
      }],
      clock: MotionClockSource::Controlled(host_id),
      reduced_motion: ReducedMotionPolicy::Never,
    }
  }
}

macro_rules! facade {
  ($name:ident, $native:ty, $docs:literal) => {
    #[doc = $docs]
    #[derive(Clone)]
    pub struct $name {
      pub(crate) state: HostState<$native>,
    }

    impl Default for $name {
      fn default() -> Self {
        Self::from_native(<$native>::default())
      }
    }

    impl $name {
      pub(crate) fn from_native(host: $native) -> Self {
        Self {
          state: HostState {
            host,
            children: Vec::new(),
            handlers: Vec::new(),
            key: None,
            element_ref: None,
            portal_target: None,
            protocol_motion: None,
          },
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

      /// Assigns typed identity within the sibling list.
      #[must_use]
      pub fn key<K: Clone + Eq + Hash + 'static>(mut self, key: K) -> Self {
        self.state.key = Some(ErasedKey::from_value(key));
        self
      }

      /// Attaches one exclusive element ref to this host.
      #[must_use]
      pub fn element_ref(mut self, element_ref: ElementRef) -> Self {
        self.state.element_ref = Some(element_ref);
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

      #[doc(hidden)]
      #[must_use]
      pub fn __protocol_motion(mut self, elapsed_micros: u64, generation: u32) -> Self {
        self.state.protocol_motion = Some(ProtocolMotion {
          elapsed_micros,
          generation,
        });
        self
      }
    }

    impl Render for $name {}

    impl Sealed for $name {
      fn descriptor(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
      }

      fn render_into(&self, sink: &mut RenderSink<'_>) {
        self::lower::<Self, $native>(self.state.clone(), sink);
      }

      fn render_owned(self, sink: &mut RenderSink<'_>) {
        self::lower::<Self, $native>(self.state, sink);
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
  "A controlled single-choice field that opens its options in a popup.\n\nUse it when a permanently visible option list would consume too much space. Selection is provisional until Rust authors the accepted [`battlement::Choice`].\n\nSee Unity's [DropdownField manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-DropdownField.html)."
);
facade!(
  Button,
  UiButton,
  "A Unity UI Toolkit control for a discrete pointer or navigation-submit command.\n\nUnity supplies standard button appearance and interaction states. Reactant forwards activations only when an `on_click` handler is authored. This host is a logical leaf.\n\nSee Unity's [Button manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Button.html)."
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
        pub fn new(text: impl Into<String>) -> Self {
          Self::from_native(<$native>::new(text))
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

impl RepeatButton {
  /// Creates a repeat button with its initial timing contract.
  #[must_use]
  pub fn new(text: impl Into<String>, delay_ms: u32, interval_ms: NonZeroU32) -> Self {
    Self::from_native(UiRepeatButton::new(text, delay_ms, interval_ms))
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
  View,
  Box,
  ToggleButtonGroup,
  GroupBox,
  PopupWindow,
  ScrollView,
  Tab,
  TabView
);

fn lower<R: 'static, H: Into<UiElement>>(state: HostState<H>, sink: &mut RenderSink<'_>) {
  let HostState {
    host,
    children: host_children,
    handlers,
    key,
    element_ref,
    portal_target,
    protocol_motion,
  } = state;
  let element = host.into();
  assert_eq!(
    TypeId::of::<R>(),
    facade_descriptor(&element),
    "Reactant façade lowered through the wrong native host catalog entry"
  );
  sink.push_facade::<R>(
    FacadeMetadata {
      key,
      element_ref,
      portal_target,
      handlers,
      protocol_motion,
    },
    element,
    |sink| {
      for child in host_children {
        child.render_owned(sink);
      }
    },
  );
}

fn facade_descriptor(element: &UiElement) -> TypeId {
  match element {
    UiElement::VisualElement(_) => TypeId::of::<View>(),
    UiElement::Box(_) => TypeId::of::<Box>(),
    UiElement::Label(_) => TypeId::of::<Label>(),
    UiElement::TextElement(_) => TypeId::of::<TextElement>(),
    UiElement::TextField(_) => TypeId::of::<TextField>(),
    UiElement::Toggle(_) => TypeId::of::<Toggle>(),
    UiElement::RadioButton(_) => TypeId::of::<RadioButton>(),
    UiElement::RadioButtonGroup(_) => TypeId::of::<RadioButtonGroup>(),
    UiElement::ToggleButtonGroup(_) => TypeId::of::<ToggleButtonGroup>(),
    UiElement::DropdownField(_) => TypeId::of::<DropdownField>(),
    UiElement::Button(_) => TypeId::of::<Button>(),
    UiElement::RepeatButton(_) => TypeId::of::<RepeatButton>(),
    UiElement::GroupBox(_) => TypeId::of::<GroupBox>(),
    UiElement::PopupWindow(_) => TypeId::of::<PopupWindow>(),
    UiElement::ScrollView(_) => TypeId::of::<ScrollView>(),
    UiElement::Scroller(_) => TypeId::of::<Scroller>(),
    UiElement::Slider(_) => TypeId::of::<Slider>(),
    UiElement::SliderInt(_) => TypeId::of::<SliderInt>(),
    UiElement::MinMaxSlider(_) => TypeId::of::<MinMaxSlider>(),
    UiElement::ProgressBar(_) => TypeId::of::<ProgressBar>(),
    UiElement::Tab(_) => TypeId::of::<Tab>(),
    UiElement::TabView(_) => TypeId::of::<TabView>(),
    UiElement::Image(_) => TypeId::of::<Image>(),
  }
}
