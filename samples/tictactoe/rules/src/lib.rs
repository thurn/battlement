//! Native rules engine for the standalone Tic-Tac-Toe sample.

use std::time::{Duration, Instant};

use fastrand::Rng;
use masonry::{
    ActionBody, ActionId, Batch, BatchId, CameraClearMode, CameraProjection, CameraState,
    ClientMessage, Color, Command, CommandBody, CommandId, Connect, CoreErrorCode, GameObject,
    GameObjectKind, ImageFit, ImageState, LocalTransform, ObjectId, ObjectIdPayload,
    ParallelCommandGroup, ParentScene, PointerButton, PointerEvent, PreparedAsset, Response,
    ResponseMessage, Scene, SceneAddress, SceneId, SessionId, SetInputEnabledPayload, Snapshot,
    TextContentPayload, TextState, Vector3,
};
use masonry_native::{Engine, EngineError};
use uuid::{Uuid, uuid};

const CONTENT_SCENE: &str = "tictactoe/content";
const BOARD_TEXTURE: &str = "tictactoe/board";
const X_TEXTURE: &str = "tictactoe/x";
const O_TEXTURE: &str = "tictactoe/o";
const FONT: &str = "tictactoe/font";
const CAMERA_ID: Uuid = uuid!("fa308d92-5ad4-4249-90dc-2d104057bc41");
const BOARD_ID: Uuid = uuid!("c8c9e10d-585b-45f4-ac19-b76746ed2d25");
const STATUS_ID: Uuid = uuid!("9b10a4a0-1367-46a8-9a2c-7c29eef033b1");
const TITLE_ID: Uuid = uuid!("860e3fa1-d047-45ae-869d-3321e9cd3142");
const BOARD_CENTER_Y: f64 = -0.7;
const BOARD_SIZE: f64 = 7.2;
const GRID_SIZE: f64 = BOARD_SIZE * 0.8;
const CELL_SIZE: f64 = GRID_SIZE / 3.0;
const MARK_SIZE: f64 = 2.25;
const AI_DELAY: Duration = Duration::from_millis(500);
const PLAYER_TURN: &str = "Your turn — click an empty square";
const THINKING: &str = "Computer thinking…";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mark {
    X,
    O,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    InProgress,
    XWins,
    OWins,
    Draw,
}

struct TicTacToeEngine {
    session_id: SessionId,
    board: [Option<Mark>; 9],
    marker_ids: [Option<ObjectId>; 9],
    outcome: Outcome,
    ai_due: Option<Instant>,
    rng: Rng,
}

impl Engine for TicTacToeEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        self.reset_state();
        Ok(Response::new(
            self.session_id,
            vec![ResponseMessage::Snapshot(self::snapshot(self.session_id))],
        ))
    }

    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        Ok(self.submit_at(message, Instant::now()))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        Ok(self.poll_at(Instant::now()))
    }
}

impl TicTacToeEngine {
    fn submit_at(
        &mut self,
        message: ClientMessage<(), CoreErrorCode>,
        now: Instant,
    ) -> Response<Command> {
        let ClientMessage::Action(action) = message else {
            return Response::new(self.session_id, Vec::new());
        };
        let ActionBody::PointerClick(payload) = action.body else {
            return Response::new(self.session_id, Vec::new());
        };
        if payload.object_id != self::object_id(BOARD_ID) || payload.button != PointerButton::Left {
            return Response::new(self.session_id, Vec::new());
        }
        if self.outcome != Outcome::InProgress {
            return self.reset_round(action.action_id);
        }
        if self.ai_due.is_some() {
            return Response::new(self.session_id, Vec::new());
        }
        let Some(index) = self::cell_index(payload.world_hit) else {
            return Response::new(self.session_id, Vec::new());
        };
        if self.board[index].is_some() {
            // An occupied square does not change the game, so there are no commands to send.
            return Response::new(self.session_id, Vec::new());
        }

        let marker = self.place_mark(index, Mark::X);
        self.outcome = self::outcome(&self.board);
        let mut commands = vec![CommandBody::object_create(marker)];
        if self.outcome == Outcome::InProgress {
            self.ai_due = Some(now + AI_DELAY);
            commands.push(self::status_command(THINKING));
            commands.push(CommandBody::InputSetEnabled(SetInputEnabledPayload {
                enabled: false,
            }));
        } else {
            commands.push(self::status_command(self::outcome_text(self.outcome)));
        }
        self::action_response(self.session_id, action.action_id, commands)
    }

    fn poll_at(&mut self, now: Instant) -> Option<Response<Command>> {
        let due = self.ai_due?;
        if now < due {
            return None;
        }
        self.ai_due = None;
        // An AI turn is scheduled only while the game is in progress, so a cell is available.
        let empty = self::empty_cells(&self.board);
        let index = empty[self.rng.usize(..empty.len())];
        let marker = self.place_mark(index, Mark::O);
        self.outcome = self::outcome(&self.board);
        Some(self::command_response(
            self.session_id,
            vec![
                CommandBody::object_create(marker),
                self::status_command(self::outcome_text(self.outcome)),
                CommandBody::InputSetEnabled(SetInputEnabledPayload { enabled: true }),
            ],
        ))
    }

    fn place_mark(&mut self, index: usize, mark: Mark) -> GameObject {
        let object_id = ObjectId::new_v4();
        self.board[index] = Some(mark);
        self.marker_ids[index] = Some(object_id);
        self::marker(object_id, index, mark)
    }

    fn reset_round(&mut self, action_id: ActionId) -> Response<Command> {
        let mut commands = self
            .marker_ids
            .iter()
            .flatten()
            .map(|object_id| {
                CommandBody::ObjectDestroy(ObjectIdPayload {
                    object_id: *object_id,
                })
            })
            .collect::<Vec<_>>();
        self.reset_state();
        commands.push(self::status_command(PLAYER_TURN));
        self::action_response(self.session_id, action_id, commands)
    }

    fn reset_state(&mut self) {
        self.board = [None; 9];
        self.marker_ids = [None; 9];
        self.outcome = Outcome::InProgress;
        self.ai_due = None;
    }
}

fn snapshot(session_id: SessionId) -> Snapshot {
    let scene_id = self::scene_id();
    let mut camera = GameObject::new(
        self::object_id(CAMERA_ID),
        GameObjectKind::Camera {
            camera: CameraState {
                projection: CameraProjection::Orthographic,
                orthographic_size: 5.6,
                clear_mode: CameraClearMode::SolidColor,
                clear_color: Color {
                    r: 0.96,
                    g: 0.93,
                    b: 0.84,
                    a: 1.0,
                },
                ..CameraState::default()
            },
        },
    );
    camera.parent_scene = ParentScene::Persistent;
    camera.local_transform.position = Vector3::new(0.0, 0.0, -10.0);

    let mut board = GameObject::new(
        self::object_id(BOARD_ID),
        GameObjectKind::Image {
            image: ImageState {
                ..ImageState::new(BOARD_TEXTURE, BOARD_SIZE, BOARD_SIZE)
            },
        },
    );
    board.parent_scene = ParentScene::Scene(scene_id);
    board.local_transform.position = Vector3::new(0.0, BOARD_CENTER_Y, 0.0);
    board.pointer_events = vec![PointerEvent::Click];

    let mut title = GameObject::new(
        self::object_id(TITLE_ID),
        GameObjectKind::Text {
            text: TextState {
                size: 4.0,
                color: Color {
                    r: 0.03,
                    g: 0.04,
                    b: 0.08,
                    a: 1.0,
                },
                ..TextState::new("TIC TAC TOE", FONT)
            },
        },
    );
    title.parent_scene = ParentScene::Persistent;
    title.local_transform.position = Vector3::new(0.0, 4.7, -0.1);

    let mut status = GameObject::new(
        self::object_id(STATUS_ID),
        GameObjectKind::Text {
            text: TextState {
                size: 3.2,
                color: Color {
                    r: 0.06,
                    g: 0.08,
                    b: 0.15,
                    a: 1.0,
                },
                wrap_width: Some(14.0),
                ..TextState::new(PLAYER_TURN, FONT)
            },
        },
    );
    status.parent_scene = ParentScene::Persistent;
    status.local_transform.position = Vector3::new(0.0, 3.75, -0.1);

    Snapshot::new(
        session_id,
        vec![
            PreparedAsset::Scene(SceneAddress::new(CONTENT_SCENE)),
            PreparedAsset::Texture(BOARD_TEXTURE.into()),
            PreparedAsset::Texture(X_TEXTURE.into()),
            PreparedAsset::Texture(O_TEXTURE.into()),
            PreparedAsset::Font(FONT.into()),
        ],
        vec![Scene::new(scene_id, CONTENT_SCENE)],
        vec![camera, board, title, status],
        self::object_id(CAMERA_ID),
    )
}

fn marker(object_id: ObjectId, index: usize, mark: Mark) -> GameObject {
    let texture = if mark == Mark::X {
        X_TEXTURE
    } else {
        O_TEXTURE
    };
    let mut marker = GameObject::new(
        object_id,
        GameObjectKind::Image {
            image: ImageState {
                fit: ImageFit::Contain,
                ..ImageState::new(texture, MARK_SIZE, MARK_SIZE)
            },
        },
    );
    marker.parent_scene = ParentScene::Scene(self::scene_id());
    marker.local_transform = LocalTransform {
        position: self::cell_position(index),
        ..LocalTransform::default()
    };
    marker
}

fn cell_index(world_hit: Vector3) -> Option<usize> {
    let half = GRID_SIZE / 2.0;
    let local_y = world_hit.y - BOARD_CENTER_Y;
    if world_hit.x < -half || world_hit.x >= half {
        return None;
    }
    if local_y < -half || local_y >= half {
        return None;
    }
    let column = ((world_hit.x + half) / CELL_SIZE).floor() as usize;
    let row = ((half - local_y) / CELL_SIZE).floor() as usize;
    Some(row * 3 + column)
}

fn cell_position(index: usize) -> Vector3 {
    let row = index / 3;
    let column = index % 3;
    Vector3::new(
        (column as f64 - 1.0) * CELL_SIZE,
        BOARD_CENTER_Y + (1.0 - row as f64) * CELL_SIZE,
        -0.05,
    )
}

fn outcome(board: &[Option<Mark>; 9]) -> Outcome {
    const LINES: [[usize; 3]; 8] = [
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8],
        [0, 4, 8],
        [2, 4, 6],
    ];
    for [first, second, third] in LINES {
        if board[first].is_some() && board[first] == board[second] && board[second] == board[third]
        {
            return if board[first] == Some(Mark::X) {
                Outcome::XWins
            } else {
                Outcome::OWins
            };
        }
    }
    if board.iter().all(Option::is_some) {
        Outcome::Draw
    } else {
        Outcome::InProgress
    }
}

fn empty_cells(board: &[Option<Mark>; 9]) -> Vec<usize> {
    board
        .iter()
        .enumerate()
        .filter_map(|(index, mark)| mark.is_none().then_some(index))
        .collect()
}

fn outcome_text(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::InProgress => PLAYER_TURN,
        Outcome::XWins => "You win! Click the board to play again.",
        Outcome::OWins => "Computer wins. Click the board to play again.",
        Outcome::Draw => "Draw! Click the board to play again.",
    }
}

fn status_command(text: &str) -> CommandBody {
    CommandBody::TextSetContent(TextContentPayload {
        object_id: self::object_id(STATUS_ID),
        text: text.to_owned(),
    })
}

fn action_response(
    session_id: SessionId,
    action_id: ActionId,
    bodies: Vec<CommandBody>,
) -> Response<Command> {
    let mut batch = self::batch(session_id, bodies);
    batch.caused_by_action_id = Some(action_id);
    Response::new(session_id, vec![ResponseMessage::Batch(batch)])
}

fn command_response(session_id: SessionId, bodies: Vec<CommandBody>) -> Response<Command> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(self::batch(session_id, bodies))],
    )
}

fn batch(session_id: SessionId, bodies: Vec<CommandBody>) -> Batch<Command> {
    Batch::new(
        BatchId::new_v4(),
        session_id,
        vec![ParallelCommandGroup::new(
            bodies
                .into_iter()
                .map(|body| Command::new(CommandId::new_v4(), body))
                .collect(),
        )],
    )
}

fn object_id(uuid: Uuid) -> ObjectId {
    ObjectId::from_uuid(uuid).expect("fixed object ID")
}

fn scene_id() -> SceneId {
    "00000000000000000000000000000001"
        .parse()
        .expect("fixed scene ID")
}

fn create_engine() -> Result<TicTacToeEngine, EngineError> {
    Ok(TicTacToeEngine {
        session_id: SessionId::new_v4(),
        board: [None; 9],
        marker_ids: [None; 9],
        outcome: Outcome::InProgress,
        ai_due: None,
        rng: Rng::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use masonry::{Action, PointerButtonPayload, ScreenPosition, TextureAddress};

    #[test]
    fn snapshot_contains_clickable_board_and_three_textures() {
        let snapshot = self::snapshot(SessionId::new_v4());

        assert_eq!(snapshot.objects.len(), 4);
        assert_eq!(
            snapshot.objects[1].pointer_events,
            vec![PointerEvent::Click]
        );
        for address in [BOARD_TEXTURE, X_TEXTURE, O_TEXTURE] {
            assert!(
                snapshot
                    .prepared_assets
                    .contains(&PreparedAsset::Texture(TextureAddress::new(address)))
            );
        }
    }

    #[test]
    fn cell_mapping_is_row_major_from_top_left() {
        assert_eq!(self::cell_index(Vector3::new(-2.4, 1.95, 0.0)), Some(0));
        assert_eq!(self::cell_index(Vector3::new(0.0, -0.45, 0.0)), Some(4));
        assert_eq!(self::cell_index(Vector3::new(2.4, -2.85, 0.0)), Some(8));
        assert_eq!(self::cell_index(Vector3::new(3.6, 0.0, 0.0)), None);
    }

    #[test]
    fn marker_positions_match_visible_grid_centers() {
        let top_left = self::cell_position(0);
        assert!((top_left.x + 1.92).abs() < 0.000_001);
        assert!((top_left.y - 1.22).abs() < 0.000_001);

        let center = self::cell_position(4);
        assert!(center.x.abs() < 0.000_001);
        assert!((center.y + 0.7).abs() < 0.000_001);

        let bottom_right = self::cell_position(8);
        assert!((bottom_right.x - 1.92).abs() < 0.000_001);
        assert!((bottom_right.y + 2.62).abs() < 0.000_001);
    }

    #[test]
    fn outcome_detects_rows_columns_diagonals_and_draws() {
        for cells in [[0, 1, 2], [0, 3, 6], [0, 4, 8], [2, 4, 6]] {
            let mut board = [None; 9];
            for cell in cells {
                board[cell] = Some(Mark::X);
            }
            assert_eq!(self::outcome(&board), Outcome::XWins);
        }
        assert_eq!(
            self::outcome(&[
                Some(Mark::X),
                Some(Mark::O),
                Some(Mark::X),
                Some(Mark::X),
                Some(Mark::O),
                Some(Mark::O),
                Some(Mark::O),
                Some(Mark::X),
                Some(Mark::X),
            ]),
            Outcome::Draw
        );
    }

    #[test]
    fn player_move_is_immediate_and_ai_move_is_polled_after_delay() {
        let mut engine = self::test_engine();
        let now = Instant::now();
        let response = engine.submit_at(self::click(engine.session_id, 4), now);
        let bodies = self::bodies(&response);

        assert!(matches!(bodies[0], CommandBody::ObjectCreate(_)));
        assert!(matches!(bodies[2], CommandBody::InputSetEnabled(_)));
        assert_eq!(engine.board[4], Some(Mark::X));
        assert!(
            engine
                .poll_at(now + AI_DELAY - Duration::from_millis(1))
                .is_none()
        );

        let response = engine
            .poll_at(now + AI_DELAY)
            .expect("AI should move at its deadline");
        assert!(matches!(
            self::bodies(&response)[0],
            CommandBody::ObjectCreate(_)
        ));
        assert_eq!(
            engine
                .board
                .iter()
                .filter(|mark| **mark == Some(Mark::O))
                .count(),
            1
        );
        assert!(engine.poll_at(now + AI_DELAY).is_none());
    }

    #[test]
    fn occupied_cell_does_not_advance_the_turn() {
        let mut engine = self::test_engine();
        engine.board[4] = Some(Mark::X);

        let response = engine.submit_at(self::click(engine.session_id, 4), Instant::now());

        assert_eq!(engine.board.iter().flatten().count(), 1);
        assert!(engine.ai_due.is_none());
        assert!(response.messages.is_empty());
    }

    #[test]
    fn winning_move_finishes_round_without_scheduling_ai() {
        let mut engine = self::test_engine();
        engine.board[0] = Some(Mark::X);
        engine.board[1] = Some(Mark::X);

        let response = engine.submit_at(self::click(engine.session_id, 2), Instant::now());

        assert_eq!(engine.outcome, Outcome::XWins);
        assert!(engine.ai_due.is_none());
        let CommandBody::TextSetContent(status) = &self::bodies(&response)[1] else {
            panic!("win should update status");
        };
        assert!(status.text.contains("You win"));
    }

    #[test]
    fn click_after_finished_round_clears_markers() {
        let mut engine = self::test_engine();
        engine.outcome = Outcome::Draw;
        engine.board[0] = Some(Mark::X);
        engine.marker_ids[0] = Some(ObjectId::new_v4());

        let response = engine.submit_at(self::click(engine.session_id, 4), Instant::now());

        assert_eq!(engine.board, [None; 9]);
        assert_eq!(engine.outcome, Outcome::InProgress);
        assert!(matches!(
            self::bodies(&response)[0],
            CommandBody::ObjectDestroy(_)
        ));
        assert!(matches!(
            self::bodies(&response)[1],
            CommandBody::TextSetContent(_)
        ));
    }

    fn test_engine() -> TicTacToeEngine {
        TicTacToeEngine {
            rng: Rng::with_seed(7),
            ..self::create_engine().expect("engine should be created")
        }
    }

    fn click(session_id: SessionId, index: usize) -> ClientMessage<(), CoreErrorCode> {
        ClientMessage::Action(Action::new(
            ActionId::new_v4(),
            session_id,
            ActionBody::PointerClick(PointerButtonPayload {
                object_id: self::object_id(BOARD_ID),
                pointer_id: 0,
                screen_position: ScreenPosition::default(),
                world_hit: self::cell_position(index),
                button: PointerButton::Left,
            }),
        ))
    }

    fn bodies(response: &Response<Command>) -> Vec<&CommandBody> {
        let ResponseMessage::Batch(batch) = &response.messages[0] else {
            panic!("response should contain a batch");
        };
        batch.groups[0]
            .commands
            .iter()
            .map(|command| &command.body)
            .collect()
    }
}

masonry_native::export_engine!(create_engine);
