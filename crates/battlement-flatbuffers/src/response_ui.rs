use std::collections::HashSet;

use battlement::{Prop, UiElement, UiVisualElementProperties};
use flatbuffers::{Allocator, FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use crate::{
  ProtocolError, common_generated as common, ui_event_generated as event_wire, ui_generated as wire,
};

pub(crate) fn write_document<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::UiDocument,
) -> Result<WIPOffset<wire::UiDocument<'a>>, ProtocolError> {
  let root_element = write_visual_element(builder, &value.element)?;
  let root_child_ids = value
    .children
    .iter()
    .map(|value| uuid(value.object_id.as_uuid()))
    .collect::<Vec<_>>();
  let root_child_ids = builder.create_vector(&root_child_ids);

  let mut flattened = Vec::new();
  let mut stack = value
    .children
    .iter()
    .rev()
    .map(|value| (value, 1_usize))
    .collect::<Vec<_>>();
  let mut ids = HashSet::new();
  ids.insert(value.root_id);
  while let Some((node, depth)) = stack.pop() {
    if depth > 1_024 {
      return Err(ProtocolError::new("UI document exceeds logical depth 1024"));
    }
    if !ids.insert(node.object_id) {
      return Err(ProtocolError::new("UI document repeats a node UUID"));
    }
    flattened.push(node);
    stack.extend(node.children.iter().rev().map(|value| (value, depth + 1)));
  }
  let nodes = flattened
    .into_iter()
    .map(|value| write_node(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let nodes = builder.create_vector(&nodes);
  let document_id = uuid(value.document_id.as_uuid());
  let root_id = uuid(value.root_id.as_uuid());
  Ok(wire::UiDocument::create(
    builder,
    &wire::UiDocumentArgs {
      document_id: Some(&document_id),
      root_id: Some(&root_id),
      root_element: Some(root_element),
      root_child_ids: Some(root_child_ids),
      nodes: Some(nodes),
    },
  ))
}

pub(crate) fn write_retained_forest<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  document_id: battlement::ObjectId,
  root_id: battlement::ObjectId,
  children: &[battlement::UiNode],
) -> Result<WIPOffset<wire::UiDocument<'a>>, ProtocolError> {
  let root_element = write_visual_element(builder, &battlement::UiVisualElement::default())?;
  let root_child_ids = children
    .iter()
    .map(|value| uuid(value.object_id.as_uuid()))
    .collect::<Vec<_>>();
  let root_child_ids = builder.create_vector(&root_child_ids);
  let mut flattened = Vec::new();
  let mut stack = children
    .iter()
    .rev()
    .map(|value| (value, 1_usize))
    .collect::<Vec<_>>();
  let mut ids = HashSet::new();
  ids.insert(root_id);
  while let Some((node, depth)) = stack.pop() {
    if depth > 1_024 {
      return Err(ProtocolError::new(
        "retained UI forest exceeds logical depth 1024",
      ));
    }
    if !ids.insert(node.object_id) {
      return Err(ProtocolError::new("retained UI forest repeats a node UUID"));
    }
    flattened.push(node);
    stack.extend(node.children.iter().rev().map(|value| (value, depth + 1)));
  }
  let nodes = flattened
    .into_iter()
    .map(|value| write_node(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let nodes = builder.create_vector(&nodes);
  let document_id = uuid(document_id.as_uuid());
  let root_id = uuid(root_id.as_uuid());
  Ok(wire::UiDocument::create(
    builder,
    &wire::UiDocumentArgs {
      document_id: Some(&document_id),
      root_id: Some(&root_id),
      root_element: Some(root_element),
      root_child_ids: Some(root_child_ids),
      nodes: Some(nodes),
    },
  ))
}

pub(crate) fn write_node<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::UiNode,
) -> Result<WIPOffset<wire::UiNode<'a>>, ProtocolError> {
  let element = write_element(builder, &value.element)?;
  let child_ids = value
    .children
    .iter()
    .map(|value| uuid(value.object_id.as_uuid()))
    .collect::<Vec<_>>();
  let child_ids = builder.create_vector(&child_ids);
  let object_id = uuid(value.object_id.as_uuid());
  Ok(wire::UiNode::create(
    builder,
    &wire::UiNodeArgs {
      object_id: Some(&object_id),
      element: Some(element),
      child_ids: Some(child_ids),
    },
  ))
}

pub(crate) fn write_subtree<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  root: &battlement::UiNode,
) -> Result<Vec<WIPOffset<wire::UiNode<'a>>>, ProtocolError> {
  let mut flattened = Vec::new();
  let mut stack = vec![(root, 0_usize)];
  let mut ids = HashSet::new();
  while let Some((node, depth)) = stack.pop() {
    if depth > 1_024 {
      return Err(ProtocolError::new("UI subtree exceeds logical depth 1024"));
    }
    if !ids.insert(node.object_id) {
      return Err(ProtocolError::new("UI subtree repeats a node UUID"));
    }
    flattened.push(node);
    stack.extend(node.children.iter().rev().map(|value| (value, depth + 1)));
  }
  flattened
    .into_iter()
    .map(|value| write_node(builder, value))
    .collect()
}

fn write_visual_element<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::UiVisualElement,
) -> Result<WIPOffset<wire::UiElement<'a>>, ProtocolError> {
  write_element_parts(
    builder,
    wire::UiElementKind::VisualElement,
    value,
    Vec::new(),
    |_| Ok(()),
  )
}

pub(crate) fn write_element<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &UiElement,
) -> Result<WIPOffset<wire::UiElement<'a>>, ProtocolError> {
  let part_styles = write_part_styles(builder, value)?;
  let kind = match value {
    UiElement::VisualElement(_) => wire::UiElementKind::VisualElement,
    UiElement::Flex(_) => wire::UiElementKind::Flex,
    UiElement::Grid(_) => wire::UiElementKind::Grid,
    UiElement::Stack(_) => wire::UiElementKind::Stack,
    UiElement::Box(_) => wire::UiElementKind::Box,
    UiElement::Label(_) => wire::UiElementKind::Label,
    UiElement::TextElement(_) => wire::UiElementKind::TextElement,
    UiElement::TextField(_) => wire::UiElementKind::TextField,
    UiElement::Toggle(_) => wire::UiElementKind::Toggle,
    UiElement::RadioButton(_) => wire::UiElementKind::RadioButton,
    UiElement::RadioButtonGroup(_) => wire::UiElementKind::RadioButtonGroup,
    UiElement::ToggleButtonGroup(_) => wire::UiElementKind::ToggleButtonGroup,
    UiElement::DropdownField(_) => wire::UiElementKind::DropdownField,
    UiElement::Button(_) => wire::UiElementKind::Button,
    UiElement::RepeatButton(_) => wire::UiElementKind::RepeatButton,
    UiElement::GroupBox(_) => wire::UiElementKind::GroupBox,
    UiElement::PopupWindow(_) => wire::UiElementKind::PopupWindow,
    UiElement::ScrollView(_) => wire::UiElementKind::ScrollView,
    UiElement::Scroller(_) => wire::UiElementKind::Scroller,
    UiElement::Slider(_) => wire::UiElementKind::Slider,
    UiElement::SliderInt(_) => wire::UiElementKind::SliderInt,
    UiElement::MinMaxSlider(_) => wire::UiElementKind::MinMaxSlider,
    UiElement::ProgressBar(_) => wire::UiElementKind::ProgressBar,
    UiElement::Tab(_) => wire::UiElementKind::Tab,
    UiElement::TabView(_) => wire::UiElementKind::TabView,
    UiElement::Image(_) => wire::UiElementKind::Image,
  };
  write_element_parts(
    builder,
    kind,
    value.visual_element(),
    part_styles,
    |properties| write_specific_properties(properties, value),
  )
}

fn write_element_parts<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  kind: wire::UiElementKind,
  value: &battlement::UiVisualElement,
  part_styles: Vec<WIPOffset<wire::PartStyle<'a>>>,
  specific: impl FnOnce(&mut Properties<'_, 'a, A>) -> Result<(), ProtocolError>,
) -> Result<WIPOffset<wire::UiElement<'a>>, ProtocolError> {
  let usage_hints = usage_hints(value.usage_hints.as_deref().unwrap_or_default());
  let mut properties = Properties::new(builder);
  properties.text(wire::UiPropertyKey::Name, &value.name);
  properties.boolean(wire::UiPropertyKey::Enabled, &value.enabled);
  properties.enumeration(
    wire::UiPropertyKey::PickingMode,
    wire::UiEnumCatalog::PickingMode,
    &value.picking_mode,
    picking_mode,
  );
  properties.enumeration(
    wire::UiPropertyKey::LanguageDirection,
    wire::UiEnumCatalog::LanguageDirection,
    &value.language_direction,
    language_direction,
  );
  properties.boolean(wire::UiPropertyKey::Focusable, &value.focusable);
  properties.integer(wire::UiPropertyKey::TabIndex, &value.tab_index);
  properties.boolean(wire::UiPropertyKey::DelegatesFocus, &value.delegates_focus);
  properties.boolean(wire::UiPropertyKey::AutoFocus, &value.auto_focus);
  properties.boolean(wire::UiPropertyKey::Inert, &value.inert);
  properties.text_list(wire::UiPropertyKey::Classes, &value.classes);
  properties.event_kinds(wire::UiPropertyKey::Events, &value.events);
  let event_subscriptions = properties.event_subscriptions(&value.event_subscriptions);
  properties.grid_item(wire::UiPropertyKey::GridItem, &value.grid_item);
  properties.stack_item(wire::UiPropertyKey::StackItem, &value.stack_item);
  properties.sticky(wire::UiPropertyKey::Sticky, &value.sticky);
  properties.paint(wire::UiPropertyKey::Paint, &value.paint);
  properties.motion(wire::UiPropertyKey::Motion, &value.motion)?;
  properties.overlay_placement(
    wire::UiPropertyKey::OverlayPlacement,
    &value.overlay_placement,
  );
  write_style(&mut properties, &value.style)?;
  specific(&mut properties)?;
  let property_values = properties.builder.create_vector(&properties.values);
  let subscriptions = properties.builder.create_vector(&event_subscriptions);
  let part_styles = properties.builder.create_vector(&part_styles);
  Ok(wire::UiElement::create(
    properties.builder,
    &wire::UiElementArgs {
      kind,
      usage_hints,
      properties: Some(property_values),
      event_subscriptions: Some(subscriptions),
      part_styles: Some(part_styles),
    },
  ))
}

fn write_part_styles<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &UiElement,
) -> Result<Vec<WIPOffset<wire::PartStyle<'a>>>, ProtocolError> {
  value
    .part_style_entries()
    .map(|(part, index, style)| {
      if part > wire::UiPart::MinMaxSliderRangeDragger.0 {
        return Err(ProtocolError::new("UI part kind is outside the schema"));
      }
      let mut properties = Properties::new(builder);
      write_style(&mut properties, style)?;
      let values = properties.builder.create_vector(&properties.values);
      Ok(wire::PartStyle::create(
        properties.builder,
        &wire::PartStyleArgs {
          part: wire::UiPart(part),
          index,
          properties: Some(values),
        },
      ))
    })
    .collect()
}

pub(crate) fn write_style_properties<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::Style,
) -> Result<Vec<WIPOffset<wire::UiProperty<'a>>>, ProtocolError> {
  let mut properties = Properties::new(builder);
  write_style(&mut properties, value)?;
  Ok(properties.values)
}

struct Properties<'b, 'a, A: Allocator> {
  builder: &'b mut FlatBufferBuilder<'a, A>,
  values: Vec<WIPOffset<wire::UiProperty<'a>>>,
}

impl<'b, 'a, A: Allocator + 'a> Properties<'b, 'a, A> {
  fn new(builder: &'b mut FlatBufferBuilder<'a, A>) -> Self {
    Self {
      builder,
      values: Vec::new(),
    }
  }

  fn push(
    &mut self,
    key: wire::UiPropertyKey,
    state: wire::PropState,
    value_type: wire::UiPropertyValue,
    value: Option<WIPOffset<UnionWIPOffset>>,
  ) {
    self.values.push(wire::UiProperty::create(
      self.builder,
      &wire::UiPropertyArgs {
        key,
        state,
        style_value_kind: wire::StyleValueKind::Value,
        value_type,
        value,
      },
    ));
  }

  fn marker<T>(&mut self, key: wire::UiPropertyKey, value: &Prop<T>) {
    if matches!(value, Prop::Reset) {
      self.push(
        key,
        wire::PropState::Reset,
        wire::UiPropertyValue::NONE,
        None,
      );
    }
  }

  fn boolean(&mut self, key: wire::UiPropertyKey, value: &Prop<bool>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let value = wire::BoolPropertyValue::create(
          self.builder,
          &wire::BoolPropertyValueArgs { value: *value },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::BoolPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn integer(&mut self, key: wire::UiPropertyKey, value: &Prop<i32>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let value = wire::IntPropertyValue::create(
          self.builder,
          &wire::IntPropertyValueArgs { value: *value },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::IntPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn unsigned(&mut self, key: wire::UiPropertyKey, value: &Prop<u32>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => self.unsigned_value(key, *value),
    }
  }

  fn nonzero(&mut self, key: wire::UiPropertyKey, value: &Prop<std::num::NonZeroU32>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => self.unsigned_value(key, value.get()),
    }
  }

  fn unsigned_value(&mut self, key: wire::UiPropertyKey, scalar: u32) {
    let value =
      wire::UIntPropertyValue::create(self.builder, &wire::UIntPropertyValueArgs { value: scalar });
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::UIntPropertyValue,
      Some(value.as_union_value()),
    );
  }

  fn float(&mut self, key: wire::UiPropertyKey, value: &Prop<f32>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let value = wire::FloatPropertyValue::create(
          self.builder,
          &wire::FloatPropertyValueArgs { value: *value },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::FloatPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn text(&mut self, key: wire::UiPropertyKey, value: &Prop<String>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let value = self.builder.create_string(value);
        let value = wire::TextPropertyValue::create(
          self.builder,
          &wire::TextPropertyValueArgs { value: Some(value) },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::TextPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn text_list(&mut self, key: wire::UiPropertyKey, value: &Prop<Vec<String>>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(values) => {
        let values = values
          .iter()
          .map(|value| self.builder.create_string(value))
          .collect::<Vec<_>>();
        let values = self.builder.create_vector(&values);
        let value = wire::TextListPropertyValue::create(
          self.builder,
          &wire::TextListPropertyValueArgs {
            values: Some(values),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::TextListPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn unsigned_list(&mut self, key: wire::UiPropertyKey, value: &Prop<Vec<u32>>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(values) => {
        let values = self.builder.create_vector(values);
        let value = wire::UIntListPropertyValue::create(
          self.builder,
          &wire::UIntListPropertyValueArgs {
            values: Some(values),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::UIntListPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn grid_tracks(&mut self, key: wire::UiPropertyKey, value: &Prop<Vec<battlement::GridTrack>>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(values) => {
        let values = values.iter().copied().map(grid_track).collect::<Vec<_>>();
        let values = self.builder.create_vector(&values);
        let encoded = wire::GridTracksPropertyValue::create(
          self.builder,
          &wire::GridTracksPropertyValueArgs {
            values: Some(values),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::GridTracksPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn grid_track(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::GridTrack>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let values = self.builder.create_vector(&[grid_track(*value)]);
        let encoded = wire::GridTracksPropertyValue::create(
          self.builder,
          &wire::GridTracksPropertyValueArgs {
            values: Some(values),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::GridTracksPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn enumeration<T: Copy>(
    &mut self,
    key: wire::UiPropertyKey,
    catalog: wire::UiEnumCatalog,
    value: &Prop<T>,
    encode: impl FnOnce(T) -> u32,
  ) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let value = wire::EnumPropertyValue::create(
          self.builder,
          &wire::EnumPropertyValueArgs {
            catalog,
            value: encode(*value),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::EnumPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn vector(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::Vector>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let point = common::Vector2d::new(f64::from(value.x), f64::from(value.y));
        let value = wire::Vector2PropertyValue::create(
          self.builder,
          &wire::Vector2PropertyValueArgs {
            value: Some(&point),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::Vector2PropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn rect(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::Rect>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let rect = common::Rectd::new(value.x, value.y, value.width, value.height);
        let value = wire::RectPropertyValue::create(
          self.builder,
          &wire::RectPropertyValueArgs { value: Some(&rect) },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::RectPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn color(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::Color>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let color = common::RgbaColor::new(value.r, value.g, value.b, value.a);
        let value = wire::ColorPropertyValue::create(
          self.builder,
          &wire::ColorPropertyValueArgs {
            value: Some(&color),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::ColorPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn asset(&mut self, key: wire::UiPropertyKey, value: &Prop<impl AssetSource>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let address = self.builder.create_string(value.address());
        let value = wire::AssetPropertyValue::create(
          self.builder,
          &wire::AssetPropertyValueArgs {
            kind: value.kind(),
            address: Some(address),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::AssetPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
  }

  fn choice(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::Choice>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let text = value
          .value
          .as_ref()
          .map(|value| self.builder.create_string(value));
        let encoded = wire::ChoicePropertyValue::create(
          self.builder,
          &wire::ChoicePropertyValueArgs {
            kind: if value.index.is_some() {
              wire::ChoiceKind::Index
            } else {
              wire::ChoiceKind::None
            },
            index: value.index.unwrap_or_default(),
            value: text,
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::ChoicePropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn lower_limit(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::LowerLimit>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let (kind, scalar) = match value {
          battlement::LowerLimit::Unbounded => (wire::LimitKind::Unbounded, 0.0),
          battlement::LowerLimit::Inclusive(value) => (wire::LimitKind::Inclusive, *value),
        };
        let encoded = wire::LimitPropertyValue::create(
          self.builder,
          &wire::LimitPropertyValueArgs {
            kind,
            value: scalar,
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::LimitPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn upper_limit(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::UpperLimit>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let (kind, scalar) = match value {
          battlement::UpperLimit::Unbounded => (wire::LimitKind::Unbounded, 0.0),
          battlement::UpperLimit::Inclusive(value) => (wire::LimitKind::Inclusive, *value),
        };
        let encoded = wire::LimitPropertyValue::create(
          self.builder,
          &wire::LimitPropertyValueArgs {
            kind,
            value: scalar,
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::LimitPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn event_kinds(&mut self, key: wire::UiPropertyKey, value: &Prop<Vec<battlement::UiEventKind>>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(values) => {
        let encoded = values.iter().copied().map(event_kind_number).collect();
        self.unsigned_list(key, &Prop::Set(encoded));
      }
    }
  }

  fn event_subscriptions(
    &mut self,
    value: &Prop<Vec<battlement::UiEventSubscription>>,
  ) -> Vec<WIPOffset<wire::UiEventSubscriptionValue<'a>>> {
    match value {
      Prop::Unset => Vec::new(),
      Prop::Reset => {
        self.marker(wire::UiPropertyKey::EventSubscriptions, value);
        Vec::new()
      }
      Prop::Set(values) => {
        self.push(
          wire::UiPropertyKey::EventSubscriptions,
          wire::PropState::Set,
          wire::UiPropertyValue::NONE,
          None,
        );
        values
          .iter()
          .map(|value| {
            wire::UiEventSubscriptionValue::create(
              self.builder,
              &wire::UiEventSubscriptionValueArgs {
                kind: event_kind(value.kind),
                phases: match value.phase {
                  battlement::UiEventPhase::Trickle => 1,
                  battlement::UiEventPhase::Target => 2,
                  battlement::UiEventPhase::Bubble => 4,
                },
              },
            )
          })
          .collect()
      }
    }
  }

  fn grid_item(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::GridItem>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let encoded = wire::GridItemPropertyValue::create(
          self.builder,
          &wire::GridItemPropertyValueArgs {
            row: value.row,
            column: value.column,
            row_span: value.row_span,
            column_span: value.column_span,
            align_self: align(value.align_self),
            justify_self: align(value.justify_self),
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::GridItemPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn stack_item(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::StackItem>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let encoded = wire::StackItemPropertyValue::create(
          self.builder,
          &wire::StackItemPropertyValueArgs {
            order: value.order,
            align_self: align(value.align_self),
            justify_self: align(value.justify_self),
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            contributes_to_size: value.contributes_to_size,
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::StackItemPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn sticky(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::Sticky>) {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let encoded = wire::StickyPropertyValue::create(
          self.builder,
          &wire::StickyPropertyValueArgs {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
            order: value.order,
          },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::StickyPropertyValue,
          Some(encoded.as_union_value()),
        );
      }
    }
  }

  fn style_marker<T>(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<T>>,
  ) -> bool {
    match value {
      Prop::Unset => true,
      Prop::Reset => {
        self.push(
          key,
          wire::PropState::Reset,
          wire::UiPropertyValue::NONE,
          None,
        );
        true
      }
      Prop::Set(battlement::StyleValue::Keyword { .. }) => {
        self.values.push(wire::UiProperty::create(
          self.builder,
          &wire::UiPropertyArgs {
            key,
            state: wire::PropState::Set,
            style_value_kind: wire::StyleValueKind::Initial,
            value_type: wire::UiPropertyValue::NONE,
            value: None,
          },
        ));
        true
      }
      Prop::Set(battlement::StyleValue::Value(_)) => false,
    }
  }

  fn style_enum<T: Copy>(
    &mut self,
    key: wire::UiPropertyKey,
    catalog: wire::UiEnumCatalog,
    value: &Prop<battlement::StyleValue<T>>,
    encode: impl FnOnce(T) -> u32,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let encoded = wire::EnumPropertyValue::create(
      self.builder,
      &wire::EnumPropertyValueArgs {
        catalog,
        value: encode(*value),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::EnumPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_float(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::FloatValue>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let encoded = wire::FloatPropertyValue::create(
      self.builder,
      &wire::FloatPropertyValueArgs { value: value.0 },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::FloatPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_integer(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::StyleValue<i32>>) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let encoded =
      wire::IntPropertyValue::create(self.builder, &wire::IntPropertyValueArgs { value: *value });
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::IntPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_length(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::Length>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    self.push_length(key, length(*value));
  }

  fn style_length_or_auto(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::LengthOrAuto>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    self.push_length(key, length_or_auto(*value));
  }

  fn push_length(&mut self, key: wire::UiPropertyKey, value: EncodedLength) {
    let encoded = wire::LengthPropertyValue::create(
      self.builder,
      &wire::LengthPropertyValueArgs {
        kind: value.kind,
        pixels: value.pixels,
        percentage: value.percentage,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::LengthPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_color(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::Color>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let color = common::RgbaColor::new(value.r, value.g, value.b, value.a);
    let encoded = wire::ColorPropertyValue::create(
      self.builder,
      &wire::ColorPropertyValueArgs {
        value: Some(&color),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::ColorPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_asset<T: StyleAsset>(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<T>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let address = self.builder.create_string(value.address());
    let encoded = wire::AssetPropertyValue::create(
      self.builder,
      &wire::AssetPropertyValueArgs {
        kind: value.kind(),
        address: Some(address),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::AssetPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_aspect_ratio(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::AspectRatio>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let (kind, width, height) = value.components().map_or(
      (wire::AspectRatioKind::Auto, 0.0, 0.0),
      |(width, height)| (wire::AspectRatioKind::Ratio, width, height),
    );
    let encoded = wire::AspectRatioPropertyValue::create(
      self.builder,
      &wire::AspectRatioPropertyValueArgs {
        kind,
        width,
        height,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::AspectRatioPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_background_position(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::BackgroundPosition>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let offset = write_length_table(self.builder, length(value.offset));
    let encoded = wire::BackgroundPositionPropertyValue::create(
      self.builder,
      &wire::BackgroundPositionPropertyValueArgs {
        keyword: background_position_keyword(value.keyword),
        offset: Some(offset),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::BackgroundPositionPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_background_repeat(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::BackgroundRepeat>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let encoded = wire::BackgroundRepeatPropertyValue::create(
      self.builder,
      &wire::BackgroundRepeatPropertyValueArgs {
        x: background_repeat_mode(value.x),
        y: background_repeat_mode(value.y),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::BackgroundRepeatPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_background_size(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::BackgroundSize>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let (kind, x, y) = if let Some((x, y)) = value.axis_values() {
      (
        wire::BackgroundSizeKind::Axes,
        Some(write_length_table(self.builder, length_or_auto(x))),
        Some(write_length_table(self.builder, length_or_auto(y))),
      )
    } else {
      let kind = match value {
        battlement::BackgroundSize::Auto => wire::BackgroundSizeKind::Auto,
        battlement::BackgroundSize::Cover => wire::BackgroundSizeKind::Cover,
        battlement::BackgroundSize::Contain => wire::BackgroundSizeKind::Contain,
        battlement::BackgroundSize::Axes { .. } => unreachable!(),
      };
      (kind, None, None)
    };
    let encoded = wire::BackgroundSizePropertyValue::create(
      self.builder,
      &wire::BackgroundSizePropertyValueArgs { kind, x, y },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::BackgroundSizePropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_cursor(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::Cursor>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let (kind, address, hotspot) = value.texture_value().map_or(
      (wire::CursorKind::Default, None, None),
      |(address, hotspot)| {
        (
          wire::CursorKind::Texture,
          Some(self.builder.create_string(address.as_str())),
          Some(common::Vector2d::new(
            f64::from(hotspot.x),
            f64::from(hotspot.y),
          )),
        )
      },
    );
    let encoded = wire::CursorPropertyValue::create(
      self.builder,
      &wire::CursorPropertyValueArgs {
        kind,
        address,
        hotspot: hotspot.as_ref(),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::CursorPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_rotate(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::Rotate>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let encoded = wire::RotatePropertyValue::create(
      self.builder,
      &wire::RotatePropertyValueArgs {
        x: value.x,
        y: value.y,
        z: value.z,
        degrees: value.degrees,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::RotatePropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_scale(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::Scale>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let encoded = wire::ScalePropertyValue::create(
      self.builder,
      &wire::ScalePropertyValueArgs {
        x: value.x,
        y: value.y,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::ScalePropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_text_shadow(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::TextShadow>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let color = common::RgbaColor::new(value.color.r, value.color.g, value.color.b, value.color.a);
    let encoded = wire::TextShadowPropertyValue::create(
      self.builder,
      &wire::TextShadowPropertyValueArgs {
        x: value.x,
        y: value.y,
        blur_radius: value.blur_radius,
        color: Some(&color),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::TextShadowPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_transform_origin(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::TransformOrigin>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let x = write_length_table(self.builder, length(value.x));
    let y = write_length_table(self.builder, length(value.y));
    let encoded = wire::TransformOriginPropertyValue::create(
      self.builder,
      &wire::TransformOriginPropertyValueArgs {
        x: Some(x),
        y: Some(y),
        z: value.z,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::TransformOriginPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_translate(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::Translate>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let x = write_length_table(self.builder, length(value.x));
    let y = write_length_table(self.builder, length(value.y));
    let encoded = wire::TranslatePropertyValue::create(
      self.builder,
      &wire::TranslatePropertyValueArgs {
        x: Some(x),
        y: Some(y),
        z: value.z,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::TranslatePropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_time_list(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::TransitionList<battlement::TimeValue>>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let values = value
      .as_slice()
      .iter()
      .map(|value| value.0)
      .collect::<Vec<_>>();
    let values = self.builder.create_vector(&values);
    let encoded = wire::FloatListPropertyValue::create(
      self.builder,
      &wire::FloatListPropertyValueArgs {
        values: Some(values),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::FloatListPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_transition_properties(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<
      battlement::StyleValue<battlement::TransitionList<battlement::TransitionProperty>>,
    >,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let values = value
      .as_slice()
      .iter()
      .copied()
      .map(transition_property)
      .collect::<Vec<_>>();
    self.push_enum_list(key, wire::UiEnumCatalog::TransitionProperty, &values);
  }

  fn style_easing_list(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::TransitionList<battlement::EasingFunction>>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let values = value
      .as_slice()
      .iter()
      .copied()
      .map(easing_function)
      .collect::<Vec<_>>();
    self.push_enum_list(key, wire::UiEnumCatalog::EasingFunction, &values);
  }

  fn push_enum_list(
    &mut self,
    key: wire::UiPropertyKey,
    catalog: wire::UiEnumCatalog,
    values: &[u32],
  ) {
    let values = self.builder.create_vector(values);
    let encoded = wire::EnumListPropertyValue::create(
      self.builder,
      &wire::EnumListPropertyValueArgs {
        catalog,
        values: Some(values),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::EnumListPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn style_text_auto_size(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::StyleValue<battlement::TextAutoSize>>,
  ) {
    if self.style_marker(key, value) {
      return;
    }
    let Prop::Set(battlement::StyleValue::Value(value)) = value else {
      unreachable!();
    };
    let (kind, min_size, max_size) = value.bounds().map_or(
      (wire::TextAutoSizeKind::None, 0.0, 0.0),
      |(min_size, max_size)| (wire::TextAutoSizeKind::BestFit, min_size, max_size),
    );
    let encoded = wire::TextAutoSizePropertyValue::create(
      self.builder,
      &wire::TextAutoSizePropertyValueArgs {
        kind,
        min_size,
        max_size,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::TextAutoSizePropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn overlay_placement(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::OverlayPlacement>,
  ) {
    match value {
      Prop::Unset => return,
      Prop::Reset => {
        self.marker(key, value);
        return;
      }
      Prop::Set(_) => {}
    }
    let Prop::Set(value) = value else {
      unreachable!();
    };
    let mut anchor = None;
    let mut initial_focus = None;
    let mut restore_focus = None;
    let mut side = wire::PlacementSide::Bottom;
    let mut placement_align = wire::PlacementAlign::Start;
    let mut main_offset = 0.0;
    let mut cross_offset = 0.0;
    let mut collision_padding = 0.0;
    let mut flip = false;
    let mut shift = false;
    let kind = match value {
      battlement::OverlayPlacement::Layer(battlement::OverlayLayer::Popover) => {
        wire::OverlayPlacementKind::PopoverLayer
      }
      battlement::OverlayPlacement::Layer(battlement::OverlayLayer::Modal) => {
        wire::OverlayPlacementKind::ModalLayer
      }
      battlement::OverlayPlacement::Popover {
        anchor: value,
        placement,
      } => {
        anchor = Some(uuid(value.as_uuid()));
        side = match placement.side {
          battlement::PlacementSide::Top => wire::PlacementSide::Top,
          battlement::PlacementSide::Right => wire::PlacementSide::Right,
          battlement::PlacementSide::Bottom => wire::PlacementSide::Bottom,
          battlement::PlacementSide::Left => wire::PlacementSide::Left,
        };
        placement_align = match placement.align {
          battlement::PlacementAlign::Start => wire::PlacementAlign::Start,
          battlement::PlacementAlign::Center => wire::PlacementAlign::Center,
          battlement::PlacementAlign::End => wire::PlacementAlign::End,
        };
        main_offset = placement.main_offset;
        cross_offset = placement.cross_offset;
        collision_padding = placement.collision_padding;
        flip = placement.flip;
        shift = placement.shift;
        wire::OverlayPlacementKind::Popover
      }
      battlement::OverlayPlacement::Modal {
        initial_focus: initial,
        restore_focus: restore,
      } => {
        initial_focus = initial.as_ref().map(|value| uuid(value.as_uuid()));
        restore_focus = restore.as_ref().map(|value| uuid(value.as_uuid()));
        wire::OverlayPlacementKind::Modal
      }
    };
    let encoded = wire::OverlayPlacementPropertyValue::create(
      self.builder,
      &wire::OverlayPlacementPropertyValueArgs {
        kind,
        anchor: anchor.as_ref(),
        initial_focus: initial_focus.as_ref(),
        restore_focus: restore_focus.as_ref(),
        side,
        align: placement_align,
        main_offset,
        cross_offset,
        collision_padding,
        flip,
        shift,
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::OverlayPlacementPropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn paint(&mut self, key: wire::UiPropertyKey, value: &Prop<battlement::PaintStyle>) {
    match value {
      Prop::Unset => return,
      Prop::Reset => {
        self.marker(key, value);
        return;
      }
      Prop::Set(_) => {}
    }
    let Prop::Set(value) = value else {
      unreachable!();
    };
    let subtree_clip = value
      .subtree_clip_path()
      .map(|value| write_paint_clip_path(self.builder, value));
    let background = value
      .background_fill()
      .map(|value| write_paint_fill(self.builder, value));
    let filters = write_paint_filters(self.builder, value.paint_filters());
    let clip_polygon = write_paint_points(self.builder, value.clip_polygon_value());
    let box_shadows = write_paint_shadows(self.builder, value.box_shadows());
    let clip_insets = value
      .clip_insets()
      .map(|value| write_paint_insets(self.builder, value));
    let layers = value
      .paint_layers()
      .iter()
      .map(|value| write_paint_layer(self.builder, value))
      .collect::<Vec<_>>();
    let layers = self.builder.create_vector(&layers);
    let blend_mode = value.paint_blend_mode();
    let encoded = wire::PaintStylePropertyValue::create(
      self.builder,
      &wire::PaintStylePropertyValueArgs {
        subtree_clip,
        blend_mode: blend_mode.map_or(wire::PaintBlendMode::Normal, paint_blend_mode),
        has_blend_mode: blend_mode.is_some(),
        background,
        filters,
        clip_polygon,
        box_shadows,
        clip_insets,
        layers: Some(layers),
      },
    );
    self.push(
      key,
      wire::PropState::Set,
      wire::UiPropertyValue::PaintStylePropertyValue,
      Some(encoded.as_union_value()),
    );
  }

  fn motion(
    &mut self,
    key: wire::UiPropertyKey,
    value: &Prop<battlement::MotionDescriptor>,
  ) -> Result<(), ProtocolError> {
    match value {
      Prop::Unset => {}
      Prop::Reset => self.marker(key, value),
      Prop::Set(value) => {
        let value = crate::response_motion_descriptor::write_descriptor(self.builder, value)?;
        let value = wire::MotionDescriptorPropertyValue::create(
          self.builder,
          &wire::MotionDescriptorPropertyValueArgs { value: Some(value) },
        );
        self.push(
          key,
          wire::PropState::Set,
          wire::UiPropertyValue::MotionDescriptorPropertyValue,
          Some(value.as_union_value()),
        );
      }
    }
    Ok(())
  }
}

struct EncodedLength {
  kind: wire::LengthKind,
  pixels: f32,
  percentage: f32,
}

trait AssetSource {
  fn kind(&self) -> wire::AssetSourceKind;
  fn address(&self) -> &str;
}

impl AssetSource for battlement::IconSource {
  fn kind(&self) -> wire::AssetSourceKind {
    match self {
      Self::Texture(_) => wire::AssetSourceKind::Texture,
      Self::Sprite(_) => wire::AssetSourceKind::Sprite,
      Self::VectorImage(_) => wire::AssetSourceKind::VectorImage,
      Self::RenderTexture(_) => wire::AssetSourceKind::RenderTexture,
    }
  }

  fn address(&self) -> &str {
    self.address()
  }
}

impl AssetSource for battlement::ImageSource {
  fn kind(&self) -> wire::AssetSourceKind {
    match self {
      Self::Texture(_) => wire::AssetSourceKind::Texture,
      Self::Sprite(_) => wire::AssetSourceKind::Sprite,
      Self::VectorImage(_) => wire::AssetSourceKind::VectorImage,
      Self::RenderTexture(_) => wire::AssetSourceKind::RenderTexture,
    }
  }

  fn address(&self) -> &str {
    self.address()
  }
}

trait StyleAsset {
  fn kind(&self) -> wire::AssetSourceKind;
  fn address(&self) -> &str;
}

impl StyleAsset for battlement::BackgroundSource {
  fn kind(&self) -> wire::AssetSourceKind {
    match self {
      Self::Texture(_) => wire::AssetSourceKind::Texture,
      Self::Sprite(_) => wire::AssetSourceKind::Sprite,
      Self::VectorImage(_) => wire::AssetSourceKind::VectorImage,
      Self::RenderTexture(_) => wire::AssetSourceKind::RenderTexture,
    }
  }

  fn address(&self) -> &str {
    match self {
      Self::Texture(value) => value.as_str(),
      Self::Sprite(value) => value.as_str(),
      Self::VectorImage(value) => value.as_str(),
      Self::RenderTexture(value) => value.as_str(),
    }
  }
}

impl StyleAsset for battlement::UiFontAddress {
  fn kind(&self) -> wire::AssetSourceKind {
    wire::AssetSourceKind::UiFont
  }

  fn address(&self) -> &str {
    self.as_str()
  }
}

impl StyleAsset for battlement::MaterialAddress {
  fn kind(&self) -> wire::AssetSourceKind {
    wire::AssetSourceKind::Material
  }

  fn address(&self) -> &str {
    self.as_str()
  }
}

fn length(value: battlement::Length) -> EncodedLength {
  match value {
    battlement::Length::Px(value) => EncodedLength {
      kind: wire::LengthKind::Pixels,
      pixels: value,
      percentage: 0.0,
    },
    battlement::Length::Percent(value) => EncodedLength {
      kind: wire::LengthKind::Percent,
      pixels: 0.0,
      percentage: value,
    },
    battlement::Length::Calc { px, percent } => EncodedLength {
      kind: wire::LengthKind::Calc,
      pixels: px,
      percentage: percent,
    },
  }
}

fn length_or_auto(value: battlement::LengthOrAuto) -> EncodedLength {
  match value {
    battlement::LengthOrAuto::Px(value) => EncodedLength {
      kind: wire::LengthKind::Pixels,
      pixels: value,
      percentage: 0.0,
    },
    battlement::LengthOrAuto::Percent(value) => EncodedLength {
      kind: wire::LengthKind::Percent,
      pixels: 0.0,
      percentage: value,
    },
    battlement::LengthOrAuto::Auto => EncodedLength {
      kind: wire::LengthKind::Auto,
      pixels: 0.0,
      percentage: 0.0,
    },
  }
}

fn write_length_table<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: EncodedLength,
) -> WIPOffset<wire::LengthPropertyValue<'a>> {
  wire::LengthPropertyValue::create(
    builder,
    &wire::LengthPropertyValueArgs {
      kind: value.kind,
      pixels: value.pixels,
      percentage: value.percentage,
    },
  )
}

fn write_paint_point<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &[battlement::Length; 2],
) -> WIPOffset<wire::PaintLengthPoint<'a>> {
  let x = write_length_table(builder, length(value[0]));
  let y = write_length_table(builder, length(value[1]));
  wire::PaintLengthPoint::create(
    builder,
    &wire::PaintLengthPointArgs {
      x: Some(x),
      y: Some(y),
    },
  )
}

fn write_paint_points<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  values: Option<&[[battlement::Length; 2]]>,
) -> Option<
  WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<wire::PaintLengthPoint<'a>>>>,
> {
  values.map(|values| {
    let values = values
      .iter()
      .map(|value| write_paint_point(builder, value))
      .collect::<Vec<_>>();
    builder.create_vector(&values)
  })
}

fn write_paint_clip_path<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::PaintClipPath,
) -> WIPOffset<wire::PaintClipPathValue<'a>> {
  let contours = value
    .contours
    .iter()
    .map(|points| {
      let points = points
        .iter()
        .map(|value| write_paint_point(builder, value))
        .collect::<Vec<_>>();
      let points = builder.create_vector(&points);
      wire::PaintContour::create(
        builder,
        &wire::PaintContourArgs {
          points: Some(points),
        },
      )
    })
    .collect::<Vec<_>>();
  let contours = builder.create_vector(&contours);
  wire::PaintClipPathValue::create(
    builder,
    &wire::PaintClipPathValueArgs {
      contours: Some(contours),
      fill_rule: match value.fill_rule {
        battlement::PaintFillRule::NonZero => wire::PaintFillRule::NonZero,
        battlement::PaintFillRule::EvenOdd => wire::PaintFillRule::EvenOdd,
      },
    },
  )
}

fn write_paint_fill<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::PaintFill,
) -> WIPOffset<wire::PaintFillValue<'a>> {
  let (kind, color, gradient) = match value {
    battlement::PaintFill::Color(value) => (
      wire::PaintFillKind::Color,
      Some(common::RgbaColor::new(value.r, value.g, value.b, value.a)),
      None,
    ),
    battlement::PaintFill::Gradient(value) => (
      wire::PaintFillKind::Gradient,
      None,
      Some(write_paint_gradient(builder, value)),
    ),
  };
  wire::PaintFillValue::create(
    builder,
    &wire::PaintFillValueArgs {
      kind,
      color: color.as_ref(),
      gradient,
    },
  )
}

fn write_paint_gradient<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::Gradient,
) -> WIPOffset<wire::PaintGradientValue<'a>> {
  let (kind, angle, center, radius, stops) = if let Some((angle, stops)) = value.linear_value() {
    (wire::PaintGradientKind::Linear, angle, None, None, stops)
  } else {
    let (center, radius, stops) = value
      .radial_value()
      .expect("closed gradient variant is radial");
    (
      wire::PaintGradientKind::Radial,
      0.0,
      Some(wire::F32Vector2::new(center[0], center[1])),
      Some(wire::F32Vector2::new(radius[0], radius[1])),
      stops,
    )
  };
  let stops = stops
    .iter()
    .map(|value| {
      let color =
        common::RgbaColor::new(value.color.r, value.color.g, value.color.b, value.color.a);
      wire::PaintGradientStop::create(
        builder,
        &wire::PaintGradientStopArgs {
          color: Some(&color),
          position: value.position,
        },
      )
    })
    .collect::<Vec<_>>();
  let stops = builder.create_vector(&stops);
  wire::PaintGradientValue::create(
    builder,
    &wire::PaintGradientValueArgs {
      kind,
      angle,
      center: center.as_ref(),
      radius: radius.as_ref(),
      stops: Some(stops),
    },
  )
}

fn write_paint_shadow<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: battlement::Shadow,
) -> WIPOffset<wire::PaintShadowValue<'a>> {
  let color = common::RgbaColor::new(value.color.r, value.color.g, value.color.b, value.color.a);
  wire::PaintShadowValue::create(
    builder,
    &wire::PaintShadowValueArgs {
      x: value.x,
      y: value.y,
      blur: value.blur,
      spread: value.spread,
      color: Some(&color),
      inset: value.inset,
    },
  )
}

fn write_paint_shadows<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  values: Option<&[battlement::Shadow]>,
) -> Option<
  WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<wire::PaintShadowValue<'a>>>>,
> {
  values.map(|values| {
    let values = values
      .iter()
      .copied()
      .map(|value| write_paint_shadow(builder, value))
      .collect::<Vec<_>>();
    builder.create_vector(&values)
  })
}

fn write_paint_filters<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  values: Option<&battlement::FilterList>,
) -> Option<
  WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<wire::PaintFilterValue<'a>>>>,
> {
  values.map(|values| {
    let values = values
      .as_slice()
      .iter()
      .map(|value| {
        let (kind, amount, shadow) = match value {
          battlement::FilterFunction::Brightness(value) => {
            (wire::PaintFilterKind::Brightness, *value, None)
          }
          battlement::FilterFunction::DropShadow(value) => (
            wire::PaintFilterKind::DropShadow,
            0.0,
            Some(write_paint_shadow(builder, *value)),
          ),
        };
        wire::PaintFilterValue::create(
          builder,
          &wire::PaintFilterValueArgs {
            kind,
            amount,
            shadow,
          },
        )
      })
      .collect::<Vec<_>>();
    builder.create_vector(&values)
  })
}

fn write_paint_insets<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &[battlement::Length; 4],
) -> WIPOffset<wire::PaintInsetsValue<'a>> {
  let top = write_length_table(builder, length(value[0]));
  let right = write_length_table(builder, length(value[1]));
  let bottom = write_length_table(builder, length(value[2]));
  let left = write_length_table(builder, length(value[3]));
  wire::PaintInsetsValue::create(
    builder,
    &wire::PaintInsetsValueArgs {
      top: Some(top),
      right: Some(right),
      bottom: Some(bottom),
      left: Some(left),
    },
  )
}

fn write_paint_layer<'a, A: Allocator + 'a>(
  builder: &mut FlatBufferBuilder<'a, A>,
  value: &battlement::PaintLayer,
) -> WIPOffset<wire::PaintLayerValue<'a>> {
  let background = write_paint_fill(builder, value.background_fill());
  let filters = write_paint_filters(builder, value.paint_filters());
  let clip_polygon = write_paint_points(builder, value.clip_polygon_value());
  let box_shadows = write_paint_shadows(builder, value.box_shadows());
  let clip_insets = value
    .clip_insets()
    .map(|value| write_paint_insets(builder, value));
  let bounds_insets = value
    .bounds_insets()
    .map(|value| write_paint_insets(builder, value));
  wire::PaintLayerValue::create(
    builder,
    &wire::PaintLayerValueArgs {
      background: Some(background),
      filters,
      clip_polygon,
      box_shadows,
      clip_insets,
      bounds_insets,
    },
  )
}

fn paint_blend_mode(value: battlement::PaintBlendMode) -> wire::PaintBlendMode {
  match value {
    battlement::PaintBlendMode::Normal => wire::PaintBlendMode::Normal,
    battlement::PaintBlendMode::Screen => wire::PaintBlendMode::Screen,
    battlement::PaintBlendMode::Additive => wire::PaintBlendMode::Additive,
  }
}

fn uuid(value: &uuid::Uuid) -> common::Uuid {
  common::Uuid::new(value.as_bytes())
}

fn usage_hints(values: &[battlement::UsageHint]) -> u32 {
  values.iter().fold(0, |mask, value| {
    mask
      | match value {
        battlement::UsageHint::DynamicTransform => 1 << 0,
        battlement::UsageHint::GroupTransform => 1 << 1,
        battlement::UsageHint::MaskContainer => 1 << 2,
        battlement::UsageHint::DynamicColor => 1 << 3,
        battlement::UsageHint::DynamicPostProcessing => 1 << 4,
        battlement::UsageHint::LargePixelCoverage => 1 << 5,
      }
  })
}

fn picking_mode(value: battlement::PickingMode) -> u32 {
  match value {
    battlement::PickingMode::Position => 0,
    battlement::PickingMode::Ignore => 1,
  }
}

fn language_direction(value: battlement::LanguageDirection) -> u32 {
  match value {
    battlement::LanguageDirection::Inherit => 0,
    battlement::LanguageDirection::Ltr => 1,
    battlement::LanguageDirection::Rtl => 2,
  }
}

fn align(value: battlement::Align) -> u32 {
  match value {
    battlement::Align::Auto => 0,
    battlement::Align::FlexStart => 1,
    battlement::Align::Center => 2,
    battlement::Align::FlexEnd => 3,
    battlement::Align::Stretch => 4,
  }
}

fn flex_direction(value: battlement::FlexDirection) -> u32 {
  match value {
    battlement::FlexDirection::Column => 0,
    battlement::FlexDirection::ColumnReverse => 1,
    battlement::FlexDirection::Row => 2,
    battlement::FlexDirection::RowReverse => 3,
  }
}

fn flex_wrap(value: battlement::FlexWrap) -> u32 {
  match value {
    battlement::FlexWrap::NoWrap => 0,
    battlement::FlexWrap::Wrap => 1,
    battlement::FlexWrap::WrapReverse => 2,
  }
}

fn justify(value: battlement::Justify) -> u32 {
  match value {
    battlement::Justify::FlexStart => 0,
    battlement::Justify::Center => 1,
    battlement::Justify::FlexEnd => 2,
    battlement::Justify::SpaceBetween => 3,
    battlement::Justify::SpaceAround => 4,
    battlement::Justify::SpaceEvenly => 5,
  }
}

fn grid_auto_flow(value: battlement::GridAutoFlow) -> u32 {
  match value {
    battlement::GridAutoFlow::Row => 0,
    battlement::GridAutoFlow::Column => 1,
  }
}

fn grid_track(value: battlement::GridTrack) -> wire::GridTrackValue {
  match value {
    battlement::GridTrack::Px(value) => {
      wire::GridTrackValue::new(wire::GridTrackKind::Pixels, value)
    }
    battlement::GridTrack::Fraction(value) => {
      wire::GridTrackValue::new(wire::GridTrackKind::Fraction, value)
    }
    battlement::GridTrack::Auto => wire::GridTrackValue::new(wire::GridTrackKind::Auto, 0.0),
  }
}

fn scroll_view_mode(value: battlement::ScrollViewMode) -> u32 {
  match value {
    battlement::ScrollViewMode::Vertical => 0,
    battlement::ScrollViewMode::Horizontal => 1,
    battlement::ScrollViewMode::VerticalAndHorizontal => 2,
  }
}

fn nested_interaction(value: battlement::NestedInteraction) -> u32 {
  match value {
    battlement::NestedInteraction::Default => 0,
    battlement::NestedInteraction::StopScrolling => 1,
    battlement::NestedInteraction::ForwardScrolling => 2,
  }
}

fn scroller_visibility(value: battlement::ScrollerVisibility) -> u32 {
  match value {
    battlement::ScrollerVisibility::Auto => 0,
    battlement::ScrollerVisibility::AlwaysVisible => 1,
    battlement::ScrollerVisibility::Hidden => 2,
  }
}

fn touch_scroll_behavior(value: battlement::TouchScrollBehavior) -> u32 {
  match value {
    battlement::TouchScrollBehavior::Unrestricted => 0,
    battlement::TouchScrollBehavior::Elastic => 1,
    battlement::TouchScrollBehavior::Clamped => 2,
  }
}

fn slider_direction(value: battlement::SliderDirection) -> u32 {
  match value {
    battlement::SliderDirection::Horizontal => 0,
    battlement::SliderDirection::Vertical => 1,
  }
}

fn image_scale_mode(value: battlement::ImageScaleMode) -> u32 {
  match value {
    battlement::ImageScaleMode::ScaleToFit => 0,
    battlement::ImageScaleMode::ScaleAndCrop => 1,
    battlement::ImageScaleMode::StretchToFill => 2,
  }
}

fn display(value: battlement::Display) -> u32 {
  match value {
    battlement::Display::Flex => 0,
    battlement::Display::None => 1,
  }
}

fn visibility(value: battlement::Visibility) -> u32 {
  match value {
    battlement::Visibility::Visible => 0,
    battlement::Visibility::Hidden => 1,
  }
}

fn overflow(value: battlement::Overflow) -> u32 {
  match value {
    battlement::Overflow::Visible => 0,
    battlement::Overflow::Hidden => 1,
  }
}

fn overflow_clip_box(value: battlement::OverflowClipBox) -> u32 {
  match value {
    battlement::OverflowClipBox::PaddingBox => 0,
    battlement::OverflowClipBox::ContentBox => 1,
  }
}

fn position(value: battlement::Position) -> u32 {
  match value {
    battlement::Position::Relative => 0,
    battlement::Position::Absolute => 1,
  }
}

fn text_overflow(value: battlement::TextOverflow) -> u32 {
  match value {
    battlement::TextOverflow::Clip => 0,
    battlement::TextOverflow::Ellipsis => 1,
  }
}

fn text_overflow_position(value: battlement::TextOverflowPosition) -> u32 {
  match value {
    battlement::TextOverflowPosition::Start => 0,
    battlement::TextOverflowPosition::Middle => 1,
    battlement::TextOverflowPosition::End => 2,
  }
}

fn white_space(value: battlement::WhiteSpace) -> u32 {
  match value {
    battlement::WhiteSpace::Normal => 0,
    battlement::WhiteSpace::NoWrap => 1,
    battlement::WhiteSpace::Pre => 2,
    battlement::WhiteSpace::PreWrap => 3,
  }
}

fn text_generator(value: battlement::TextGenerator) -> u32 {
  match value {
    battlement::TextGenerator::Standard => 0,
    battlement::TextGenerator::Advanced => 1,
  }
}

fn editor_text_rendering_mode(value: battlement::EditorTextRenderingMode) -> u32 {
  match value {
    battlement::EditorTextRenderingMode::Sdf => 0,
    battlement::EditorTextRenderingMode::Bitmap => 1,
  }
}

fn font_style(value: battlement::FontStyle) -> u32 {
  match value {
    battlement::FontStyle::Normal => 0,
    battlement::FontStyle::Bold => 1,
    battlement::FontStyle::Italic => 2,
    battlement::FontStyle::BoldAndItalic => 3,
  }
}

fn text_anchor(value: battlement::TextAnchor) -> u32 {
  match value {
    battlement::TextAnchor::UpperLeft => 0,
    battlement::TextAnchor::UpperCenter => 1,
    battlement::TextAnchor::UpperRight => 2,
    battlement::TextAnchor::MiddleLeft => 3,
    battlement::TextAnchor::MiddleCenter => 4,
    battlement::TextAnchor::MiddleRight => 5,
    battlement::TextAnchor::LowerLeft => 6,
    battlement::TextAnchor::LowerCenter => 7,
    battlement::TextAnchor::LowerRight => 8,
  }
}

fn slice_type(value: battlement::SliceType) -> u32 {
  match value {
    battlement::SliceType::Sliced => 0,
    battlement::SliceType::Tiled => 1,
  }
}

fn background_position_keyword(value: battlement::BackgroundPositionKeyword) -> u32 {
  match value {
    battlement::BackgroundPositionKeyword::Center => 0,
    battlement::BackgroundPositionKeyword::Top => 1,
    battlement::BackgroundPositionKeyword::Bottom => 2,
    battlement::BackgroundPositionKeyword::Left => 3,
    battlement::BackgroundPositionKeyword::Right => 4,
  }
}

fn background_repeat_mode(value: battlement::BackgroundRepeatMode) -> u32 {
  match value {
    battlement::BackgroundRepeatMode::NoRepeat => 0,
    battlement::BackgroundRepeatMode::Repeat => 1,
    battlement::BackgroundRepeatMode::Round => 2,
    battlement::BackgroundRepeatMode::Space => 3,
  }
}

fn easing_function(value: battlement::EasingFunction) -> u32 {
  use battlement::EasingFunction as Easing;
  match value {
    Easing::Ease => 0,
    Easing::EaseIn => 1,
    Easing::EaseOut => 2,
    Easing::EaseInOut => 3,
    Easing::Linear => 4,
    Easing::EaseInSine => 5,
    Easing::EaseOutSine => 6,
    Easing::EaseInOutSine => 7,
    Easing::EaseInCubic => 8,
    Easing::EaseOutCubic => 9,
    Easing::EaseInOutCubic => 10,
    Easing::EaseInCirc => 11,
    Easing::EaseOutCirc => 12,
    Easing::EaseInOutCirc => 13,
    Easing::EaseInElastic => 14,
    Easing::EaseOutElastic => 15,
    Easing::EaseInOutElastic => 16,
    Easing::EaseInBack => 17,
    Easing::EaseOutBack => 18,
    Easing::EaseInOutBack => 19,
    Easing::EaseInBounce => 20,
    Easing::EaseOutBounce => 21,
    Easing::EaseInOutBounce => 22,
  }
}

fn transition_property(value: battlement::TransitionProperty) -> u32 {
  use battlement::TransitionProperty as Property;
  use event_wire::TransitionProperty as Wire;
  u32::from(
    match value {
      Property::All => Wire::All,
      Property::AlignContent => Wire::AlignContent,
      Property::AlignItems => Wire::AlignItems,
      Property::AlignSelf => Wire::AlignSelf,
      Property::AspectRatio => Wire::AspectRatio,
      Property::BackgroundColor => Wire::BackgroundColor,
      Property::BackgroundImage => Wire::BackgroundImage,
      Property::BackgroundPositionX => Wire::BackgroundPositionX,
      Property::BackgroundPositionY => Wire::BackgroundPositionY,
      Property::BackgroundRepeat => Wire::BackgroundRepeat,
      Property::BackgroundSize => Wire::BackgroundSize,
      Property::BorderBottomColor => Wire::BorderBottomColor,
      Property::BorderBottomLeftRadius => Wire::BorderBottomLeftRadius,
      Property::BorderBottomRightRadius => Wire::BorderBottomRightRadius,
      Property::BorderBottomWidth => Wire::BorderBottomWidth,
      Property::BorderLeftColor => Wire::BorderLeftColor,
      Property::BorderLeftWidth => Wire::BorderLeftWidth,
      Property::BorderRightColor => Wire::BorderRightColor,
      Property::BorderRightWidth => Wire::BorderRightWidth,
      Property::BorderTopColor => Wire::BorderTopColor,
      Property::BorderTopLeftRadius => Wire::BorderTopLeftRadius,
      Property::BorderTopRightRadius => Wire::BorderTopRightRadius,
      Property::BorderTopWidth => Wire::BorderTopWidth,
      Property::Bottom => Wire::Bottom,
      Property::Color => Wire::Color,
      Property::Cursor => Wire::Cursor,
      Property::Display => Wire::Display,
      Property::FlexBasis => Wire::FlexBasis,
      Property::FlexDirection => Wire::FlexDirection,
      Property::FlexGrow => Wire::FlexGrow,
      Property::FlexShrink => Wire::FlexShrink,
      Property::FlexWrap => Wire::FlexWrap,
      Property::FontSize => Wire::FontSize,
      Property::Height => Wire::Height,
      Property::JustifyContent => Wire::JustifyContent,
      Property::Left => Wire::Left,
      Property::LetterSpacing => Wire::LetterSpacing,
      Property::MarginBottom => Wire::MarginBottom,
      Property::MarginLeft => Wire::MarginLeft,
      Property::MarginRight => Wire::MarginRight,
      Property::MarginTop => Wire::MarginTop,
      Property::MaxHeight => Wire::MaxHeight,
      Property::MaxWidth => Wire::MaxWidth,
      Property::MinHeight => Wire::MinHeight,
      Property::MinWidth => Wire::MinWidth,
      Property::Opacity => Wire::Opacity,
      Property::Overflow => Wire::Overflow,
      Property::PaddingBottom => Wire::PaddingBottom,
      Property::PaddingLeft => Wire::PaddingLeft,
      Property::PaddingRight => Wire::PaddingRight,
      Property::PaddingTop => Wire::PaddingTop,
      Property::Position => Wire::Position,
      Property::Right => Wire::Right,
      Property::Rotate => Wire::Rotate,
      Property::Scale => Wire::Scale,
      Property::TextOverflow => Wire::TextOverflow,
      Property::TextShadow => Wire::TextShadow,
      Property::Top => Wire::Top,
      Property::TransformOrigin => Wire::TransformOrigin,
      Property::TransitionDelay => Wire::TransitionDelay,
      Property::TransitionDuration => Wire::TransitionDuration,
      Property::TransitionProperty => Wire::TransitionProperty,
      Property::TransitionTimingFunction => Wire::TransitionTimingFunction,
      Property::Translate => Wire::Translate,
      Property::UnityBackgroundImageTintColor => Wire::UnityBackgroundImageTintColor,
      Property::UnityEditorTextRenderingMode => Wire::UnityEditorTextRenderingMode,
      Property::UnityFontDefinition => Wire::UnityFontDefinition,
      Property::UnityFontStyleAndWeight => Wire::UnityFontStyleAndWeight,
      Property::UnityMaterial => Wire::UnityMaterial,
      Property::UnityOverflowClipBox => Wire::UnityOverflowClipBox,
      Property::UnityParagraphSpacing => Wire::UnityParagraphSpacing,
      Property::UnitySliceBottom => Wire::UnitySliceBottom,
      Property::UnitySliceLeft => Wire::UnitySliceLeft,
      Property::UnitySliceRight => Wire::UnitySliceRight,
      Property::UnitySliceScale => Wire::UnitySliceScale,
      Property::UnitySliceTop => Wire::UnitySliceTop,
      Property::UnitySliceType => Wire::UnitySliceType,
      Property::UnityTextAlign => Wire::UnityTextAlign,
      Property::UnityTextAutoSize => Wire::UnityTextAutoSize,
      Property::UnityTextGenerator => Wire::UnityTextGenerator,
      Property::UnityTextOutlineColor => Wire::UnityTextOutlineColor,
      Property::UnityTextOutlineWidth => Wire::UnityTextOutlineWidth,
      Property::UnityTextOverflowPosition => Wire::UnityTextOverflowPosition,
      Property::Visibility => Wire::Visibility,
      Property::WhiteSpace => Wire::WhiteSpace,
      Property::Width => Wire::Width,
      Property::WordSpacing => Wire::WordSpacing,
    }
    .0,
  )
}

fn event_kind_number(value: battlement::UiEventKind) -> u32 {
  u32::from(event_kind(value).0)
}

fn event_kind(value: battlement::UiEventKind) -> wire::UiSubscriptionKind {
  use battlement::UiEventKind as Event;
  use wire::UiSubscriptionKind as Wire;
  match value {
    Event::AccessibilityAction => Wire::AccessibilityAction,
    Event::PointerDown => Wire::PointerDown,
    Event::PointerMove => Wire::PointerMove,
    Event::PointerUp => Wire::PointerUp,
    Event::PointerCancel => Wire::PointerCancel,
    Event::Click => Wire::Click,
    Event::PointerEnter => Wire::PointerEnter,
    Event::PointerLeave => Wire::PointerLeave,
    Event::PointerOver => Wire::PointerOver,
    Event::PointerOut => Wire::PointerOut,
    Event::Wheel => Wire::Wheel,
    Event::PointerCapture => Wire::PointerCapture,
    Event::PointerCaptureOut => Wire::PointerCaptureOut,
    Event::KeyDown => Wire::KeyDown,
    Event::KeyUp => Wire::KeyUp,
    Event::NavigationMove => Wire::NavigationMove,
    Event::NavigationCancel => Wire::NavigationCancel,
    Event::FocusIn => Wire::FocusIn,
    Event::Focus => Wire::Focus,
    Event::FocusOut => Wire::FocusOut,
    Event::Blur => Wire::Blur,
    Event::GeometryChanged => Wire::GeometryChanged,
    Event::AttachToPanel => Wire::AttachToPanel,
    Event::DetachFromPanel => Wire::DetachFromPanel,
    Event::TransitionStart => Wire::TransitionStart,
    Event::TransitionEnd => Wire::TransitionEnd,
    Event::TransitionCancel => Wire::TransitionCancel,
    Event::ValueChanging => Wire::ValueChanging,
    Event::ValueCommitted => Wire::ValueCommitted,
    Event::Input => Wire::Input,
    Event::SelectionChanged => Wire::SelectionChanged,
    Event::LinkEnter => Wire::LinkEnter,
    Event::LinkLeave => Wire::LinkLeave,
    Event::LinkDown => Wire::LinkDown,
    Event::LinkUp => Wire::LinkUp,
    Event::ScrollSettled => Wire::ScrollSettled,
    Event::ScrollChanged => Wire::ScrollChanged,
    Event::TabSelectionRequested => Wire::TabSelectionRequested,
    Event::TabCloseRequested => Wire::TabCloseRequested,
    Event::TabReorderRequested => Wire::TabReorderRequested,
  }
}

fn write_specific_properties<'a, A: Allocator + 'a>(
  properties: &mut Properties<'_, 'a, A>,
  value: &UiElement,
) -> Result<(), ProtocolError> {
  macro_rules! text_element {
    ($value:expr) => {{
      let value = $value;
      properties.text(wire::UiPropertyKey::Text, &value.text);
      properties.boolean(wire::UiPropertyKey::EnableRichText, &value.enable_rich_text);
      properties.boolean(
        wire::UiPropertyKey::EmojiFallbackSupport,
        &value.emoji_fallback_support,
      );
      properties.boolean(
        wire::UiPropertyKey::ParseEscapeSequences,
        &value.parse_escape_sequences,
      );
      properties.boolean(
        wire::UiPropertyKey::DisplayTooltipWhenElided,
        &value.display_tooltip_when_elided,
      );
      properties.boolean(wire::UiPropertyKey::Selectable, &value.selectable);
      properties.boolean(
        wire::UiPropertyKey::DoubleClickSelectsWord,
        &value.double_click_selects_word,
      );
      properties.boolean(
        wire::UiPropertyKey::TripleClickSelectsLine,
        &value.triple_click_selects_line,
      );
      properties.boolean(
        wire::UiPropertyKey::SelectAllOnFocus,
        &value.select_all_on_focus,
      );
      properties.boolean(
        wire::UiPropertyKey::SelectAllOnMouseUp,
        &value.select_all_on_mouse_up,
      );
    }};
  }
  match value {
    UiElement::VisualElement(_) | UiElement::Box(_) => {}
    UiElement::Flex(value) => {
      properties.enumeration(
        wire::UiPropertyKey::Direction,
        wire::UiEnumCatalog::FlexDirection,
        &value.direction,
        flex_direction,
      );
      properties.enumeration(
        wire::UiPropertyKey::Wrap,
        wire::UiEnumCatalog::FlexWrap,
        &value.wrap,
        flex_wrap,
      );
      properties.enumeration(
        wire::UiPropertyKey::AlignItems,
        wire::UiEnumCatalog::Align,
        &value.align_items,
        align,
      );
      properties.enumeration(
        wire::UiPropertyKey::JustifyContent,
        wire::UiEnumCatalog::Justify,
        &value.justify_content,
        justify,
      );
      properties.float(wire::UiPropertyKey::RowGap, &value.row_gap);
      properties.float(wire::UiPropertyKey::ColumnGap, &value.column_gap);
    }
    UiElement::Grid(value) => {
      properties.grid_tracks(wire::UiPropertyKey::Columns, &value.columns);
      properties.grid_tracks(wire::UiPropertyKey::Rows, &value.rows);
      properties.grid_track(wire::UiPropertyKey::AutoColumns, &value.auto_columns);
      properties.grid_track(wire::UiPropertyKey::AutoRows, &value.auto_rows);
      properties.enumeration(
        wire::UiPropertyKey::AutoFlow,
        wire::UiEnumCatalog::GridAutoFlow,
        &value.auto_flow,
        grid_auto_flow,
      );
      properties.float(wire::UiPropertyKey::RowGap, &value.row_gap);
      properties.float(wire::UiPropertyKey::ColumnGap, &value.column_gap);
      properties.enumeration(
        wire::UiPropertyKey::AlignItems,
        wire::UiEnumCatalog::Align,
        &value.align_items,
        align,
      );
      properties.enumeration(
        wire::UiPropertyKey::JustifyItems,
        wire::UiEnumCatalog::Align,
        &value.justify_items,
        align,
      );
    }
    UiElement::Stack(value) => {
      properties.enumeration(
        wire::UiPropertyKey::AlignItems,
        wire::UiEnumCatalog::Align,
        &value.align_items,
        align,
      );
      properties.enumeration(
        wire::UiPropertyKey::JustifyItems,
        wire::UiEnumCatalog::Align,
        &value.justify_items,
        align,
      );
    }
    UiElement::Label(value) => text_element!(value),
    UiElement::TextElement(value) => text_element!(value),
    UiElement::TextField(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.text(wire::UiPropertyKey::Value, &value.value);
      properties.boolean(wire::UiPropertyKey::Multiline, &value.multiline);
      properties.enumeration(
        wire::UiPropertyKey::VerticalScrollerVisibility,
        wire::UiEnumCatalog::ScrollerVisibility,
        &value.vertical_scroller_visibility,
        scroller_visibility,
      );
      properties.boolean(wire::UiPropertyKey::Password, &value.password);
      properties.boolean(wire::UiPropertyKey::ReadOnly, &value.read_only);
      properties.text(wire::UiPropertyKey::Placeholder, &value.placeholder);
      properties.boolean(
        wire::UiPropertyKey::HidePlaceholderOnFocus,
        &value.hide_placeholder_on_focus,
      );
      properties.unsigned(wire::UiPropertyKey::CursorIndex, &value.cursor_index);
      properties.unsigned(wire::UiPropertyKey::SelectIndex, &value.select_index);
      properties.boolean(
        wire::UiPropertyKey::SelectAllOnFocus,
        &value.select_all_on_focus,
      );
      properties.boolean(
        wire::UiPropertyKey::SelectAllOnMouseUp,
        &value.select_all_on_mouse_up,
      );
    }
    UiElement::Toggle(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.text(wire::UiPropertyKey::Text, &value.text);
      properties.boolean(wire::UiPropertyKey::Value, &value.value);
    }
    UiElement::RadioButton(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.text(wire::UiPropertyKey::Text, &value.text);
      properties.boolean(wire::UiPropertyKey::Value, &value.value);
    }
    UiElement::RadioButtonGroup(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.text_list(wire::UiPropertyKey::Choices, &value.choices);
      properties.unsigned(wire::UiPropertyKey::SelectedIndex, &value.selected_index);
    }
    UiElement::ToggleButtonGroup(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.boolean(
        wire::UiPropertyKey::MultipleSelection,
        &value.multiple_selection,
      );
      properties.boolean(
        wire::UiPropertyKey::AllowEmptySelection,
        &value.allow_empty_selection,
      );
      properties.unsigned_list(
        wire::UiPropertyKey::SelectedIndices,
        &value.selected_indices,
      );
    }
    UiElement::DropdownField(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.boolean(wire::UiPropertyKey::ShowMixedValue, &value.show_mixed_value);
      properties.text_list(wire::UiPropertyKey::Choices, &value.choices);
      properties.choice(wire::UiPropertyKey::Selection, &value.selection);
    }
    UiElement::Button(value) => {
      properties.text(wire::UiPropertyKey::Text, &value.text);
      properties.boolean(wire::UiPropertyKey::EnableRichText, &value.enable_rich_text);
      properties.boolean(
        wire::UiPropertyKey::EmojiFallbackSupport,
        &value.emoji_fallback_support,
      );
      properties.boolean(
        wire::UiPropertyKey::ParseEscapeSequences,
        &value.parse_escape_sequences,
      );
      properties.boolean(
        wire::UiPropertyKey::DisplayTooltipWhenElided,
        &value.display_tooltip_when_elided,
      );
      properties.asset(wire::UiPropertyKey::Icon, &value.icon);
    }
    UiElement::RepeatButton(value) => {
      properties.text(wire::UiPropertyKey::Text, &value.text);
      properties.unsigned(wire::UiPropertyKey::DelayMs, &value.delay_ms);
      properties.nonzero(wire::UiPropertyKey::IntervalMs, &value.interval_ms);
      properties.boolean(wire::UiPropertyKey::EnableRichText, &value.enable_rich_text);
      properties.boolean(
        wire::UiPropertyKey::EmojiFallbackSupport,
        &value.emoji_fallback_support,
      );
      properties.boolean(
        wire::UiPropertyKey::ParseEscapeSequences,
        &value.parse_escape_sequences,
      );
      properties.boolean(
        wire::UiPropertyKey::DisplayTooltipWhenElided,
        &value.display_tooltip_when_elided,
      );
    }
    UiElement::GroupBox(value) => {
      properties.text(wire::UiPropertyKey::Text, &value.text);
    }
    UiElement::PopupWindow(value) => text_element!(value),
    UiElement::ScrollView(value) => {
      properties.enumeration(
        wire::UiPropertyKey::Mode,
        wire::UiEnumCatalog::ScrollViewMode,
        &value.mode,
        scroll_view_mode,
      );
      properties.enumeration(
        wire::UiPropertyKey::NestedInteraction,
        wire::UiEnumCatalog::NestedInteraction,
        &value.nested_interaction,
        nested_interaction,
      );
      properties.enumeration(
        wire::UiPropertyKey::HorizontalScrollerVisibility,
        wire::UiEnumCatalog::ScrollerVisibility,
        &value.horizontal_scroller_visibility,
        scroller_visibility,
      );
      properties.enumeration(
        wire::UiPropertyKey::VerticalScrollerVisibility,
        wire::UiEnumCatalog::ScrollerVisibility,
        &value.vertical_scroller_visibility,
        scroller_visibility,
      );
      properties.vector(wire::UiPropertyKey::ScrollOffset, &value.scroll_offset);
      properties.float(
        wire::UiPropertyKey::HorizontalPageSize,
        &value.horizontal_page_size,
      );
      properties.float(
        wire::UiPropertyKey::VerticalPageSize,
        &value.vertical_page_size,
      );
      properties.float(
        wire::UiPropertyKey::MouseWheelScrollSize,
        &value.mouse_wheel_scroll_size,
      );
      properties.enumeration(
        wire::UiPropertyKey::TouchScrollBehavior,
        wire::UiEnumCatalog::TouchScrollBehavior,
        &value.touch_scroll_behavior,
        touch_scroll_behavior,
      );
      properties.float(
        wire::UiPropertyKey::ScrollDecelerationRate,
        &value.scroll_deceleration_rate,
      );
      properties.float(wire::UiPropertyKey::Elasticity, &value.elasticity);
      properties.unsigned(
        wire::UiPropertyKey::ElasticAnimationInterval,
        &value.elastic_animation_interval,
      );
    }
    UiElement::Scroller(value) => {
      properties.float(wire::UiPropertyKey::LowValue, &value.low_value);
      properties.float(wire::UiPropertyKey::HighValue, &value.high_value);
      properties.enumeration(
        wire::UiPropertyKey::Direction,
        wire::UiEnumCatalog::SliderDirection,
        &value.direction,
        slider_direction,
      );
      properties.float(wire::UiPropertyKey::Value, &value.value);
    }
    UiElement::Slider(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.float(wire::UiPropertyKey::LowValue, &value.low_value);
      properties.float(wire::UiPropertyKey::HighValue, &value.high_value);
      properties.float(wire::UiPropertyKey::Value, &value.value);
      properties.boolean(wire::UiPropertyKey::Fill, &value.fill);
      properties.float(wire::UiPropertyKey::PageSize, &value.page_size);
      properties.boolean(wire::UiPropertyKey::ShowInputField, &value.show_input_field);
      properties.enumeration(
        wire::UiPropertyKey::Direction,
        wire::UiEnumCatalog::SliderDirection,
        &value.direction,
        slider_direction,
      );
      properties.boolean(wire::UiPropertyKey::Inverted, &value.inverted);
    }
    UiElement::SliderInt(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.integer(wire::UiPropertyKey::LowValue, &value.low_value);
      properties.integer(wire::UiPropertyKey::HighValue, &value.high_value);
      properties.integer(wire::UiPropertyKey::Value, &value.value);
      properties.boolean(wire::UiPropertyKey::Fill, &value.fill);
      properties.float(wire::UiPropertyKey::PageSize, &value.page_size);
      properties.boolean(wire::UiPropertyKey::ShowInputField, &value.show_input_field);
      properties.enumeration(
        wire::UiPropertyKey::Direction,
        wire::UiEnumCatalog::SliderDirection,
        &value.direction,
        slider_direction,
      );
      properties.boolean(wire::UiPropertyKey::Inverted, &value.inverted);
    }
    UiElement::MinMaxSlider(value) => {
      properties.text(wire::UiPropertyKey::Label, &value.label);
      properties.float(wire::UiPropertyKey::MinValue, &value.min_value);
      properties.float(wire::UiPropertyKey::MaxValue, &value.max_value);
      properties.lower_limit(wire::UiPropertyKey::LowLimit, &value.low_limit);
      properties.upper_limit(wire::UiPropertyKey::HighLimit, &value.high_limit);
    }
    UiElement::ProgressBar(value) => {
      properties.float(wire::UiPropertyKey::LowValue, &value.low_value);
      properties.float(wire::UiPropertyKey::HighValue, &value.high_value);
      properties.float(wire::UiPropertyKey::Value, &value.value);
      properties.text(wire::UiPropertyKey::Title, &value.title);
    }
    UiElement::Tab(value) => {
      properties.text(wire::UiPropertyKey::Text, &value.text);
      properties.asset(wire::UiPropertyKey::Icon, &value.icon);
      properties.boolean(wire::UiPropertyKey::Closeable, &value.closeable);
    }
    UiElement::TabView(value) => {
      properties.unsigned(
        wire::UiPropertyKey::SelectedTabIndex,
        &value.selected_tab_index,
      );
      properties.boolean(wire::UiPropertyKey::Reorderable, &value.reorderable);
    }
    UiElement::Image(value) => {
      properties.asset(wire::UiPropertyKey::Source, &value.source);
      properties.rect(wire::UiPropertyKey::SourceRect, &value.source_rect);
      properties.color(wire::UiPropertyKey::TintColor, &value.tint_color);
      properties.enumeration(
        wire::UiPropertyKey::ScaleMode,
        wire::UiEnumCatalog::ImageScaleMode,
        &value.scale_mode,
        image_scale_mode,
      );
      properties.rect(wire::UiPropertyKey::Uv, &value.uv);
    }
  }
  Ok(())
}

fn write_style<'a, A: Allocator + 'a>(
  properties: &mut Properties<'_, 'a, A>,
  value: &battlement::Style,
) -> Result<(), ProtocolError> {
  use wire::UiPropertyKey as Key;
  properties.style_enum(
    Key::StyleAlignContent,
    wire::UiEnumCatalog::Align,
    &value.align_content,
    align,
  );
  properties.style_enum(
    Key::StyleAlignItems,
    wire::UiEnumCatalog::Align,
    &value.align_items,
    align,
  );
  properties.style_enum(
    Key::StyleAlignSelf,
    wire::UiEnumCatalog::Align,
    &value.align_self,
    align,
  );
  properties.style_aspect_ratio(Key::StyleAspectRatio, &value.aspect_ratio);
  properties.style_color(Key::StyleBackgroundColor, &value.background_color);
  properties.style_asset(Key::StyleBackgroundImage, &value.background_image);
  properties.style_background_position(Key::StyleBackgroundPositionX, &value.background_position_x);
  properties.style_background_position(Key::StyleBackgroundPositionY, &value.background_position_y);
  properties.style_background_repeat(Key::StyleBackgroundRepeat, &value.background_repeat);
  properties.style_background_size(Key::StyleBackgroundSize, &value.background_size);
  properties.style_color(Key::StyleBorderBottomColor, &value.border_bottom_color);
  properties.style_length(
    Key::StyleBorderBottomLeftRadius,
    &value.border_bottom_left_radius,
  );
  properties.style_length(
    Key::StyleBorderBottomRightRadius,
    &value.border_bottom_right_radius,
  );
  properties.style_float(Key::StyleBorderBottomWidth, &value.border_bottom_width);
  properties.style_color(Key::StyleBorderLeftColor, &value.border_left_color);
  properties.style_float(Key::StyleBorderLeftWidth, &value.border_left_width);
  properties.style_color(Key::StyleBorderRightColor, &value.border_right_color);
  properties.style_float(Key::StyleBorderRightWidth, &value.border_right_width);
  properties.style_color(Key::StyleBorderTopColor, &value.border_top_color);
  properties.style_length(Key::StyleBorderTopLeftRadius, &value.border_top_left_radius);
  properties.style_length(
    Key::StyleBorderTopRightRadius,
    &value.border_top_right_radius,
  );
  properties.style_float(Key::StyleBorderTopWidth, &value.border_top_width);
  properties.style_length_or_auto(Key::StyleBottom, &value.bottom);
  properties.style_color(Key::StyleColor, &value.color);
  properties.style_cursor(Key::StyleCursor, &value.cursor);
  properties.style_enum(
    Key::StyleDisplay,
    wire::UiEnumCatalog::Display,
    &value.display,
    display,
  );
  properties.style_length_or_auto(Key::StyleFlexBasis, &value.flex_basis);
  properties.style_enum(
    Key::StyleFlexDirection,
    wire::UiEnumCatalog::FlexDirection,
    &value.flex_direction,
    flex_direction,
  );
  properties.style_float(Key::StyleFlexGrow, &value.flex_grow);
  properties.style_float(Key::StyleFlexShrink, &value.flex_shrink);
  properties.style_enum(
    Key::StyleFlexWrap,
    wire::UiEnumCatalog::FlexWrap,
    &value.flex_wrap,
    flex_wrap,
  );
  properties.style_length(Key::StyleFontSize, &value.font_size);
  properties.style_length_or_auto(Key::StyleHeight, &value.height);
  properties.style_enum(
    Key::StyleJustifyContent,
    wire::UiEnumCatalog::Justify,
    &value.justify_content,
    justify,
  );
  properties.style_length_or_auto(Key::StyleLeft, &value.left);
  properties.style_length(Key::StyleLetterSpacing, &value.letter_spacing);
  properties.style_length_or_auto(Key::StyleMarginBottom, &value.margin_bottom);
  properties.style_length_or_auto(Key::StyleMarginLeft, &value.margin_left);
  properties.style_length_or_auto(Key::StyleMarginRight, &value.margin_right);
  properties.style_length_or_auto(Key::StyleMarginTop, &value.margin_top);
  properties.style_length_or_auto(Key::StyleMaxHeight, &value.max_height);
  properties.style_length_or_auto(Key::StyleMaxWidth, &value.max_width);
  properties.style_length_or_auto(Key::StyleMinHeight, &value.min_height);
  properties.style_length_or_auto(Key::StyleMinWidth, &value.min_width);
  properties.style_float(Key::StyleOpacity, &value.opacity);
  properties.style_enum(
    Key::StyleOverflow,
    wire::UiEnumCatalog::Overflow,
    &value.overflow,
    overflow,
  );
  properties.style_length(Key::StylePaddingBottom, &value.padding_bottom);
  properties.style_length(Key::StylePaddingLeft, &value.padding_left);
  properties.style_length(Key::StylePaddingRight, &value.padding_right);
  properties.style_length(Key::StylePaddingTop, &value.padding_top);
  properties.style_enum(
    Key::StylePosition,
    wire::UiEnumCatalog::Position,
    &value.position,
    position,
  );
  properties.style_length_or_auto(Key::StyleRight, &value.right);
  properties.style_rotate(Key::StyleRotate, &value.rotate);
  properties.style_scale(Key::StyleScale, &value.scale);
  properties.style_enum(
    Key::StyleTextOverflow,
    wire::UiEnumCatalog::TextOverflow,
    &value.text_overflow,
    text_overflow,
  );
  properties.style_text_shadow(Key::StyleTextShadow, &value.text_shadow);
  properties.style_length_or_auto(Key::StyleTop, &value.top);
  properties.style_transform_origin(Key::StyleTransformOrigin, &value.transform_origin);
  properties.style_time_list(Key::StyleTransitionDelay, &value.transition_delay);
  properties.style_time_list(Key::StyleTransitionDuration, &value.transition_duration);
  properties.style_transition_properties(Key::StyleTransitionProperty, &value.transition_property);
  properties.style_easing_list(
    Key::StyleTransitionTimingFunction,
    &value.transition_timing_function,
  );
  properties.style_translate(Key::StyleTranslate, &value.translate);
  properties.style_color(
    Key::StyleUnityBackgroundImageTintColor,
    &value.unity_background_image_tint_color,
  );
  properties.style_enum(
    Key::StyleUnityEditorTextRenderingMode,
    wire::UiEnumCatalog::EditorTextRenderingMode,
    &value.unity_editor_text_rendering_mode,
    editor_text_rendering_mode,
  );
  properties.style_asset(Key::StyleUnityFontDefinition, &value.unity_font_definition);
  properties.style_enum(
    Key::StyleUnityFontStyleAndWeight,
    wire::UiEnumCatalog::FontStyle,
    &value.unity_font_style_and_weight,
    font_style,
  );
  properties.style_asset(Key::StyleUnityMaterial, &value.unity_material);
  properties.style_enum(
    Key::StyleUnityOverflowClipBox,
    wire::UiEnumCatalog::OverflowClipBox,
    &value.unity_overflow_clip_box,
    overflow_clip_box,
  );
  properties.style_length(
    Key::StyleUnityParagraphSpacing,
    &value.unity_paragraph_spacing,
  );
  properties.style_integer(Key::StyleUnitySliceBottom, &value.unity_slice_bottom);
  properties.style_integer(Key::StyleUnitySliceLeft, &value.unity_slice_left);
  properties.style_integer(Key::StyleUnitySliceRight, &value.unity_slice_right);
  properties.style_float(Key::StyleUnitySliceScale, &value.unity_slice_scale);
  properties.style_integer(Key::StyleUnitySliceTop, &value.unity_slice_top);
  properties.style_enum(
    Key::StyleUnitySliceType,
    wire::UiEnumCatalog::SliceType,
    &value.unity_slice_type,
    slice_type,
  );
  properties.style_enum(
    Key::StyleUnityTextAlign,
    wire::UiEnumCatalog::TextAnchor,
    &value.unity_text_align,
    text_anchor,
  );
  properties.style_text_auto_size(Key::StyleUnityTextAutoSize, &value.unity_text_auto_size);
  properties.style_enum(
    Key::StyleUnityTextGenerator,
    wire::UiEnumCatalog::TextGenerator,
    &value.unity_text_generator,
    text_generator,
  );
  properties.style_color(
    Key::StyleUnityTextOutlineColor,
    &value.unity_text_outline_color,
  );
  properties.style_float(
    Key::StyleUnityTextOutlineWidth,
    &value.unity_text_outline_width,
  );
  properties.style_enum(
    Key::StyleUnityTextOverflowPosition,
    wire::UiEnumCatalog::TextOverflowPosition,
    &value.unity_text_overflow_position,
    text_overflow_position,
  );
  properties.style_enum(
    Key::StyleVisibility,
    wire::UiEnumCatalog::Visibility,
    &value.visibility,
    visibility,
  );
  properties.style_enum(
    Key::StyleWhiteSpace,
    wire::UiEnumCatalog::WhiteSpace,
    &value.white_space,
    white_space,
  );
  properties.style_length_or_auto(Key::StyleWidth, &value.width);
  properties.style_length(Key::StyleWordSpacing, &value.word_spacing);
  Ok(())
}
