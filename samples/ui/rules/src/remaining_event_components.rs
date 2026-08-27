use battlement::{
    Box, Button, Command, Label, ObjectId, TextElement, UiElement, UiEvent, UiEventBody,
    UiEventKind, UiNode, VisualElement, object_id,
};

use crate::{design_system, remaining_event_styles};

pub(crate) const LINK_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000002");
pub(crate) const LINK_INSPECTOR_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000003");
pub(crate) const TARGET_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000004");
pub(crate) const TARGET_LABEL_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000007");
pub(crate) const LIFECYCLE_INSPECTOR_ID: ObjectId =
    object_id!("24100000-0000-4000-8000-000000000005");
pub(crate) const ACTION_ID: ObjectId = object_id!("24100000-0000-4000-8000-000000000006");

#[derive(Default)]
pub(crate) struct LifecycleTimeline {
    link_entered: bool,
    link_down: bool,
    link_up: bool,
    link_left: bool,
    link_identity: Option<String>,
    selection: Option<(u32, u32)>,
    attached: bool,
    detached: bool,
    geometry: bool,
    geometry_detail: Option<String>,
    transition_started: bool,
    transition_ended: bool,
    transition_cancelled: bool,
}

pub(crate) fn page(page_id: ObjectId, settled: bool) -> UiNode {
    UiNode::new(page_id, VisualElement::new().name("remaining-events-page"))
        .child(node(
            Label::new("REMAINING EVENTS").style(design_system::eyebrow().flex_shrink(0)),
        ))
        .child(node(
            Label::new("Every quiet UI signal, made visible.")
                .style(design_system::title().flex_shrink(0)),
        ))
        .child(node(Label::new("Interact with the cyan link, then run the layout pulse. The inspectors expose identity recovery, coalesced selection, finite geometry, panel lifecycle, and transitions.").style(remaining_event_styles::intro())))
        .child(
            node(VisualElement::new().style(remaining_event_styles::columns()))
                .child(link_card())
                .child(lifecycle_card(settled)),
        )
}

pub(crate) fn event_commands(
    timeline: &mut LifecycleTimeline,
    event: &UiEvent,
) -> Option<Vec<Command>> {
    let (inspector, message) = match &event.body {
        UiEventBody::LinkEnter(value) => {
            timeline.link_entered = true;
            timeline.link_identity = Some(format!("{} · {}", value.link_id, value.link_text));
            (LINK_INSPECTOR_ID, link_message(timeline))
        }
        UiEventBody::LinkDown(_) => {
            timeline.link_down = true;
            (LINK_INSPECTOR_ID, link_message(timeline))
        }
        UiEventBody::LinkUp(_) => {
            timeline.link_up = true;
            (LINK_INSPECTOR_ID, link_message(timeline))
        }
        UiEventBody::LinkLeave(value) => {
            timeline.link_left = true;
            timeline.link_identity = Some(format!("{} · {}", value.link_id, value.link_text));
            (LINK_INSPECTOR_ID, link_message(timeline))
        }
        UiEventBody::SelectionChanged(value) if event.target_id == LINK_ID => {
            timeline.selection = Some((value.cursor_index, value.selection_index));
            (LINK_INSPECTOR_ID, link_message(timeline))
        }
        UiEventBody::GeometryChanged(value) if event.target_id == TARGET_ID => {
            timeline.geometry = true;
            timeline.geometry_detail = Some(format!(
                "finite old → new rect · {:.0} × {:.0} → {:.0} × {:.0}",
                value.previous.width,
                value.previous.height,
                value.current.width,
                value.current.height
            ));
            (
                LIFECYCLE_INSPECTOR_ID,
                lifecycle_message(timeline, String::new()),
            )
        }
        UiEventBody::AttachToPanel(_) if event.target_id == TARGET_ID => {
            timeline.attached = true;
            (
                LIFECYCLE_INSPECTOR_ID,
                lifecycle_message(timeline, "target joined panel".to_owned()),
            )
        }
        UiEventBody::DetachFromPanel(_) if event.target_id == TARGET_ID => {
            (LIFECYCLE_INSPECTOR_ID, {
                timeline.detached = true;
                lifecycle_message(timeline, "target left panel".to_owned())
            })
        }
        UiEventBody::TransitionStart(value) if event.target_id == TARGET_ID => {
            timeline.transition_started = true;
            (
                LIFECYCLE_INSPECTOR_ID,
                lifecycle_message(
                    timeline,
                    format!("{} supported properties · start", value.properties.len()),
                ),
            )
        }
        UiEventBody::TransitionEnd(value) if event.target_id == TARGET_ID => {
            timeline.transition_ended = true;
            (
                LIFECYCLE_INSPECTOR_ID,
                lifecycle_message(
                    timeline,
                    format!(
                        "{} properties · {:.0} ms",
                        value.properties.len(),
                        value.elapsed_ms
                    ),
                ),
            )
        }
        UiEventBody::TransitionCancel(value) if event.target_id == TARGET_ID => {
            (LIFECYCLE_INSPECTOR_ID, {
                timeline.transition_cancelled = true;
                lifecycle_message(
                    timeline,
                    format!(
                        "{} properties interrupted · {:.0} ms",
                        value.properties.len(),
                        value.elapsed_ms
                    ),
                )
            })
        }
        _ => return None,
    };
    Some(vec![Command::update_visual_element(
        inspector,
        Label::new(message),
    )])
}

pub(crate) fn timeline_commands(timeline: &LifecycleTimeline) -> Vec<Command> {
    vec![
        Command::update_visual_element(LINK_INSPECTOR_ID, Label::new(link_message(timeline))),
        Command::update_visual_element(
            LIFECYCLE_INSPECTOR_ID,
            Label::new(lifecycle_message(timeline, String::new())),
        ),
    ]
}

fn link_message(timeline: &LifecycleTimeline) -> String {
    let selection = timeline.selection.map_or_else(
        || "waiting".to_owned(),
        |(cursor, anchor)| format!("observed · cursor {cursor}, anchor {anchor}"),
    );
    format!(
        "01  ENTER      {}\n02  DOWN       {}\n03  UP         {}\n04  LEAVE      {}\n05  SELECTION  {}\n{}",
        observed(timeline.link_entered),
        observed(timeline.link_down),
        observed(timeline.link_up),
        observed(timeline.link_left),
        selection,
        timeline
            .link_identity
            .as_deref()
            .unwrap_or("Identity is cached per pointer; unmatched leave is dropped.")
    )
}

fn lifecycle_message(timeline: &LifecycleTimeline, detail: String) -> String {
    let detail = match (&timeline.geometry_detail, detail.is_empty()) {
        (Some(geometry), true) => geometry.clone(),
        (Some(geometry), false) => format!("{geometry}\n{detail}"),
        (None, _) => detail,
    };
    format!(
        "01  ATTACH    {}\n02  GEOMETRY  {}\n03  START     {}\n04  END       {}\n05  CANCEL    {}\n06  DETACH    {}\n{}",
        observed(timeline.attached),
        observed(timeline.geometry),
        observed(timeline.transition_started),
        observed(timeline.transition_ended),
        observed(timeline.transition_cancelled),
        observed(timeline.detached),
        detail
    )
}

fn observed(value: bool) -> &'static str {
    if value { "observed" } else { "waiting" }
}

pub(crate) fn target_command(settled: bool) -> Command {
    Command::update_visual_element(
        TARGET_ID,
        Box::default().style(remaining_event_styles::target(settled)),
    )
}

pub(crate) fn target_label_command(settled: bool) -> Command {
    Command::update_visual_element(
        TARGET_LABEL_ID,
        Label::new(if settled { "SETTLED" } else { "READY" }),
    )
}

pub(crate) fn action_command(settled: bool) -> Command {
    Command::update_visual_element(
        ACTION_ID,
        Button::new(if settled {
            "Reset layout pulse"
        } else {
            "Run layout pulse"
        }),
    )
}

fn link_card() -> UiNode {
    node(Box::new().style(remaining_event_styles::card(true)))
        .child(node(
            Label::new("RICH LINK + SELECTION").style(remaining_event_styles::caption()),
        ))
        .child(UiNode::new(
            LINK_ID,
            TextElement::new("Open the <link=field-guide><color=#52EAF5><u>FIELD GUIDE</u></color></link> for deployment details.")
                .name("remaining-rich-link")
                .rich_text(true)
                .selectable(true)
                .events([
                    UiEventKind::LinkEnter,
                    UiEventKind::LinkLeave,
                    UiEventKind::LinkDown,
                    UiEventKind::LinkUp,
                    UiEventKind::SelectionChanged,
                ])
                .style(remaining_event_styles::link_surface()),
        ))
        .child(UiNode::new(
            LINK_INSPECTOR_ID,
            Label::new("01  ENTER      waiting\n02  DOWN       waiting\n03  UP         waiting\n04  LEAVE      waiting\n05  SELECTION  waiting\nDrag across the link copy to coalesce selection indices.").style(remaining_event_styles::inspector()),
        ))
        .child(node(Label::new("ENTER → DOWN → UP → LEAVE · then drag to select\nUnmatched leave is dropped. Different pointers never share identity.").style(remaining_event_styles::legend())))
}

fn lifecycle_card(settled: bool) -> UiNode {
    node(Box::new().style(remaining_event_styles::card(false)))
        .child(node(
            Label::new("GEOMETRY + LIFECYCLE").style(remaining_event_styles::caption()),
        ))
        .child(
            node(VisualElement::new().style(remaining_event_styles::stage())).child(
                UiNode::new(
                    TARGET_ID,
                    Box::new()
                        .name("remaining-transition-target")
                        .events([
                            UiEventKind::GeometryChanged,
                            UiEventKind::AttachToPanel,
                            UiEventKind::DetachFromPanel,
                            UiEventKind::TransitionStart,
                            UiEventKind::TransitionEnd,
                            UiEventKind::TransitionCancel,
                        ])
                        .style(remaining_event_styles::target(settled)),
                )
                .child(UiNode::new(
                    TARGET_LABEL_ID,
                    Label::new(if settled { "SETTLED" } else { "READY" }),
                )),
            ),
        )
        .child(UiNode::new(
            LIFECYCLE_INSPECTOR_ID,
            Label::new("01  ATTACH    waiting\n02  GEOMETRY  waiting\n03  START     waiting\n04  END       waiting\n05  CANCEL    waiting\n06  DETACH    waiting").style(remaining_event_styles::inspector()),
        ))
        .child(UiNode::new(
            ACTION_ID,
            Button::new(if settled { "Reset layout pulse" } else { "Run layout pulse" })
                .name("remaining-layout-action")
                .events([UiEventKind::Click])
                .style(remaining_event_styles::button()),
        ))
}

fn node(element: impl Into<UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
