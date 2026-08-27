use battlement::{
    Box, Button, Command, Label, ObjectId, ParallelCommandGroup, ScrollView, ScrollerVisibility,
    UiEventKind, UiNode, VisualElement, object_id,
};

use crate::{coverage, coverage_styles, design_system};

pub(crate) const BACK_ID: ObjectId = object_id!("28000000-0000-4000-8000-000000000100");
pub(crate) const GROUP_IDS: [ObjectId; 7] = [
    object_id!("28000000-0000-4000-8000-000000000101"),
    object_id!("28000000-0000-4000-8000-000000000102"),
    object_id!("28000000-0000-4000-8000-000000000103"),
    object_id!("28000000-0000-4000-8000-000000000104"),
    object_id!("28000000-0000-4000-8000-000000000105"),
    object_id!("28000000-0000-4000-8000-000000000106"),
    object_id!("28000000-0000-4000-8000-000000000107"),
];

pub(crate) fn page(page_id: ObjectId) -> UiNode {
    let total = coverage::validate().expect("release coverage ledger must be complete");
    let mut grid = node(Box::new().style(coverage_styles::grid()));
    for (index, group) in coverage::GROUPS.iter().enumerate() {
        let count = group.capabilities.len();
        grid = grid.child(UiNode::new(
            GROUP_IDS[index],
            Button::new(format!(
                "{}\n{count} / {count}\nLIVE  {}\nTEST  {}",
                group.title, group.specimen, group.test_family
            ))
            .events([UiEventKind::Click])
            .style(coverage_styles::card()),
        ));
    }
    UiNode::new(page_id, VisualElement::new().name("coverage-page"))
        .child(node(
            Label::new("RELEASE COVERAGE").style(design_system::eyebrow()),
        ))
        .child(node(
            Label::new("Every contract, traced to proof").style(coverage_styles::title()),
        ))
        .child(node(
            Label::new("Every public UI capability maps to a live specimen and automated test.")
                .style(coverage_styles::intro()),
        ))
        .child(
            node(Box::new().style(coverage_styles::summary()))
                .child(node(
                    Label::new(format!("ALL {total} CAPABILITIES MAPPED"))
                        .style(coverage_styles::summary_text()),
                ))
                .child(node(
                    Label::new("LIVE + TESTED").style(coverage_styles::summary_text()),
                )),
        )
        .child(grid)
}

pub(crate) fn category_index(id: ObjectId) -> Option<usize> {
    GROUP_IDS.iter().position(|candidate| *candidate == id)
}

pub(crate) fn detail_commands(
    page_id: ObjectId,
    canvas_id: ObjectId,
    index: usize,
) -> Vec<ParallelCommandGroup<Command>> {
    vec![
        ParallelCommandGroup::new(vec![Command::destroy_visual_element(page_id)]),
        ParallelCommandGroup::new(vec![Command::create_visual_element(
            canvas_id,
            detail_page(page_id, index),
        )]),
    ]
}

fn detail_page(page_id: ObjectId, index: usize) -> UiNode {
    let group = &coverage::GROUPS[index];
    let mut ledger = node(
        ScrollView::new()
            .vertical_scroller_visibility(ScrollerVisibility::AlwaysVisible)
            .style(coverage_styles::ledger()),
    );
    for capability in group.capabilities {
        ledger = ledger.child(node(
            Label::new(format!(
                "{capability}  |  LIVE {}  |  TEST {}",
                group.specimen, group.test_family
            ))
            .style(coverage_styles::ledger_row()),
        ));
    }
    UiNode::new(page_id, VisualElement::new().name("coverage-detail"))
        .child(UiNode::new(
            BACK_ID,
            Button::new("← ALL CATEGORIES")
                .events([UiEventKind::Click])
                .style(coverage_styles::back_button()),
        ))
        .child(node(
            Label::new(group.title).style(coverage_styles::title()),
        ))
        .child(node(
            Label::new(format!(
                "{} INDIVIDUAL MAPPINGS  •  LIVE {}  •  TEST {}",
                group.capabilities.len(),
                group.specimen,
                group.test_family
            ))
            .style(coverage_styles::detail_intro()),
        ))
        .child(ledger)
}

fn node(element: impl Into<battlement::UiElement>) -> UiNode {
    UiNode::new(ObjectId::new_v4(), element)
}
