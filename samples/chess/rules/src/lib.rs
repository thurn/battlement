//! Native rules engine for the standalone chess sample.

mod ai;
pub mod audio;
mod persistence;
mod spawn;

use std::{
    array,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

use cozy_chess::{Board, Color, File, GameStatus, Move, Piece, Rank, Square};
use fastrand::Rng;
use masonry::{
    ActionBody, ActionId, Batch, BatchId, BatchStart, ClientMessage, Command, CommandBody, Connect,
    CoreErrorCode, DragMode, GameObject, GameObjectKind, GridLayout, ImageState, KeyCode,
    MaterialAssignment, ObjectId, ObjectSetActivePayload, PointerButton, PointerEvent,
    PositionPayload, PreparedAsset, PropertyCommand, Quaternion, Response, Scene, SceneId, SessionId,
    Snapshot, Vector3, object_id, scene_id,
};
use masonry_native::{Engine, EngineError};

use crate::audio::{
    CAPTURE_SOUNDS, CASTLE_SOUND, CHECK_SOUND, DRAW_SOUND, DROP_SOUNDS, INVALID_DROP_SOUND,
    MusicPlaylist, PICKUP_SOUNDS, PLAYER_LOSS_SOUND, PLAYER_WIN_SOUND, PROMOTION_SOUND, RESET_SOUND,
    VOLUME_DOWN_SOUND, VOLUME_UP_SOUND,
};

const SCENE_ID: SceneId = scene_id!("36630324-bd92-4497-b328-3599930dffa9");
const AI_THINK_TIME: Duration = Duration::from_secs(2);
const PIECE_IDS: [ObjectId; 32] = [
    object_id!("3adb6e99-244d-47c1-8599-0697f311e1fc"),
    object_id!("9e94f5b6-e0f9-49f3-b770-3759d99fb16b"),
    object_id!("ab0352e6-869f-4dd7-ac86-c7436fc792ab"),
    object_id!("1465dcba-109a-4ad7-9fa0-a3553df8e2de"),
    object_id!("c64e16be-c3e3-425f-b55f-41062f124803"),
    object_id!("8c12a5c5-36d7-4c56-a0eb-ac9b1892f586"),
    object_id!("e8b4c350-7a4f-4301-8bce-7a8c2f2c6746"),
    object_id!("116a2d86-cf53-466d-b72a-29b057fc55e3"),
    object_id!("a11dbb79-1a3c-4761-9b6c-113bf58cb890"),
    object_id!("ea4e3481-34db-4c71-b12d-4a495c226757"),
    object_id!("7e119714-8c56-4c09-b47b-0953870003dd"),
    object_id!("222da336-8a5c-4e5e-b93f-5c1cd2c82144"),
    object_id!("1b502848-cf43-44fa-961d-0fa360186fba"),
    object_id!("e5021330-6761-4f88-92d5-f295a81a2644"),
    object_id!("5e81e0c4-01fe-49cb-9b4f-52917e09f531"),
    object_id!("9afb4bf4-2893-4c70-8f10-4ee88d027b7c"),
    object_id!("cf192a04-d27f-4132-82e5-941cc45ffa2b"),
    object_id!("41382898-5ca9-48b3-9111-2ab3ea98c142"),
    object_id!("570391ac-ceb5-4ba3-83ce-cebb8c33162e"),
    object_id!("6750c66a-3a07-4f4f-8256-6ad46c55193b"),
    object_id!("2f2f4744-6ff3-45ae-b03d-63637ed07313"),
    object_id!("7065f5ef-90cd-4f64-afaa-1e134064b7cb"),
    object_id!("b89582ef-8d68-400c-8a9f-46b026268150"),
    object_id!("01efbb5b-d8e4-4883-827a-5058054d1904"),
    object_id!("fd810f3e-b116-4a67-9eec-ecc4b2e5fe92"),
    object_id!("84a35802-262e-45ed-96bb-8055169a6395"),
    object_id!("3e3cde8f-f278-47dc-aad9-e6c5312e7363"),
    object_id!("d1f48e79-ce8e-4f41-ad29-e53f20a6aef3"),
    object_id!("5f703cb7-61ae-477b-8808-dd715ca85436"),
    object_id!("0e28cae9-9119-4df7-a576-9f184b47d91d"),
    object_id!("3bb6fccc-55ab-46b6-b9a1-65657e55add9"),
    object_id!("c9f6606b-d204-4f82-a3c5-7113469984e8"),
];
const MUSIC_VOLUME_STEP: f64 = 0.1;
const HIGHLIGHT_HEIGHT: f64 = 0.02;
const HIGHLIGHT_SCALE: f64 = 0.09;
const CAMERA_BUTTON_DEPTH: f64 = 1.5;
const CAMERA_VERTICAL_FOV_RADIANS: f64 = std::f64::consts::PI / 3.0;
const REFRESH_BUTTON_SIZE: f64 = 0.16;
const REFRESH_BUTTON_MARGIN: f64 = 0.12;
const CAMERA_ROTATION: Quaternion =
    Quaternion::new(0.58184814, -0.001219943, 0.0008727778, 0.813296);

/// Address of the decorated board scene.
pub const CONTENT_SCENE: &str = "chess/content";
/// Address of the white pawn prefab.
pub const WHITE_PAWN_PREFAB: &str = "chess/white/pawn";
/// Address of the white rook prefab.
pub const WHITE_ROOK_PREFAB: &str = "chess/white/rook";
/// Address of the white knight prefab.
pub const WHITE_KNIGHT_PREFAB: &str = "chess/white/knight";
/// Address of the white bishop prefab.
pub const WHITE_BISHOP_PREFAB: &str = "chess/white/bishop";
/// Address of the white queen prefab.
pub const WHITE_QUEEN_PREFAB: &str = "chess/white/queen";
/// Address of the white king prefab.
pub const WHITE_KING_PREFAB: &str = "chess/white/king";
/// Address of the black pawn prefab.
pub const BLACK_PAWN_PREFAB: &str = "chess/black/pawn";
/// Address of the black rook prefab.
pub const BLACK_ROOK_PREFAB: &str = "chess/black/rook";
/// Address of the black knight prefab.
pub const BLACK_KNIGHT_PREFAB: &str = "chess/black/knight";
/// Address of the black bishop prefab.
pub const BLACK_BISHOP_PREFAB: &str = "chess/black/bishop";
/// Address of the black queen prefab.
pub const BLACK_QUEEN_PREFAB: &str = "chess/black/queen";
/// Address of the black king prefab.
pub const BLACK_KING_PREFAB: &str = "chess/black/king";
/// Addresses of all chess-piece prefabs.
pub const PIECE_PREFABS: [&str; 12] = [
    WHITE_PAWN_PREFAB,
    WHITE_ROOK_PREFAB,
    WHITE_KNIGHT_PREFAB,
    WHITE_BISHOP_PREFAB,
    WHITE_QUEEN_PREFAB,
    WHITE_KING_PREFAB,
    BLACK_PAWN_PREFAB,
    BLACK_ROOK_PREFAB,
    BLACK_KNIGHT_PREFAB,
    BLACK_BISHOP_PREFAB,
    BLACK_QUEEN_PREFAB,
    BLACK_KING_PREFAB,
];
/// Address of NotJam's “Critical”.
pub const CRITICAL_MUSIC: &str = "chess/music/critical";
/// Address of NotJam's “Switch with Me”.
pub const SWITCH_WITH_ME_MUSIC: &str = "chess/music/switch-with-me";
/// Address of NotJam's “Breakbeat Chips”.
pub const BREAKBEAT_CHIPS_MUSIC: &str = "chess/music/breakbeat-chips";
/// Address of NotJam's “Drag and Dread”.
pub const DRAG_AND_DREAD_MUSIC: &str = "chess/music/drag-and-dread";
/// Background-music playlist order.
pub const MUSIC_TRACKS: [&str; 4] = [
    CRITICAL_MUSIC,
    SWITCH_WITH_ME_MUSIC,
    BREAKBEAT_CHIPS_MUSIC,
    DRAG_AND_DREAD_MUSIC,
];
/// Address of the rounded Play button texture.
pub const PLAY_BUTTON_TEXTURE: &str = "chess/play-button";
/// Address of the translucent green legal-square material.
pub const LEGAL_SQUARE_MATERIAL: &str = "chess/legal-square";
/// Address of the Nova Shader healing effect used when pieces appear.
pub const PIECE_SPAWN_EFFECT: &str = "chess/effects/piece-spawn";
/// Stable identity of the Play button.
pub const PLAY_BUTTON_ID: ObjectId = object_id!("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");
/// Address of the new-game refresh button texture.
pub const REFRESH_BUTTON_TEXTURE: &str = "chess/refresh-button";
/// Stable identity of the new-game refresh button.
pub const REFRESH_BUTTON_ID: ObjectId = object_id!("35b288b3-6d72-48af-aeb9-e8f11d63e3ea");
/// Native chess rules engine with a parallel computer opponent.
pub struct ChessEngine {
    session_id: SessionId,
    starting_board: Board,
    board: Board,
    objects: [Option<ObjectId>; 64],
    highlight_ids: [ObjectId; 64],
    started: bool,
    ai_move: Option<PendingAi>,
    think_time: Duration,
    music: MusicPlaylist,
    persistent_data_path: Option<PathBuf>,
    screen_aspect: f64,
    rng: Rng,
    now: Box<dyn Fn() -> Instant>,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<ChessEngine, EngineError> {
    #[cfg(target_arch = "wasm32")]
    ai::initialize_parallelism();
    Ok(self::engine_for_board(
        Board::default(),
        AI_THINK_TIME,
        Rng::new(),
        Instant::now,
    ))
}

/// Creates a chess engine driven by a caller-supplied clock.
pub fn create_engine_with_clock(now: impl Fn() -> Instant + 'static) -> ChessEngine {
    self::engine_for_board(Board::default(), AI_THINK_TIME, Rng::new(), now)
}

/// Creates an engine with a custom AI budget for simulations and tests.
pub fn create_engine_with_think_time(think_time: Duration) -> ChessEngine {
    self::engine_for_board(Board::default(), think_time, Rng::new(), Instant::now)
}

/// Creates a deterministic engine for spawn-sequence simulations.
pub fn create_seeded_engine(seed: u64) -> ChessEngine {
    self::engine_for_board(
        Board::default(),
        AI_THINK_TIME,
        Rng::with_seed(seed),
        Instant::now,
    )
}

/// Creates an engine from a FEN position for fake-client simulations.
pub fn create_engine_with_position(
    fen: &str,
    think_time: Duration,
) -> Result<ChessEngine, EngineError> {
    Ok(self::engine_for_board(
        fen.parse()
            .map_err(|error| EngineError::new(format!("invalid chess position: {error}")))?,
        think_time,
        Rng::new(),
        Instant::now,
    ))
}

fn engine_for_board(
    board: Board,
    think_time: Duration,
    rng: Rng,
    now: impl Fn() -> Instant + 'static,
) -> ChessEngine {
    ChessEngine {
        session_id: SessionId::new_v4(),
        starting_board: board.clone(),
        objects: [None; 64],
        highlight_ids: array::from_fn(|_| ObjectId::new_v4()),
        board,
        started: false,
        ai_move: None,
        think_time,
        music: MusicPlaylist::new(),
        persistent_data_path: None,
        screen_aspect: 16.0 / 9.0,
        rng,
        now: Box::new(now),
    }
}

impl Engine for ChessEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        self.highlight_ids = array::from_fn(|_| ObjectId::new_v4());
        self.persistent_data_path = message.persistent_data_path.map(PathBuf::from);
        self.screen_aspect = if message.screen.height == 0 {
            16.0 / 9.0
        } else {
            f64::from(message.screen.width) / f64::from(message.screen.height)
        };
        let saved_board = self
            .persistent_data_path
            .as_deref()
            .and_then(persistence::load);
        self.started = saved_board.is_some();
        self.board = saved_board.unwrap_or_else(|| self.starting_board.clone());
        self.objects = if self.started {
            self::objects_for_board(&self.board)
        } else {
            [None; 64]
        };
        self.ai_move = None;
        self.music = MusicPlaylist::new();
        if self.started {
            self.music.reset((self.now)());
            if self.board.side_to_move() == Color::Black
                && self.board.status() == GameStatus::Ongoing
            {
                self.start_ai();
            }
        }
        Ok(Response::snapshot(self.snapshot()))
    }

    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        let empty = Response::empty(self.session_id);
        let Some(action) = message.into_action() else {
            return Ok(empty);
        };
        match action.body {
            ActionBody::PointerClick(payload)
                if payload.object_id == PLAY_BUTTON_ID && payload.button == PointerButton::Left =>
            {
                self.start_game(action.action_id)
            }
            ActionBody::PointerClick(payload)
                if payload.object_id == REFRESH_BUTTON_ID
                    && payload.button == PointerButton::Left =>
            {
                self.new_game(action.action_id)
            }
            ActionBody::DragEnd(payload) => {
                self.submit_drag(action.action_id, payload.object_id, payload.world_position)
            }
            ActionBody::DragStart(payload) => {
                let commands = self.highlight_commands(payload.object_id);
                if commands.is_empty() {
                    Ok(empty)
                } else {
                    Ok(audio::response_for_action(
                        self.session_id,
                        action.action_id,
                        commands,
                    ))
                }
            }
            ActionBody::KeyDown(payload) if payload.key == KeyCode::ArrowUp => {
                let volume = self
                    .music
                    .set_volume(self.music.volume() + MUSIC_VOLUME_STEP);
                Ok(audio::response_for_action(
                    self.session_id,
                    action.action_id,
                    volume
                        .into_iter()
                        .chain([audio::play_sound(VOLUME_UP_SOUND)]),
                ))
            }
            ActionBody::KeyDown(payload) if payload.key == KeyCode::ArrowDown => {
                let volume = self
                    .music
                    .set_volume(self.music.volume() - MUSIC_VOLUME_STEP);
                Ok(audio::response_for_action(
                    self.session_id,
                    action.action_id,
                    volume
                        .into_iter()
                        .chain([audio::play_sound(VOLUME_DOWN_SOUND)]),
                ))
            }
            _ => Ok(empty),
        }
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        if !self.started {
            return Ok(None);
        }
        Ok(self
            .poll_ai()?
            .or_else(|| self.music.poll(self.session_id, (self.now)())))
    }
}

impl ChessEngine {
    fn start_game(&mut self, action_id: ActionId) -> Result<Response<Command>, EngineError> {
        if self.started {
            return Ok(Response::empty(self.session_id));
        }
        self.started = true;
        self.objects = self::objects_for_board(&self.board);
        self.persist_board()?;
        self.music.reset((self.now)());
        let mut white = Vec::new();
        let mut black = Vec::new();
        for square in Square::ALL {
            if let Some(object_id) = self.objects[square as usize] {
                let color = self
                    .board
                    .color_on(square)
                    .expect("mapped pieces have a color");
                let object = self::piece_object(
                    object_id,
                    square,
                    color,
                    self.board
                        .piece_on(square)
                        .expect("mapped pieces have a type"),
                );
                if color == Color::White {
                    white.push(object);
                } else {
                    black.push(object);
                }
            }
        }
        let ai_turn =
            self.board.side_to_move() == Color::Black && self.board.status() == GameStatus::Ongoing;
        if ai_turn {
            self.start_ai();
        }
        Ok(Response::batch(spawn::batch(
            self.session_id,
            action_id,
            white,
            black,
            self::refresh_button(self.screen_aspect),
            !ai_turn,
            &mut self.rng,
        )))
    }

    fn submit_drag(
        &mut self,
        action_id: ActionId,
        object_id: ObjectId,
        world_position: Vector3,
    ) -> Result<Response<Command>, EngineError> {
        let hide_highlights = self.hide_highlight_commands();
        let Some(from) = self::find_square(&self.objects, object_id) else {
            return Ok(audio::response_for_action(
                self.session_id,
                action_id,
                hide_highlights
                    .into_iter()
                    .chain([audio::play_sound(INVALID_DROP_SOUND)]),
            ));
        };
        let target = self::square_at(world_position);
        let Some(mv) = self::player_move(&self.board, from, target) else {
            return Ok(audio::response_for_action(
                self.session_id,
                action_id,
                hide_highlights
                    .into_iter()
                    .chain([
                        self::move_command(object_id, from),
                        audio::play_sound(INVALID_DROP_SOUND),
                    ]),
            ));
        };

        let mut commands = hide_highlights;
        commands.extend(self.apply_move(mv)?);
        if self.board.status() == GameStatus::Ongoing {
            self.start_ai();
            commands.push(CommandBody::set_input_enabled(false));
        }
        Ok(audio::response_for_action(
            self.session_id,
            action_id,
            commands,
        ))
    }

    fn highlight_commands(&mut self, object_id: ObjectId) -> Vec<CommandBody> {
        let Some(from) = self::find_square(&self.objects, object_id) else {
            return Vec::new();
        };
        if self.board.side_to_move() != Color::White
            || self.board.color_on(from) != Some(Color::White)
        {
            return Vec::new();
        }

        let mut targets = [false; 64];
        self.board.generate_moves_for(from.bitboard(), |moves| {
            for mv in moves {
                targets[self::visible_destination(&self.board, mv) as usize] = true;
            }
            false
        });
        let mut commands = self
            .highlight_ids
            .iter()
            .zip(targets)
            .filter_map(|(&object_id, active)| {
                active.then_some(CommandBody::ObjectSetActive(ObjectSetActivePayload {
                    object_id,
                    active: true,
                }))
            })
            .collect::<Vec<_>>();
        commands.push(audio::play_sound(audio::random_sound(
            &mut self.rng,
            &PICKUP_SOUNDS,
        )));
        commands
    }

    fn hide_highlight_commands(&self) -> Vec<CommandBody> {
        self.highlight_ids
            .map(|object_id| {
                CommandBody::ObjectSetActive(ObjectSetActivePayload {
                    object_id,
                    active: false,
                })
            })
            .into()
    }

    fn poll_ai(&mut self) -> Result<Option<Response<Command>>, EngineError> {
        let Some(receiver) = &self.ai_move else {
            return Ok(None);
        };
        match receiver.try_recv() {
            Ok(mv) => {
                self.ai_move = None;
                let mut commands = self.apply_move(mv)?;
                commands.push(CommandBody::set_input_enabled(true));
                Ok(Some(Response::batch(
                    Batch::new(
                        BatchId::new_v4(),
                        self.session_id,
                        vec![audio::parallel_group(commands)],
                    )
                        .start(BatchStart::AfterEarlierBlockingWork),
                )))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.ai_move = None;
                Ok(Some(Response::commands(
                    self.session_id,
                    [CommandBody::set_input_enabled(true)],
                )))
            }
        }
    }

    fn start_ai(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let board = self.board.clone();
        let think_time = self.think_time;
        rayon::spawn(move || {
            if let Some(mv) = ai::choose_move(&board, think_time) {
                let _ = sender.send(mv);
            }
        });
        self.ai_move = Some(receiver);
    }

    fn apply_move(&mut self, mv: Move) -> Result<Vec<CommandBody>, EngineError> {
        let color = self
            .board
            .color_on(mv.from)
            .expect("legal moves have a moving piece");
        let piece = self
            .board
            .piece_on(mv.from)
            .expect("legal moves have a moving piece");
        let is_castle = piece == Piece::King && self.board.color_on(mv.to) == Some(color);
        let mut commands = if is_castle {
            self.apply_castle(mv, color)
        } else {
            self.apply_standard_move(mv, color, piece)
        };
        self.board.play_unchecked(mv);
        self.persist_board()?;
        match self.board.status() {
            GameStatus::Won if self.board.side_to_move() == Color::Black => {
                commands.push(audio::play_sound(PLAYER_WIN_SOUND));
            }
            GameStatus::Won => commands.push(audio::play_sound(PLAYER_LOSS_SOUND)),
            GameStatus::Drawn => commands.push(audio::play_sound(DRAW_SOUND)),
            GameStatus::Ongoing if !self.board.checkers().is_empty() => {
                commands.push(audio::play_sound(CHECK_SOUND));
            }
            GameStatus::Ongoing => {}
        }
        commands.shrink_to_fit();
        Ok(commands)
    }

    fn apply_castle(&mut self, mv: Move, color: Color) -> Vec<CommandBody> {
        let rank = if color == Color::White {
            Rank::First
        } else {
            Rank::Eighth
        };
        let short = mv.to.file() > mv.from.file();
        let king_to = Square::new(if short { File::G } else { File::C }, rank);
        let rook_to = Square::new(if short { File::F } else { File::D }, rank);
        let king = self.objects[mv.from as usize]
            .take()
            .expect("castling king has an object");
        let rook = self.objects[mv.to as usize]
            .take()
            .expect("castling rook has an object");
        self.objects[king_to as usize] = Some(king);
        self.objects[rook_to as usize] = Some(rook);
        vec![
            self::move_command(king, king_to),
            self::move_command(rook, rook_to),
            audio::play_sound(CASTLE_SOUND),
        ]
    }

    fn apply_standard_move(&mut self, mv: Move, color: Color, piece: Piece) -> Vec<CommandBody> {
        let capture = if piece == Piece::Pawn
            && mv.from.file() != mv.to.file()
            && self.board.piece_on(mv.to).is_none()
        {
            Square::new(mv.to.file(), mv.from.rank())
        } else {
            mv.to
        };
        let captured = self.objects[capture as usize].take();
        let is_capture = captured.is_some();
        let mut commands = captured
            .map(CommandBody::object_destroy)
            .into_iter()
            .collect::<Vec<_>>();
        let moving = self.objects[mv.from as usize]
            .take()
            .expect("legal moving pieces have an object");
        if let Some(promotion) = mv.promotion {
            commands.push(CommandBody::object_destroy(moving));
            let promoted = ObjectId::new_v4();
            self.objects[mv.to as usize] = Some(promoted);
            commands.push(CommandBody::object_create(self::piece_object(
                promoted, mv.to, color, promotion,
            )));
        } else {
            self.objects[mv.to as usize] = Some(moving);
            commands.push(self::move_command(moving, mv.to));
        }
        commands.push(audio::play_sound(if mv.promotion.is_some() {
            PROMOTION_SOUND
        } else if is_capture {
            audio::random_sound(&mut self.rng, &CAPTURE_SOUNDS)
        } else {
            audio::random_sound(&mut self.rng, &DROP_SOUNDS)
        }));
        commands
    }

    fn new_game(&mut self, action_id: ActionId) -> Result<Response<Command>, EngineError> {
        let was_started = self.started;
        let previous_objects = self.objects.iter().flatten().copied().collect::<Vec<_>>();
        self.ai_move = None;
        self.board = self.starting_board.clone();
        self.objects = self::objects_for_board(&self.board);
        self.started = true;
        self.persist_board()?;
        if !was_started {
            self.music.reset((self.now)());
        }

        let mut commands = previous_objects
            .into_iter()
            .map(CommandBody::object_destroy)
            .collect::<Vec<_>>();
        if !was_started {
            commands.push(CommandBody::object_destroy(PLAY_BUTTON_ID));
        }
        for square in Square::ALL {
            if let Some(object_id) = self.objects[square as usize] {
                commands.push(CommandBody::object_create(self::piece_object(
                    object_id,
                    square,
                    self.board
                        .color_on(square)
                        .expect("mapped pieces have a color"),
                    self.board
                        .piece_on(square)
                        .expect("mapped pieces have a type"),
                )));
            }
        }
        commands.push(CommandBody::set_input_enabled(true));
        commands.push(audio::play_sound(RESET_SOUND));
        Ok(audio::response_for_action(
            self.session_id,
            action_id,
            commands,
        ))
    }

    fn persist_board(&self) -> Result<(), EngineError> {
        let Some(path) = &self.persistent_data_path else {
            return Ok(());
        };
        persistence::save(path, &self.board).map_err(EngineError::new)
    }

    fn snapshot(&self) -> Snapshot {
        let mut objects = self
            .highlight_ids
            .iter()
            .zip(Square::ALL)
            .map(|(&object_id, square)| self::highlight_object(object_id, square))
            .collect::<Vec<_>>();
        if self.started {
            objects.push(self::refresh_button(self.screen_aspect));
            for square in Square::ALL {
                if let Some(object_id) = self.objects[square as usize] {
                    objects.push(self::piece_object(
                        object_id,
                        square,
                        self.board
                            .color_on(square)
                            .expect("mapped pieces have a color"),
                        self.board
                            .piece_on(square)
                            .expect("mapped pieces have a type"),
                    ));
                }
            }
        } else {
            objects.push(
                GameObject::new(
                    PLAY_BUTTON_ID,
                    ImageState::new(PLAY_BUTTON_TEXTURE, 0.8, 0.24),
                )
                .position(Vector3::new(0.0, 6.38, -3.86))
                .rotation(CAMERA_ROTATION)
                .pointer_events([PointerEvent::Click]),
            );
        }
        Snapshot::new_with_main_camera(
            self.session_id,
            self::prepared_assets(),
            vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
            objects,
        )
        .input_disabled(
            self.started
                && self.board.side_to_move() == Color::Black
                && self.board.status() == GameStatus::Ongoing,
        )
        .global_keys([KeyCode::ArrowUp, KeyCode::ArrowDown])
    }
}

type PendingAi = Receiver<Move>;

fn highlight_object(object_id: ObjectId, square: Square) -> GameObject {
    let position = self::square_position(square);
    GameObject::new(
        object_id,
        GameObjectKind::Plane {
            materials: vec![MaterialAssignment::new(0, LEGAL_SQUARE_MATERIAL)],
        },
    )
    .active(false)
    .position(Vector3::new(position.x, HIGHLIGHT_HEIGHT, position.z))
    .scale(Vector3::new(HIGHLIGHT_SCALE, 1.0, HIGHLIGHT_SCALE))
}
fn piece_object(object_id: ObjectId, square: Square, color: Color, piece: Piece) -> GameObject {
    let object = GameObject::new(
        object_id,
        GameObjectKind::prefab(self::address(color, piece)),
    )
    .position(self::square_position(square));
    let object = if color == Color::White {
        object.draggable(DragMode::SnapToPointer)
    } else {
        object
    };
    if color == Color::Black {
        object.rotation(Quaternion::new(0.0, 1.0, 0.0, 0.0))
    } else {
        object
    }
}

fn refresh_button(screen_aspect: f64) -> GameObject {
    let half_height = CAMERA_BUTTON_DEPTH * (CAMERA_VERTICAL_FOV_RADIANS / 2.0).tan();
    let right = half_height * screen_aspect - REFRESH_BUTTON_SIZE / 2.0 - REFRESH_BUTTON_MARGIN;
    let up = half_height - REFRESH_BUTTON_SIZE / 2.0 - REFRESH_BUTTON_MARGIN;
    GameObject::new(
        REFRESH_BUTTON_ID,
        ImageState::new(
            REFRESH_BUTTON_TEXTURE,
            REFRESH_BUTTON_SIZE,
            REFRESH_BUTTON_SIZE,
        ),
    )
    .position(Vector3::new(
        right,
        8.0 - 0.946201 * CAMERA_BUTTON_DEPTH + 0.323579 * up,
        -3.75 + 0.323579 * CAMERA_BUTTON_DEPTH + 0.946201 * up,
    ))
    .rotation(CAMERA_ROTATION)
    .pointer_events([PointerEvent::Click])
}

fn player_move(board: &Board, from: Square, target: Square) -> Option<Move> {
    if board.side_to_move() != Color::White || board.color_on(from) != Some(Color::White) {
        return None;
    }
    let target = if board.piece_on(from) == Some(Piece::King) {
        match target {
            Square::G1 => Square::H1,
            Square::C1 => Square::A1,
            _ => target,
        }
    } else {
        target
    };
    let promotion = if board.piece_on(from) == Some(Piece::Pawn) && target.rank() == Rank::Eighth {
        Some(Piece::Queen)
    } else {
        None
    };
    let candidate = Move {
        from,
        to: target,
        promotion,
    };
    board.is_legal(candidate).then_some(candidate)
}

fn visible_destination(board: &Board, mv: Move) -> Square {
    let color = board.color_on(mv.from);
    if board.piece_on(mv.from) != Some(Piece::King) || board.color_on(mv.to) != color {
        return mv.to;
    }
    Square::new(
        if mv.to.file() > mv.from.file() {
            File::G
        } else {
            File::C
        },
        mv.from.rank(),
    )
}

fn move_command(object_id: ObjectId, square: Square) -> CommandBody {
    CommandBody::TransformSetWorldPosition(PropertyCommand::canceling(PositionPayload {
        object_id,
        position: self::square_position(square),
    }))
}

fn find_square(objects: &[Option<ObjectId>; 64], object_id: ObjectId) -> Option<Square> {
    objects
        .iter()
        .position(|&candidate| candidate == Some(object_id))
        .map(Square::index)
}

fn square_at(position: Vector3) -> Square {
    let file = (position.x + 3.5).round().clamp(0.0, 7.0) as usize;
    let rank = (position.z + 3.5).round().clamp(0.0, 7.0) as usize;
    Square::new(File::index(file), Rank::index(rank))
}

fn square_position(square: Square) -> Vector3 {
    self::board_grid().position(square.file() as u32, square.rank() as u32)
}

fn board_grid() -> GridLayout {
    GridLayout::centered(
        Vector3::ZERO,
        8,
        8,
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
}

fn objects_for_board(board: &Board) -> [Option<ObjectId>; 64] {
    let mut objects = [None; 64];
    let mut object_ids = PIECE_IDS.into_iter();
    for square in Square::ALL {
        if board.piece_on(square).is_some() {
            objects[square as usize] = Some(
                object_ids
                    .next()
                    .expect("legal chess positions contain at most 32 pieces"),
            );
        }
    }
    objects
}

fn prepared_assets() -> Vec<PreparedAsset> {
    let mut assets = vec![
        PreparedAsset::scene(CONTENT_SCENE),
        PreparedAsset::texture(PLAY_BUTTON_TEXTURE),
        PreparedAsset::material(LEGAL_SQUARE_MATERIAL),
        PreparedAsset::texture(REFRESH_BUTTON_TEXTURE),
        PreparedAsset::particle_effect(PIECE_SPAWN_EFFECT),
    ];
    assets.extend(MUSIC_TRACKS.map(PreparedAsset::audio_clip));
    assets.extend(audio::SOUND_EFFECTS.map(PreparedAsset::audio_clip));
    for color in Color::ALL {
        for piece in Piece::ALL {
            assets.push(PreparedAsset::prefab(self::address(color, piece)));
        }
    }
    assets
}

fn address(color: Color, piece: Piece) -> &'static str {
    match (color, piece) {
        (Color::White, Piece::Pawn) => WHITE_PAWN_PREFAB,
        (Color::White, Piece::Rook) => WHITE_ROOK_PREFAB,
        (Color::White, Piece::Knight) => WHITE_KNIGHT_PREFAB,
        (Color::White, Piece::Bishop) => WHITE_BISHOP_PREFAB,
        (Color::White, Piece::Queen) => WHITE_QUEEN_PREFAB,
        (Color::White, Piece::King) => WHITE_KING_PREFAB,
        (Color::Black, Piece::Pawn) => BLACK_PAWN_PREFAB,
        (Color::Black, Piece::Rook) => BLACK_ROOK_PREFAB,
        (Color::Black, Piece::Knight) => BLACK_KNIGHT_PREFAB,
        (Color::Black, Piece::Bishop) => BLACK_BISHOP_PREFAB,
        (Color::Black, Piece::Queen) => BLACK_QUEEN_PREFAB,
        (Color::Black, Piece::King) => BLACK_KING_PREFAB,
    }
}

masonry_native::export_engine!(create_engine);
