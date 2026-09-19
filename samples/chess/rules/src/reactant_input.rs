//! App-local selection, menu, settings, and native input routing.

use std::{
  cell::{Cell, RefCell},
  collections::HashSet,
  rc::Rc,
};

use battlement::{
  ControllerButton, ControllerInputSettings, DebugUiSurface, ObjectId, PhysicalKey, Vector3,
};
use battlement_native::CoreActionBodyView;
use cozy_chess::{Color, GameStatus, Square};
use reactant::{
  DispatchResult, GameConsumer, GameHandle, GameStatus as RulesStatus,
  app::App,
  prelude::{DisplayStore, ExternalStore},
};

use crate::{
  MUSIC_TRACKS,
  position::ChessMove,
  reactant_game::{ChessAction, ChessGame, ChessState, StartMode},
};

const DEFAULT_MUSIC_VOLUME: f64 = 0.35;

const GLOBAL_KEYS: [PhysicalKey; 18] = [
  PhysicalKey::ArrowLeft,
  PhysicalKey::ArrowRight,
  PhysicalKey::ArrowUp,
  PhysicalKey::ArrowDown,
  PhysicalKey::Enter,
  PhysicalKey::NumpadEnter,
  PhysicalKey::Space,
  PhysicalKey::Escape,
  PhysicalKey::Minus,
  PhysicalKey::Equal,
  PhysicalKey::KeyR,
  PhysicalKey::ShiftLeft,
  PhysicalKey::ShiftRight,
  PhysicalKey::ControlLeft,
  PhysicalKey::ControlRight,
  PhysicalKey::MetaLeft,
  PhysicalKey::MetaRight,
  PhysicalKey::KeyL,
];

const CONTROLLER_BUTTONS: [ControllerButton; 5] = [
  ControllerButton::South,
  ControllerButton::East,
  ControllerButton::LeftShoulder,
  ControllerButton::RightShoulder,
  ControllerButton::Start,
];

/// One app-local effect delivered after input without entering rules state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LocalEffect {
  Sound(battlement::AudioClipAddress),
  Invalid,
  ShowDebug(DebugUiSurface),
}

/// Selection, menu, settings, and one-shot app state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LocalState {
  pub(crate) selected: Option<Square>,
  pub(crate) cursor: Square,
  pub(crate) cursor_visible: bool,
  pub(crate) pause_open: bool,
  pub(crate) confirm_new_game: bool,
  pub(crate) volume: f64,
  pub(crate) effect_serial: u64,
  pub(crate) effect: Option<LocalEffect>,
  pub(crate) music_generation: u64,
  pub(crate) music_track: usize,
  pub(crate) held: HashSet<PhysicalKey>,
}

#[derive(Clone)]
pub(crate) struct AppControl(Rc<AppControlState>);

struct AppControlState {
  local: DisplayStore<LocalState>,
  replacement: RefCell<Option<ReplacementRequest>>,
  sound_cursor: Cell<usize>,
}

#[derive(Clone, Copy)]
pub(crate) struct ReplacementRequest {
  pub(crate) mode: StartMode,
  pub(crate) cursor_visible: bool,
}

/// App-owned handles and settings consumed by the rendered component tree.
pub(crate) struct ChessModel {
  pub(crate) control: AppControl,
  pub(crate) consumer: RefCell<Option<GameConsumer<ChessGame>>>,
  pub(crate) game: RefCell<Option<GameHandle<ChessGame>>>,
}

impl Default for LocalState {
  fn default() -> Self {
    Self {
      selected: None,
      cursor: crate::cursor::START,
      cursor_visible: false,
      pause_open: false,
      confirm_new_game: false,
      volume: DEFAULT_MUSIC_VOLUME,
      effect_serial: 0,
      effect: None,
      music_generation: 0,
      music_track: 0,
      held: HashSet::new(),
    }
  }
}

impl AppControl {
  fn new() -> Self {
    Self(Rc::new(AppControlState {
      local: DisplayStore::new(LocalState::default()),
      replacement: RefCell::new(None),
      sound_cursor: Cell::new(0),
    }))
  }

  pub(crate) fn store(&self) -> DisplayStore<LocalState> {
    self.0.local.clone()
  }

  pub(crate) fn snapshot(&self) -> LocalState {
    self.0.local.snapshot()
  }

  pub(crate) fn prepare_review(&self, state: crate::visual_state::VisualState) {
    if state == crate::visual_state::VisualState::Paused {
      self.0.local.update(|local| local.pause_open = true);
    }
  }

  pub(crate) fn visual_state(&self, state: &ChessState) -> crate::visual_state::VisualState {
    let local = self.snapshot();
    if local.pause_open {
      crate::visual_state::VisualState::Paused
    } else if local.selected.is_some() {
      crate::visual_state::VisualState::Selected
    } else {
      state.visual_state()
    }
  }

  pub(crate) fn select(&self, state: &ChessState, square: Square) {
    if state.board().side_to_move() != Color::White
      || state.board().status() != GameStatus::Ongoing
      || state.board().color_on(square) != Some(Color::White)
    {
      self.0.local.update(|local| {
        local.cursor = square;
        local.selected = None;
      });
      return;
    }
    let sounds = crate::audio::PICKUP_SOUNDS;
    let index = self.0.sound_cursor.get() % sounds.len();
    self.0.sound_cursor.set(index + 1);
    self.0.local.update(|local| {
      local.cursor = square;
      local.selected = Some(square);
      Self::effect(local, LocalEffect::Sound(sounds[index].clone()));
    });
  }

  pub(crate) fn activate_square(&self, game: &GameHandle<ChessGame>, target: Square) {
    if game.status() != RulesStatus::Ready {
      return;
    }
    let state = game.accepted_state();
    if state.board().color_on(target) == Some(Color::White) {
      self.select(&state, target);
      return;
    }
    let local = self.snapshot();
    let Some(from) = local.selected else {
      self.0.local.update(|value| value.cursor = target);
      return;
    };
    let action = ChessMove { from, to: target };
    if state.legal_move(action).is_none() {
      self.0.local.update(|value| {
        value.cursor = target;
        Self::effect(value, LocalEffect::Invalid);
      });
      return;
    }
    if game.dispatch(ChessAction::Move(action)) == DispatchResult::Started {
      self.0.local.update(|value| {
        value.cursor = target;
        value.selected = None;
      });
    }
  }

  pub(crate) fn drag_start(&self, game: &GameHandle<ChessGame>, piece: ObjectId) {
    if game.status() != RulesStatus::Ready {
      return;
    }
    let state = game.accepted_state();
    if let Some(square) = piece_square(&state, piece)
      && state.board().color_on(square) == Some(Color::White)
    {
      self.select(&state, square);
    }
  }

  pub(crate) fn drag_end(
    &self,
    game: &GameHandle<ChessGame>,
    piece: ObjectId,
    world_position: Vector3,
  ) {
    let state = game.accepted_state();
    let Some(from) = piece_square(&state, piece) else {
      self
        .0
        .local
        .update(|local| Self::effect(local, LocalEffect::Invalid));
      return;
    };
    let target = crate::square_at(world_position);
    if target == from {
      self.select(&state, from);
    } else {
      self.activate_square(game, target);
    }
  }

  pub(crate) fn cancel_selection(&self) {
    self.0.local.update(|local| {
      if let Some(selected) = local.selected.take() {
        local.cursor = selected;
      }
      local.cursor_visible = true;
    });
  }

  pub(crate) fn request_new_game(&self, cursor_visible: bool) {
    let local = self.snapshot();
    if local.confirm_new_game {
      self.0.replacement.replace(Some(ReplacementRequest {
        mode: StartMode::Refresh,
        cursor_visible,
      }));
      return;
    }
    self.0.local.update(|value| {
      value.confirm_new_game = true;
      Self::effect(value, LocalEffect::Invalid);
    });
  }

  pub(crate) fn start(&self, game: &GameHandle<ChessGame>) {
    let _ = game.dispatch(ChessAction::Start(StartMode::Fresh));
  }

  pub(crate) fn request_restart(&self) {
    self.0.replacement.replace(Some(ReplacementRequest {
      mode: StartMode::Restart,
      cursor_visible: true,
    }));
  }

  pub(crate) fn take_replacement(&self) -> Option<ReplacementRequest> {
    self.0.replacement.borrow_mut().take()
  }

  pub(crate) fn reset_for_replacement(&self, cursor_visible: bool) {
    let previous = self.snapshot();
    self.0.local.set(LocalState {
      cursor_visible,
      volume: previous.volume,
      music_generation: previous.music_generation,
      music_track: previous.music_track,
      ..LocalState::default()
    });
  }

  pub(crate) fn next_music(&self) {
    self.0.local.update(|local| {
      local.music_track = (local.music_track + 1) % MUSIC_TRACKS.len();
      local.music_generation = local
        .music_generation
        .checked_add(1)
        .expect("music generation overflow");
    });
  }

  pub(crate) fn start_music(&self) {
    self.0.local.update(|local| {
      if local.music_generation == 0 {
        local.music_generation = 1;
        local.music_track = 0;
      }
    });
  }

  pub(crate) fn restart_music(&self) {
    self.0.local.update(|local| {
      local.music_track = 0;
      local.music_generation = local
        .music_generation
        .checked_add(1)
        .expect("music generation overflow");
    });
  }

  fn effect(local: &mut LocalState, effect: LocalEffect) {
    local.effect_serial = local
      .effect_serial
      .checked_add(1)
      .expect("local effect serial overflow");
    local.effect = Some(effect);
  }
}

impl ChessModel {
  pub(crate) fn new() -> Self {
    Self {
      control: AppControl::new(),
      consumer: RefCell::new(None),
      game: RefCell::new(None),
    }
  }

  pub(crate) fn game(&self) -> GameHandle<ChessGame> {
    self
      .game
      .borrow()
      .as_ref()
      .expect("chess game is initialized")
      .clone()
  }

  fn handle_core(&mut self, body: CoreActionBodyView<'_>) {
    let game = self.game();
    let state = game.accepted_state();
    match body {
      CoreActionBodyView::KeyDown(value) => self.key_down(&game, &state, value.physical_key()),
      CoreActionBodyView::KeyUp(value) => {
        self.control.0.local.update(|local| {
          local.held.remove(&value.physical_key());
        });
      }
      CoreActionBodyView::ControllerButtonDown(value) => {
        self.controller_button(&game, &state, value.controller_button());
      }
      CoreActionBodyView::ControllerNavigate(value) => {
        self.move_cursor(crate::cursor::moved_in_direction(
          self.control.snapshot().cursor,
          value.controller_direction(),
        ));
      }
      CoreActionBodyView::DragStart(value) => {
        let piece = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        self.control.drag_start(&game, piece);
      }
      CoreActionBodyView::DragEnd(value) => {
        let piece = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        let [x, y, z] = value.world_position();
        self.control.drag_end(&game, piece, Vector3::new(x, y, z));
      }
      _ => {}
    }
  }

  fn key_down(&self, game: &GameHandle<ChessGame>, state: &ChessState, key: PhysicalKey) {
    self.control.0.local.update(|local| {
      local.held.insert(key);
    });
    let held = self.control.snapshot().held;
    if held.contains(&PhysicalKey::KeyR)
      && held
        .iter()
        .any(|key| matches!(key, PhysicalKey::ShiftLeft | PhysicalKey::ShiftRight))
      && held.iter().any(|key| {
        matches!(
          key,
          PhysicalKey::ControlLeft
            | PhysicalKey::ControlRight
            | PhysicalKey::MetaLeft
            | PhysicalKey::MetaRight
        )
      })
    {
      self.control.request_restart();
      return;
    }
    match key {
      PhysicalKey::KeyL => self.control.0.local.update(|local| {
        AppControl::effect(local, LocalEffect::ShowDebug(DebugUiSurface::LogViewer));
      }),
      PhysicalKey::ArrowLeft
      | PhysicalKey::ArrowRight
      | PhysicalKey::ArrowUp
      | PhysicalKey::ArrowDown
        if state.started() =>
      {
        self.move_cursor(crate::cursor::moved(self.control.snapshot().cursor, key));
      }
      PhysicalKey::Escape if self.control.snapshot().selected.is_some() => {
        self.control.cancel_selection();
      }
      PhysicalKey::Escape if state.started() => self.toggle_pause(),
      PhysicalKey::Equal => self.adjust_volume(0.1),
      PhysicalKey::Minus => self.adjust_volume(-0.1),
      PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space if !state.started() => {
        let _ = game.dispatch(ChessAction::Start(StartMode::Fresh));
        self
          .control
          .0
          .local
          .update(|local| local.cursor_visible = true);
      }
      PhysicalKey::Enter | PhysicalKey::NumpadEnter | PhysicalKey::Space => {
        self.activate_cursor(game);
      }
      _ => {}
    }
  }

  fn controller_button(
    &self,
    game: &GameHandle<ChessGame>,
    state: &ChessState,
    button: ControllerButton,
  ) {
    let local = self.control.snapshot();
    match button {
      ControllerButton::Start if state.started() => self.toggle_pause(),
      ControllerButton::South if local.pause_open => self.control.request_new_game(true),
      ControllerButton::East if local.pause_open && local.confirm_new_game => {
        self
          .control
          .0
          .local
          .update(|value| value.confirm_new_game = false);
      }
      ControllerButton::East if local.pause_open => self.toggle_pause(),
      ControllerButton::LeftShoulder if local.pause_open => self.adjust_volume(-0.1),
      ControllerButton::RightShoulder if local.pause_open => self.adjust_volume(0.1),
      ControllerButton::LeftShoulder if state.started() => self.cycle_cursor(state, false),
      ControllerButton::RightShoulder if state.started() => self.cycle_cursor(state, true),
      ControllerButton::South if state.started() => self.activate_cursor(game),
      ControllerButton::East if state.started() => self.control.cancel_selection(),
      ControllerButton::South => {
        let _ = game.dispatch(ChessAction::Start(StartMode::Fresh));
        self
          .control
          .0
          .local
          .update(|value| value.cursor_visible = true);
      }
      _ => {}
    }
  }

  fn move_cursor(&self, square: Square) {
    self.control.0.local.update(|local| {
      local.cursor = square;
      local.cursor_visible = true;
    });
  }

  fn cycle_cursor(&self, state: &ChessState, forward: bool) {
    let local = self.control.snapshot();
    let candidates = if let Some(selected) = local.selected {
      crate::legal_destinations(state.board(), selected)
    } else {
      Square::ALL
        .into_iter()
        .filter(|square| {
          state.board().color_on(*square) == Some(Color::White)
            && !crate::legal_destinations(state.board(), *square).is_empty()
        })
        .collect()
    };
    if candidates.is_empty() {
      return;
    }
    let current = candidates.iter().position(|square| *square == local.cursor);
    let index = match (current, forward) {
      (Some(index), true) => (index + 1) % candidates.len(),
      (Some(0), false) | (None, false) => candidates.len() - 1,
      (Some(index), false) => index - 1,
      (None, true) => 0,
    };
    self.move_cursor(candidates[index]);
  }

  fn activate_cursor(&self, game: &GameHandle<ChessGame>) {
    let local = self.control.snapshot();
    if local.selected == Some(local.cursor) {
      self.control.cancel_selection();
    } else {
      self.control.activate_square(game, local.cursor);
    }
  }

  fn toggle_pause(&self) {
    self.control.0.local.update(|local| {
      local.pause_open = !local.pause_open;
      local.confirm_new_game = false;
    });
  }

  fn adjust_volume(&self, delta: f64) {
    self.control.0.local.update(|local| {
      local.volume = (local.volume + delta).clamp(0.0, 1.0);
      AppControl::effect(
        local,
        LocalEffect::Sound(if delta > 0.0 {
          crate::VOLUME_UP_SOUND
        } else {
          crate::VOLUME_DOWN_SOUND
        }),
      );
    });
  }
}

pub(crate) fn configure_app(app: App<ChessModel>) -> App<ChessModel> {
  app
    .global_keys(GLOBAL_KEYS)
    .controller_input(
      ControllerInputSettings::new()
        .buttons(CONTROLLER_BUTTONS)
        .stick_dead_zone(0.35)
        .repeat_timing_ms(275, 125),
    )
    .on_core_action(|model, body| model.handle_core(body))
}

fn piece_square(state: &ChessState, piece: ObjectId) -> Option<Square> {
  Square::ALL
    .into_iter()
    .find(|square| state.piece(*square).is_some_and(|value| value.id == piece))
}
