//! React-shaped application and screen composition for the chess sample.

use std::{rc::Rc, time::Duration};

use battlement::{ImageFit, ObjectId, ParentScene, Prop, Quaternion, Vector3, object_id};
use cozy_chess::{Board, Color, GameStatus};
use reactant::{
  Application, DispatchResult, GameHandle, GameRoot, GameStatus as RulesStatus, PersistentState,
  hooks,
  prelude::{
    AnimationPlayback, Button, Component, Display, Either, EventCallback, KeyRenderExt, Label,
    PickingMode, Position, Render, Style,
  },
  rules::{DisplayConnection, ExecutionMode},
  world::{Camera, SceneRoot, Sprite},
};
use trox::ls;

use crate::{
  chess_board::ChessBoard,
  chess_ui_state::{
    AppScreen, ChessUiController, ChessUiState, SessionStart, UiAction, use_chess_ui,
  },
  persistence::SavedGame,
  promotion_dialog::PromotionDialog,
  reactant_game::{ChessAction, ChessContext, ChessGame, ChessPolicy, ChessState},
};

const PLAY_BUTTON_ID: ObjectId = object_id!("4cf7cb75-ec8f-44ec-88c9-c83ca3869f43");
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
  /// Initial semantic marker exposed by the view.
  pub visual_state: crate::visual_state::VisualState,
  /// Whether the configured session should be marked as restored.
  pub origin_saved: bool,
  /// Search budget for each computer move.
  pub think_time: Duration,
  /// Optional seed for presentation-only sound selection.
  pub seed: Option<u64>,
  /// Whether host persistence may replace the configured initial state.
  pub load_persistence: bool,
}

/// Assembles the Reactant application, document, camera, and global input policy.
///
/// The application owns host setup once; components below it remain declarative
/// and can focus on state, events, and effects.
pub fn application(config: ChessConfig) -> Application {
  let app = Application::new(crate::assets::CONTENT)
    .child(ChessApp { config })
    .document(|mut document| {
      document.root_id = crate::visual_state::ROOT_ID;
      document.element.picking_mode = Prop::Set(battlement::PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      Camera::new()
        .perspective(60.0)
        .position(Vector3::new(0.0, 8.0, -3.75))
        .rotation(CAMERA_ROTATION)
        .into_object(camera.object_id)
    });
  crate::reactant_input::configure_application(app)
}

/// Root component that chooses the title or active-session subtree.
struct ChessApp {
  config: ChessConfig,
}

/// Title presentation and its input-independent Play callback.
struct TitleScreen {
  on_play: EventCallback<()>,
  control: ChessUiController,
  diagnostics: bool,
}

/// Active game composition around an already-mounted rules handle.
struct ChessScreen {
  game: GameHandle<ChessGame>,
  control: ChessUiController,
  diagnostics: bool,
  persistence: PersistentState<SavedGame>,
}

/// Semantic and diagnostic projection of the current game result.
struct GameStatusView {
  visual_state: crate::visual_state::VisualState,
  origin_saved: bool,
  diagnostics: bool,
}

/// Hidden semantic controls mirroring the world-space refresh affordance.
struct GameControls {
  pause_open: bool,
  confirm_new_game: bool,
  on_request_new_game: EventCallback<()>,
}

/// Effect-only component that schedules the computer action when appropriate.
struct TurnCoordinator {
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
    let persistence = reactant::use_persistent_state::<SavedGame>("chess-game.json");
    let diagnostics = reactant::use_host_module("battlement.diagnostics");
    // Persistence is an input to initial assembly, not an ongoing competing
    // source of truth. Once mounted, the rules session owns the logical state.
    let initial_state = hooks::use_memo(
      {
        let configured = self.config.initial_state.clone();
        let saved = persistence.value().cloned();
        let load = self.config.load_persistence;
        move || {
          if load {
            saved
              .and_then(|saved| saved.board())
              .map(ChessState::resumed)
              .or(configured)
          } else {
            configured
          }
        }
      },
      (),
    );
    let initial_local = hooks::use_memo(
      {
        let state = initial_state.clone();
        let restored = self.config.load_persistence && persistence.value().is_some();
        let visual_state = if restored {
          crate::visual_state::VisualState::Resumed
        } else {
          self.config.visual_state
        };
        let origin_saved = self.config.origin_saved || restored;
        move || {
          let mut local = ChessUiState::default();
          if state.is_some() {
            local.screen = AppScreen::Game;
            local.visual_state = visual_state;
            local.origin_saved = origin_saved;
            if visual_state == crate::visual_state::VisualState::Paused {
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
    let screen = match local.screen {
      AppScreen::Title => {
        let start = control.clone();
        Either::left(TitleScreen {
          on_play: EventCallback::new(move |()| start.request_start(false)),
          control: control.clone(),
          diagnostics,
        })
      }
      AppScreen::Game => {
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
            think_time: self.config.think_time,
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
      crate::reactant_effects::GameEffects::new(control, diagnostics),
      screen,
    )
  }
}

/// Keyed owner of one rules worker and its session-scoped effects.
struct ChessSession {
  state: ChessState,
  generation: u64,
  mode: Option<SessionStart>,
  think_time: Duration,
  seed: Option<u64>,
  control: ChessUiController,
  diagnostics: bool,
  persistence: PersistentState<SavedGame>,
}

impl Component for ChessSession {
  /// Mounts the typed game, installs input, and declares session-scoped effects.
  fn render(&self) -> impl Render {
    let think_time = self.think_time;
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
          think_time,
          seed.map_or_else(fastrand::Rng::new, fastrand::Rng::with_seed),
        )
      },
    );
    crate::reactant_input::use_chess_input(self.control.clone(), Some(game.clone()));
    if matches!(
      self.mode,
      Some(SessionStart::Restart | SessionStart::Refresh)
    ) {
      let persistence = self.persistence.clone();
      hooks::use_effect(move || persistence.clear(), ());
    }
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
      persistence: self.persistence.clone(),
    })
  }
}

impl Component for TitleScreen {
  /// Renders matching semantic and world-space Play controls.
  ///
  /// Both controls share one callback so accessible UI activation and scene
  /// picking follow exactly the same transition.
  fn render(&self) -> impl Render {
    crate::reactant_input::use_chess_input(self.control.clone(), None);
    (
      status_markers(crate::visual_state::VisualState::Title),
      Button::new(ls("Play chess"))
        .host_name("play-chess")
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(12.0)
            .top(44.0)
            .width(180.0)
            .height(40.0)
            .opacity(0.0),
        )
        .on_press(self.on_play.clone()),
      SceneRoot::new(ParentScene::PrimaryScene).child(
        Sprite::new()
          .id(*PLAY_BUTTON_ID.as_uuid())
          .texture(crate::assets::PLAY_BUTTON)
          .size(0.8, 0.24)
          .fit(ImageFit::Stretch)
          .position(Vector3::new(0.0, 6.38, -3.86))
          .rotation(CAMERA_ROTATION)
          .on_click(self.on_play.clone()),
      ),
      self
        .diagnostics
        .then(|| crate::reactant_effects::diagnostics_view("ongoing", "new")),
    )
  }
}

impl Component for ChessScreen {
  /// Composes independent coordinators around the visible board and prompt UI.
  ///
  /// Small effect-only components are idiomatic when each concern reads different
  /// hooks. They avoid one monolithic render function and make dependencies clear.
  fn render(&self) -> impl Render {
    let local = self.control.snapshot();
    let state_result = reactant::use_game_selector::<ChessGame, _>(ChessState::result);
    let publication = reactant::use_game_publication::<ChessGame>();
    let published_result = publication
      .as_deref()
      .map(crate::reactant_game::ChessAnimation::result)
      .or(state_result);
    let visual_state = if local.pause_open() || local.selected.is_some() {
      local.resolved_visual_state()
    } else {
      published_result.unwrap_or(local.visual_state)
    };
    let activate = {
      let control = self.control.clone();
      let game = self.game.clone();
      EventCallback::new(move |square| control.dispatch(Some(&game), UiAction::Activate(square)))
    };
    let move_piece = {
      let control = self.control.clone();
      let game = self.game.clone();
      EventCallback::new(move |(from, to)| {
        control.move_piece(&game, from, to);
      })
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
      PersistenceCoordinator {
        persistence: self.persistence.clone(),
      },
      TurnCoordinator {
        game: self.game.clone(),
      },
      GameStatusView {
        visual_state,
        origin_saved: local.origin_saved,
        diagnostics: self.diagnostics,
      }
      .key(visual_state.registry_key()),
      ChessBoard {
        ui: local.clone(),
        on_activate: activate,
        on_move: move_piece,
        on_request_new_game: request_new_game.clone(),
        on_opening_finished,
        game: self.game.clone(),
        control: self.control.clone(),
      },
      PromotionDialog,
      GameControls {
        pause_open: local.pause_open(),
        confirm_new_game: local.confirm_new_game(),
        on_request_new_game: request_new_game,
      },
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
    (
      status_markers(self.visual_state),
      self
        .diagnostics
        .then(|| crate::reactant_effects::diagnostics_view(game_status, game_origin)),
    )
  }
}

impl Component for GameControls {
  /// Renders the semantic new-game action only while the pause overlay is open.
  fn render(&self) -> impl Render {
    self.pause_open.then(|| {
      Button::new(ls(if self.confirm_new_game {
        "Confirm new game"
      } else {
        "New game"
      }))
      .host_name("new-game")
      .style(
        Style::new()
          .position(Position::Absolute)
          .left(12.0)
          .top(44.0)
          .width(180.0)
          .height(40.0)
          .opacity(0.0),
      )
      .on_press(self.on_request_new_game.clone())
    })
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
    hooks::use_effect(
      move || {
        if ready {
          assert_eq!(
            game.dispatch(ChessAction::ComputerMove),
            DispatchResult::Started
          );
        }
      },
      (ready, state.board().to_string()),
    );
  }
}

/// Renders a stable, hidden semantic node for every known visual state.
///
/// Stable nodes make black-box automation query a fixed topology; only the active
/// node receives text, so no test needs to inspect Rust state or visual styling.
fn status_markers(active: crate::visual_state::VisualState) -> impl Render {
  crate::visual_state::VisualState::ALL
    .into_iter()
    .map(|state| {
      Label::new(ls(if state == active { state.label() } else { "" }))
        .name(state.registry_key())
        .picking_mode(PickingMode::Ignore)
        .style(Style::new().display(Display::None))
    })
    .collect::<Vec<_>>()
}
