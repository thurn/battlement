//! Reactant component tree and shared checkpoint sequences for complete chess.

use std::time::Duration;

use battlement::{
  MaterialAssignment, ObjectId, PanelPoint, ParentScene, PointerButton, Quaternion, ScreenSize,
  Vector3,
};
use cozy_chess::{Color, GameStatus, Square};
use reactant::{
  GameApp, GameHandle, GameRoot, GameStatus as RulesStatus, SnapshotAnimation,
  animation_controls::{AnimationSequence, MotionSelector, SequencePosition},
  app::App,
  callback::IntoCallback,
  event::ReactantEvent,
  hooks,
  key::KeyRenderExt,
  prelude::{
    Button, Component, Easing, MotionComponentExt, MotionProps, PickingMode, Position, Render,
    Style, StyleTarget, Transition, use_object_ref,
  },
  rules::ExecutionMode,
  world,
};
use trox::ls;

use crate::{
  PIECE_SPAWN_EFFECT_LIFETIME_MS, PIECE_SPAWN_SEQUENCE_DURATION_MS,
  reactant_fixture::{FixtureAnimation, FixturePiece},
  reactant_game::{ChessAnimation, ChessContext, ChessGame, ChessPolicy, ChessState},
  reactant_input::{AppControl, ChessModel},
};

const MOVE_DURATION: Duration = Duration::from_millis(300);
const KNIGHT_DURATION: Duration = Duration::from_millis(320);

struct BoardView {
  game: GameHandle<ChessGame>,
  control: AppControl,
  diagnostics: bool,
}

struct PieceView {
  piece: FixturePiece,
  square: Square,
  reference: reactant::prelude::ObjectRef,
  control: AppControl,
  game: GameHandle<ChessGame>,
  spawning: bool,
  interactive: bool,
}

pub(crate) fn app(
  state: ChessState,
  think_time: Duration,
  seed: Option<u64>,
  diagnostics: bool,
) -> App<ChessModel> {
  let mut app = App::with_model(crate::assets::CONTENT, ChessModel::new());
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
  let app = app
    .root(move |model| {
      let game = model.game();
      (
        crate::reactant_effects::EffectsView::new(model.control.clone(), diagnostics),
        GameRoot::new(BoardView {
          game,
          control: model.control.clone(),
          diagnostics,
        }),
      )
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

impl Component for BoardView {
  fn render(&self) -> impl Render {
    let state = reactant::use_game_state::<ChessGame>();
    let status = reactant::use_game_status::<ChessGame>();
    let screen = reactant::app_context::use_viewport_size();
    let local = hooks::use_external_store(self.control.store());
    let scope = reactant::animation_controls::use_animation_scope();
    let event_scope = scope.clone();
    let references: [reactant::prelude::ObjectRef; 64] = std::array::from_fn(|_| use_object_ref());
    let event_references = references.clone();
    reactant::use_animate::<ChessGame>(move |animation| {
      Some(SnapshotAnimation::sequence(
        event_scope.clone(),
        sequence(animation, &event_references),
      ))
    });

    let visual_state = if local.pause_open {
      crate::visual_state::VisualState::Paused
    } else if local.selected.is_some() {
      crate::visual_state::VisualState::Selected
    } else {
      state.visual_state()
    };
    let pieces = state.started().then(|| {
      Square::ALL
        .into_iter()
        .filter_map(|square| {
          state.piece(square).map(|piece| {
            PieceView {
              piece,
              square,
              reference: piece_reference(&references, piece.id),
              control: self.control.clone(),
              game: self.game.clone(),
              spawning: state.spawning(),
              interactive: status == RulesStatus::Ready,
            }
            .key(piece.id)
          })
        })
        .collect::<Vec<_>>()
    });
    let legal = local
      .selected
      .map(|square| crate::legal_destinations(state.board(), square))
      .unwrap_or_default();
    let control = self.control.clone();
    let game = self.game.clone();
    let title = (!state.started()).then(|| {
      Button::new(ls("Play chess"))
        .host_name("play-chess")
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(12.0)
            .top(44.0)
            .width(180.0)
            .height(40.0),
        )
        .disabled(status != RulesStatus::Ready)
        .on_press(move || control.start(&game))
    });
    let control = self.control.clone();
    let game = self.game.clone();
    let play_sprite = (!state.started()).then(|| {
      world::Sprite::new()
        .id(*crate::PLAY_BUTTON_ID.as_uuid())
        .texture(crate::assets::PLAY_BUTTON)
        .size(0.8, 0.24)
        .fit(battlement::ImageFit::Stretch)
        .position(Vector3::new(0.0, 6.38, -3.86))
        .rotation(crate::CAMERA_ROTATION)
        .focusable(true)
        .on_click((move || control.start(&game)).into_callback())
    });
    let control = self.control.clone();
    let reset = (state.started() && local.pause_open).then(|| {
      Button::new(ls(if local.confirm_new_game {
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
          .height(40.0),
      )
      .on_press(move || control.request_new_game(false))
    });
    let control = self.control.clone();
    let refresh_sprite = (state.started() && local.pause_open).then(|| {
      let aspect = if screen.height == 0 {
        1.0
      } else {
        f64::from(screen.width) / f64::from(screen.height)
      };
      let half_height =
        crate::CAMERA_BUTTON_DEPTH * (crate::CAMERA_VERTICAL_FOV_RADIANS / 2.0).tan();
      let right =
        half_height * aspect - crate::REFRESH_BUTTON_SIZE / 2.0 - crate::REFRESH_BUTTON_MARGIN;
      let up = half_height - crate::REFRESH_BUTTON_SIZE / 2.0 - crate::REFRESH_BUTTON_MARGIN;
      world::Sprite::new()
        .id(*crate::REFRESH_BUTTON_ID.as_uuid())
        .texture(crate::assets::REFRESH_BUTTON)
        .size(crate::REFRESH_BUTTON_SIZE, crate::REFRESH_BUTTON_SIZE)
        .fit(battlement::ImageFit::Stretch)
        .position(Vector3::new(
          right,
          8.0 - 0.946201 * crate::CAMERA_BUTTON_DEPTH + 0.323579 * up,
          -3.75 + 0.323579 * crate::CAMERA_BUTTON_DEPTH + 0.946201 * up,
        ))
        .rotation(crate::CAMERA_ROTATION)
        .focusable(true)
        .on_click((move || control.request_new_game(false)).into_callback())
    });
    let highlights = Square::ALL
      .into_iter()
      .map(|square| {
        let mut position = crate::square_position(square);
        position.y = crate::HIGHLIGHT_HEIGHT;
        let active = legal.contains(&square);
        let control = self.control.clone();
        let game = self.game.clone();
        world::Plane::new()
          .id(*crate::highlight_id(square as usize).as_uuid())
          .position(position)
          .scale(Vector3::new(
            crate::HIGHLIGHT_SCALE,
            1.0,
            crate::HIGHLIGHT_SCALE,
          ))
          .active(active)
          .materials([MaterialAssignment::new(0, crate::assets::LEGAL_SQUARE)])
          .focusable(active)
          .on_click((move || control.activate_square(&game, square)).into_callback())
      })
      .collect::<Vec<_>>();
    let cursor_active = state.started() && (local.cursor_visible || local.selected.is_some());
    let cursor_scale = if status == RulesStatus::Busy {
      0.55
    } else {
      1.0
    };
    let cursor = world::Group::new()
      .id(*crate::cursor::EFFECT_ID.as_uuid())
      .position(crate::square_position(local.cursor))
      .scale(Vector3::new(cursor_scale, cursor_scale, cursor_scale))
      .active(cursor_active)
      .child(world::Prefab::at(crate::assets::effects::PIECE_SELECTED));
    let game_status = match state.board().status() {
      GameStatus::Ongoing => "ongoing",
      GameStatus::Drawn => "drawn",
      GameStatus::Won => "won",
    };
    let game_origin = if state.origin_saved() { "saved" } else { "new" };

    (
      reactant::prelude::Label::new(ls(visual_state.label()))
        .name(visual_state.registry_key())
        .picking_mode(PickingMode::Ignore)
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(12.0)
            .top(12.0)
            .height(24.0),
        )
        .key(visual_state.object_id()),
      title,
      reset,
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child((pieces, highlights, cursor, play_sprite, refresh_sprite))
          .motion(MotionProps::new().animation_scope(scope)),
      ),
      self
        .diagnostics
        .then(|| crate::reactant_effects::diagnostics_view(game_status, game_origin)),
    )
  }
}

impl Component for PieceView {
  fn render(&self) -> impl Render {
    let screen = reactant::app_context::use_viewport_size();
    let control = self.control.clone();
    let game = self.game.clone();
    let square = self.square;
    let pointer_control = self.control.clone();
    let pointer_game = self.game.clone();
    let pointer_piece = self.piece.id;
    let release_control = self.control.clone();
    let release_game = self.game.clone();
    let cancel_control = self.control.clone();
    world::BoxHitRegion::new()
      .id(*self.piece.id.as_uuid())
      .size(Vector3::new(0.9, 1.5, 0.9))
      .position(crate::square_position(square))
      .scale(if self.spawning {
        Vector3::ZERO
      } else {
        Vector3::ONE
      })
      .rotation(if self.piece.color == Color::Black {
        Quaternion::new(0.0, 1.0, 0.0, 0.0)
      } else {
        Quaternion::IDENTITY
      })
      .reference(self.reference.clone())
      .focusable(self.piece.color == Color::White && self.interactive)
      .capture_on_press(self.interactive)
      .events(
        world::PointerHandlers::new()
          .on_pointer_down(
            move |event: ReactantEvent<battlement::PointerButtonEvent>| {
              if event.payload().button == PointerButton::Left {
                pointer_control.drag_start(&pointer_game, pointer_piece);
              }
            },
          )
          .on_pointer_up(
            move |event: ReactantEvent<battlement::PointerButtonEvent>| {
              if event.payload().button == PointerButton::Left
                && let Some(target) = panel_square(event.payload().position, screen)
              {
                release_control.activate_square(&release_game, target);
              }
            },
          )
          .on_pointer_cancel(move |_: ReactantEvent<battlement::PointerCancelEvent>| {
            cancel_control.cancel_selection();
          }),
      )
      .on_click((move || control.activate_square(&game, square)).into_callback())
      .child(world::Prefab::at(crate::address(
        self.piece.color,
        self.piece.kind,
      )))
      .motion(MotionProps::new().motion_name("chess-piece"))
  }
}

fn sequence(
  animation: &ChessAnimation,
  references: &[reactant::prelude::ObjectRef; 64],
) -> AnimationSequence {
  match animation {
    ChessAnimation::Opening {
      spawn,
      beats,
      sound,
    } => {
      let mut sequence = AnimationSequence::new().play_sound(sound.clone());
      if !spawn {
        return sequence;
      }
      for (beat, pieces) in beats.iter().enumerate() {
        let at = Duration::from_millis(
          crate::CRITICAL_FIRST_BEAT_OFFSET_MS + beat as u64 * crate::CRITICAL_BEAT_INTERVAL_MS,
        );
        for piece in pieces {
          let reference = piece_reference(references, *piece);
          sequence = sequence
            .animate(
              MotionSelector::object(reference.clone()),
              StyleTarget::new()
                .local_scale_x(1.0)
                .local_scale_y(1.0)
                .local_scale_z(1.0),
              Transition::tween().duration_secs(0.2).ease(Easing::EaseOut),
            )
            .at(SequencePosition::Absolute(at))
            .particle_for(
              crate::assets::effects::PIECE_SPAWN,
              reference.local_point(Vector3::ZERO).capture_at_start(),
              Duration::from_millis(PIECE_SPAWN_EFFECT_LIFETIME_MS),
            )
            .at(SequencePosition::Absolute(at));
        }
      }
      sequence
        .animate(
          MotionSelector::ScopeRoot,
          StyleTarget::new().local_scale_factor_x(1.0),
          Transition::tween()
            .duration_secs(PIECE_SPAWN_SEQUENCE_DURATION_MS as f64 / 1_000.0)
            .ease(Easing::Linear),
        )
        .at(SequencePosition::Absolute(Duration::ZERO))
    }
    ChessAnimation::Movement {
      movement,
      sound,
      final_sound,
    } => {
      let capture_or_promotion = matches!(
        movement,
        FixtureAnimation::Capture { .. } | FixtureAnimation::Promotion { .. }
      );
      let knight = matches!(
        movement,
        FixtureAnimation::Knight { .. }
          | FixtureAnimation::Capture {
            knight_corner: Some(_),
            ..
          }
      );
      let arrival = if knight {
        KNIGHT_DURATION
      } else {
        MOVE_DURATION
      };
      let mut sequence = crate::reactant_fixture::sequence(movement, references);
      sequence = sequence
        .play_sound(sound.clone())
        .at(SequencePosition::Absolute(if capture_or_promotion {
          arrival
        } else {
          Duration::ZERO
        }));
      if let Some(sound) = final_sound {
        sequence = sequence
          .play_sound(sound.clone())
          .at(SequencePosition::Absolute(arrival));
      }
      sequence
    }
  }
}

fn piece_reference(
  references: &[reactant::prelude::ObjectRef; 64],
  piece: ObjectId,
) -> reactant::prelude::ObjectRef {
  let index = piece
    .as_uuid()
    .as_bytes()
    .iter()
    .skip(10)
    .fold(0_usize, |value, byte| (value << 8) | usize::from(*byte));
  references[index].clone()
}

fn panel_square(point: PanelPoint, screen: ScreenSize) -> Option<Square> {
  if screen.width == 0 || screen.height == 0 {
    return None;
  }
  let width = f64::from(screen.width);
  let height = f64::from(screen.height);
  let tangent = (crate::CAMERA_VERTICAL_FOV_RADIANS / 2.0).tan();
  let ray = Vector3::new(
    (2.0 * point.x / width - 1.0) * width / height * tangent,
    (1.0 - 2.0 * point.y / height) * tangent,
    1.0,
  );
  let direction = rotate(crate::CAMERA_ROTATION, ray);
  if direction.y >= -f64::EPSILON {
    return None;
  }
  let camera = Vector3::new(0.0, 8.0, -3.75);
  let distance = -camera.y / direction.y;
  Some(crate::square_at(Vector3::new(
    camera.x + direction.x * distance,
    0.0,
    camera.z + direction.z * distance,
  )))
}

fn rotate(rotation: Quaternion, value: Vector3) -> Vector3 {
  let dot = rotation.x * value.x + rotation.y * value.y + rotation.z * value.z;
  let length = rotation.x * rotation.x + rotation.y * rotation.y + rotation.z * rotation.z;
  Vector3::new(
    2.0 * dot * rotation.x
      + (rotation.w * rotation.w - length) * value.x
      + 2.0 * rotation.w * (rotation.y * value.z - rotation.z * value.y),
    2.0 * dot * rotation.y
      + (rotation.w * rotation.w - length) * value.y
      + 2.0 * rotation.w * (rotation.z * value.x - rotation.x * value.z),
    2.0 * dot * rotation.z
      + (rotation.w * rotation.w - length) * value.z
      + 2.0 * rotation.w * (rotation.x * value.y - rotation.y * value.x),
  )
}
