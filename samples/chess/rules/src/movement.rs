use cozy_chess::Square;
use masonry::{
    ActionId, Batch, BatchId, Command, CommandBody, Easing, GridLayout, ObjectId, PositionPayload,
    PropertyCommand, Response, SessionId, Tween, TweenPositionPayload, Vector3,
};

use crate::audio;

const MOVE_DURATION_MS: u64 = 300;
const KNIGHT_LONG_LEG_DURATION_MS: u64 = 200;
const KNIGHT_SHORT_LEG_DURATION_MS: u64 = 120;

pub(crate) fn command(object_id: ObjectId, square: Square, animate: bool) -> CommandBody {
    if animate {
        self::tween_command(object_id, self::square_position(square), MOVE_DURATION_MS)
    } else {
        CommandBody::TransformSetWorldPosition(PropertyCommand::canceling(PositionPayload {
            object_id,
            position: self::square_position(square),
        }))
    }
}

pub(crate) fn knight_first_leg(object_id: ObjectId, from: Square, to: Square) -> CommandBody {
    let from = self::square_position(from);
    let to = self::square_position(to);
    let corner = if (to.x - from.x).abs() > (to.z - from.z).abs() {
        Vector3::new(to.x, to.y, from.z)
    } else {
        Vector3::new(from.x, to.y, to.z)
    };
    self::tween_command(object_id, corner, KNIGHT_LONG_LEG_DURATION_MS)
}

pub(crate) fn knight_second_leg(object_id: ObjectId, to: Square) -> CommandBody {
    self::tween_command(
        object_id,
        self::square_position(to),
        KNIGHT_SHORT_LEG_DURATION_MS,
    )
}

pub(crate) fn response_for_groups(
    session_id: SessionId,
    action_id: ActionId,
    groups: Vec<Vec<CommandBody>>,
) -> Response<Command> {
    Response::batch(
        Batch::new(
            BatchId::new_v4(),
            session_id,
            groups.into_iter().map(audio::parallel_group).collect(),
        )
        .caused_by_action_id(action_id),
    )
}

fn tween_command(object_id: ObjectId, position: Vector3, duration_ms: u64) -> CommandBody {
    CommandBody::TransformTweenWorldPosition(PropertyCommand::canceling(TweenPositionPayload {
        object_id,
        position,
        tween: Tween::new()
            .duration_ms(duration_ms)
            .easing(Easing::InOutSine),
    }))
}

fn square_position(square: Square) -> Vector3 {
    GridLayout::centered(
        Vector3::ZERO,
        8,
        8,
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
    .position(square.file() as u32, square.rank() as u32)
}
