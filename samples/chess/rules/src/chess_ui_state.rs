//! App-local reducer state shared by every chess input source.

use std::{
  cell::{Cell, RefCell},
  collections::HashSet,
  rc::Rc,
};

use battlement::{DebugUiSurface, ObjectId, PhysicalKey};
use cozy_chess::{Color, GameStatus, Square};
use reactant::{
  DispatchResult, GameHandle, GameStatus as RulesStatus,
  prelude::{AnimationPlayback, DisplayStore, ExternalStore},
};

use crate::{
  MUSIC_TRACKS,
  reactant_game::{ChessAction, ChessGame, ChessState},
};

const DEFAULT_MUSIC_VOLUME: f64 = 0.35;

/// Application screen selected independently of rules state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppScreen {
  Title,
  Game,
}

/// Transient application overlays, independent of rules execution and prompts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Overlay {
  Pause { confirm_new_game: bool },
}

/// Presentation intents shared by pointer, keyboard, controller, and drag input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum UiAction {
  Select(Square),
  Activate(Square),
  BeginDrag(ObjectId),
  EndDrag(ObjectId, Square),
  CancelSelection,
  MoveCursor(Square),
  CycleCursor(bool),
  TogglePause,
  RequestNewGame { cursor_visible: bool },
  DismissNewGameConfirmation,
  SetVolume(f64),
  KeyDown(PhysicalKey),
  KeyUp(PhysicalKey),
  ShowDebug(DebugUiSurface),
}

/// App-owned reason for creating or replacing a chess session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionStart {
  Fresh,
  Restart,
  Refresh,
}

/// One app-local effect delivered after input without entering rules state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LocalEffect {
  Sound(battlement::AudioClipAddress),
  Invalid,
  ShowDebug(DebugUiSurface),
}

/// Selection, navigation, overlays, settings, and one-shot app effects.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChessUiState {
  pub(crate) screen: AppScreen,
  pub(crate) visual_state: crate::visual_state::VisualState,
  pub(crate) origin_saved: bool,
  pub(crate) spawning: bool,
  pub(crate) opening_generation: u64,
  pub(crate) opening: Option<SessionStart>,
  pub(crate) selected: Option<Square>,
  pub(crate) cursor: Square,
  pub(crate) cursor_visible: bool,
  pub(crate) overlay: Option<Overlay>,
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
  local: DisplayStore<ChessUiState>,
  replacement: RefCell<Option<ReplacementRequest>>,
  opening_playbacks: RefCell<Vec<AnimationPlayback>>,
  sound_cursor: Cell<usize>,
}

#[derive(Clone, Copy)]
pub(crate) struct ReplacementRequest {
  pub(crate) mode: SessionStart,
  pub(crate) cursor_visible: bool,
}

impl Default for ChessUiState {
  fn default() -> Self {
    Self {
      screen: AppScreen::Title,
      visual_state: crate::visual_state::VisualState::Title,
      origin_saved: false,
      spawning: false,
      opening_generation: 0,
      opening: None,
      selected: None,
      cursor: crate::cursor::START,
      cursor_visible: false,
      overlay: None,
      volume: DEFAULT_MUSIC_VOLUME,
      effect_serial: 0,
      effect: None,
      music_generation: 0,
      music_track: 0,
      held: HashSet::new(),
    }
  }
}

impl ChessUiState {
  pub(crate) const fn resolved_visual_state(&self) -> crate::visual_state::VisualState {
    if self.pause_open() {
      crate::visual_state::VisualState::Paused
    } else if self.selected.is_some() {
      crate::visual_state::VisualState::Selected
    } else {
      self.visual_state
    }
  }

  pub(crate) const fn pause_open(&self) -> bool {
    matches!(self.overlay, Some(Overlay::Pause { .. }))
  }

  pub(crate) const fn confirm_new_game(&self) -> bool {
    matches!(
      self.overlay,
      Some(Overlay::Pause {
        confirm_new_game: true
      })
    )
  }
}

impl AppControl {
  pub(crate) fn new(local: ChessUiState) -> Self {
    Self(Rc::new(AppControlState {
      local: DisplayStore::new(local),
      replacement: RefCell::new(None),
      opening_playbacks: RefCell::new(Vec::new()),
      sound_cursor: Cell::new(0),
    }))
  }

  pub(crate) fn store(&self) -> DisplayStore<ChessUiState> {
    self.0.local.clone()
  }

  pub(crate) fn snapshot(&self) -> ChessUiState {
    self.0.local.snapshot()
  }

  pub(crate) fn prepare_review(&self, state: crate::visual_state::VisualState) {
    if state == crate::visual_state::VisualState::Paused {
      self.0.local.update(|local| {
        local.screen = AppScreen::Game;
        local.overlay = Some(Overlay::Pause {
          confirm_new_game: false,
        });
      });
    }
  }

  pub(crate) fn visual_state(&self) -> crate::visual_state::VisualState {
    self.snapshot().resolved_visual_state()
  }

  pub(crate) fn dispatch(&self, game: Option<&GameHandle<ChessGame>>, action: UiAction) {
    match action {
      UiAction::Select(square) => self.select(&required_game(game).accepted_state(), square),
      UiAction::Activate(square) => self.activate_square(required_game(game), square),
      UiAction::BeginDrag(piece) => self.drag_start(required_game(game), piece),
      UiAction::EndDrag(piece, target) => self.drag_end(required_game(game), piece, target),
      UiAction::CancelSelection => self.cancel_selection(),
      UiAction::MoveCursor(square) => self.move_cursor(square),
      UiAction::CycleCursor(forward) => {
        self.cycle_cursor(&required_game(game).accepted_state(), forward);
      }
      UiAction::TogglePause => self.toggle_pause(),
      UiAction::RequestNewGame { cursor_visible } => self.request_new_game(cursor_visible),
      UiAction::DismissNewGameConfirmation => self.0.local.update(|local| {
        local.overlay = Some(Overlay::Pause {
          confirm_new_game: false,
        });
      }),
      UiAction::SetVolume(volume) => self.set_volume(volume),
      UiAction::KeyDown(key) => {
        self.0.local.update(|local| {
          local.held.insert(key);
        });
      }
      UiAction::KeyUp(key) => {
        self.0.local.update(|local| {
          local.held.remove(&key);
        });
      }
      UiAction::ShowDebug(surface) => self
        .0
        .local
        .update(|local| Self::effect(local, LocalEffect::ShowDebug(surface))),
    }
  }

  pub(crate) fn request_restart(&self) {
    self.0.replacement.replace(Some(ReplacementRequest {
      mode: SessionStart::Restart,
      cursor_visible: true,
    }));
  }

  pub(crate) fn request_start(&self, cursor_visible: bool) {
    self.0.replacement.replace(Some(ReplacementRequest {
      mode: SessionStart::Fresh,
      cursor_visible,
    }));
  }

  pub(crate) fn take_replacement(&self) -> Option<ReplacementRequest> {
    self.0.replacement.borrow_mut().take()
  }

  pub(crate) fn begin_session(&self, mode: SessionStart, cursor_visible: bool, origin_saved: bool) {
    let previous = self.snapshot();
    let opening_generation = previous
      .opening_generation
      .checked_add(1)
      .expect("opening generation overflow");
    self.0.local.set(ChessUiState {
      screen: AppScreen::Game,
      visual_state: match mode {
        SessionStart::Fresh => crate::visual_state::VisualState::Initial,
        SessionStart::Restart => crate::visual_state::VisualState::Restarted,
        SessionStart::Refresh => crate::visual_state::VisualState::Refreshed,
      },
      origin_saved,
      spawning: mode != SessionStart::Refresh,
      opening_generation,
      opening: Some(mode),
      cursor_visible,
      volume: previous.volume,
      music_generation: previous.music_generation,
      music_track: previous.music_track,
      ..ChessUiState::default()
    });
  }

  pub(crate) fn finish_opening(&self, generation: u64) {
    self.0.local.update(|local| {
      if local.opening_generation == generation {
        local.spawning = false;
      }
    });
  }

  pub(crate) fn retain_opening(&self, playback: AnimationPlayback) {
    self.0.opening_playbacks.borrow_mut().push(playback);
  }

  pub(crate) fn cancel_opening(&self) {
    self.0.local.update(|local| {
      local.opening_generation = local
        .opening_generation
        .checked_add(1)
        .expect("opening generation overflow");
      local.opening = None;
      local.spawning = false;
    });
  }

  pub(crate) fn observe_visual_state(&self, state: crate::visual_state::VisualState) {
    self.0.local.update(|local| local.visual_state = state);
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

  fn select(&self, state: &ChessState, square: Square) {
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

  fn activate_square(&self, game: &GameHandle<ChessGame>, target: Square) {
    if game.status() != RulesStatus::Ready {
      return;
    }
    let state = game.accepted_state();
    if state.board().color_on(target) == Some(Color::White) {
      self.dispatch(Some(game), UiAction::Select(target));
      return;
    }
    let local = self.snapshot();
    let Some(from) = local.selected else {
      self.0.local.update(|value| value.cursor = target);
      return;
    };
    if state.legal_moves(from, target).is_empty() {
      self.0.local.update(|value| {
        value.cursor = target;
        Self::effect(value, LocalEffect::Invalid);
      });
      return;
    }
    if game.dispatch(ChessAction::MoveTo { from, to: target }) == DispatchResult::Started {
      self.0.local.update(|value| {
        value.cursor = target;
        value.selected = None;
      });
    }
  }

  fn drag_start(&self, game: &GameHandle<ChessGame>, piece: ObjectId) {
    if game.status() != RulesStatus::Ready {
      return;
    }
    let state = game.accepted_state();
    if let Some(square) = piece_square(&state, piece)
      && state.board().color_on(square) == Some(Color::White)
    {
      self.dispatch(Some(game), UiAction::Select(square));
    }
  }

  fn drag_end(&self, game: &GameHandle<ChessGame>, piece: ObjectId, target: Square) {
    let state = game.accepted_state();
    let Some(from) = piece_square(&state, piece) else {
      self
        .0
        .local
        .update(|local| Self::effect(local, LocalEffect::Invalid));
      return;
    };
    if target == from {
      self.select(&state, from);
    } else {
      self.activate_square(game, target);
    }
  }

  fn cancel_selection(&self) {
    self.0.local.update(|local| {
      if let Some(selected) = local.selected.take() {
        local.cursor = selected;
      }
      local.cursor_visible = true;
    });
  }

  fn move_cursor(&self, square: Square) {
    self.0.local.update(|local| {
      local.cursor = square;
      local.cursor_visible = true;
    });
  }

  fn cycle_cursor(&self, state: &ChessState, forward: bool) {
    let local = self.snapshot();
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

  fn toggle_pause(&self) {
    self.0.local.update(|local| {
      local.overlay = if local.pause_open() {
        None
      } else {
        Some(Overlay::Pause {
          confirm_new_game: false,
        })
      };
    });
  }

  fn request_new_game(&self, cursor_visible: bool) {
    if self.snapshot().confirm_new_game() {
      self.0.replacement.replace(Some(ReplacementRequest {
        mode: SessionStart::Refresh,
        cursor_visible,
      }));
      return;
    }
    self.0.local.update(|value| {
      value.overlay = Some(Overlay::Pause {
        confirm_new_game: true,
      });
      Self::effect(value, LocalEffect::Invalid);
    });
  }

  fn set_volume(&self, volume: f64) {
    self.0.local.update(|local| {
      let increased = volume > local.volume;
      local.volume = volume.clamp(0.0, 1.0);
      Self::effect(
        local,
        LocalEffect::Sound(if increased {
          crate::VOLUME_UP_SOUND
        } else {
          crate::VOLUME_DOWN_SOUND
        }),
      );
    });
  }

  fn effect(local: &mut ChessUiState, effect: LocalEffect) {
    local.effect_serial = local
      .effect_serial
      .checked_add(1)
      .expect("local effect serial overflow");
    local.effect = Some(effect);
  }
}

fn piece_square(state: &ChessState, piece: ObjectId) -> Option<Square> {
  Square::ALL.into_iter().find(|square| {
    state
      .piece(*square)
      .is_some_and(|value| value.entity_id == piece)
  })
}

fn required_game(game: Option<&GameHandle<ChessGame>>) -> &GameHandle<ChessGame> {
  game.expect("game action requires an active chess session")
}
