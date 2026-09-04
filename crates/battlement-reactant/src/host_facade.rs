use std::{any::TypeId, boxed::Box as Boxed, rc::Rc};

use battlement::UiElement;

use crate::{
  element_ref::ElementRef,
  event_handler::Handler,
  host::{
    Box, Button, DropdownField, Flex, Grid, GroupBox, Image, Label, MinMaxSlider, PopupWindow,
    ProgressBar, RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, Slider,
    SliderInt, Stack, Tab, TabView, TextElement, TextField, Toggle, ToggleButtonGroup, View,
  },
  key::ErasedKey,
  motion::MotionProps,
  overlay::OverlayReference,
  portal::PortalTarget,
  render::{Node, RenderSink},
  render_value::Sealed,
  semantics::SemanticProps,
};

#[derive(Clone)]
pub(crate) struct HostState<H> {
  pub(crate) host: H,
  pub(crate) localizers: Vec<Rc<dyn Fn(H) -> H>>,
  pub(crate) children: Vec<Node>,
  pub(crate) handlers: Vec<Handler>,
  pub(crate) key: Option<ErasedKey>,
  pub(crate) element_ref: Option<ElementRef>,
  pub(crate) portal_target: Option<PortalTarget>,
  pub(crate) motion: MotionProps,
  pub(crate) semantic: Option<SemanticProps>,
  pub(crate) overlay_reference: Option<OverlayReference>,
}

pub(crate) struct FacadeMetadata {
  pub(crate) key: Option<ErasedKey>,
  pub(crate) element_ref: Option<ElementRef>,
  pub(crate) portal_target: Option<PortalTarget>,
  pub(crate) handlers: Vec<Handler>,
  pub(crate) motion: MotionProps,
  pub(crate) semantic: Option<SemanticProps>,
  pub(crate) retained_render: Option<Node>,
  pub(crate) overlay_reference: Option<OverlayReference>,
}

pub(crate) fn lower<R: 'static, H: Clone + Into<UiElement>>(
  state: &HostState<H>,
  retained_render: Option<Node>,
  sink: &mut RenderSink<'_>,
) {
  let (metadata, element) = self::prepare_input(state, retained_render);
  assert_eq!(
    TypeId::of::<R>(),
    self::facade_descriptor(&element),
    "Reactant facade lowered through the wrong native host catalog entry"
  );
  sink.push_facade::<R>(metadata, element, |sink| {
    for child in &state.children {
      child.render_into(sink);
    }
  });
}

fn prepare_input<H: Clone + Into<UiElement>>(
  state: &HostState<H>,
  retained_render: Option<Node>,
) -> (Boxed<FacadeMetadata>, Boxed<UiElement>) {
  (
    Boxed::new(FacadeMetadata {
      key: state.key.clone(),
      element_ref: state.element_ref.clone(),
      portal_target: state.portal_target.clone(),
      handlers: state.handlers.clone(),
      motion: state.motion.clone(),
      semantic: state.semantic.clone(),
      retained_render,
      overlay_reference: state.overlay_reference.clone(),
    }),
    Boxed::new(
      state
        .localizers
        .iter()
        .fold(state.host.clone(), |host, localize| localize(host))
        .into(),
    ),
  )
}

fn facade_descriptor(element: &UiElement) -> TypeId {
  match element {
    UiElement::VisualElement(_) => TypeId::of::<View>(),
    UiElement::Flex(_) => TypeId::of::<Flex>(),
    UiElement::Grid(_) => TypeId::of::<Grid>(),
    UiElement::Stack(_) => TypeId::of::<Stack>(),
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
