use battlement::{
  Command, ObjectId, ScrollerVisibility, UiBox, UiButton, UiElement, UiEventKind, UiGroupBox,
  UiLabel, UiNode, UiRadioButtonGroup, UiScrollView, UiSlider, UiTab, UiTabView, UiTextField,
  UiVisualElement, object_id,
};

use crate::{asset_catalog::ui::assets, complex_part_styles, design_system};

pub(crate) const SLIDER_ID: ObjectId = object_id!("0121421b-c595-4eb8-9689-88e02dd62669");
pub(crate) const TEXT_FIELD_ID: ObjectId = object_id!("be19363c-cd77-43ea-985a-648794120a25");
pub(crate) const TAB_VIEW_ID: ObjectId = object_id!("f632950b-10b2-442c-91e9-814f765fe9ec");
pub(crate) const OVERVIEW_TAB_ID: ObjectId = object_id!("f3340a19-f237-4baa-91ea-7fa6a62c1ee7");
pub(crate) const TITLE_GROUP_ID: ObjectId = object_id!("139c41bc-e97b-4da9-9f70-1c58f9136953");
pub(crate) const RADIO_GROUP_ID: ObjectId = object_id!("d66ba54f-2976-42d2-a90a-aa2292413af5");
pub(crate) const STATE_ID: ObjectId = object_id!("f6c2b1a9-435b-42c8-9f9c-19987be90b2b");

pub(crate) fn page(page_id: ObjectId, toggle_id: ObjectId, revealed: bool) -> UiNode {
  UiNode::new(page_id, UiVisualElement::new().name("complex-parts-page"))
        .child(node(UiLabel::new("COMPLEX PART STYLING").style(design_system::eyebrow())))
        .child(node(UiLabel::new("State first, anatomy second").style(design_system::title())))
        .child(node(UiLabel::new("Named Rust methods style Unity-owned layers. Aggregate state creates conditional parts first; shared option styles always yield to indexed overrides.").style(complex_part_styles::intro())))
        .child(node(UiVisualElement::new().style(complex_part_styles::gallery()))
            .child(slider_card(revealed)).child(scroll_card()).child(tab_card(revealed)).child(option_card(toggle_id, revealed)))
}

pub(crate) fn update_commands(toggle_id: ObjectId, revealed: bool) -> Vec<Command> {
  let slider = UiSlider::new().fill(revealed).show_input_field(revealed);
  let slider = if revealed {
    slider
      .fill_style(complex_part_styles::slider_fill())
      .text_input_style(complex_part_styles::slider_input())
  } else {
    slider
  };
  let text = UiTextField::new()
    .value(if revealed {
      "Multiline scroll part"
    } else {
      "Single line"
    })
    .multiline(revealed)
    .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible);
  let text = if revealed {
    text
      .multiline_scroll_view_style(complex_part_styles::multiline_scroll())
      .vertical_scroller_style(complex_part_styles::multiline_scroller())
      .vertical_dragger_style(complex_part_styles::multiline_dragger())
  } else {
    text
  };
  let tab = UiTab::default()
    .closeable(revealed)
    .icon_style(complex_part_styles::tab_icon(revealed));
  let tab = if revealed {
    tab.close_button_style(complex_part_styles::tab_close())
  } else {
    tab
  };
  let title = UiGroupBox::new().text(if revealed { "AUTHORED TITLE" } else { "" });
  let title = if revealed {
    title.title_style(complex_part_styles::conditional_title())
  } else {
    title
  };
  vec![
    Command::update_visual_element(SLIDER_ID, slider),
    Command::update_visual_element(TEXT_FIELD_ID, text),
    Command::update_visual_element(OVERVIEW_TAB_ID, tab),
    Command::update_visual_element(TAB_VIEW_ID, UiTabView::new().selected_tab_index(0)),
    Command::update_visual_element(TITLE_GROUP_ID, title),
    Command::update_visual_element(
      RADIO_GROUP_ID,
      UiRadioButtonGroup::new().all_options_style(complex_part_styles::all_options_state(revealed)),
    ),
    Command::update_visual_element(
      toggle_id,
      UiButton::new(if revealed {
        "Remove conditional parts"
      } else {
        "Create conditional parts"
      })
      .style(complex_part_styles::toggle_button(revealed)),
    ),
    Command::update_visual_element(
      STATE_ID,
      UiLabel::new(if revealed {
        "STATE · ON"
      } else {
        "STATE · OFF"
      })
      .style(complex_part_styles::state(revealed)),
    ),
  ]
}

fn slider_card(revealed: bool) -> UiNode {
  let slider = UiSlider::new()
    .name("complex-parts-slider")
    .label("Signal")
    .low_value(0.0)
    .high_value(100.0)
    .value(64.0)
    .fill(revealed)
    .show_input_field(revealed)
    .style(complex_part_styles::slider())
    .label_style(complex_part_styles::slider_label())
    .track_style(complex_part_styles::slider_track())
    .dragger_style(complex_part_styles::slider_dragger());
  let slider = if revealed {
    slider
      .fill_style(complex_part_styles::slider_fill())
      .text_input_style(complex_part_styles::slider_input())
  } else {
    slider
  };
  let notes = UiTextField::new()
    .value(if revealed {
      "Multiline scroll part"
    } else {
      "Single line"
    })
    .multiline(revealed)
    .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
    .style(complex_part_styles::text_field())
    .input_style(complex_part_styles::text_input())
    .text_element_style(complex_part_styles::text_copy());
  let notes = if revealed {
    notes
      .multiline_scroll_view_style(complex_part_styles::multiline_scroll())
      .vertical_scroller_style(complex_part_styles::multiline_scroller())
      .vertical_dragger_style(complex_part_styles::multiline_dragger())
  } else {
    notes
  };
  node(UiBox::new().style(complex_part_styles::card()))
    .child(node(
      UiLabel::new("SLIDER ANATOMY").style(complex_part_styles::caption()),
    ))
    .child(node(
      UiLabel::new("PARTS · label / track / fill / thumb / input")
        .style(complex_part_styles::anatomy()),
    ))
    .child(UiNode::new(SLIDER_ID, slider))
    .child(UiNode::new(TEXT_FIELD_ID, notes))
}

fn scroll_card() -> UiNode {
  node(UiBox::new().style(complex_part_styles::card()))
        .child(node(UiLabel::new("SCROLL ANATOMY").style(complex_part_styles::caption())))
        .child(node(UiLabel::new("PARTS · viewport / content / scroller / dragger").style(complex_part_styles::anatomy())))
        .child(node(UiScrollView::new().vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
            .style(complex_part_styles::scroll()).viewport_style(complex_part_styles::viewport())
            .content_container_style(complex_part_styles::content()).vertical_scroller_style(complex_part_styles::scroller())
            .vertical_dragger_style(complex_part_styles::scroll_dragger()))
            .child(node(UiLabel::new("Viewport clips this content layer.\n\nThe scroller owns its slider, track, and dragger.\n\nEvery lookup stays owner-scoped."))))
}

fn tab_card(revealed: bool) -> UiNode {
  let overview = UiTab::new("Overview")
    .icon(assets::VECTOR.clone())
    .closeable(revealed)
    .header_style(complex_part_styles::tab_header())
    .label_style(complex_part_styles::tab_label())
    .icon_style(complex_part_styles::tab_icon(revealed))
    .underline_style(complex_part_styles::tab_underline())
    .content_container_style(complex_part_styles::tab_content());
  let overview = if revealed {
    overview.close_button_style(complex_part_styles::tab_close())
  } else {
    overview
  };
  let title = UiGroupBox::new().text(if revealed { "AUTHORED TITLE" } else { "" });
  let title = if revealed {
    title.title_style(complex_part_styles::conditional_title())
  } else {
    title
  };
  node(UiBox::new().style(complex_part_styles::card()))
    .child(node(
      UiLabel::new("TAB ANATOMY").style(complex_part_styles::caption()),
    ))
    .child(node(
      UiLabel::new("PARTS · header / icon / underline / content / title")
        .style(complex_part_styles::anatomy()),
    ))
    .child(
      UiNode::new(
        TAB_VIEW_ID,
        UiTabView::new()
          .name("complex-parts-tabs")
          .selected_tab_index(0)
          .style(complex_part_styles::tab_view())
          .header_container_style(complex_part_styles::tab_headers()),
      )
      .child(
        UiNode::new(OVERVIEW_TAB_ID, overview).child(
          UiNode::new(TITLE_GROUP_ID, title).child(node(
            UiLabel::new("Overview remains selected while parts materialize.")
              .style(complex_part_styles::tab_copy()),
          )),
        ),
      )
      .child(node(
        UiTab::new("Details")
          .header_style(complex_part_styles::tab_header())
          .label_style(complex_part_styles::tab_label()),
      )),
    )
}

fn option_card(toggle_id: ObjectId, revealed: bool) -> UiNode {
  node(UiBox::new().style(complex_part_styles::card()))
    .child(node(
      UiLabel::new("INDEX + CONDITION").style(complex_part_styles::caption()),
    ))
    .child(node(
      UiLabel::new("ALL OPTIONS → OPTION[1] · deterministic precedence")
        .style(complex_part_styles::anatomy()),
    ))
    .child(UiNode::new(
      RADIO_GROUP_ID,
      UiRadioButtonGroup::new()
        .choices(["Scout", "Guard", "Engineer"])
        .selected_index(1)
        .style(complex_part_styles::options())
        .option_style(1, complex_part_styles::highlighted_option())
        .option_text_style(1, complex_part_styles::highlighted_text())
        .all_options_style(complex_part_styles::all_options()),
    ))
    .child(
      node(UiVisualElement::new().style(complex_part_styles::toggle_row()))
        .child(UiNode::new(
          toggle_id,
          UiButton::new(if revealed {
            "Remove conditional parts"
          } else {
            "Create conditional parts"
          })
          .events([UiEventKind::Click])
          .style(complex_part_styles::toggle_button(revealed)),
        ))
        .child(UiNode::new(
          STATE_ID,
          UiLabel::new(if revealed {
            "STATE · ON"
          } else {
            "STATE · OFF"
          })
          .style(complex_part_styles::state(revealed)),
        )),
    )
}

fn node(element: impl Into<UiElement>) -> UiNode {
  UiNode::new(ObjectId::new_v4(), element)
}
