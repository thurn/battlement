use battlement::{
  Command, F32Range, LowerLimit, ObjectId, UiBox, UiElement, UiEvent, UiEventBody, UiEventKind,
  UiLabel, UiMinMaxSlider, UiNode, UiProgressBar, UiValue, UiVisualElement, UpperLimit, object_id,
};

use crate::{design_system, range_styles};

pub(crate) const RESOURCE_RANGE_ID: ObjectId = object_id!("4be5cd99-a70d-4dca-af82-57dc73f91eea");
pub(crate) const RANGE_STATUS_ID: ObjectId = object_id!("cb0e1e49-857d-4a3b-a95e-f0dce69060d8");
const RANGE_MIN_LABEL_ID: ObjectId = object_id!("84f04b1d-ae2c-4394-893a-015077b885fa");
const RANGE_MAX_LABEL_ID: ObjectId = object_id!("8d372df4-587b-434c-a82b-c74759734136");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("range-page"))
        .child(node(
            UiLabel::new("MIN MAX SLIDER + PROGRESS BAR").style(design_system::eyebrow()),
        ))
        .child(node(
            UiLabel::new("Choose a safe window. Read the outcome.").style(design_system::title()),
        ))
        .child(node(
            UiLabel::new(
                "Two thumbs author one ordered range on release. Progress bars are display-only snapshots: they report state without pretending to accept input.",
            )
            .style(range_styles::intro()),
        ))
        .child(
            node(UiVisualElement::new().style(range_styles::gallery()))
                .child(range_card())
                .child(progress_card()),
        )
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
  match (&event.target_id, &event.body) {
    (&RESOURCE_RANGE_ID, UiEventBody::ValueChanging(value)) => {
      let UiValue::F32Range(range) = value.proposed else {
        return None;
      };
      Some(range_commands("LIVE", range))
    }
    (&RESOURCE_RANGE_ID, UiEventBody::ValueCommitted(value)) => {
      let UiValue::F32Range(range) = value.proposed else {
        return None;
      };
      let mut commands = range_commands("COMMITTED", range);
      commands.insert(
        0,
        Command::update_visual_element(
          RESOURCE_RANGE_ID,
          UiMinMaxSlider::new()
            .min_value(range.min)
            .max_value(range.max),
        ),
      );
      Some(commands)
    }
    _ => None,
  }
}

fn range_card() -> UiNode {
  node(UiBox::new().style(range_styles::range_card()))
    .child(node(
      UiLabel::new("RESOURCE WINDOW").style(range_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Bounded 0-100 · ordered dual thumbs · live preview + final commit")
        .style(range_styles::help()),
    ))
    .child(UiNode::new(
      RESOURCE_RANGE_ID,
      UiMinMaxSlider::new()
        .name("resource-range")
        .low_limit(LowerLimit::Inclusive(0.0))
        .high_limit(UpperLimit::Inclusive(100.0))
        .min_value(24.0)
        .max_value(76.0)
        .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted])
        .style(range_styles::range_slider()),
    ))
    .child(
      node(UiVisualElement::new().style(range_styles::endpoint_row()))
        .child(UiNode::new(
          RANGE_MIN_LABEL_ID,
          UiLabel::new("MIN · 24%").style(range_styles::endpoint()),
        ))
        .child(UiNode::new(
          RANGE_MAX_LABEL_ID,
          UiLabel::new("MAX · 76%").style(range_styles::endpoint()),
        )),
    )
    .child(UiNode::new(
      RANGE_STATUS_ID,
      UiLabel::new("COMMITTED  reserve 24-76%")
        .name("range-status")
        .style(range_styles::range_status()),
    ))
}

fn progress_card() -> UiNode {
  node(UiBox::new().style(range_styles::progress_card()))
    .child(node(
      UiLabel::new("STAGED PROGRESS · OUTPUT ONLY").style(range_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Three authored states; no event subscription and no interactive affordance.")
        .style(range_styles::help()),
    ))
    .child(progress("QUEUED · 12%", 12.0))
    .child(progress("STREAMING · 48%", 48.0))
    .child(progress("VERIFIED · 86%", 86.0))
}

fn progress(title: &str, value: f32) -> UiNode {
  node(
    UiProgressBar::new()
      .low_value(0.0)
      .high_value(100.0)
      .value(value)
      .title(title)
      .style(range_styles::progress()),
  )
}

fn range_commands(prefix: &str, range: F32Range) -> Vec<Command> {
  vec![
    Command::update_visual_element(
      RANGE_MIN_LABEL_ID,
      UiLabel::new(format!("MIN · {:.0}%", range.min)),
    ),
    Command::update_visual_element(
      RANGE_MAX_LABEL_ID,
      UiLabel::new(format!("MAX · {:.0}%", range.max)),
    ),
    Command::update_visual_element(
      RANGE_STATUS_ID,
      UiLabel::new(format!(
        "{prefix}  reserve {:.0}-{:.0}%",
        range.min, range.max
      )),
    ),
  ]
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
