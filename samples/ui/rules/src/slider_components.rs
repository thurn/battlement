use battlement::{
  ObjectId, SliderDirection, UiBox, UiElement, UiEventKind, UiLabel, UiNode, UiSlider, UiSliderInt,
  UiVisualElement, object_id,
};
use battlement_native::{EngineError, UiEventActionView, UiValueView};

use crate::{design_system, native_ui::NativeUiResponseBuilder, slider_styles};

pub(crate) const CONTINUOUS_ID: ObjectId = object_id!("08e45324-236a-469d-a4f8-f2f40922a9b8");
pub(crate) const STEPPED_ID: ObjectId = object_id!("c1ad6472-f8ae-40cb-9d21-60f6e544db53");
pub(crate) const CONTINUOUS_VALUE_ID: ObjectId = object_id!("27420acd-df31-45fa-99c2-4bf6bde37f7e");
pub(crate) const STEPPED_VALUE_ID: ObjectId = object_id!("12988004-2b5a-4d6d-9eb6-4960f656394b");
pub(crate) const LIVE_STATUS_ID: ObjectId = object_id!("13ba592a-5f70-4a64-892a-21a919479e5d");
pub(crate) const COMMIT_STATUS_ID: ObjectId = object_id!("0d1be49a-b9fc-437d-8d48-d2724e7efe1f");

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("slider-page"))
        .child(node(UiLabel::new("SLIDER + SLIDER INT").style(design_system::eyebrow())))
        .child(node(
            UiLabel::new("Tune continuously. Commit once.").style(design_system::title()),
        ))
        .child(node(
            UiLabel::new(
                "Native drag values stay local while Rust observes optional live proposals. Release sends one final value for Rust to author or reject.",
            )
            .style(slider_styles::intro()),
        ))
        .child(
            node(UiVisualElement::new().style(slider_styles::gallery()))
                .child(continuous_card())
                .child(stepped_card()),
        )
        .child(inspector())
}

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, EngineError> {
  let target_id = ObjectId::from_bytes(event.target_id()).expect("validated UI target UUID");
  let value = match event.event_kind() {
    UiEventKind::ValueChanging => match event.value_changing() {
      Some(value) => value,
      None => return Ok(false),
    },
    UiEventKind::ValueCommitted => match event.value_commit() {
      Some(value) => value.proposed(),
      None => return Ok(false),
    },
    _ => return Ok(false),
  };
  match (target_id, event.event_kind(), value) {
    (CONTINUOUS_ID, UiEventKind::ValueChanging, UiValueView::F32(proposed)) => {
      response.label(
        LIVE_STATUS_ID,
        &format!("LIVE  thrust trim  {proposed:.1}%"),
      )?;
    }
    (STEPPED_ID, UiEventKind::ValueChanging, UiValueView::I32(proposed)) => {
      response.label(LIVE_STATUS_ID, &format!("LIVE  shield step  {proposed}"))?;
    }
    (CONTINUOUS_ID, UiEventKind::ValueCommitted, UiValueView::F32(proposed)) => {
      let slider = response
        .writer()
        .slider_builder()
        .float_value(proposed)
        .finish();
      response.update(CONTINUOUS_ID, slider)?;
      response.label(CONTINUOUS_VALUE_ID, &format!("FINAL · {proposed:.1}%"))?;
      response.label(
        COMMIT_STATUS_ID,
        &format!("COMMITTED  horizontal value {proposed:.1}"),
      )?;
    }
    (STEPPED_ID, UiEventKind::ValueCommitted, UiValueView::I32(proposed)) => {
      let slider = response
        .writer()
        .slider_int_builder()
        .int_value(proposed)
        .finish();
      response.update(STEPPED_ID, slider)?;
      response.label(STEPPED_VALUE_ID, &format!("FINAL · STEP {proposed}"))?;
      response.label(
        COMMIT_STATUS_ID,
        &format!("COMMITTED  vertical integer {proposed}"),
      )?;
    }
    _ => return Ok(false),
  }
  Ok(true)
}

fn continuous_card() -> UiNode {
  node(UiBox::new().style(slider_styles::card()))
    .child(node(
      UiLabel::new("CONTINUOUS + FILLED").style(slider_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Horizontal float · 0–100 · page 5 · editable numeric field")
        .style(slider_styles::help()),
    ))
    .child(UiNode::new(
      CONTINUOUS_ID,
      UiSlider::new()
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
      UiLabel::new("FINAL · 42.0%")
        .name("continuous-final-value")
        .style(slider_styles::final_value()),
    ))
}

fn stepped_card() -> UiNode {
  node(UiBox::new().style(slider_styles::final_card()))
    .child(node(
      UiLabel::new("STEPPED + INVERTED").style(slider_styles::caption()),
    ))
    .child(node(
      UiLabel::new("Vertical integer · 0–8 · top is low · exact whole steps")
        .style(slider_styles::help()),
    ))
    .child(
      node(UiVisualElement::new().style(slider_styles::vertical_row()))
        .child(UiNode::new(
          STEPPED_ID,
          UiSliderInt::new()
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
          node(UiVisualElement::new().style(slider_styles::scale()))
            .child(node(
              UiLabel::new("0  LOW").style(slider_styles::scale_label()),
            ))
            .child(node(
              UiLabel::new("4  MID").style(slider_styles::scale_label()),
            ))
            .child(node(
              UiLabel::new("8  HIGH").style(slider_styles::scale_label()),
            )),
        ),
    )
    .child(UiNode::new(
      STEPPED_VALUE_ID,
      UiLabel::new("FINAL · STEP 3")
        .name("stepped-final-value")
        .style(slider_styles::final_value()),
    ))
}

fn inspector() -> UiNode {
  node(UiBox::new().style(slider_styles::inspector()))
    .child(UiNode::new(
      LIVE_STATUS_ID,
      UiLabel::new("LIVE  waiting for pointer capture")
        .name("slider-live-status")
        .style(slider_styles::live_status()),
    ))
    .child(UiNode::new(
      COMMIT_STATUS_ID,
      UiLabel::new("COMMITTED  42.0 float  ·  3 integer")
        .name("slider-commit-status")
        .style(slider_styles::commit_status()),
    ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
