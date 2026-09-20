//! React-shaped application and screen composition for the chess sample.

use std::{rc::Rc, time::Duration};

use battlement::{ParentScene, Vector3};
use cozy_chess::{Color, GameStatus};
use reactant::{
  Application, DispatchResult, GameHandle, GameRoot, GameStatus as RulesStatus, PersistentState,
  prelude::{
    Button, Component, Display, Either, EventCallback, KeyRenderExt, PickingMode, Position, Render,
    Style,
  },
  rules::{DisplayConnection, ExecutionMode},
  world,
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

#[derive(Clone)]
pub(crate) struct ChessConfig {
  pub(crate) starting_board: cozy_chess::Board,
  pub(crate) initial_state: Option<ChessState>,
  pub(crate) visual_state: crate::visual_state::VisualState,
  pub(crate) origin_saved: bool,
  pub(crate) think_time: Duration,
  pub(crate) seed: Option<u64>,
  pub(crate) load_persistence: bool,
}

struct ChessApp {
  config: ChessConfig,
}

struct TitleScreen {
  on_play: EventCallback<()>,
  control: ChessUiController,
  diagnostics: bool,
}

struct ChessScreen {
  game: GameHandle<ChessGame>,
  control: ChessUiController,
  diagnostics: bool,
  persistence: PersistentState<SavedGame>,
}

struct GameStatusView {
  visual_state: crate::visual_state::VisualState,
  origin_saved: bool,
  diagnostics: bool,
}

struct GameControls {
  pause_open: bool,
  confirm_new_game: bool,
  on_request_new_game: EventCallback<()>,
}

struct TurnCoordinator {
  game: GameHandle<ChessGame>,
}

struct PersistenceCoordinator {
  persistence: PersistentState<SavedGame>,
}

pub(crate) fn application(config: ChessConfig) -> Application {
  let app = Application::new(crate::assets::CONTENT)
    .child(ChessApp { config })
    .document(|mut document| {
      document.root_id = crate::visual_state::ROOT_ID;
      document.element.picking_mode = battlement::Prop::Set(battlement::PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .perspective(60.0)
        .position(Vector3::new(0.0, 8.0, -3.75))
        .rotation(crate::CAMERA_ROTATION)
        .into_object(camera.object_id)
    });
  crate::reactant_input::configure_application(app)
}

impl Component for ChessApp {
  fn render(&self) -> impl Render {
    let persistence = reactant::use_persistent_state::<SavedGame>("chess-game.json");
    let diagnostics = reactant::use_host_module("battlement.diagnostics");
    let initial_state = reactant::hooks::use_memo(
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
    let initial_local = reactant::hooks::use_memo(
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
            .unwrap_or_else(|| ChessState::with_generation(self.config.starting_board.clone(), 0))
        } else {
          ChessState::with_generation(
            self.config.starting_board.clone(),
            u32::try_from(local.opening_generation).expect("chess generation exceeds u32"),
          )
        };
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
      reactant::hooks::use_effect(move || persistence.clear(), ());
    }
    let music = self.control.clone();
    let restart_music = matches!(self.mode, Some(SessionStart::Restart));
    reactant::hooks::use_effect(
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
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Sprite::new()
          .id(*crate::PLAY_BUTTON_ID.as_uuid())
          .texture(crate::assets::PLAY_BUTTON)
          .size(0.8, 0.24)
          .fit(battlement::ImageFit::Stretch)
          .position(Vector3::new(0.0, 6.38, -3.86))
          .rotation(crate::CAMERA_ROTATION)
          .on_click(self.on_play.clone()),
      ),
      self
        .diagnostics
        .then(|| crate::reactant_effects::diagnostics_view("ongoing", "new")),
    )
  }
}

impl Component for ChessScreen {
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
    let opening_playbacks =
      reactant::hooks::use_ref(Vec::<reactant::prelude::AnimationPlayback>::new());
    let on_opening_finished = Rc::new(
      move |playback: reactant::prelude::AnimationPlayback, generation: u64| {
        let completion_control = opening_control.clone();
        playback.on_complete(move || completion_control.finish_opening(generation));
        opening_playbacks.with_mut(|active| active.push(playback));
      },
    );
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
    reactant::hooks::use_effect(move || persistence.update(saved), position);
  }
}

impl Component for GameStatusView {
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
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let status = reactant::use_game_status::<ChessGame>();
    let ready = status == RulesStatus::Ready
      && state.board().status() == GameStatus::Ongoing
      && state.board().side_to_move() == Color::Black;
    let game = self.game.clone();
    reactant::hooks::use_effect(
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

fn status_markers(active: crate::visual_state::VisualState) -> impl Render {
  crate::visual_state::VisualState::ALL
    .into_iter()
    .map(|state| {
      reactant::prelude::Label::new(ls(if state == active { state.label() } else { "" }))
        .name(state.registry_key())
        .picking_mode(PickingMode::Ignore)
        .style(Style::new().display(Display::None))
    })
    .collect::<Vec<_>>()
}
