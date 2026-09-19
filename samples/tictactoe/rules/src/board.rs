use std::time::Duration;

use battlement::{Color, ImageFit, ObjectId, ParentScene, Vector3, object_id};
use reactant::{
  GameHandle,
  callback::Callback,
  prelude::{Component, Render, SnapshotAnimation},
  world,
};

use crate::{
  BOARD_ID, BOARD_TEXTURE, CELL_IDS, FONT, O_MARK_IDS, O_TEXTURE, STATUS_ID, TITLE_ID, X_MARK_IDS,
  X_TEXTURE,
  rules::{Action, Animation, Mark, State, TicTacToe},
};

const BOARD_CENTER_Y: f64 = -0.7;
const BOARD_SIZE: f64 = 7.2;
const GRID_SIZE: f64 = BOARD_SIZE * 0.8;
const CELL_SIZE: f64 = GRID_SIZE / 3.0;
const MARK_SIZE: f64 = 2.25;
const AI_DELAY: Duration = Duration::from_millis(100);
const RESET_ID: ObjectId = object_id!("74a04f17-7b7f-4b80-b748-39af24d0d424");
pub(crate) struct Board {
  game: GameHandle<TicTacToe>,
}

impl Board {
  pub(crate) fn new(game: GameHandle<TicTacToe>) -> Self {
    Self { game }
  }
}

impl Component for Board {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<TicTacToe>();
    reactant::use_animate::<TicTacToe>(|animation| match animation {
      Animation::HumanMove => Some(SnapshotAnimation::wait(AI_DELAY)),
      Animation::PlayerFinished | Animation::AiResponse => None,
    });

    let inputs = if state.is_terminal() {
      vec![self.reset_input()]
    } else {
      state
        .input_cells()
        .map(|index| self.cell_input(&state, index))
        .collect::<Vec<_>>()
    };
    let marks = state
      .board
      .iter()
      .enumerate()
      .filter_map(|(index, mark)| mark.map(|mark| self.mark(index, mark)))
      .collect::<Vec<_>>();

    (
      world::SceneRoot::new(ParentScene::PrimaryScene).child((
        world::Sprite::new()
          .id(*BOARD_ID.as_uuid())
          .texture(BOARD_TEXTURE)
          .size(BOARD_SIZE, BOARD_SIZE)
          .fit(ImageFit::Stretch)
          .position(Vector3::new(0.0, BOARD_CENTER_Y, 0.0)),
        inputs,
        marks,
      )),
      world::SceneRoot::new(ParentScene::Persistent).child((
        world::Text::new()
          .id(*TITLE_ID.as_uuid())
          .font(FONT)
          .text(format!("TIC TAC TOE — ROUND {}", state.round))
          .size(4.0)
          .color(Color::rgb(0.03, 0.04, 0.08))
          .position(Vector3::new(0.0, 4.7, -0.1)),
        world::Text::new()
          .id(*STATUS_ID.as_uuid())
          .font(FONT)
          .text(state.status_text())
          .size(3.2)
          .wrapping(Some(14.0))
          .color(Color::rgb(0.06, 0.08, 0.15))
          .position(Vector3::new(0.0, 3.75, -0.1)),
      )),
    )
  }
}

impl Board {
  fn cell_input(&self, state: &State, index: usize) -> world::BoxHitRegion {
    debug_assert!(state.accepts_cell(index));
    let game = self.game.clone();
    let activate = Callback::new(move |()| {
      let _ = game.dispatch(Action::Cell(index));
    });
    world::BoxHitRegion::new()
      .id(*CELL_IDS[index].as_uuid())
      .size(Vector3::new(CELL_SIZE, CELL_SIZE, 0.2))
      .position(cell_position(index, 0.0))
      .focusable(true)
      .on_click(activate.clone())
      .navigation(world::NavigationHandlers::new().on_activate(activate))
  }

  fn reset_input(&self) -> world::BoxHitRegion {
    let game = self.game.clone();
    let activate = Callback::new(move |()| {
      let _ = game.dispatch(Action::Reset);
    });
    world::BoxHitRegion::new()
      .id(*RESET_ID.as_uuid())
      .size(Vector3::new(BOARD_SIZE, BOARD_SIZE, 0.2))
      .position(Vector3::new(0.0, BOARD_CENTER_Y, 0.0))
      .focusable(true)
      .on_click(activate.clone())
      .navigation(world::NavigationHandlers::new().on_activate(activate))
  }

  fn mark(&self, index: usize, mark: Mark) -> world::Sprite {
    world::Sprite::new()
      .id(*marker_id(index, mark).as_uuid())
      .texture(if mark == Mark::X {
        X_TEXTURE
      } else {
        O_TEXTURE
      })
      .size(MARK_SIZE, MARK_SIZE)
      .fit(ImageFit::Contain)
      .position(cell_position(index, -0.05))
  }
}

fn marker_id(index: usize, mark: Mark) -> ObjectId {
  match mark {
    Mark::X => X_MARK_IDS[index],
    Mark::O => O_MARK_IDS[index],
  }
}

fn cell_position(index: usize, z: f64) -> Vector3 {
  let row = index / 3;
  let column = index % 3;
  Vector3::new(
    (column as f64 - 1.0) * CELL_SIZE,
    BOARD_CENTER_Y + (1.0 - row as f64) * CELL_SIZE,
    z,
  )
}
