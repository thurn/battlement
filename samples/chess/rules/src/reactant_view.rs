//! React-shaped application and screen composition for the chess sample.

use std::{rc::Rc, time::Duration};

use battlement::{ParentScene, Vector3};
use cozy_chess::{Color, GameStatus};
use reactant::{
  DispatchResult, GameApp, GameHandle, GameRoot, GameStatus as RulesStatus,
  app::App,
  prelude::{
    Button, Component, Display, Either, EventCallback, PickingMode, Position, Render, Style,
  },
  rules::ExecutionMode,
  world,
};
use trox::ls;

use crate::{
  chess_board::ChessBoard,
  chess_ui_state::{AppControl, AppScreen, ChessUiState, UiAction},
  promotion_dialog::PromotionDialog,
  reactant_game::{ChessAction, ChessContext, ChessGame, ChessPolicy, ChessState},
  reactant_input::ChessModel,
};

struct ChessApp {
  game: Option<GameHandle<ChessGame>>,
  control: AppControl,
  diagnostics: bool,
}

struct TitleScreen {
  on_play: EventCallback<()>,
  diagnostics: bool,
}

struct ChessScreen {
  game: GameHandle<ChessGame>,
  control: AppControl,
  diagnostics: bool,
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

pub(crate) fn app(
  state: Option<ChessState>,
  visual_state: crate::visual_state::VisualState,
  origin_saved: bool,
  think_time: Duration,
  seed: Option<u64>,
  diagnostics: bool,
) -> App<ChessModel> {
  let mut local = ChessUiState::default();
  if state.is_some() {
    local.screen = AppScreen::Game;
    local.visual_state = visual_state;
    local.origin_saved = origin_saved;
  }
  let mut app = App::with_model(crate::assets::CONTENT, ChessModel::new(local));
  if let Some(state) = state {
    let game = app.start_game::<ChessGame>(state, move |connection| {
      ChessContext::new(
        ExecutionMode::Interactive {
          connection,
          policy: ChessPolicy,
        },
        think_time,
        seed.map_or_else(fastrand::Rng::new, fastrand::Rng::with_seed),
      )
    });
    let consumer = app.game_consumer::<ChessGame>();
    consumer.resume_automatic_submission();
    app.model().consumer.replace(Some(consumer));
    app.model().game.replace(Some(game));
  }
  let app = app
    .root(move |model| ChessApp {
      game: model.game(),
      control: model.control.clone(),
      diagnostics,
    })
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
  crate::reactant_input::configure_app(app)
}

impl Component for ChessApp {
  fn render(&self) -> impl Render {
    let local = reactant::hooks::use_external_store(self.control.store());
    let screen = match local.screen {
      AppScreen::Title => {
        let control = self.control.clone();
        Either::left(TitleScreen {
          on_play: EventCallback::new(move |()| control.request_start(false)),
          diagnostics: self.diagnostics,
        })
      }
      AppScreen::Game => Either::right(GameRoot::new(ChessScreen {
        game: self
          .game
          .clone()
          .expect("game screen requires an active chess session"),
        control: self.control.clone(),
        diagnostics: self.diagnostics,
      })),
    };
    (
      crate::reactant_effects::GameEffects::new(self.control.clone(), self.diagnostics),
      screen,
    )
  }
}

impl Component for TitleScreen {
  fn render(&self) -> impl Render {
    (
      status_label(crate::visual_state::VisualState::Title),
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
    let local = reactant::hooks::use_external_store(self.control.store());
    let activate = {
      let control = self.control.clone();
      let game = self.game.clone();
      EventCallback::new(move |square| control.dispatch(Some(&game), UiAction::Activate(square)))
    };
    let move_piece = {
      let control = self.control.clone();
      let game = self.game.clone();
      EventCallback::new(move |(from, to)| {
        control.dispatch(Some(&game), UiAction::Select(from));
        control.dispatch(Some(&game), UiAction::Activate(to));
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
    let on_opening_finished = Rc::new(
      move |playback: reactant::prelude::AnimationPlayback, generation: u64| {
        let completion_control = opening_control.clone();
        playback.on_complete(move || completion_control.finish_opening(generation));
        opening_control.retain_opening(playback);
      },
    );
    let piece_control = self.control.clone();
    let on_piece_mounted = Rc::new(move |entity, host| {
      piece_control.register_piece_host(entity, host);
    });
    (
      TurnCoordinator {
        game: self.game.clone(),
      },
      GameStatusView {
        visual_state: local.resolved_visual_state(),
        origin_saved: local.origin_saved,
        diagnostics: self.diagnostics,
      },
      ChessBoard {
        ui: local.clone(),
        on_activate: activate,
        on_move: move_piece,
        on_request_new_game: request_new_game.clone(),
        on_opening_finished,
        on_piece_mounted,
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
      status_label(self.visual_state),
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

fn status_label(visual_state: crate::visual_state::VisualState) -> impl Render {
  reactant::prelude::Label::new(ls(visual_state.label()))
    .name(visual_state.registry_key())
    .picking_mode(PickingMode::Ignore)
    .style(Style::new().display(Display::None))
    .id(*visual_state.object_id().as_uuid())
}
