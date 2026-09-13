use battlement::{
  ObjectId, ScrollViewMode, ScrollerVisibility, SliderDirection, UiBox, UiElement, UiEventKind,
  UiLabel, UiNode, UiScrollView, UiScroller, UiVisualElement, object_id,
};
use battlement_native::{EngineError, UiEventActionView, UiValueView};

use crate::{design_system, native_ui::NativeUiResponseBuilder, scroll_styles};

const PRIMARY_ID: ObjectId = object_id!("d24fec17-cb8a-4b9c-a604-da4113d6ef9b");
const SCROLLER_ID: ObjectId = object_id!("df12adf3-3a6c-4900-bb15-1f53117f1a8e");
const SCROLL_STATUS_ID: ObjectId = object_id!("898a986b-893d-48d8-bd68-5d39ef58c086");
const SCROLLER_STATUS_ID: ObjectId = object_id!("a7338149-f968-40a3-9bdd-e7640546e2fe");

pub(crate) struct ScrollIds {
  pub(crate) primary: ObjectId,
  pub(crate) scroller: ObjectId,
  pub(crate) scroll_status: ObjectId,
  pub(crate) scroller_status: ObjectId,
}

pub(crate) fn ids() -> ScrollIds {
  ScrollIds {
    primary: PRIMARY_ID,
    scroller: SCROLLER_ID,
    scroll_status: SCROLL_STATUS_ID,
    scroller_status: SCROLLER_STATUS_ID,
  }
}

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, EngineError> {
  let target_id = ObjectId::from_bytes(event.target_id()).expect("validated UI target UUID");
  match (target_id, event.event_kind()) {
    (PRIMARY_ID, UiEventKind::ScrollChanged) => response.label(SCROLL_STATUS_ID, "Moving")?,
    (PRIMARY_ID, UiEventKind::ScrollSettled) => {
      let Some((x, y)) = event.scroll_offset() else {
        return Ok(false);
      };
      response.label(SCROLL_STATUS_ID, &format!("Settled {x:.0} × {y:.0}"))?;
    }
    (SCROLLER_ID, UiEventKind::ValueChanging) => {
      let Some(UiValueView::F32(proposed)) = event.value_changing() else {
        return Ok(false);
      };
      response.label(SCROLLER_STATUS_ID, &format!("Preview {proposed:.0}"))?;
    }
    (SCROLLER_ID, UiEventKind::ValueCommitted) => {
      let Some(commit) = event.value_commit() else {
        return Ok(false);
      };
      let UiValueView::F32(proposed) = commit.proposed() else {
        return Ok(false);
      };
      let scroller = response
        .writer()
        .scroller_builder()
        .float_value(proposed)
        .finish();
      response.update(SCROLLER_ID, scroller)?;
      response.label(SCROLLER_STATUS_ID, &format!("Committed {proposed:.0}"))?;
    }
    _ => return Ok(false),
  }
  Ok(true)
}

pub(crate) fn scroll_page(page_id: ObjectId, ids: &ScrollIds) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("scroll-page"))
    .child(node(
      UiLabel::new("SCROLL CONTROLS").style(design_system::eyebrow()),
    ))
    .child(node(
      UiLabel::new("Motion, bounded and owned").style(design_system::title()),
    ))
    .child(
      node(UiVisualElement::new().style(scroll_styles::layout()))
        .child(
          node(UiBox::new().style(scroll_styles::scroll_specimen()))
            .child(node(
              UiLabel::new("TWO-AXIS SCROLL").style(scroll_styles::caption()),
            ))
            .child(
              UiNode::new(
                ids.primary,
                UiScrollView::new()
                  .name("primary-scroll")
                  .mode(ScrollViewMode::VerticalAndHorizontal)
                  .horizontal_scroller_visibility(ScrollerVisibility::AlwaysVisible)
                  .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
                  .mouse_wheel_scroll_size(1.0)
                  .events([UiEventKind::ScrollChanged, UiEventKind::ScrollSettled])
                  .style(scroll_styles::primary_scroll()),
              )
              .child(
                node(UiVisualElement::new().style(scroll_styles::map()))
                  .child(node(
                    UiLabel::new("SECTOR GRID").style(scroll_styles::map_title()),
                  ))
                  .child(gallery())
                  .child(node(
                    UiLabel::new("Beyond the viewport").style(scroll_styles::map_note()),
                  )),
              ),
            )
            .child(UiNode::new(
              ids.scroll_status,
              UiLabel::new("Settled 0 × 0")
                .name("scroll-settlement-status")
                .style(scroll_styles::status()),
            )),
        )
        .child(
          node(UiBox::new().style(scroll_styles::control_specimen()))
            .child(node(
              UiLabel::new("CONTROLLED VALUE").style(scroll_styles::caption()),
            ))
            .child(node(
              UiLabel::new("Rust owns release").style(scroll_styles::control_heading()),
            ))
            .child(UiNode::new(
              ids.scroller,
              UiScroller::new()
                .name("controlled-scroller")
                .low_value(0.0)
                .high_value(100.0)
                .value(42.0)
                .direction(SliderDirection::Horizontal)
                .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted])
                .style(scroll_styles::scroller()),
            ))
            .child(UiNode::new(
              ids.scroller_status,
              UiLabel::new("Committed 42")
                .name("scroller-value-status")
                .style(scroll_styles::value()),
            ))
            .child(node(
              UiLabel::new("Drag and release").style(scroll_styles::control_note()),
            )),
        ),
    )
}

fn gallery() -> UiNode {
  node(UiVisualElement::new().style(scroll_styles::gallery())).children([
    gallery_card("ALPHA", "Ready"),
    gallery_card("BRAVO", "Moving"),
    gallery_card("CHARLIE", "Holding"),
    gallery_card("DELTA", "Clear"),
  ])
}

fn gallery_card(title: &str, status: &str) -> UiNode {
  node(UiBox::new().style(scroll_styles::card()))
    .child(node(UiLabel::new(title).style(scroll_styles::card_title())))
    .child(node(
      UiLabel::new(status).style(scroll_styles::card_status()),
    ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
