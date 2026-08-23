//! Native rules engine for the standalone chess sample.

mod ai;
pub mod assets;
pub mod audio;
mod cursor;
mod movement;
mod persistence;
mod presentation;
mod spawn;

use std::{
    array,
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

use battlement::{
    ActionBody, ActionId, AudioClipAddress, Batch, BatchId, BatchStart, ClientMessage, Command,
    CommandBody, Connect, ControllerButton, CoreErrorCode, DragMode, GameObject, GameObjectKind,
    GridLayout, ImageState, KeyCode, MaterialAssignment, ObjectId, ObjectSetActivePayload,
    ParticleSpawnLocation, ParticleSpawnPayload, PointerButton, PointerEvent, PrefabAddress,
    PreparedAsset, Quaternion, Response, SceneId, SessionId, Vector3, object_id, scene_id,
};
use battlement_native::{Engine, EngineError, threading::AdaptiveThreadPool};
use cozy_chess::{Board, Color, File, GameStatus, Move, Piece, Rank, Square};
use fastrand::Rng;

use crate::assets::{black, effects, music, white};
use crate::audio::{
    CAPTURE_SOUNDS, CASTLE_SOUND, CHECK_SOUND, DRAW_SOUND, DROP_SOUNDS, INVALID_DROP_SOUND,
    MusicPlaylist, PICKUP_SOUNDS, PLAYER_LOSS_SOUND, PLAYER_WIN_SOUND, PROMOTION_SOUND,
    RESET_SOUND, VOLUME_DOWN_SOUND, VOLUME_UP_SOUND,
};

const SCENE_ID: SceneId = scene_id!("36630324-bd92-4497-b328-3599930dffa9");
const AI_THINK_TIME: Duration = Duration::from_secs(2);
const MUSIC_VOLUME_STEP: f64 = 0.1;
const HIGHLIGHT_HEIGHT: f64 = 0.02;
const HIGHLIGHT_SCALE: f64 = 0.09;
const CAMERA_BUTTON_DEPTH: f64 = 1.5;
const CAMERA_VERTICAL_FOV_RADIANS: f64 = std::f64::consts::PI / 3.0;
const REFRESH_BUTTON_SIZE: f64 = 0.16;
const REFRESH_BUTTON_MARGIN: f64 = 0.12;
const CAPTURE_EFFECT_LIFETIME_MS: u64 = 2_000;
const CAMERA_ROTATION: Quaternion =
    Quaternion::new(0.58184814, -0.001219943, 0.0008727778, 0.813296);

/// Addresses of all chess-piece prefabs.
pub const PIECE_PREFABS: [PrefabAddress; 12] = [
    white::PAWN,
    white::ROOK,
    white::KNIGHT,
    white::BISHOP,
    white::QUEEN,
    white::KING,
    black::PAWN,
    black::ROOK,
    black::KNIGHT,
    black::BISHOP,
    black::QUEEN,
    black::KING,
];
/// Background-music playlist order.
pub const MUSIC_TRACKS: [AudioClipAddress; 4] = [
    music::CRITICAL,
    music::SWITCH_WITH_ME,
    music::BREAKBEAT_CHIPS,
    music::DRAG_AND_DREAD,
];
/// Delay from the start of “Critical” to its first beat.
pub const CRITICAL_FIRST_BEAT_OFFSET_MS: u64 = 80;
/// Beat interval of “Critical”.
pub const CRITICAL_BEAT_INTERVAL_MS: u64 = 570;
/// Number of “Critical” beats used for the piece spawn-in sequence.
pub const PIECE_SPAWN_BEAT_COUNT: usize = 8;
/// Duration of the piece spawn-in sequence in milliseconds.
pub const PIECE_SPAWN_SEQUENCE_DURATION_MS: u64 =
    CRITICAL_FIRST_BEAT_OFFSET_MS + (PIECE_SPAWN_BEAT_COUNT - 1) as u64 * CRITICAL_BEAT_INTERVAL_MS;
/// Stable identity of the Play button.
pub const PLAY_BUTTON_ID: ObjectId = object_id!("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");
/// Stable identity of the new-game refresh button.
pub const REFRESH_BUTTON_ID: ObjectId = object_id!("35b288b3-6d72-48af-aeb9-e8f11d63e3ea");
/// Native chess rules engine with a parallel computer opponent.
pub struct ChessEngine {
    thread_pool: AdaptiveThreadPool,
    session_id: SessionId,
    starting_board: Board,
    board: Board,
    objects: [Option<ObjectId>; 64],
    highlight_ids: [ObjectId; 64],
    cursor: Square,
    cursor_visible: bool,
    selected: Option<Square>,
    pause_open: bool,
    confirm_new_game: bool,
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
    self::engine_for_board(Board::default(), AI_THINK_TIME, Rng::new(), Instant::now)
}

/// Creates a chess engine driven by a caller-supplied clock.
pub fn create_engine_with_clock(now: impl Fn() -> Instant + 'static) -> ChessEngine {
    self::engine_for_board(Board::default(), AI_THINK_TIME, Rng::new(), now)
        .expect("thread pool should initialize")
}

/// Creates an engine with a custom AI budget for simulations and tests.
pub fn create_engine_with_think_time(think_time: Duration) -> ChessEngine {
    self::engine_for_board(Board::default(), think_time, Rng::new(), Instant::now)
        .expect("thread pool should initialize")
}

/// Creates a deterministic engine for spawn-sequence simulations.
pub fn create_seeded_engine(seed: u64) -> ChessEngine {
    self::engine_for_board(
        Board::default(),
        AI_THINK_TIME,
        Rng::with_seed(seed),
        Instant::now,
    )
    .expect("thread pool should initialize")
}

/// Creates an engine from a FEN position for fake-client simulations.
pub fn create_engine_with_position(
    fen: &str,
    think_time: Duration,
) -> Result<ChessEngine, EngineError> {
    self::engine_for_board(
        fen.parse()
            .map_err(|error| EngineError::new(format!("invalid chess position: {error}")))?,
        think_time,
        Rng::new(),
        Instant::now,
    )
}

fn engine_for_board(
    board: Board,
    think_time: Duration,
    rng: Rng,
    now: impl Fn() -> Instant + 'static,
) -> Result<ChessEngine, EngineError> {
    Ok(ChessEngine {
        thread_pool: AdaptiveThreadPool::new()?,
        session_id: SessionId::new_v4(),
        starting_board: board.clone(),
        objects: [None; 64],
        highlight_ids: array::from_fn(|_| ObjectId::new_v4()),
        cursor: cursor::START,
        cursor_visible: false,
        selected: None,
        pause_open: false,
        confirm_new_game: false,
        board,
        started: false,
        ai_move: None,
        think_time,
        music: MusicPlaylist::new(),
        persistent_data_path: None,
        screen_aspect: 16.0 / 9.0,
        rng,
        now: Box::new(now),
    })
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
        self.cursor = cursor::START;
        self.cursor_visible = false;
        self.selected = None;
        self.pause_open = false;
        self.confirm_new_game = false;
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
                self.start_game(action.action_id, false)
            }
            ActionBody::PointerClick(payload)
                if payload.object_id == REFRESH_BUTTON_ID
                    && payload.button == PointerButton::Left
                    && self.pause_open =>
            {
                self.confirm_or_start_new_game(action.action_id, false)
            }
            ActionBody::PointerClick(payload) if payload.button == PointerButton::Left => {
                self.submit_click(action.action_id, payload.object_id)
            }
            ActionBody::DragEnd(payload) => {
                self.submit_drag(action.action_id, payload.object_id, payload.world_position)
            }
            ActionBody::DragStart(payload) => {
                let Some(square) = self::find_square(&self.objects, payload.object_id) else {
                    return Ok(empty);
                };
                self.cursor = square;
                self.selected = None;
                let commands = self
                    .hide_highlight_commands()
                    .into_iter()
                    .chain(self.cursor_commands(square, false))
                    .chain(self.highlight_commands(payload.object_id))
                    .collect::<Vec<_>>();
                Ok(audio::response_for_action(
                    self.session_id,
                    action.action_id,
                    commands,
                ))
            }
            ActionBody::KeyDown(payload)
                if !self.started
                    && matches!(
                        payload.key,
                        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space
                    ) =>
            {
                self.start_game(action.action_id, true)
            }
            ActionBody::KeyDown(payload)
                if self.started
                    && matches!(
                        payload.key,
                        KeyCode::ArrowLeft
                            | KeyCode::ArrowRight
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowDown
                    ) =>
            {
                self.move_cursor(action.action_id, payload.key)
            }
            ActionBody::KeyDown(payload)
                if self.started
                    && matches!(
                        payload.key,
                        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space
                    ) =>
            {
                self.activate_cursor(action.action_id)
            }
            ActionBody::KeyDown(payload)
                if self.started && payload.key == KeyCode::Escape && self.selected.is_some() =>
            {
                self.cancel_selection(action.action_id)
            }
            ActionBody::KeyDown(payload) if self.started && payload.key == KeyCode::Escape => {
                self.toggle_pause(action.action_id)
            }
            ActionBody::KeyDown(payload) if payload.key == KeyCode::Equal => {
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
            ActionBody::KeyDown(payload) if payload.key == KeyCode::Minus => {
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
            ActionBody::ControllerButtonDown(payload)
                if payload.button == ControllerButton::South && !self.started =>
            {
                self.start_game(action.action_id, true)
            }
            ActionBody::ControllerButtonDown(payload)
                if payload.button == ControllerButton::Start && self.started =>
            {
                self.toggle_pause(action.action_id)
            }
            ActionBody::ControllerButtonDown(payload) if self.pause_open => {
                self.handle_pause_button(action.action_id, payload.button)
            }
            ActionBody::ControllerButtonDown(payload)
                if payload.button == ControllerButton::South && self.started =>
            {
                self.activate_cursor(action.action_id)
            }
            ActionBody::ControllerButtonDown(payload)
                if payload.button == ControllerButton::East && self.started =>
            {
                self.cancel_selection(action.action_id)
            }
            ActionBody::ControllerButtonDown(payload)
                if payload.button == ControllerButton::LeftShoulder && self.started =>
            {
                self.cycle_cursor(action.action_id, false)
            }
            ActionBody::ControllerButtonDown(payload)
                if payload.button == ControllerButton::RightShoulder && self.started =>
            {
                self.cycle_cursor(action.action_id, true)
            }
            ActionBody::ControllerNavigate(payload) if self.started && !self.pause_open => {
                self.move_cursor_direction(action.action_id, payload.direction)
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
    fn submit_drag(
        &mut self,
        action_id: ActionId,
        object_id: ObjectId,
        world_position: Vector3,
    ) -> Result<Response<Command>, EngineError> {
        let Some(from) = self::find_square(&self.objects, object_id) else {
            return Ok(audio::response_for_action(
                self.session_id,
                action_id,
                self.hide_highlight_commands()
                    .into_iter()
                    .chain([audio::play_sound(INVALID_DROP_SOUND)]),
            ));
        };
        let target = self::square_at(world_position);
        if self.board.side_to_move() != Color::White || self.board.status() != GameStatus::Ongoing {
            self.cursor = from;
            self.selected = None;
            return Ok(audio::response_for_action(
                self.session_id,
                action_id,
                [movement::command(object_id, from, false)]
                    .into_iter()
                    .chain(self.hide_highlight_commands())
                    .chain(self.cursor_commands(from, false)),
            ));
        }
        if target == from {
            self.cursor = from;
            self.selected = Some(from);
            return Ok(audio::response_for_action(
                self.session_id,
                action_id,
                [movement::command(object_id, from, false)]
                    .into_iter()
                    .chain(self.cursor_commands(from, true)),
            ));
        }

        self.selected = None;
        let hide_highlights = self.hide_highlight_commands();
        let Some(mv) = self::player_move(&self.board, from, target) else {
            self.cursor = from;
            return Ok(audio::response_for_action(
                self.session_id,
                action_id,
                hide_highlights
                    .into_iter()
                    .chain([
                        movement::command(object_id, from, false),
                        audio::play_sound(INVALID_DROP_SOUND),
                    ])
                    .chain(self.cursor_commands(from, false)),
            ));
        };

        self.cursor = target;
        let mut commands = hide_highlights;
        commands.extend(self.apply_move(mv, false)?.into_iter().flatten());
        commands.extend(self.cursor_commands(target, false));
        if self.board.status() == GameStatus::Ongoing {
            commands.extend([
                cursor::dim_command(true),
                CommandBody::set_input_enabled(false),
            ]);
            self.start_ai();
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
                let mut groups = self.apply_move(mv, true)?;
                groups.last_mut().unwrap().extend([
                    cursor::dim_command(false),
                    CommandBody::set_input_enabled(true),
                ]);
                Ok(Some(Response::batch(
                    Batch::new(
                        BatchId::new_v4(),
                        self.session_id,
                        groups.into_iter().map(audio::parallel_group).collect(),
                    )
                    .start(BatchStart::AfterEarlierBlockingWork),
                )))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.ai_move = None;
                Ok(None)
            }
        }
    }

    fn start_ai(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let board = self.board.clone();
        let think_time = self.think_time;
        let search = move || {
            if let Some(mv) = ai::choose_move(&board, think_time) {
                let _ = sender.send(mv);
            }
        };
        // The reusable executor keeps browser policy out of the game: Web mobile
        // runs now without nested workers, while Web desktop and native schedule
        // the exact same Rayon search asynchronously on their parallel pools.
        self.thread_pool.execute(search);
        self.ai_move = Some(receiver);
    }

    fn apply_move(
        &mut self,
        mv: Move,
        animate: bool,
    ) -> Result<Vec<Vec<CommandBody>>, EngineError> {
        let color = self
            .board
            .color_on(mv.from)
            .expect("legal moves have a moving piece");
        let piece = self
            .board
            .piece_on(mv.from)
            .expect("legal moves have a moving piece");
        let is_castle = piece == Piece::King && self.board.color_on(mv.to) == Some(color);
        let mut groups = if is_castle {
            vec![self.apply_castle(mv, color, animate)]
        } else {
            self.apply_standard_move(mv, color, piece, animate)
        };
        self.board.play_unchecked(mv);
        self.persist_board()?;
        match self.board.status() {
            GameStatus::Won if self.board.side_to_move() == Color::Black => {
                groups
                    .last_mut()
                    .unwrap()
                    .push(audio::play_sound(PLAYER_WIN_SOUND));
            }
            GameStatus::Won => groups
                .last_mut()
                .unwrap()
                .push(audio::play_sound(PLAYER_LOSS_SOUND)),
            GameStatus::Drawn => groups
                .last_mut()
                .unwrap()
                .push(audio::play_sound(DRAW_SOUND)),
            GameStatus::Ongoing if !self.board.checkers().is_empty() => {
                groups
                    .last_mut()
                    .unwrap()
                    .push(audio::play_sound(CHECK_SOUND));
            }
            GameStatus::Ongoing => {}
        }
        groups.shrink_to_fit();
        Ok(groups)
    }

    fn apply_castle(&mut self, mv: Move, color: Color, animate: bool) -> Vec<CommandBody> {
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
            movement::command(king, king_to, animate),
            movement::command(rook, rook_to, animate),
            audio::play_sound(CASTLE_SOUND),
        ]
    }

    fn apply_standard_move(
        &mut self,
        mv: Move,
        color: Color,
        piece: Piece,
        animate: bool,
    ) -> Vec<Vec<CommandBody>> {
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
        let mut first = Vec::new();
        if !animate && let Some(captured) = captured {
            first.push(CommandBody::object_destroy(captured));
            first.push(self::capture_effect(capture));
        }
        let moving = self.objects[mv.from as usize]
            .take()
            .expect("legal moving pieces have an object");
        if let Some(promotion) = mv.promotion {
            first.push(movement::command(moving, mv.to, animate));
            let promoted = ObjectId::new_v4();
            self.objects[mv.to as usize] = Some(promoted);
            let mut promotion_commands = captured
                .filter(|_| animate)
                .map(CommandBody::object_destroy)
                .into_iter()
                .collect::<Vec<_>>();
            if animate && is_capture {
                promotion_commands.push(self::capture_effect(capture));
            }
            promotion_commands.extend([
                CommandBody::object_destroy(moving),
                CommandBody::object_create(self::piece_object(promoted, mv.to, color, promotion)),
                audio::play_sound(PROMOTION_SOUND),
            ]);
            return vec![first, promotion_commands];
        }

        self.objects[mv.to as usize] = Some(moving);
        if animate && piece == Piece::Knight {
            first.push(movement::knight_first_leg(moving, mv.from, mv.to));
        } else {
            first.push(movement::command(moving, mv.to, animate));
        }
        let sound = audio::play_sound(if is_capture {
            audio::random_sound(&mut self.rng, &CAPTURE_SOUNDS)
        } else {
            audio::random_sound(&mut self.rng, &DROP_SOUNDS)
        });
        let mut groups = if animate && piece == Piece::Knight {
            vec![first, vec![movement::knight_second_leg(moving, mv.to)]]
        } else {
            vec![first]
        };
        if animate && let Some(captured) = captured {
            groups.push(vec![
                CommandBody::object_destroy(captured),
                self::capture_effect(capture),
                sound,
            ]);
        } else {
            groups.first_mut().unwrap().push(sound);
        }
        groups
    }

    fn new_game(
        &mut self,
        action_id: ActionId,
        cursor_visible: bool,
    ) -> Result<Response<Command>, EngineError> {
        let was_started = self.started;
        let previous_objects = self.objects.iter().flatten().copied().collect::<Vec<_>>();
        self.ai_move = None;
        self.cursor = cursor::START;
        self.cursor_visible = cursor_visible;
        self.selected = None;
        self.pause_open = false;
        self.confirm_new_game = false;
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
        if was_started {
            commands.extend(self.hide_highlight_commands());
            commands.push(CommandBody::ObjectSetActive(ObjectSetActivePayload {
                object_id: REFRESH_BUTTON_ID,
                active: false,
            }));
        } else {
            commands.push(CommandBody::object_create(cursor::object(
                self.cursor,
                false,
            )));
        }
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
        commands.extend(cursor::state_commands(
            self.cursor,
            true,
            self.cursor_visible,
        ));
        commands.push(CommandBody::set_input_enabled(true));
        commands.push(audio::play_sound(RESET_SOUND));
        if self.board.side_to_move() == Color::Black && self.board.status() == GameStatus::Ongoing {
            self.start_ai();
        }
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
}

fn legal_destinations(board: &Board, from: Square) -> Vec<Square> {
    let mut destinations = Vec::new();
    board.generate_moves_for(from.bitboard(), |moves| {
        destinations.extend(
            moves
                .into_iter()
                .map(|mv| self::visible_destination(board, mv)),
        );
        false
    });
    destinations.sort_unstable();
    destinations.dedup();
    destinations
}

type PendingAi = Receiver<Move>;

fn highlight_object(object_id: ObjectId, square: Square) -> GameObject {
    let position = self::square_position(square);
    GameObject::new(
        object_id,
        GameObjectKind::Plane {
            materials: vec![MaterialAssignment::new(0, assets::LEGAL_SQUARE)],
        },
    )
    .active(false)
    .position(Vector3::new(position.x, HIGHLIGHT_HEIGHT, position.z))
    .scale(Vector3::new(HIGHLIGHT_SCALE, 1.0, HIGHLIGHT_SCALE))
    .pointer_events([PointerEvent::Click])
}

fn piece_object(object_id: ObjectId, square: Square, color: Color, piece: Piece) -> GameObject {
    let object = GameObject::new(
        object_id,
        GameObjectKind::prefab(self::address(color, piece)),
    )
    .position(self::square_position(square))
    .pointer_events([PointerEvent::Click]);
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
            assets::REFRESH_BUTTON,
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

fn find_square(objects: &[Option<ObjectId>; 64], object_id: ObjectId) -> Option<Square> {
    objects
        .iter()
        .position(|&candidate| candidate == Some(object_id))
        .map(Square::index)
}

fn find_highlight(highlight_ids: &[ObjectId; 64], object_id: ObjectId) -> Option<Square> {
    highlight_ids
        .iter()
        .position(|&candidate| candidate == object_id)
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

fn capture_effect(square: Square) -> CommandBody {
    CommandBody::ParticleSpawn(ParticleSpawnPayload {
        address: effects::CAPTURE,
        location: ParticleSpawnLocation::WorldPosition(self::square_position(square)),
        lifetime_ms: CAPTURE_EFFECT_LIFETIME_MS,
    })
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
    array::from_fn(|index| {
        board
            .piece_on(Square::index(index))
            .map(|_| ObjectId::new_v4())
    })
}

fn prepared_assets() -> Vec<PreparedAsset> {
    let mut assets = vec![
        PreparedAsset::scene(assets::CONTENT),
        PreparedAsset::texture(assets::PLAY_BUTTON),
        PreparedAsset::material(assets::LEGAL_SQUARE),
        PreparedAsset::texture(assets::REFRESH_BUTTON),
        PreparedAsset::prefab(effects::PIECE_SELECTED),
        PreparedAsset::particle_effect(effects::PIECE_SPAWN),
        PreparedAsset::particle_effect(effects::CAPTURE),
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

fn address(color: Color, piece: Piece) -> PrefabAddress {
    match (color, piece) {
        (Color::White, Piece::Pawn) => white::PAWN,
        (Color::White, Piece::Rook) => white::ROOK,
        (Color::White, Piece::Knight) => white::KNIGHT,
        (Color::White, Piece::Bishop) => white::BISHOP,
        (Color::White, Piece::Queen) => white::QUEEN,
        (Color::White, Piece::King) => white::KING,
        (Color::Black, Piece::Pawn) => black::PAWN,
        (Color::Black, Piece::Rook) => black::ROOK,
        (Color::Black, Piece::Knight) => black::KNIGHT,
        (Color::Black, Piece::Bishop) => black::BISHOP,
        (Color::Black, Piece::Queen) => black::QUEEN,
        (Color::Black, Piece::King) => black::KING,
    }
}

battlement_native::export_engine!(create_engine);
