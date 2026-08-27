use battlement::{
  Button, Command, Image, Label, ObjectId, UiElement, UiEvent, UiEventBody, UiEventKind, UiNode,
  VisualElement, object_id,
};

use crate::{asset_catalog::ui::assets, render_mode_styles};

pub(crate) const DETAILS_BUTTON_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000004");
const DETAILS_ID: ObjectId = object_id!("26100000-0000-4000-8000-000000000005");

pub(crate) fn page(page_id: ObjectId, details_expanded: bool) -> UiNode {
  UiNode::new(page_id, VisualElement::new().name("render-modes-page"))
    .child(node(
      Label::new("DOCUMENT RENDERED TO TEXTURE").style(render_mode_styles::page_title()),
    ))
    .child(node(
      Label::new("A separate UI document, displayed here as a live texture.")
        .style(render_mode_styles::intro()),
    ))
    .child(
      node(VisualElement::new().style(render_mode_styles::composition()))
        .child(target_preview())
        .child(scale_contracts(details_expanded)),
    )
}

pub(crate) fn target_document(root_id: ObjectId) -> UiNode {
  UiNode::new(
    root_id,
    VisualElement::new()
      .name("target-texture-document")
      .style(render_mode_styles::target_root()),
  )
  .child(node(
    Label::new("BATTLEMENT SIGNAL").style(render_mode_styles::target_title()),
  ))
  .child(node(
    Label::new("● LIVE").style(render_mode_styles::target_status()),
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
          VisualElement::new().style(render_mode_styles::details(*details_expanded)),
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
  node(VisualElement::new().style(render_mode_styles::preview_column())).child(node(
    Image::new()
      .source(assets::RENDER_TEXTURE.clone())
      .style(render_mode_styles::monitor_image()),
  ))
}

fn scale_contracts(details_expanded: bool) -> UiNode {
  node(VisualElement::new().style(render_mode_styles::contracts()))
    .child(node(
      Label::new("CURRENT SCALE").style(render_mode_styles::contract_heading()),
    ))
    .child(mode("CONSTANT PIXEL"))
    .child(UiNode::new(
      DETAILS_BUTTON_ID,
      details_button(details_expanded, false),
    ))
    .child(
      UiNode::new(
        DETAILS_ID,
        VisualElement::new().style(render_mode_styles::details(details_expanded)),
      )
      .child(node(
        Label::new("ALTERNATIVE CONTRACTS").style(render_mode_styles::detail_heading()),
      ))
      .child(node(
        Label::new("Physical Size · scales from display DPI").style(render_mode_styles::detail()),
      ))
      .child(node(
        Label::new("Screen Size · scales from viewport dimensions")
          .style(render_mode_styles::detail()),
      ))
      .child(node(
        Label::new("CURRENT OUTPUT").style(render_mode_styles::detail_heading()),
      ))
      .child(node(
        Label::new("Scale 1.0 · canvas 1280 × 720 · output 512 × 384")
          .style(render_mode_styles::detail()),
      ))
      .child(node(
        Label::new("Reference DPI 96 · fallback 110 · display 0")
          .style(render_mode_styles::detail()),
      ))
      .child(node(
        Label::new("Pointer input requires coordinate mapping").style(render_mode_styles::detail()),
      )),
    )
}

fn mode(title: &str) -> UiNode {
  node(VisualElement::new().style(render_mode_styles::mode())).child(node(
    Label::new(format!("{title}  ·  ACTIVE")).style(render_mode_styles::mode_name()),
  ))
}

fn details_button(expanded: bool, focused: bool) -> Button {
  Button::new(if expanded {
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
