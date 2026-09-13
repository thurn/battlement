use battlement::{
  ObjectId, UiBox, UiButton, UiElement, UiEventKind, UiLabel, UiNode, UiRadioButtonGroup,
  UiToggleButtonGroup, UiVisualElement, object_id,
};
use battlement_native::{EngineError, UiEventActionView, UiValueView};

use crate::{choice_group_styles, design_system, native_ui::NativeUiResponseBuilder};

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

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, EngineError> {
  if event.event_kind() != UiEventKind::ValueCommitted {
    return Ok(false);
  }
  let target_id = ObjectId::from_bytes(event.target_id()).expect("validated UI target UUID");
  let Some(commit) = event.value_commit() else {
    return Ok(false);
  };
  match target_id {
    FORMATION_ID => {
      let (UiValueView::Index(previous), UiValueView::Index(Some(selected))) =
        (commit.previous(), commit.proposed())
      else {
        return Ok(false);
      };
      let group = response
        .writer()
        .radio_button_group_builder()
        .selected_index(Some(selected))
        .finish();
      response.update(FORMATION_ID, group)?;
      response.label(
        STATUS_ID,
        &format!("FORMATION · {} committed", label(&FORMATIONS, selected)),
      )?;
      response.label(
        FORMATION_SUMMARY_ID,
        &format!("SELECTED INDEX · {selected}"),
      )?;
      response.label(
        HISTORY_ID,
        &format!(
          "EXCLUSIVE  {} → {}  |  index {} → {}",
          optional_label(&FORMATIONS, previous),
          label(&FORMATIONS, selected),
          optional_index(previous),
          selected,
        ),
      )?;
    }
    FILTER_ID => {
      let (UiValueView::Indices(previous), UiValueView::Indices(proposed)) =
        (commit.previous(), commit.proposed())
      else {
        return Ok(false);
      };
      let previous = previous.iter().collect::<Vec<_>>();
      let proposed = proposed.iter().collect::<Vec<_>>();
      let group = response
        .writer()
        .toggle_button_group_builder()
        .selected_indices(proposed.iter().copied())
        .finish();
      response.update(FILTER_ID, group)?;
      response.label(
        FILTER_SUMMARY_ID,
        &format!("SELECTED INDICES · {}", format_indices(&proposed)),
      )?;
      response.label(
        STATUS_ID,
        &format!("FILTERS · {}", selected_labels(&proposed)),
      )?;
      response.label(
        HISTORY_ID,
        &format!(
          "MULTI  {} → {}  |  sorted index set",
          format_indices(&previous),
          format_indices(&proposed),
        ),
      )?;
      for (index, (object_id, text)) in [
        (FILTER_AIR_ID, "AIR"),
        (FILTER_LAND_ID, "LAND"),
        (FILTER_SEA_ID, "SEA"),
      ]
      .into_iter()
      .enumerate()
      {
        let active = proposed.binary_search(&(index as u32)).is_ok();
        let (background, foreground) = if active {
          ([0.98, 0.72, 0.24, 1.0], [0.012, 0.025, 0.045, 1.0])
        } else {
          ([0.045, 0.12, 0.14, 1.0], [0.94, 0.98, 0.99, 1.0])
        };
        let button = response
          .writer()
          .button_builder()
          .text(&filter_button_text(text, active))
          .background_color(background)
          .color(foreground)
          .finish();
        response.update(object_id, button)?;
      }
    }
    _ => return Ok(false),
  }
  Ok(true)
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
