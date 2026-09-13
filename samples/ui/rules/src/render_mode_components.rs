use battlement::{
  ObjectId, PickingMode, UiButton, UiElement, UiEventKind, UiImage, UiLabel, UiNode,
  UiVisualElement, object_id,
};
use battlement_native::UiEventActionView;

use crate::{
  asset_catalog::ui::assets, design_system, native_ui::NativeUiResponseBuilder, render_mode_styles,
};

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

pub(crate) fn write_event_response(
  event: UiEventActionView<'_>,
  details_expanded: &mut bool,
  response: &mut NativeUiResponseBuilder,
) -> Result<bool, battlement_native::EngineError> {
  if ObjectId::from_bytes(event.target_id()).ok() != Some(DETAILS_BUTTON_ID) {
    return Ok(false);
  }
  let focused = match event.event_kind() {
    UiEventKind::Click => {
      *details_expanded = !*details_expanded;
      true
    }
    UiEventKind::FocusIn => true,
    UiEventKind::FocusOut => false,
    _ => return Ok(false),
  };
  let border = if focused {
    design_system::ACCENT
  } else {
    battlement::Color::rgb(0.12, 0.40, 0.44)
  };
  let button = response
    .writer()
    .button_builder()
    .text(if *details_expanded {
      "HIDE DETAILS"
    } else {
      "SHOW DETAILS"
    })
    .height(44.0)
    .background_color(rgba(battlement::Color::rgb(0.035, 0.12, 0.14)))
    .color(rgba(design_system::CYAN))
    .border_color(rgba(border))
    .border_width(if focused { 3.0 } else { 1.0 })
    .border_radius(6.0)
    .font_size(13.0)
    .text_align_middle_center()
    .margin_edges(4.0, 0.0, 0.0, 0.0)
    .finish();
  response.update(DETAILS_BUTTON_ID, button)?;
  if event.event_kind() == UiEventKind::Click {
    let details = response
      .writer()
      .visual_element_builder()
      .displayed(*details_expanded)
      .background_color(rgba(battlement::Color::rgb(0.025, 0.085, 0.105)))
      .padding(12.0, 12.0)
      .margin_edges(8.0, 0.0, 0.0, 0.0)
      .finish();
    response.update(DETAILS_ID, details)?;
  }
  Ok(true)
}

fn rgba(value: battlement::Color) -> [f64; 4] {
  [value.r, value.g, value.b, value.a]
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
