//! Direct Battlement primitive rendering and child composition.

#![allow(private_interfaces)]

use std::any::TypeId;

use battlement::{
  Box, Button, DropdownField, GroupBox, Image, Label, MinMaxSlider, PopupWindow, ProgressBar, Prop,
  RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, Slider, SliderInt, Tab,
  TabView, TextElement, TextField, Toggle, ToggleButtonGroup, UiNode, VisualElement,
  VisualElementProperties,
};

use crate::{
  render::{Render, RenderSink},
  render_value::Sealed,
};

/// Adds declarative children to Battlement primitives that accept them.
///
/// Primitive properties must be set before the first child because this
/// adapter deliberately does not expose primitive-specific property methods.
/// Leaf controls do not implement this trait.
///
/// ```
/// use battlement::{Label, VisualElement};
/// use battlement_reactant::primitive::ContainerRenderExt;
///
/// let _panel = VisualElement::new()
///     .name("status")
///     .child(Label::new("Ready"))
///     .child(Label::new("Connected"));
/// ```
///
/// ```compile_fail
/// use battlement::Label;
/// use battlement_reactant::primitive::ContainerRenderExt;
///
/// let _invalid = Label::new("leaf").child(Label::new("child"));
/// ```
///
/// ```compile_fail
/// use battlement::{Label, Scroller};
/// use battlement_reactant::primitive::ContainerRenderExt;
///
/// let _invalid = Scroller::new().child(Label::new("child"));
/// ```
///
/// ```compile_fail
/// use battlement::{Label, VisualElement};
/// use battlement_reactant::primitive::ContainerRenderExt;
///
/// let _invalid = VisualElement::new().child(Label::new("child")).name("late");
/// ```
pub trait ContainerRenderExt: private::Container + Sized {
  /// Appends one render value as a logical child.
  fn child<R: Render>(self, child: R) -> Children<Self, R> {
    Children {
      host: self,
      children: child,
    }
  }

  /// Collects and appends homogeneous logical children immediately.
  fn children<I>(self, children: I) -> Children<Self, Vec<I::Item>>
  where
    I: IntoIterator,
    I::Item: Render,
  {
    self.child(children.into_iter().collect())
  }
}

/// A native container and the render values authored beneath it.
///
/// This adapter does not introduce another native element. Further child calls
/// append values while retaining the original primitive as the host.
pub struct Children<H, C> {
  host: H,
  children: C,
}

impl<H, C> Children<H, C> {
  /// Appends one render value after the existing logical children.
  pub fn child<R: Render>(self, child: R) -> Children<H, (C, R)> {
    Children {
      host: self.host,
      children: (self.children, child),
    }
  }

  /// Collects and appends homogeneous logical children immediately.
  pub fn children<I>(self, children: I) -> Children<H, (C, Vec<I::Item>)>
  where
    I: IntoIterator,
    I::Item: Render,
  {
    self.child(children.into_iter().collect())
  }
}

impl<H: private::Host, C: Render> Render for Children<H, C> {}

impl<H: private::Host, C: Render> Sealed for Children<H, C> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<H>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    validate_subscriptions(&self.host);
    sink.push_host_with_children::<H>(
      |object_id, children| UiNode::new(object_id, self.host.clone()).children(children),
      |children| self.children.render_into(children),
    );
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    validate_subscriptions(&self.host);
    sink.push_host_with_children::<H>(
      |object_id, children| UiNode::new(object_id, self.host).children(children),
      |children| self.children.render_owned(children),
    );
  }
}

fn validate_subscriptions(value: &impl VisualElementProperties) {
  let visual = value.visual_element();
  let authored_events = matches!(&visual.events, Prop::Set(values) if !values.is_empty());
  let authored_routes =
    matches!(&visual.event_subscriptions, Prop::Set(values) if !values.is_empty());
  assert!(
    !authored_events && !authored_routes,
    "Reactant owns native event subscriptions"
  );
}

macro_rules! host_primitives {
  ($($primitive:ty),+ $(,)?) => {
    $(
      impl private::Host for $primitive {}

      impl Render for $primitive {}

      impl Sealed for $primitive {
        fn descriptor(&self) -> TypeId {
          TypeId::of::<Self>()
        }

        fn render_into(&self, sink: &mut RenderSink<'_>) {
          validate_subscriptions(self);
          sink.push_host::<Self>(|object_id| UiNode::new(object_id, self.clone()));
        }
      }
    )+
  };
}

macro_rules! container_primitives {
  ($($primitive:ty),+ $(,)?) => {
    $(
      impl private::Container for $primitive {}
      impl ContainerRenderExt for $primitive {}
    )+
  };
}

host_primitives!(
  VisualElement,
  Box,
  Label,
  TextElement,
  TextField,
  Toggle,
  RadioButton,
  RadioButtonGroup,
  ToggleButtonGroup,
  DropdownField,
  Button,
  RepeatButton,
  GroupBox,
  PopupWindow,
  ScrollView,
  Scroller,
  Slider,
  SliderInt,
  MinMaxSlider,
  ProgressBar,
  Tab,
  TabView,
  Image,
);

container_primitives!(
  VisualElement,
  Box,
  ToggleButtonGroup,
  GroupBox,
  PopupWindow,
  ScrollView,
  Tab,
  TabView,
);

pub(crate) mod private {
  use battlement::{UiElement, VisualElementProperties};

  pub trait Host: Clone + Into<UiElement> + VisualElementProperties + 'static {}

  pub trait Container {}
}
