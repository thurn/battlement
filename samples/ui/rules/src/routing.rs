use battlement::{ActionId, Batch, BatchId, Command, ParallelCommandGroup, Response, SessionId};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Page {
    Components,
    Interactions,
    Hierarchy,
    Assets,
    Layout,
    Appearance,
    Backgrounds,
    Transforms,
    Typography,
    Buttons,
    Containers,
    Scroll,
    Tabs,
    TextFields,
    BooleanControls,
    ChoiceGroups,
    Dropdowns,
    Sliders,
    Ranges,
    Parts,
    ComplexParts,
    PointerRouting,
    KeyboardNavigation,
    RemainingEvents,
    Actions,
    RenderModes,
    WorldSpace,
    Coverage,
}

pub(crate) fn single_ui_command_response(
    session_id: SessionId,
    action_id: ActionId,
    commands: Vec<Command>,
) -> Response<Command> {
    Response::batch(
        Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![ParallelCommandGroup::new(commands)],
        )
        .caused_by_action_id(action_id),
    )
}
