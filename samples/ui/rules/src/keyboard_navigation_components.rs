use battlement::{
  Command, FocusDirection, ObjectId, UiBox, UiButton, UiElement, UiEvent, UiEventBody, UiEventKind,
  UiEventPhase, UiEventSubscription, UiLabel, UiNode, UiVisualElement, object_id,
};
use battlement_native::UiEventActionView;
use std::fmt;

use crate::{design_system, keyboard_navigation_styles, native_ui::NativeUiResponseBuilder};

pub(crate) const TARGETS: [ObjectId; 4] = [
  object_id!("23100000-0000-4000-8000-000000000001"),
  object_id!("23100000-0000-4000-8000-000000000002"),
  object_id!("23100000-0000-4000-8000-000000000003"),
  object_id!("23100000-0000-4000-8000-000000000004"),
];
pub(crate) const INSPECTOR_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000005");
pub(crate) const FOCUS_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000006");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(
        page_id,
        UiVisualElement::new()
            .name("keyboard-navigation-page")
            .event_subscriptions(ancestor_subscriptions()),
    )
    .child(node(
        UiLabel::new("KEYBOARD + FOCUS").style(design_system::eyebrow().flex_shrink(0)),
    ))
    .child(node(
        UiLabel::new("One focus path. Every activation.")
            .style(design_system::title().flex_shrink(0)),
    ))
    .child(node(UiLabel::new("Physical keys and UI navigation intent arrive as separate typed events. Moving focus updates the amber ring; submit becomes one route-wide Click. The inspector keeps each layer visible.").style(keyboard_navigation_styles::intro())))
    .child(
        node(UiVisualElement::new().style(keyboard_navigation_styles::columns()))
            .child(grid_card())
            .child(inspector_card()),
    )
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
  let index = TARGETS.iter().position(|id| *id == event.target_id)?;
  let name = target_name(index);
  let focus_status = match &event.body {
    UiEventBody::FocusIn(value) | UiEventBody::Focus(value) => Some(format!(
      "● {name} ← {} · {}",
      related_name(value.related_target_id),
      focus_direction_name(value.direction)
    )),
    _ => None,
  };
  let message = match &event.body {
    UiEventBody::FocusIn(value) | UiEventBody::Focus(value) => format!(
      "FOCUS RELATION\n{name} gained focus\nfrom {}\ndirection {:?}",
      related_name(value.related_target_id),
      value.direction
    ),
    UiEventBody::FocusOut(value) | UiEventBody::Blur(value) => format!(
      "FOCUS RELATION\n{name} released focus\nto {}\ndirection {:?}",
      related_name(value.related_target_id),
      value.direction
    ),
    UiEventBody::KeyDown(value) => format!(
      "PHYSICAL KEY DOWN\ncode {:?}\ntext {:?}\nmodifiers {:?}",
      value.physical_key, value.text, value.modifiers
    ),
    UiEventBody::KeyUp(value) => format!(
      "PHYSICAL KEY UP\ncode {:?}\ntext {:?}\nmodifiers {:?}",
      value.physical_key, value.text, value.modifiers
    ),
    UiEventBody::NavigationMove(value) => format!(
      "NAVIGATION MOVE\ndirection {:?}\nvector {:.1}, {:.1}",
      value.direction, value.move_vector.x, value.move_vector.y
    ),
    UiEventBody::NavigationCancel(_) => {
      "NAVIGATION CANCEL\nEscape stayed in UI focus routing.".to_owned()
    }
    UiEventBody::Click(battlement::ClickEvent::NavigationSubmit) => format!(
      "ACTIVATED · {name}\nNavigation submit became exactly one Click.\nNo duplicate NavigationSubmit action crossed the transport."
    ),
    UiEventBody::Click(battlement::ClickEvent::Pointer { .. }) => {
      format!("ACTIVATED · {name}\nPointer Click used the same Rust handler.")
    }
    _ => return None,
  };
  let mut commands = vec![Command::update_visual_element(
    INSPECTOR_ID,
    UiLabel::new(message),
  )];
  if let Some(focus_status) = focus_status {
    commands.extend(
      TARGETS
        .into_iter()
        .enumerate()
        .map(|(target_index, target_id)| {
          Command::update_visual_element(
            target_id,
            UiButton::default().style(keyboard_navigation_styles::target(target_index == index)),
          )
        }),
    );
    commands.push(Command::update_visual_element(
      FOCUS_ID,
      UiLabel::new(focus_status),
    ));
  }
  Some(commands)
}

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, battlement_native::EngineError> {
  let target_id =
    ObjectId::from_bytes(event.target_id()).expect("UI event view validates target UUIDs");
  let Some(index) = TARGETS.iter().position(|id| *id == target_id) else {
    return Ok(false);
  };
  let name = target_name(index);
  let mut focus_status = None;
  let message = match event.event_kind() {
    UiEventKind::FocusIn | UiEventKind::Focus => {
      let value = event.focus().expect("focus body");
      let related = related_name_bytes(value.related_target_id());
      focus_status = Some(format!(
        "● {name} ← {related} · {}",
        focus_direction_name(value.direction())
      ));
      format!(
        "FOCUS RELATION\n{name} gained focus\nfrom {related}\ndirection {:?}",
        value.direction()
      )
    }
    UiEventKind::FocusOut | UiEventKind::Blur => {
      let value = event.focus().expect("focus body");
      format!(
        "FOCUS RELATION\n{name} released focus\nto {}\ndirection {:?}",
        related_name_bytes(value.related_target_id()),
        value.direction()
      )
    }
    UiEventKind::KeyDown | UiEventKind::KeyUp => {
      let value = event.key().expect("key body");
      format!(
        "PHYSICAL KEY {}\ncode {:?}\ntext {:?}\nmodifiers {:?}",
        if event.event_kind() == UiEventKind::KeyDown {
          "DOWN"
        } else {
          "UP"
        },
        value.physical_key(),
        value.text(),
        ModifiersDebug(value.modifiers())
      )
    }
    UiEventKind::NavigationMove => {
      let value = event.navigation().expect("navigation body");
      let movement = value.movement();
      format!(
        "NAVIGATION MOVE\ndirection {:?}\nvector {:.1}, {:.1}",
        value.direction(),
        movement.0,
        movement.1
      )
    }
    UiEventKind::NavigationCancel => {
      "NAVIGATION CANCEL\nEscape stayed in UI focus routing.".to_owned()
    }
    UiEventKind::PointerEnter | UiEventKind::PointerLeave => return Ok(true),
    UiEventKind::Click if event.click_kind() == Some(1) => format!(
      "ACTIVATED · {name}\nNavigation submit became exactly one Click.\nNo duplicate NavigationSubmit action crossed the transport."
    ),
    UiEventKind::Click if event.click_kind() == Some(0) => {
      format!("ACTIVATED · {name}\nPointer Click used the same Rust handler.")
    }
    _ => return Ok(false),
  };
  response.label(INSPECTOR_ID, &message)?;
  if let Some(focus_status) = focus_status {
    for (target_index, target_id) in TARGETS.into_iter().enumerate() {
      let focused = target_index == index;
      let background = if focused {
        battlement::Color::rgb(0.10, 0.28, 0.32)
      } else {
        battlement::Color::rgb(0.04, 0.14, 0.17)
      };
      let border = if focused {
        battlement::Color::rgb(1.0, 0.69, 0.22)
      } else {
        battlement::Color::rgb(0.10, 0.42, 0.46)
      };
      let element = response
        .writer()
        .button_builder()
        .width_percent(44.0)
        .height(92.0)
        .margin(8.0, 8.0)
        .background_color(rgba(background))
        .color(rgba(battlement::Color::rgb(0.92, 0.97, 0.98)))
        .border_color(rgba(border))
        .border_width(if focused { 4.0 } else { 1.0 })
        .border_radius(10.0)
        .font_size(17.0)
        .text_align_middle_center()
        .finish();
      response.update(target_id, element)?;
    }
    response.label(FOCUS_ID, &focus_status)?;
  }
  Ok(true)
}

struct ModifiersDebug(u32);

impl fmt::Debug for ModifiersDebug {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    const NAMES: [&str; 7] = [
      "Alt",
      "Control",
      "Command",
      "Shift",
      "CapsLock",
      "Numeric",
      "FunctionKey",
    ];
    formatter.write_str("KeyModifiers([")?;
    let mut separator = "";
    for (index, name) in NAMES.into_iter().enumerate() {
      if self.0 & (1 << index) == 0 {
        continue;
      }
      formatter.write_str(separator)?;
      formatter.write_str(name)?;
      separator = ", ";
    }
    formatter.write_str("])")
  }
}

fn related_name_bytes(value: Option<[u8; 16]>) -> &'static str {
  value
    .and_then(|id| ObjectId::from_bytes(id).ok())
    .and_then(|id| TARGETS.iter().position(|value| *value == id))
    .map(target_name)
    .unwrap_or("outside Battlement UI")
}

fn rgba(value: battlement::Color) -> [f64; 4] {
  [value.r, value.g, value.b, value.a]
}

fn grid_card() -> UiNode {
  node(UiBox::new().style(keyboard_navigation_styles::card(true)))
    .child(node(
      UiLabel::new("FOCUS GRID").style(keyboard_navigation_styles::caption()),
    ))
    .child(UiNode::new(
      FOCUS_ID,
      UiLabel::new("○ READY · FOCUS A COMMAND").style(keyboard_navigation_styles::focus_status()),
    ))
    .child(
      node(UiVisualElement::new().style(keyboard_navigation_styles::grid()))
        .child(target(0, "ALPHA\nRecon"))
        .child(target(1, "BRAVO\nDeploy"))
        .child(target(2, "CHARLIE\nDefend"))
        .child(target(3, "DELTA\nExtract")),
    )
}

fn inspector_card() -> UiNode {
  node(UiBox::new().style(keyboard_navigation_styles::card(false)))
        .child(node(UiLabel::new("ACTIVATION INSPECTOR").style(keyboard_navigation_styles::caption())))
        .child(UiNode::new(
            INSPECTOR_ID,
            UiLabel::new("No focused input yet.\n\nMapped keys use the shared W3C PhysicalKey type. Unmapped native key codes stay None rather than becoming arbitrary strings.").style(keyboard_navigation_styles::inspector()),
        ))
        .child(node(UiLabel::new("Button submit checks the entire logical route for Click. One matching subscription receives Click::NavigationSubmit; without one, no action is emitted.").style(keyboard_navigation_styles::hint())))
}

fn target(index: usize, label: &str) -> UiNode {
  UiNode::new(
    TARGETS[index],
    UiButton::new(label)
      .name(format!("keyboard-target-{index}"))
      .events(target_kinds())
      .style(keyboard_navigation_styles::target(false)),
  )
}

fn target_kinds() -> [UiEventKind; 11] {
  [
    UiEventKind::Click,
    UiEventKind::KeyDown,
    UiEventKind::KeyUp,
    UiEventKind::NavigationMove,
    UiEventKind::NavigationCancel,
    UiEventKind::FocusIn,
    UiEventKind::Focus,
    UiEventKind::FocusOut,
    UiEventKind::Blur,
    UiEventKind::PointerEnter,
    UiEventKind::PointerLeave,
  ]
}

fn ancestor_subscriptions() -> Vec<UiEventSubscription> {
  [
    UiEventKind::KeyDown,
    UiEventKind::KeyUp,
    UiEventKind::NavigationMove,
    UiEventKind::NavigationCancel,
    UiEventKind::FocusIn,
    UiEventKind::FocusOut,
    UiEventKind::Click,
  ]
  .into_iter()
  .flat_map(|kind| {
    [
      UiEventSubscription::new(kind, UiEventPhase::Trickle),
      UiEventSubscription::new(kind, UiEventPhase::Bubble),
    ]
  })
  .collect()
}

fn related_name(value: Option<ObjectId>) -> &'static str {
  value
    .and_then(|id| TARGETS.iter().position(|value| *value == id))
    .map(target_name)
    .unwrap_or("outside Battlement UI")
}

fn target_name(index: usize) -> &'static str {
  ["ALPHA", "BRAVO", "CHARLIE", "DELTA"][index]
}

fn focus_direction_name(direction: FocusDirection) -> &'static str {
  match direction {
    FocusDirection::None => "NO DIRECTION",
    FocusDirection::Unspecified => "UNSPECIFIED",
    FocusDirection::Left => "LEFT",
    FocusDirection::Right => "RIGHT",
    FocusDirection::Other(_) => "CUSTOM DIRECTION",
  }
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
