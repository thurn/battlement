//! React-shaped application and screen composition for the chess sample.

use std::{rc::Rc, time::Duration};

use battlement::{Color as UiColor, Position, Prop, Quaternion, Style, Vector3};
use cozy_chess::{Board, Color, GameStatus};
use reactant::{
  Application, DispatchResult, GameHandle, GameRoot, GameStatus as RulesStatus, PersistentState,
  hooks,
  prelude::{AnimationPlayback, Button, Component, Either, EventCallback, KeyRenderExt, Render},
  rules::{DisplayConnection, ExecutionMode},
  world::Camera,
};
use trox::ls;

use crate::{
  chess_board::ChessBoard,
  chess_ui_state::{
    AppScreen, ChessUiController, ChessUiState, SessionStart, UiAction, use_chess_ui,
  },
  menu::ChessMenu,
  persistence::SavedGame,
  promotion_dialog::PromotionDialog,
  reactant_game::{ChessAction, ChessContext, ChessGame, ChessPolicy, ChessState},
  settings::SettingsRoot,
};
use crate::{
  opponent::Opponent,
  reactant_effects::{self, GameEffects},
  reactant_input,
  visual_state::VisualState,
};
use battlement::PickingMode;
use reactant::PersistenceBackend;

pub(super) const CAMERA_ROTATION: Quaternion =
  Quaternion::new(0.58184814, -0.001219943, 0.0008727778, 0.813296);

#[derive(Clone)]
/// Typed inputs used to assemble every chess application variant.
///
/// Configuration is plain data so exported constructors can share one component
/// tree. This is preferable to branching on environment or scenario names deep
/// inside Reactant components.
pub struct ChessConfig {
  /// Board used when a new rules session has no supplied or restored state.
  pub starting_board: Board,
  /// Optional state that mounts directly into an active session.
  pub initial_state: Option<ChessState>,
  /// Initial presentation mode.
  pub visual_state: VisualState,
  /// Whether the configured session should be marked as restored.
  pub origin_saved: bool,
  /// Synchronous move selector and turn admission policy.
  pub opponent: Opponent,
  /// Optional seed for presentation-only sound selection.
  pub seed: Option<u64>,
  /// Raw storage used for saved progress and local preferences.
  pub persistence: Option<Rc<dyn PersistenceBackend>>,
}

/// Assembles the Reactant application, document, camera, and global input policy.
///
/// The application owns host setup once; components below it remain declarative
/// and can focus on state, events, and effects.
pub fn application(config: ChessConfig) -> Application {
  let app = Application::new(crate::assets::CONTENT)
    .child(SettingsRoot {
      backend: config.persistence.clone(),
      children: ChessAssembly { config }.into(),
    })
    .document(|mut document| {
      document.root_id = crate::visual_state::ROOT_ID;
      document.element.picking_mode = Prop::Set(PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      Camera::new()
        .perspective(60.0)
        .position(Vector3::new(0.0, 8.0, -3.75))
        .rotation(CAMERA_ROTATION)
        .into_object(camera.object_id)
    });
  reactant_input::configure_application(app)
}

/// Root component that chooses the title or active-session subtree.
struct ChessAssembly {
  config: ChessConfig,
}

struct PersistentChessApp {
  config: ChessConfig,
  backend: Rc<dyn PersistenceBackend>,
}

struct ChessApp {
  config: ChessConfig,
  persistence: Option<PersistentState<SavedGame>>,
}

impl Component for ChessAssembly {
  fn render(&self) -> impl Render {
    match &self.config.persistence {
      Some(backend) => Either::left(PersistentChessApp {
        config: self.config.clone(),
        backend: backend.clone(),
      }),
      None => Either::right(ChessApp {
        config: self.config.clone(),
        persistence: None,
      }),
    }
  }
}

impl Component for PersistentChessApp {
  fn render(&self) -> impl Render {
    let persistence = reactant::use_persistent_state_with::<SavedGame>(
      crate::persistence::SAVE_FILE_NAME,
      self.backend.clone(),
    );
    if !persistence.hydrated() && persistence.error().is_none() {
      return Either::left(());
    }
    Either::right(ChessApp {
      config: self.config.clone(),
      persistence: Some(persistence),
    })
  }
}

/// Menu presentation and its input-independent Play callback.
struct TitleScreen {
  control: ChessUiController,
  diagnostics: bool,
}

/// Active game composition around an already-mounted rules handle.
struct ChessScreen {
  opponent: Opponent,
  game: GameHandle<ChessGame>,
  control: ChessUiController,
  diagnostics: bool,
  persistence: Option<PersistentState<SavedGame>>,
}

/// Semantic and diagnostic projection of the current game result.
struct GameStatusView {
  origin_saved: bool,
  diagnostics: bool,
}

/// Effect-only component that schedules the computer action when appropriate.
struct TurnCoordinator {
  opponent: Opponent,
  game: GameHandle<ChessGame>,
}

/// Effect-only component that persists the latest accepted publication.
struct PersistenceCoordinator {
  persistence: PersistentState<SavedGame>,
}

impl Component for ChessApp {
  /// Restores persistent state once, creates app-local state, and selects a screen.
  ///
  /// Hooks are evaluated unconditionally before screen composition. Conditional
  /// behavior is expressed by rendering keyed child components, which preserves
  /// Reactant's hook ordering and gives each rules-session generation a lifecycle.
  fn render(&self) -> impl Render {
    let persistence = self.persistence.clone();
    let diagnostics = reactant::use_host_module("battlement.diagnostics");
    // Persistence is an input to initial assembly, not an ongoing competing
    // source of truth. Once mounted, the rules session owns the logical state.
    let initial_state = hooks::use_memo(
      {
        let configured = self.config.initial_state.clone();
        let saved = persistence.as_ref().and_then(|p| p.value().cloned());
        move || {
          saved
            .and_then(|saved| saved.board())
            .map(ChessState::resumed)
            .or(configured)
        }
      },
      (),
    );
    let initial_local = hooks::use_memo(
      {
        let state = initial_state.clone();
        let restored = persistence
          .as_ref()
          .and_then(|p| p.value())
          .and_then(SavedGame::board)
          .is_some();
        let visual_state = if restored {
          VisualState::Resumed
        } else {
          self.config.visual_state
        };
        let origin_saved = self.config.origin_saved || restored;
        move || {
          let mut local = ChessUiState::default();
          if state.is_some() && !restored {
            local.screen = AppScreen::Game;
            local.visual_state = visual_state;
            local.origin_saved = origin_saved;
            if visual_state == VisualState::Paused {
              local.overlay = Some(crate::chess_ui_state::Overlay::Pause {
                confirm_new_game: false,
              });
            }
          }
          local
        }
      },
      (),
    );
    let control = use_chess_ui(initial_local);
    let local = control.snapshot();
    let restored = persistence
      .as_ref()
      .and_then(|p| p.value())
      .and_then(SavedGame::board)
      .is_some();
    let play = control.clone();
    let on_play = EventCallback::new(move |()| {
      if restored && play.current().screen == AppScreen::Title {
        play.resume_saved();
      } else {
        play.request_start(false);
      }
    });
    let screen = match local.screen {
      AppScreen::Title => Either::left(TitleScreen {
        control: control.clone(),
        diagnostics,
      }),
      AppScreen::Game | AppScreen::Menu => {
        let state = if local.opening_generation == 0 {
          initial_state
            .clone()
            .unwrap_or_else(|| ChessState::new(self.config.starting_board.clone()))
        } else {
          ChessState::with_generation(self.config.starting_board.clone(), local.opening_generation)
        };
        // Keying by generation intentionally remounts hooks, worker state, and
        // piece identities when the user starts a replacement session.
        Either::right(
          ChessSession {
            state,
            generation: local.opening_generation,
            mode: local.opening,
            opponent: self.config.opponent.clone(),
            seed: self.config.seed,
            control: control.clone(),
            diagnostics,
            persistence: persistence.clone(),
          }
          .key(local.opening_generation),
        )
      }
    };
    (
      GameEffects::new(control, diagnostics),
      screen,
      ChessMenu {
        active: local.screen != AppScreen::Game,
        on_play,
      },
    )
  }
}

/// Keyed owner of one rules worker and its session-scoped effects.
struct ChessSession {
  state: ChessState,
  generation: u64,
  mode: Option<SessionStart>,
  opponent: Opponent,
  seed: Option<u64>,
  control: ChessUiController,
  diagnostics: bool,
  persistence: Option<PersistentState<SavedGame>>,
}

impl Component for ChessSession {
  /// Mounts the typed game, installs input, and declares session-scoped effects.
  fn render(&self) -> impl Render {
    let opponent = self.opponent.clone();
    let seed = self.seed;
    let game = reactant::use_game::<ChessGame, _>(
      self.generation,
      self.state.clone(),
      move |connection: DisplayConnection<ChessGame>| {
        ChessContext::new(
          ExecutionMode::Interactive {
            connection,
            policy: ChessPolicy,
          },
          opponent.clone(),
          seed.map_or_else(fastrand::Rng::new, fastrand::Rng::with_seed),
        )
      },
    );
    reactant_input::use_chess_input(
      self.control.clone(),
      (self.control.snapshot().screen == AppScreen::Game).then(|| game.clone()),
    );
    if matches!(
      self.mode,
      Some(SessionStart::Restart | SessionStart::Refresh)
    ) {
      let persistence = self.persistence.clone();
      hooks::use_effect(
        move || {
          if let Some(p) = persistence {
            p.clear();
          }
        },
        (),
      );
    }
    let opponent = self.opponent.clone();
    hooks::use_effect(move || opponent.reset(), ());
    let music = self.control.clone();
    let restart_music = matches!(self.mode, Some(SessionStart::Restart));
    hooks::use_effect(
      move || {
        if restart_music {
          music.restart_music();
        } else {
          music.start_music();
        }
      },
      (),
    );
    let music = self.control.clone();
    reactant::use_interval(Duration::from_secs(120), move || music.next_music());
    GameRoot::new(ChessScreen {
      game,
      control: self.control.clone(),
      diagnostics: self.diagnostics,
      opponent: self.opponent.clone(),
      persistence: self.persistence.clone(),
    })
  }
}

impl Component for TitleScreen {
  /// The menu owns both pointer and assistive activation.
  fn render(&self) -> impl Render {
    reactant_input::use_chess_input(self.control.clone(), None);
    (self
      .diagnostics
      .then(|| reactant_effects::diagnostics_view("ongoing", "new")),)
  }
}

impl Component for ChessScreen {
  /// Composes independent coordinators around the visible board and prompt UI.
  ///
  /// Small effect-only components are idiomatic when each concern reads different
  /// hooks. They avoid one monolithic render function and make dependencies clear.
  fn render(&self) -> impl Render {
    let local = self.control.snapshot();
    let menu = self.control.clone();
    let activate = {
      let control = self.control.clone();
      let game = self.game.clone();
      EventCallback::new(move |square| control.dispatch(Some(&game), UiAction::Activate(square)))
    };
    let request_new_game = {
      let control = self.control.clone();
      let game = self.game.clone();
      EventCallback::new(move |()| {
        control.dispatch(
          Some(&game),
          UiAction::RequestNewGame {
            cursor_visible: false,
          },
        );
      })
    };
    let opening_control = self.control.clone();
    let opening_playbacks = hooks::use_ref(Vec::<AnimationPlayback>::new());
    let on_opening_finished = Rc::new(move |playback: AnimationPlayback, generation: u64| {
      let completion_control = opening_control.clone();
      playback.on_complete(move || completion_control.finish_opening(generation));
      opening_playbacks.with_mut(|active| active.push(playback));
    });
    (
      self
        .persistence
        .clone()
        .map(|persistence| PersistenceCoordinator { persistence }),
      TurnCoordinator {
        opponent: self.opponent.clone(),
        game: self.game.clone(),
      },
      self.diagnostics.then_some(GameStatusView {
        origin_saved: local.origin_saved,
        diagnostics: true,
      }),
      ChessBoard {
        ui: local.clone(),
        on_activate: activate,
        on_request_new_game: request_new_game.clone(),
        on_opening_finished,
        game: self.game.clone(),
        control: self.control.clone(),
      },
      (local.screen == AppScreen::Game).then(|| {
        Button::new(ls("Main menu"))
          .style(
            Style::new()
              .position(Position::Absolute)
              .top(16)
              .left(16)
              .width(140)
              .height(48)
              .background_color(UiColor::rgb(0.03, 0.09, 0.18))
              .color(UiColor::WHITE),
          )
          .on_press(move || menu.show_menu())
      }),
      PromotionDialog,
    )
  }
}

impl Component for PersistenceCoordinator {
  /// Persists the board attached to the latest publication checkpoint.
  ///
  /// During animation, the ordinary game-state hook may still expose the prior
  /// accepted snapshot. Publications carry the exact post-move board that should
  /// survive an app exit at that moment.
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let publication = reactant::use_game_publication::<ChessGame>();
    let board = publication
      .as_deref()
      .map(crate::reactant_game::ChessAnimation::accepted_board)
      .unwrap_or_else(|| state.board());
    let position = board.to_string();
    let saved = SavedGame::new(board);
    let persistence = self.persistence.clone();
    hooks::use_effect(move || persistence.update(saved), position);
  }
}

impl Component for GameStatusView {
  /// Projects logical terminal status and app origin into host diagnostics.
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let game_status = match state.board().status() {
      GameStatus::Ongoing => "ongoing",
      GameStatus::Drawn => "drawn",
      GameStatus::Won => "won",
    };
    let game_origin = if self.origin_saved { "saved" } else { "new" };
    (self
      .diagnostics
      .then(|| reactant_effects::diagnostics_view(game_status, game_origin)),)
  }
}

impl Component for TurnCoordinator {
  /// Dispatches exactly one computer action when the accepted state becomes ready.
  ///
  /// Deriving `ready` during render and dispatching inside an effect avoids a
  /// state change during reconciliation. The dependency tuple retriggers only
  /// when readiness or the accepted position changes.
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let status = reactant::use_game_status::<ChessGame>();
    let ready = status == RulesStatus::Ready
      && state.board().status() == GameStatus::Ongoing
      && state.board().side_to_move() == Color::Black;
    let game = self.game.clone();
    let opponent = self.opponent.clone();
    hooks::use_effect(
      move || {
        if ready && opponent.permitted() {
          assert_eq!(
            game.dispatch(ChessAction::ComputerMove),
            DispatchResult::Started
          );
        }
      },
      (ready, state.board().hash(), self.opponent.revision()),
    );
  }
}
