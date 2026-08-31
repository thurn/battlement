use battlement::{
  ObjectId, UiBox, UiButton, UiElement, UiEventKind, UiImage, UiLabel, UiNode, UiVisualElement,
};

use crate::{asset_catalog::ui::assets, design_system, world_space_styles};

pub(crate) fn page(page_id: ObjectId) -> UiNode {
  UiNode::new(
        page_id,
        UiVisualElement::new()
            .name("world-space-page")
            .picking_mode(battlement::PickingMode::Ignore),
    )
        .child(node(UiLabel::new("WORLD-SPACE INPUT").style(design_system::eyebrow())))
        .child(node(
            UiLabel::new("One input route. Three panel modes.").style(world_space_styles::page_title()),
        ))
        .child(node(UiLabel::new("The screen shell explains the contract, the monitor displays a leased target texture, and the cyan console is a real world-space UIDocument picked through Unity's process-wide panel input configuration.").style(world_space_styles::intro())))
        .child(
            node(UiVisualElement::new().style(world_space_styles::columns()))
                .child(contract_card())
                .child(target_card()),
        )
        .child(node(
            UiLabel::new("SCREEN OVERLAY   •   TARGET TEXTURE   •   WORLD DOCUMENT")
                .style(world_space_styles::stage_footer()),
        ))
}

pub(crate) fn document(root_id: ObjectId, button_id: ObjectId, status_id: ObjectId) -> UiNode {
  UiNode::new(
    root_id,
    UiVisualElement::new()
      .name("world-space-console")
      .style(world_space_styles::world_root()),
  )
  .child(
    node(UiBox::new().style(world_space_styles::world_panel()))
      .child(node(
        UiLabel::new("WORLD CONSOLE  /  LIVE").style(world_space_styles::caption()),
      ))
      .child(node(
        UiLabel::new("Ray-picked UI Toolkit panel").style(world_space_styles::world_title()),
      ))
      .child(node(
        UiLabel::new("Layer 0  •  25-unit inclusive range  •  explicit camera")
          .style(world_space_styles::detail()),
      ))
      .child(UiNode::new(
        button_id,
        UiButton::new("ACTIVATE WORLD CONTROL")
          .name("world-space-action")
          .events([UiEventKind::Click])
          .style(world_space_styles::world_button()),
      ))
      .child(UiNode::new(
        status_id,
        UiLabel::new("UI action count  /  0")
          .name("world-space-status")
          .style(world_space_styles::world_status(false)),
      )),
  )
}

fn contract_card() -> UiNode {
  node(UiBox::new().style(world_space_styles::card()))
    .child(node(
      UiLabel::new("PROCESS-WIDE INPUT CONTRACT").style(world_space_styles::caption()),
    ))
    .child(line("EVENT", "active + project-owned"))
    .child(line("CAMERA", "explicit + 25-unit reach"))
    .child(line("ROUTING", "Always + collider filtered"))
}

fn target_card() -> UiNode {
  node(UiBox::new().style(world_space_styles::card()))
    .child(node(
      UiLabel::new("TARGET-TEXTURE MONITOR").style(world_space_styles::caption()),
    ))
    .child(
      node(UiBox::new().style(world_space_styles::monitor())).child(node(
        UiImage::new()
          .source(assets::RENDER_TEXTURE.clone())
          .style(world_space_styles::monitor_image()),
      )),
    )
    .child(node(
      UiLabel::new("Separate screen-space document output").style(world_space_styles::detail()),
    ))
}

fn line(title: &str, value: &str) -> UiNode {
  node(UiVisualElement::new().style(world_space_styles::line()))
    .child(node(
      UiLabel::new(title).style(world_space_styles::line_title()),
    ))
    .child(node(
      UiLabel::new(value).style(world_space_styles::line_detail()),
    ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
