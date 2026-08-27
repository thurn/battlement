use battlement::{
  Box, Command, Label, ObjectId, Slider, SliderDirection, SliderInt, UiElement, UiEvent,
  UiEventBody, UiEventKind, UiNode, UiValue, VisualElement, object_id,
};

use crate::{design_system, slider_styles};

pub(crate) const CONTINUOUS_ID: ObjectId = object_id!("08e45324-236a-469d-a4f8-f2f40922a9b8");
pub(crate) const STEPPED_ID: ObjectId = object_id!("c1ad6472-f8ae-40cb-9d21-60f6e544db53");
pub(crate) const CONTINUOUS_VALUE_ID: ObjectId = object_id!("27420acd-df31-45fa-99c2-4bf6bde37f7e");
pub(crate) const STEPPED_VALUE_ID: ObjectId = object_id!("12988004-2b5a-4d6d-9eb6-4960f656394b");
pub(crate) const LIVE_STATUS_ID: ObjectId = object_id!("13ba592a-5f70-4a64-892a-21a919479e5d");
pub(crate) const COMMIT_STATUS_ID: ObjectId = object_id!("0d1be49a-b9fc-437d-8d48-d2724e7efe1f");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, VisualElement::new().name("slider-page"))
        .child(node(Label::new("SLIDER + SLIDER INT").style(design_system::eyebrow())))
        .child(node(
            Label::new("Tune continuously. Commit once.").style(design_system::title()),
        ))
        .child(node(
            Label::new(
                "Native drag values stay local while Rust observes optional live proposals. Release sends one final value for Rust to author or reject.",
            )
            .style(slider_styles::intro()),
        ))
        .child(
            node(VisualElement::new().style(slider_styles::gallery()))
                .child(continuous_card())
                .child(stepped_card()),
        )
        .child(inspector())
}

pub(crate) fn event_commands(event: &UiEvent) -> Option<Vec<Command>> {
  match (&event.target_id, &event.body) {
    (&CONTINUOUS_ID, UiEventBody::ValueChanging(value)) => {
      let UiValue::F32(proposed) = value.proposed else {
        return None;
      };
      Some(vec![Command::update_visual_element(
        LIVE_STATUS_ID,
        Label::new(format!("LIVE  thrust trim  {proposed:.1}%")),
      )])
    }
    (&STEPPED_ID, UiEventBody::ValueChanging(value)) => {
      let UiValue::I32(proposed) = value.proposed else {
        return None;
      };
      Some(vec![Command::update_visual_element(
        LIVE_STATUS_ID,
        Label::new(format!("LIVE  shield step  {proposed}")),
      )])
    }
    (&CONTINUOUS_ID, UiEventBody::ValueCommitted(value)) => {
      let UiValue::F32(proposed) = value.proposed else {
        return None;
      };
      Some(vec![
        Command::update_visual_element(CONTINUOUS_ID, Slider::new().value(proposed)),
        Command::update_visual_element(
          CONTINUOUS_VALUE_ID,
          Label::new(format!("FINAL · {proposed:.1}%")),
        ),
        Command::update_visual_element(
          COMMIT_STATUS_ID,
          Label::new(format!("COMMITTED  horizontal value {proposed:.1}")),
        ),
      ])
    }
    (&STEPPED_ID, UiEventBody::ValueCommitted(value)) => {
      let UiValue::I32(proposed) = value.proposed else {
        return None;
      };
      Some(vec![
        Command::update_visual_element(STEPPED_ID, SliderInt::new().value(proposed)),
        Command::update_visual_element(
          STEPPED_VALUE_ID,
          Label::new(format!("FINAL · STEP {proposed}")),
        ),
        Command::update_visual_element(
          COMMIT_STATUS_ID,
          Label::new(format!("COMMITTED  vertical integer {proposed}")),
        ),
      ])
    }
    _ => None,
  }
}

fn continuous_card() -> UiNode {
  node(Box::new().style(slider_styles::card()))
    .child(node(
      Label::new("CONTINUOUS + FILLED").style(slider_styles::caption()),
    ))
    .child(node(
      Label::new("Horizontal float · 0–100 · page 5 · editable numeric field")
        .style(slider_styles::help()),
    ))
    .child(UiNode::new(
      CONTINUOUS_ID,
      Slider::new()
        .name("continuous-slider")
        .label("THRUST TRIM")
        .low_value(0.0)
        .high_value(100.0)
        .value(42.0)
        .page_size(5.0)
        .fill(true)
        .show_input_field(true)
        .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted])
        .style(slider_styles::horizontal_slider()),
    ))
    .child(UiNode::new(
      CONTINUOUS_VALUE_ID,
      Label::new("FINAL · 42.0%")
        .name("continuous-final-value")
        .style(slider_styles::final_value()),
    ))
}

fn stepped_card() -> UiNode {
  node(Box::new().style(slider_styles::final_card()))
    .child(node(
      Label::new("STEPPED + INVERTED").style(slider_styles::caption()),
    ))
    .child(node(
      Label::new("Vertical integer · 0–8 · top is low · exact whole steps")
        .style(slider_styles::help()),
    ))
    .child(
      node(VisualElement::new().style(slider_styles::vertical_row()))
        .child(UiNode::new(
          STEPPED_ID,
          SliderInt::new()
            .name("stepped-slider")
            .label("SHIELD")
            .low_value(0)
            .high_value(8)
            .value(3)
            .page_size(1.0)
            .fill(true)
            .direction(SliderDirection::Vertical)
            .inverted(true)
            .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted])
            .style(slider_styles::vertical_slider()),
        ))
        .child(
          node(VisualElement::new().style(slider_styles::scale()))
            .child(node(
              Label::new("0  LOW").style(slider_styles::scale_label()),
            ))
            .child(node(
              Label::new("4  MID").style(slider_styles::scale_label()),
            ))
            .child(node(
              Label::new("8  HIGH").style(slider_styles::scale_label()),
            )),
        ),
    )
    .child(UiNode::new(
      STEPPED_VALUE_ID,
      Label::new("FINAL · STEP 3")
        .name("stepped-final-value")
        .style(slider_styles::final_value()),
    ))
}

fn inspector() -> UiNode {
  node(Box::new().style(slider_styles::inspector()))
    .child(UiNode::new(
      LIVE_STATUS_ID,
      Label::new("LIVE  waiting for pointer capture")
        .name("slider-live-status")
        .style(slider_styles::live_status()),
    ))
    .child(UiNode::new(
      COMMIT_STATUS_ID,
      Label::new("COMMITTED  42.0 float  ·  3 integer")
        .name("slider-commit-status")
        .style(slider_styles::commit_status()),
    ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
