use battlement::{
  Command, ObjectId, UiBox, UiButton, UiElement, UiEvent, UiEventBody, UiEventKind, UiLabel,
  UiNode, UiRadioButtonGroup, UiToggleButtonGroup, UiValue, UiVisualElement, object_id,
};

use crate::{choice_group_styles, design_system};

pub(crate) const FORMATION_ID: ObjectId = object_id!("34ee78d0-a503-4d77-b61d-bbd86cf39e41");
pub(crate) const FILTER_ID: ObjectId = object_id!("17805693-79d9-46ac-97db-1694047f8a9e");
pub(crate) const FILTER_SUMMARY_ID: ObjectId = object_id!("01d7f042-cdae-4e9c-8020-817d5e83ae18");
pub(crate) const FORMATION_SUMMARY_ID: ObjectId =
  object_id!("4102978d-3631-405f-aafb-1103a03b3b57");
const FILTER_AIR_ID: ObjectId = object_id!("18129142-6ea4-45ff-8a5c-ce209a9d38e3");
const FILTER_LAND_ID: ObjectId = object_id!("6900f397-8c07-4caf-84bf-d094a0a7cd75");
const FILTER_SEA_ID: ObjectId = object_id!("29d2a3ce-c825-41ca-9965-5b4502865df8");
pub(crate) const STATUS_ID: ObjectId = object_id!("6553e506-c92a-4f50-995e-58380393bb6f");
pub(crate) const HISTORY_ID: ObjectId = object_id!("84a701b8-cce9-4165-9637-9b7a24856d7d");

const FORMATIONS: [&str; 3] = ["LINE", "WEDGE", "COLUMN"];
const FILTERS: [&str; 3] = ["AIR", "LAND", "SEA"];

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("choice-groups-page"))
    .child(node(
      UiLabel::new("SELECTION GROUPS").style(design_system::eyebrow()),
    ))
    .child(node(
      UiLabel::new("Choose one. Combine many.").style(design_system::title()),
    ))
    .child(node(
      UiLabel::new(
        "Radio groups commit one index; toggle-button groups commit a sorted set of indices.",
      )
      .style(choice_group_styles::intro()),
    ))
    .child(
      node(UiVisualElement::new().style(choice_group_styles::gallery()))
        .child(formation_card())
        .child(filter_card()),
    )
    .child(inspector())
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
  let UiEventBody::ValueCommitted(value) = &event.body else {
    return None;
  };
  match event.target_id {
    FORMATION_ID => {
      let (previous, proposed) = indices(&value.previous, &value.proposed)?;
      let selected = proposed?;
      Some(vec![
        Command::update_visual_element(
          FORMATION_ID,
          UiRadioButtonGroup::new().selected_index(selected),
        ),
        Command::update_visual_element(
          STATUS_ID,
          UiLabel::new(format!(
            "FORMATION · {} committed",
            label(&FORMATIONS, selected)
          )),
        ),
        Command::update_visual_element(
          FORMATION_SUMMARY_ID,
          UiLabel::new(format!("SELECTED INDEX · {selected}")),
        ),
        Command::update_visual_element(
          HISTORY_ID,
          UiLabel::new(format!(
            "EXCLUSIVE  {} → {}  |  index {} → {}",
            optional_label(&FORMATIONS, previous),
            label(&FORMATIONS, selected),
            optional_index(previous),
            selected,
          )),
        ),
      ])
    }
    FILTER_ID => {
      let (previous, proposed) = index_sets(&value.previous, &value.proposed)?;
      let mut commands = vec![
        Command::update_visual_element(
          FILTER_ID,
          UiToggleButtonGroup::new().selected_indices(proposed.iter().copied()),
        ),
        Command::update_visual_element(
          FILTER_SUMMARY_ID,
          UiLabel::new(format!("SELECTED INDICES · {}", format_indices(proposed))),
        ),
        Command::update_visual_element(
          STATUS_ID,
          UiLabel::new(format!("FILTERS · {}", selected_labels(proposed))),
        ),
        Command::update_visual_element(
          HISTORY_ID,
          UiLabel::new(format!(
            "MULTI  {} → {}  |  sorted index set",
            format_indices(previous),
            format_indices(proposed),
          )),
        ),
      ];
      commands.extend(filter_button_commands(proposed));
      Some(commands)
    }
    _ => None,
  }
}

fn formation_card() -> UiNode {
  node(UiBox::new().style(choice_group_styles::card()))
    .child(node(
      UiLabel::new("EXCLUSIVE FORMATION").style(choice_group_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Exactly one option is committed. The event carries one zero-based index.")
        .style(choice_group_styles::help()),
    ))
    .child(UiNode::new(
      FORMATION_ID,
      UiRadioButtonGroup::new()
        .name("formation-choice")
        .label("FORMATION")
        .choices(FORMATIONS)
        .selected_index(0)
        .events([UiEventKind::ValueCommitted])
        .style(choice_group_styles::radio_group()),
    ))
    .child(UiNode::new(
      FORMATION_SUMMARY_ID,
      UiLabel::new("SELECTED INDEX · 0")
        .name("formation-summary")
        .style(choice_group_styles::selection_summary()),
    ))
}

fn filter_card() -> UiNode {
  node(UiBox::new().style(choice_group_styles::final_card()))
    .child(node(
      UiLabel::new("MULTI-SELECT FILTER").style(choice_group_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Ordinary button children become a compact, mask-backed selection set.")
        .style(choice_group_styles::help()),
    ))
    .child(node(
      UiLabel::new("UNIT FILTERS · MULTIPLE").style(choice_group_styles::field_label()),
    ))
    .child(
      UiNode::new(
        FILTER_ID,
        UiToggleButtonGroup::new()
          .name("multi-filter")
          .multiple_selection(true)
          .allow_empty_selection(true)
          .selected_indices([0, 2])
          .events([UiEventKind::ValueCommitted])
          .style(choice_group_styles::toggle_group()),
      )
      .child(filter_button(FILTER_AIR_ID, "filter-air", "AIR", true))
      .child(filter_button(FILTER_LAND_ID, "filter-land", "LAND", false))
      .child(filter_button(FILTER_SEA_ID, "filter-sea", "SEA", true)),
    )
    .child(UiNode::new(
      FILTER_SUMMARY_ID,
      UiLabel::new("SELECTED INDICES · [0, 2]")
        .name("filter-summary")
        .style(choice_group_styles::selection_summary()),
    ))
}

fn filter_button(object_id: ObjectId, name: &str, text: &str, selected: bool) -> UiNode {
  UiNode::new(
    object_id,
    UiButton::new(filter_button_text(text, selected))
      .name(name)
      .style(choice_group_styles::toggle_button(selected)),
  )
}

fn filter_button_commands(selected: &[u32]) -> Vec<Command> {
  [
    (FILTER_AIR_ID, "AIR"),
    (FILTER_LAND_ID, "LAND"),
    (FILTER_SEA_ID, "SEA"),
  ]
  .into_iter()
  .enumerate()
  .map(|(index, (object_id, text))| {
    let active = selected.binary_search(&(index as u32)).is_ok();
    Command::update_visual_element(
      object_id,
      UiButton::new(filter_button_text(text, active))
        .style(choice_group_styles::toggle_button(active)),
    )
  })
  .collect()
}

fn filter_button_text(text: &str, selected: bool) -> String {
  format!("{text}  {}", if selected { "ON" } else { "OFF" })
}

fn inspector() -> UiNode {
  node(UiBox::new().style(choice_group_styles::inspector()))
    .child(UiNode::new(
      STATUS_ID,
      UiLabel::new("READY · controlled selection groups")
        .name("choice-status")
        .style(choice_group_styles::status()),
    ))
    .child(UiNode::new(
      HISTORY_ID,
      UiLabel::new("RADIO [0]  |  MULTI [0, 2]")
        .name("choice-history")
        .style(choice_group_styles::history()),
    ))
}

fn indices(previous: &UiValue, proposed: &UiValue) -> Option<(Option<u32>, Option<u32>)> {
  match (previous, proposed) {
    (UiValue::Index(previous), UiValue::Index(proposed)) => Some((*previous, *proposed)),
    _ => None,
  }
}

fn index_sets<'a>(previous: &'a UiValue, proposed: &'a UiValue) -> Option<(&'a [u32], &'a [u32])> {
  match (previous, proposed) {
    (UiValue::Indices(previous), UiValue::Indices(proposed)) => Some((previous, proposed)),
    _ => None,
  }
}

fn optional_label<'a>(values: &'a [&'a str], index: Option<u32>) -> &'a str {
  index.map_or("NONE", |value| label(values, value))
}

fn label<'a>(values: &'a [&'a str], index: u32) -> &'a str {
  values[index as usize]
}

fn optional_index(index: Option<u32>) -> String {
  index.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn format_indices(values: &[u32]) -> String {
  format!(
    "[{}]",
    values
      .iter()
      .map(u32::to_string)
      .collect::<Vec<_>>()
      .join(", ")
  )
}

fn selected_labels(values: &[u32]) -> String {
  if values.is_empty() {
    return "NONE".to_owned();
  }
  values
    .iter()
    .map(|index| label(&FILTERS, *index))
    .collect::<Vec<_>>()
    .join(" + ")
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
