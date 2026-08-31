use battlement::{
  Command, ObjectId, PickingMode, UiButton, UiElement, UiEvent, UiEventBody, UiEventKind, UiImage,
  UiLabel, UiNode, UiVisualElement, object_id,
};

use crate::{asset_catalog::ui::assets, render_mode_styles};

pub(crate) const DETAILS_BUTTON_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000004");
const DETAILS_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000005");

pub(crate) fn page(page_id: ObjectId, details_expanded: bool) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("render-modes-page"))
    .child(node(
      UiLabel::new("DOCUMENT RENDERED TO TEXTURE").style(render_mode_styles::page_title()),
    ))
    .child(node(
      UiLabel::new("A separate UI document, displayed here as a live texture.")
        .style(render_mode_styles::intro()),
    ))
    .child(
      node(UiVisualElement::new().style(render_mode_styles::composition()))
        .child(target_preview())
        .child(scale_contracts(details_expanded)),
    )
}

pub(crate) fn target_document(root_id: ObjectId) -> UiNode {
  UiNode::new(
    root_id,
    UiVisualElement::new()
      .name("target-texture-document")
      .picking_mode(PickingMode::Ignore)
      .style(render_mode_styles::target_root()),
  )
  .child(node(
    UiLabel::new("BATTLEMENT SIGNAL")
      .picking_mode(PickingMode::Ignore)
      .style(render_mode_styles::target_title()),
  ))
  .child(node(
    UiLabel::new("● LIVE")
      .picking_mode(PickingMode::Ignore)
      .style(render_mode_styles::target_status()),
  ))
}

pub(crate) fn event_commands(event: &UiEvent, details_expanded: &mut bool) -> Option<Vec<Command>> {
  if event.target_id != DETAILS_BUTTON_ID {
    return None;
  }
  match event.body {
    UiEventBody::Click(_) => {
      *details_expanded = !*details_expanded;
      Some(vec![
        Command::update_visual_element(DETAILS_BUTTON_ID, details_button(*details_expanded, true)),
        Command::update_visual_element(
          DETAILS_ID,
          UiVisualElement::new().style(render_mode_styles::details(*details_expanded)),
        ),
      ])
    }
    UiEventBody::FocusIn(_) => Some(vec![Command::update_visual_element(
      DETAILS_BUTTON_ID,
      details_button(*details_expanded, true),
    )]),
    UiEventBody::FocusOut(_) => Some(vec![Command::update_visual_element(
      DETAILS_BUTTON_ID,
      details_button(*details_expanded, false),
    )]),
    _ => None,
  }
}

fn target_preview() -> UiNode {
  node(UiVisualElement::new().style(render_mode_styles::preview_column())).child(node(
    UiImage::new()
      .source(assets::RENDER_TEXTURE.clone())
      .style(render_mode_styles::monitor_image()),
  ))
}

fn scale_contracts(details_expanded: bool) -> UiNode {
  node(UiVisualElement::new().style(render_mode_styles::contracts()))
    .child(node(
      UiLabel::new("CURRENT SCALE").style(render_mode_styles::contract_heading()),
    ))
    .child(mode("CONSTANT PIXEL"))
    .child(UiNode::new(
      DETAILS_BUTTON_ID,
      details_button(details_expanded, false),
    ))
    .child(
      UiNode::new(
        DETAILS_ID,
        UiVisualElement::new().style(render_mode_styles::details(details_expanded)),
      )
      .child(node(
        UiLabel::new("ALTERNATIVE CONTRACTS").style(render_mode_styles::detail_heading()),
      ))
      .child(node(
        UiLabel::new("Physical Size · scales from display DPI").style(render_mode_styles::detail()),
      ))
      .child(node(
        UiLabel::new("Screen Size · scales from viewport dimensions")
          .style(render_mode_styles::detail()),
      ))
      .child(node(
        UiLabel::new("CURRENT OUTPUT").style(render_mode_styles::detail_heading()),
      ))
      .child(node(
        UiLabel::new("Scale 1.0 · canvas 1280 × 720 · output 512 × 384")
          .style(render_mode_styles::detail()),
      ))
      .child(node(
        UiLabel::new("Reference DPI 96 · fallback 110 · display 0")
          .style(render_mode_styles::detail()),
      ))
      .child(node(
        UiLabel::new("Pointer input requires coordinate mapping")
          .style(render_mode_styles::detail()),
      )),
    )
}

fn mode(title: &str) -> UiNode {
  node(UiVisualElement::new().style(render_mode_styles::mode())).child(node(
    UiLabel::new(format!("{title}  ·  ACTIVE")).style(render_mode_styles::mode_name()),
  ))
}

fn details_button(expanded: bool, focused: bool) -> UiButton {
  UiButton::new(if expanded {
    "HIDE DETAILS"
  } else {
    "SHOW DETAILS"
  })
  .name("panel-target-details")
  .focusable(true)
  .events([
    UiEventKind::Click,
    UiEventKind::FocusIn,
    UiEventKind::FocusOut,
  ])
  .style(render_mode_styles::details_button(focused))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
