//! App-local reducer state shared by every chess input source.

use std::{cell::Cell, collections::HashSet, rc::Rc};

use battlement::{AudioClipAddress, DebugUiSurface, ObjectId, PhysicalKey};
use cozy_chess::{Color, GameStatus, Square};
use reactant::{DispatchResult, GameHandle, GameStatus as RulesStatus, hooks};

use crate::reactant_game::{ChessAction, ChessGame, ChessState};

const DEFAULT_MUSIC_VOLUME: f64 = 0.35;

/// Application screen selected independently of rules state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppScreen {
  /// Start screen before a rules session is mounted.
  Title,
  /// Main menu opened over an existing rules session.
  Menu,
  /// Active board backed by a mounted rules session.
  Game,
}

/// Transient application overlays, independent of rules execution and prompts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Overlay {
  /// Paused controls, optionally asking for a second new-game activation.
  Pause {
    /// Whether the next new-game request should replace the session.
    confirm_new_game: bool,
  },
}

/// Presentation intents shared by pointer, keyboard, controller, and drag input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UiAction {
  /// Selects a player piece at a square.
  Select(Square),
  /// Activates a square through click, keyboard, or controller input.
  Activate(Square),
  /// Begins dragging a stable piece identity.
  BeginDrag(ObjectId),
  /// Ends a drag over an optional board square.
  EndDrag(ObjectId, Option<Square>),
  /// Clears the current piece selection.
  CancelSelection,
  /// Moves the shared keyboard/controller cursor.
  MoveCursor(Square),
  /// Cycles through selectable pieces or legal targets.
  CycleCursor(bool),
  /// Opens or closes the pause overlay.
  TogglePause,
  /// Requests a new game, preserving whether non-pointer input needs a cursor.
  RequestNewGame { cursor_visible: bool },
  /// Returns from the confirmation state to the ordinary pause overlay.
  DismissNewGameConfirmation,
  /// Sets normalized music volume.
  SetVolume(f64),
  /// Records a held key before handling chords.
  KeyDown(PhysicalKey),
  /// Releases a held key.
  KeyUp(PhysicalKey),
  /// Requests one host debug surface.
  ShowDebug(DebugUiSurface),
}

/// App-owned reason for creating or replacing a chess session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStart {
  /// First session created from the title screen.
  Fresh,
  /// Replacement requested through the keyboard shortcut.
  Restart,
  /// Replacement confirmed through the pause UI.
  Refresh,
}

/// One app-local effect delivered after input without entering rules state.
#[derive(Clone, Debug, PartialEq)]
pub enum LocalEffect {
  /// Plays one sound without changing logical rules state.
  Sound(AudioClipAddress),
  /// Plays invalid-action feedback and vibrates the controller.
  Invalid,
  /// Opens a host-provided debug surface.
  ShowDebug(DebugUiSurface),
}

/// Selection, navigation, overlays, settings, and one-shot app effects.
#[derive(Clone, Debug, PartialEq)]
pub struct ChessUiState {
  /// Top-level screen independent of rules worker state.
  pub screen: AppScreen,
  /// Last durable semantic presentation state.
  pub visual_state: crate::visual_state::VisualState,
  /// Whether this session began from persisted board data.
  pub origin_saved: bool,
  /// Whether opening animation currently suppresses interaction.
  pub spawning: bool,
  /// Identity used to remount sessions and ignore stale animation completions.
  pub opening_generation: u64,
  /// Opening animation requested for the current generation.
  pub opening: Option<SessionStart>,
  /// Source square selected for the next move.
  pub selected: Option<Square>,
  /// Piece and square to restore after a rejected drag.
  pub drag_restore: Option<(ObjectId, Square)>,
  /// Dependency token that retriggers repeated restores to the same square.
  pub drag_restore_generation: u64,
  /// Shared keyboard and controller cursor square.
  pub cursor: Square,
  /// Whether the cursor is visible without an active selection.
  pub cursor_visible: bool,
  /// Current app-owned overlay.
  pub overlay: Option<Overlay>,
  /// Normalized music volume.
  pub volume: f64,
  /// Dependency token that delivers repeated equal one-shot effects.
  pub effect_serial: u64,
  /// Most recently requested app-local effect.
  pub effect: Option<LocalEffect>,
  /// Playback generation incremented to restart or change music.
  pub music_generation: u64,
  /// Index into the app-level music playlist.
  pub music_track: usize,
  /// Physical keys currently held for chord recognition.
  pub held: HashSet<PhysicalKey>,
}

/// Reducer-style interface shared by components and every input source.
///
/// The render snapshot is convenient for declarative output, while `current`
/// gives event handlers the latest committed value even when their closure was
/// created by an earlier render.
#[derive(Clone)]
pub struct ChessUiController {
  local: ChessUiState,
  current: reactant::hooks::Ref<ChessUiState>,
  dispatch: reactant::hooks::ReducerDispatch<ChessUiState>,
  sound_cursor: Rc<Cell<usize>>,
}

/// Creates app-local reducer state and a controller safe for event closures.
///
/// Reactant's reducer snapshot updates on render. The companion ref is updated
/// both eagerly by events and after commit, so several events arriving before a
/// rerender still reduce from the latest value rather than a captured snapshot.
pub fn use_chess_ui(initial: ChessUiState) -> ChessUiController {
  let (local, dispatch) = hooks::use_reducer(|_, next| next, initial);
  let current = hooks::use_ref(local.clone());
  let committed = current.clone();
  let next = local.clone();
  hooks::use_commit_effect(
    move || {
      committed.replace(next);
    },
    local.clone(),
  );
  let sound_cursor = hooks::use_memo(|| Rc::new(Cell::new(0)), ());
  ChessUiController {
    local,
    current,
    dispatch,
    sound_cursor,
  }
}

impl Default for ChessUiState {
  /// Starts on the title screen with no transient interaction or host effects.
  fn default() -> Self {
    Self {
      screen: AppScreen::Title,
      visual_state: crate::visual_state::VisualState::Title,
      origin_saved: false,
      spawning: false,
      opening_generation: 0,
      opening: None,
      selected: None,
      drag_restore: None,
      drag_restore_generation: 0,
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
  /// Returns whether the pause overlay currently owns interaction.
  pub const fn pause_open(&self) -> bool {
    matches!(self.overlay, Some(Overlay::Pause { .. }))
  }

  /// Returns whether a second new-game request should replace the session.
  pub const fn confirm_new_game(&self) -> bool {
    matches!(
      self.overlay,
      Some(Overlay::Pause {
        confirm_new_game: true
      })
    )
  }
}

impl ChessUiController {
  /// Opens the saved board selected from the startup menu.
  pub fn resume_saved(&self) {
    self.update(|local| {
      local.screen = AppScreen::Game;
      local.visual_state = crate::visual_state::VisualState::Resumed;
      local.origin_saved = true;
      local.overlay = None;
    });
  }

  /// Shows the main menu and releases the active board session.
  pub fn show_menu(&self) {
    self.update(|local| {
      local.screen = AppScreen::Menu;
      local.overlay = None;
      local.selected = None;
    });
  }

  /// Returns the render-frame snapshot used for declarative composition.
  pub fn snapshot(&self) -> ChessUiState {
    self.local.clone()
  }

  /// Returns the latest value for callbacks that may outlive their render frame.
  pub fn current(&self) -> ChessUiState {
    self.current.get()
  }

  /// Applies one local-state transaction to both the live ref and reducer.
  fn update(&self, update: impl FnOnce(&mut ChessUiState)) {
    let mut local = self.current();
    update(&mut local);
    self.current.replace(local.clone());
    self.dispatch.send(local);
  }

  /// Routes normalized presentation intent through one state transition boundary.
  ///
  /// Pointer, drag, keyboard, controller, and semantic controls all use this
  /// method. Centralization keeps affordance behavior consistent and leaves the
  /// rules worker responsible only for authoritative chess actions.
  pub fn dispatch(&self, game: Option<&GameHandle<ChessGame>>, action: UiAction) {
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
      UiAction::DismissNewGameConfirmation => self.update(|local| {
        local.overlay = Some(Overlay::Pause {
          confirm_new_game: false,
        });
      }),
      UiAction::SetVolume(volume) => self.set_volume(volume),
      UiAction::KeyDown(key) => {
        self.update(|local| {
          local.held.insert(key);
        });
      }
      UiAction::KeyUp(key) => {
        self.update(|local| {
          local.held.remove(&key);
        });
      }
      UiAction::ShowDebug(surface) => {
        self.update(|local| Self::effect(local, LocalEffect::ShowDebug(surface)))
      }
    }
  }

  /// Replaces app-local state for a newly mounted rules session.
  ///
  /// User settings survive the reset, while transient selection and effects do
  /// not. Incrementing the generation keys both the rules session and callbacks.
  pub fn begin_session(&self, mode: SessionStart, cursor_visible: bool, origin_saved: bool) {
    let previous = self.current();
    let opening_generation = previous
      .opening_generation
      .checked_add(1)
      .expect("opening generation overflow");
    let next = ChessUiState {
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
    };
    self.current.replace(next.clone());
    self.dispatch.send(next);
  }

  /// Starts a replacement session requested by the global restart chord.
  pub fn request_restart(&self) {
    self.begin_session(SessionStart::Restart, true, false);
  }

  /// Starts the first session, choosing cursor visibility by input modality.
  pub fn request_start(&self, cursor_visible: bool) {
    self.begin_session(SessionStart::Fresh, cursor_visible, false);
  }

  /// Enables interaction after the matching opening animation completes.
  ///
  /// Comparing generations prevents an old playback callback from unlocking a
  /// replacement session that has already mounted.
  pub fn finish_opening(&self, generation: u64) {
    self.update(|local| {
      if local.opening_generation == generation {
        local.spawning = false;
      }
    });
  }

  /// Advances the playlist and increments the playback dependency token.
  pub fn next_music(&self) {
    self.update(|local| {
      local.music_track = (local.music_track + 1) % crate::reactant_effects::music_track_count();
      local.music_generation = local
        .music_generation
        .checked_add(1)
        .expect("music generation overflow");
    });
  }

  /// Starts the first track once without restarting it on later renders.
  pub fn start_music(&self) {
    self.update(|local| {
      if local.music_generation == 0 {
        local.music_generation = 1;
        local.music_track = 0;
      }
    });
  }

  /// Returns to the first track with a new playback generation.
  pub fn restart_music(&self) {
    self.update(|local| {
      local.music_track = 0;
      local.music_generation = local
        .music_generation
        .checked_add(1)
        .expect("music generation overflow");
    });
  }

  /// Selects a movable player piece and queues presentation-only pickup feedback.
  fn select(&self, state: &ChessState, square: Square) {
    if state.board().side_to_move() != Color::White
      || state.board().status() != GameStatus::Ongoing
      || state.board().color_on(square) != Some(Color::White)
    {
      self.update(|local| {
        local.cursor = square;
        local.selected = None;
      });
      return;
    }
    let sounds = crate::audio::PICKUP_SOUNDS;
    let index = self.sound_cursor.get() % sounds.len();
    self.sound_cursor.set(index + 1);
    self.update(|local| {
      local.cursor = square;
      local.selected = Some(square);
      Self::effect(local, LocalEffect::Sound(sounds[index].clone()));
    });
  }

  /// Interprets a square activation against the latest accepted rules snapshot.
  fn activate_square(&self, game: &GameHandle<ChessGame>, target: Square) {
    if game.status() != RulesStatus::Ready || self.current().spawning {
      return;
    }
    let state = game.accepted_state();
    if state.board().color_on(target) == Some(Color::White) {
      self.dispatch(Some(game), UiAction::Select(target));
      return;
    }
    let local = self.current();
    let Some(from) = local.selected else {
      self.update(|value| value.cursor = target);
      return;
    };
    if state.legal_moves(from, target).is_empty() {
      self.update(|value| {
        value.cursor = target;
        Self::effect(value, LocalEffect::Invalid);
      });
      return;
    }
    if game.dispatch(ChessAction::MoveTo { from, to: target }) == DispatchResult::Started {
      self.update(|value| {
        value.cursor = target;
        value.selected = None;
      });
    }
  }

  /// Converts a draggable host identity back into the ordinary selection flow.
  fn drag_start(&self, game: &GameHandle<ChessGame>, piece: ObjectId) {
    if game.status() != RulesStatus::Ready || self.current().spawning {
      return;
    }
    let state = game.accepted_state();
    if let Some(square) = piece_square(&state, piece)
      && state.board().color_on(square) == Some(Color::White)
    {
      self.dispatch(Some(game), UiAction::Select(square));
    }
  }

  /// Commits a legal drag target or schedules a visual snap-back.
  fn drag_end(&self, game: &GameHandle<ChessGame>, piece: ObjectId, target: Option<Square>) {
    let state = game.accepted_state();
    let Some(from) = piece_square(&state, piece) else {
      self.update(|local| Self::effect(local, LocalEffect::Invalid));
      return;
    };
    let ready = game.status() == RulesStatus::Ready && !self.current().pause_open();
    let Some(target) = target.filter(|_| ready) else {
      self.restore_drag(piece, from, false);
      return;
    };
    if target == from {
      self.select(&state, from);
    } else if state.legal_moves(from, target).is_empty() {
      self.restore_drag(piece, from, true);
    } else {
      self.activate_square(game, target);
    }
  }

  /// Requests an imperative Motion restore while keeping the logical board unchanged.
  fn restore_drag(&self, entity: ObjectId, square: Square, invalid: bool) {
    self.update(|local| {
      local.drag_restore = Some((entity, square));
      local.drag_restore_generation = local
        .drag_restore_generation
        .checked_add(1)
        .expect("drag restore generation overflow");
      local.selected = None;
      local.cursor = square;
      local.cursor_visible = true;
      if invalid {
        Self::effect(local, LocalEffect::Invalid);
      }
    });
  }

  /// Clears selection and leaves the cursor at the previously selected piece.
  fn cancel_selection(&self) {
    self.update(|local| {
      if let Some(selected) = local.selected.take() {
        local.cursor = selected;
      }
      local.cursor_visible = true;
    });
  }

  /// Moves and reveals the non-pointer cursor.
  fn move_cursor(&self, square: Square) {
    self.update(|local| {
      local.cursor = square;
      local.cursor_visible = true;
    });
  }

  /// Cycles through legal targets or through player pieces that can move.
  ///
  /// Candidate derivation from rules state keeps controller navigation useful in
  /// sparse endgames without embedding a second focus graph in the view.
  fn cycle_cursor(&self, state: &ChessState, forward: bool) {
    let local = self.current();
    let candidates = if let Some(selected) = local.selected {
      state.legal_destinations(selected)
    } else {
      Square::ALL
        .into_iter()
        .filter(|square| {
          state.board().color_on(*square) == Some(Color::White)
            && !state.legal_destinations(*square).is_empty()
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

  /// Toggles the app-owned pause overlay without pausing or mutating rules state.
  fn toggle_pause(&self) {
    self.update(|local| {
      local.overlay = if local.pause_open() {
        None
      } else {
        Some(Overlay::Pause {
          confirm_new_game: false,
        })
      };
    });
  }

  /// Implements the two-step destructive new-game confirmation flow.
  fn request_new_game(&self, cursor_visible: bool) {
    if self.current().confirm_new_game() {
      self.begin_session(SessionStart::Refresh, cursor_visible, false);
      return;
    }
    self.update(|value| {
      value.overlay = Some(Overlay::Pause {
        confirm_new_game: true,
      });
      Self::effect(value, LocalEffect::Invalid);
    });
  }

  /// Stores clamped volume and emits directional audible feedback.
  fn set_volume(&self, volume: f64) {
    self.update(|local| {
      let increased = volume > local.volume;
      local.volume = volume.clamp(0.0, 1.0);
      Self::effect(
        local,
        LocalEffect::Sound(if increased {
          crate::audio::VOLUME_UP_SOUND
        } else {
          crate::audio::VOLUME_DOWN_SOUND
        }),
      );
    });
  }

  /// Records a one-shot effect and changes its dependency token even when equal.
  fn effect(local: &mut ChessUiState, effect: LocalEffect) {
    local.effect_serial = local
      .effect_serial
      .checked_add(1)
      .expect("local effect serial overflow");
    local.effect = Some(effect);
  }
}

/// Finds the current square for a stable presentation identity.
fn piece_square(state: &ChessState, piece: ObjectId) -> Option<Square> {
  Square::ALL.into_iter().find(|square| {
    state
      .piece(*square)
      .is_some_and(|value| value.identity.object_id == piece)
  })
}

/// Makes misuse of game-only actions fail at the central dispatch boundary.
fn required_game(game: Option<&GameHandle<ChessGame>>) -> &GameHandle<ChessGame> {
  game.expect("game action requires an active chess session")
}
