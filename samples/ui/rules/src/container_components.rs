use battlement::{
  ObjectId, UiButton, UiElement, UiEventKind, UiGroupBox, UiLabel, UiNode, UiPopupWindow,
  UiVisualElement,
};

use crate::{container_styles, design_system};

pub(crate) struct ContainerIds {
  pub(crate) titled_group: ObjectId,
  pub(crate) empty_group: ObjectId,
  pub(crate) dynamic_group: ObjectId,
  pub(crate) dynamic_child: ObjectId,
  pub(crate) dynamic_action: ObjectId,
  pub(crate) popup: ObjectId,
}

pub(crate) fn containers_page(
  page_id: ObjectId,
  ids: &ContainerIds,
  dynamic_title_visible: bool,
) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("containers-page"))
        .child(node(UiLabel::new("CONTAINERS").style(design_system::eyebrow())))
        .child(node(
            UiLabel::new("Structure you can see and trust").style(design_system::title()),
        ))
        .child(node(
            UiLabel::new(
                "GroupBox titles appear only when authored; PopupWindow keeps every child in its public content container.",
            )
            .style(container_styles::intro()),
        ))
        .child(
            node(UiVisualElement::new().style(container_styles::gallery()))
                .child(container_specimen(
                    "TITLED + POPULATED",
                    UiNode::new(
                        ids.titled_group,
                        UiGroupBox::new()
                            .text("AUDIO SETTINGS")
                            .name("titled-group")
                            .style(container_styles::group()),
                    )
                    .child(node(
                        UiLabel::new("Music  /  80%").style(container_styles::group_content()),
                    ))
                    .child(node(
                        UiLabel::new("Effects  /  65%").style(container_styles::group_content()),
                    )),
                    "A native title label precedes two logical children.",
                ))
                .child(container_specimen(
                    "UNTITLED + EMPTY",
                    UiNode::new(
                        ids.empty_group,
                        UiGroupBox::new()
                            .name("empty-group")
                            .style(container_styles::empty_group()),
                    ),
                    "No title label and no authored content container entries.",
                ))
                .child(container_specimen(
                    "DYNAMIC TITLE",
                    UiNode::new(
                        ids.dynamic_group,
                        UiGroupBox::new()
                            .text(if dynamic_title_visible {
                                "TACTICAL OVERRIDES"
                            } else {
                                ""
                            })
                            .name("dynamic-group")
                            .style(container_styles::group()),
                    )
                    .child(UiNode::new(
                        ids.dynamic_child,
                        UiLabel::new(if dynamic_title_visible {
                            "Title created; authored content stayed in place."
                        } else {
                            "No internal title label; content stays mounted."
                        })
                        .name("dynamic-group-content")
                        .style(container_styles::group_content()),
                    ))
                    .child(UiNode::new(
                        ids.dynamic_action,
                        UiButton::new(if dynamic_title_visible {
                            "Remove title"
                        } else {
                            "Add title"
                        })
                        .name("dynamic-title-action")
                        .events([UiEventKind::Click])
                        .style(container_styles::action()),
                    )),
                    "The title part is created or removed without moving content.",
                ))
                .child(container_specimen(
                    "POPUP CONTENT ROUTE",
                    UiNode::new(
                        ids.popup,
                        UiPopupWindow::new()
                            .text("<b>DEPLOYMENT CARD</b> / <link=field-guide>FIELD GUIDE</link>")
                            .rich_text(true)
                            .name("popup-window")
                            .style(container_styles::popup()),
                    )
                    .child(node(
                        UiLabel::new("Sector 7  /  clear").style(container_styles::popup_content()),
                    ))
                    .child(node(
                        UiLabel::new("Squad ETA  /  04:20")
                            .style(container_styles::popup_content()),
                    )),
                    "Rich heading text stays separate from ordered popup children.",
                )),
        )
}

fn container_specimen(caption: &str, control: UiNode, help: &str) -> UiNode {
  node(UiVisualElement::new().style(container_styles::specimen()))
    .child(node(
      UiLabel::new(caption).style(container_styles::caption()),
    ))
    .child(control)
    .child(node(UiLabel::new(help).style(container_styles::help())))
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
