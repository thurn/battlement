use std::collections::HashMap;

use battlement::{
  self, Command, CommandBody, ObjectId, UiElement, UiNode, VisualElementCreate,
  VisualElementProperties,
};
use serde_json::{Map, Value};

pub(crate) fn commands(
  parent_id: ObjectId,
  previous: &[UiNode],
  desired: &[UiNode],
) -> Vec<Command> {
  let previous_nodes = self::node_map(previous);
  let desired_nodes = self::node_map(desired);
  let mut plan = Plan::default();
  self::reconcile_children(
    parent_id,
    previous,
    desired,
    &previous_nodes,
    &desired_nodes,
    &mut plan,
  );
  plan
    .destroys
    .into_iter()
    .chain(plan.creates)
    .chain(plan.properties)
    .collect()
}

pub(crate) fn requires_remount(previous: &UiElement, desired: &UiElement) -> bool {
  if previous.kind() != desired.kind() {
    return true;
  }
  if previous.visual_element().usage_hints != desired.visual_element().usage_hints {
    return true;
  }
  let Some(patch) = self::property_patch(previous, desired) else {
    return false;
  };
  let mut merged = previous.clone();
  merged.apply_update(&patch);
  battlement::validate_element_state(&merged).is_err()
}

#[derive(Default)]
struct Plan {
  destroys: Vec<Command>,
  creates: Vec<Command>,
  properties: Vec<Command>,
}

fn reconcile_children(
  parent_id: ObjectId,
  previous: &[UiNode],
  desired: &[UiNode],
  previous_nodes: &HashMap<ObjectId, &UiNode>,
  desired_nodes: &HashMap<ObjectId, &UiNode>,
  plan: &mut Plan,
) {
  for child in previous {
    if !desired_nodes.contains_key(&child.object_id) {
      plan
        .destroys
        .push(Command::destroy_visual_element(child.object_id));
    }
  }
  for (index, child) in desired.iter().enumerate() {
    let Some(previous_child) = previous_nodes.get(&child.object_id) else {
      let create = VisualElementCreate::new(parent_id, child.clone())
        .child_index(u32::try_from(index).expect("validated child index fits u32"));
      plan
        .creates
        .push(Command::new_v4(CommandBody::VisualElementCreate(Box::new(
          create,
        ))));
      continue;
    };
    if previous_child.element.kind() != child.element.kind() {
      plan
        .destroys
        .push(Command::destroy_visual_element(previous_child.object_id));
      let create = VisualElementCreate::new(parent_id, child.clone())
        .child_index(u32::try_from(index).expect("validated child index fits u32"));
      plan
        .creates
        .push(Command::new_v4(CommandBody::VisualElementCreate(Box::new(
          create,
        ))));
      continue;
    }
    self::reconcile_children(
      child.object_id,
      &previous_child.children,
      &child.children,
      previous_nodes,
      desired_nodes,
      plan,
    );
    if let Some(patch) = self::property_patch(&previous_child.element, &child.element) {
      battlement::validate_element_update(&patch)
        .expect("Reactant generated an invalid property patch");
      plan
        .properties
        .push(Command::update_visual_element(child.object_id, patch));
    }
  }
}

fn property_patch(previous: &UiElement, desired: &UiElement) -> Option<UiElement> {
  assert_eq!(
    previous.kind(),
    desired.kind(),
    "host kind changed during diff"
  );
  let (variant, previous) = self::variant_fields(previous);
  let (desired_variant, desired) = self::variant_fields(desired);
  debug_assert_eq!(variant, desired_variant);
  let fields = self::diff_fields(&previous, &desired);
  if fields.is_empty() {
    return None;
  }
  serde_json::from_value(Value::Object(Map::from_iter([(
    variant,
    Value::Object(fields),
  )])))
  .expect("Reactant property diff preserves the element wire shape")
}

fn variant_fields(element: &UiElement) -> (String, Map<String, Value>) {
  let value = serde_json::to_value(element).expect("UI elements serialize");
  let Value::Object(variants) = value else {
    unreachable!("UI element is an enum object");
  };
  let (variant, Value::Object(fields)) = variants
    .into_iter()
    .next()
    .expect("UI element has a variant")
  else {
    unreachable!("UI element variant contains properties");
  };
  (variant, fields)
}

fn diff_fields(previous: &Map<String, Value>, desired: &Map<String, Value>) -> Map<String, Value> {
  let mut patch = Map::new();
  for (name, value) in desired {
    let difference = match name.as_str() {
      "style" => self::diff_object(previous.get(name), Some(value)),
      "parts" => self::diff_parts(previous.get(name), Some(value)),
      "usage_hints" => None,
      _ if previous.get(name) == Some(value) => None,
      _ => Some(value.clone()),
    };
    if let Some(difference) = difference {
      patch.insert(name.clone(), difference);
    }
  }
  for (name, value) in previous {
    if desired.contains_key(name) || value.is_null() {
      continue;
    }
    let reset = match name.as_str() {
      "style" => self::diff_object(Some(value), None),
      "parts" => self::diff_parts(Some(value), None),
      "usage_hints" => None,
      _ => Some(Value::Null),
    };
    if let Some(reset) = reset {
      patch.insert(name.clone(), reset);
    }
  }
  patch
}

fn diff_object(previous: Option<&Value>, desired: Option<&Value>) -> Option<Value> {
  let empty = Map::new();
  let previous = previous.and_then(Value::as_object).unwrap_or(&empty);
  let desired = desired.and_then(Value::as_object).unwrap_or(&empty);
  let difference = self::diff_fields(previous, desired);
  (!difference.is_empty()).then_some(Value::Object(difference))
}

fn diff_parts(previous: Option<&Value>, desired: Option<&Value>) -> Option<Value> {
  let previous = previous
    .and_then(Value::as_array)
    .map_or(&[][..], Vec::as_slice);
  let desired = desired
    .and_then(Value::as_array)
    .map_or(&[][..], Vec::as_slice);
  let desired_keys = desired.iter().map(self::part_key).collect::<Vec<_>>();
  let mut patch = Vec::new();
  for part in desired {
    let key = self::part_key(part);
    let previous = previous
      .iter()
      .find(|candidate| self::part_key(candidate) == key);
    if let Some(value) = self::part_patch(previous, Some(part)) {
      patch.push(value);
    }
  }
  for part in previous {
    if !desired_keys.contains(&self::part_key(part))
      && let Some(value) = self::part_patch(Some(part), None)
    {
      patch.push(value);
    }
  }
  (!patch.is_empty()).then_some(Value::Array(patch))
}

fn part_key(value: &Value) -> (Value, Option<u64>) {
  let fields = value.as_object().expect("part style is an object");
  (
    fields.get("part").expect("part style has a part").clone(),
    fields.get("index").and_then(Value::as_u64),
  )
}

fn part_patch(previous: Option<&Value>, desired: Option<&Value>) -> Option<Value> {
  let source = desired.or(previous).expect("part patch has a source");
  let source = source.as_object().expect("part style is an object");
  let style = self::diff_object(
    previous.and_then(|value| value.get("style")),
    desired.and_then(|value| value.get("style")),
  )?;
  let mut patch = Map::from_iter([(
    "part".to_owned(),
    source.get("part").expect("part style has a part").clone(),
  )]);
  if let Some(index) = source.get("index") {
    patch.insert("index".to_owned(), index.clone());
  }
  patch.insert("style".to_owned(), style);
  Some(Value::Object(patch))
}

fn node_map(nodes: &[UiNode]) -> HashMap<ObjectId, &UiNode> {
  let mut result = HashMap::new();
  for node in nodes {
    self::collect_nodes(node, &mut result);
  }
  result
}

fn collect_nodes<'a>(node: &'a UiNode, result: &mut HashMap<ObjectId, &'a UiNode>) {
  result.insert(node.object_id, node);
  for child in &node.children {
    self::collect_nodes(child, result);
  }
}
