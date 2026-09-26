//! Shared visual offsets leave chess hit regions and the HUD stationary.

use cozy_chess::GameStatus;
use reactant::{hooks, motion_config, prelude::*, world::Group};

use crate::{
  motion,
  position::Movement,
  reactant_game::{ChessAnimation, ChessGame},
  settings,
};

/// One host-sampled offset shared by every visual branch of the board.
#[derive(Clone, PartialEq)]
pub struct BoardShake {
  x: TypedMotionValue<f32>,
  z: TypedMotionValue<f32>,
}

/// Offsets visual children without moving their interaction parent.
pub fn presentation(child: impl Render) -> impl Render {
  Presentation {
    children: child.into(),
  }
}

/// Authors one bounded offset per capture or checkmate publication.
pub fn use_shake() -> BoardShake {
  let reduced = motion_config::use_reduced_motion();
  let enabled = settings::use_settings().desired.screenshake && !reduced;
  let progress = use_motion_value(1.0_f32);
  let amplitude = use_motion_value(0.0_f32);
  let x = use_transform(
    progress.clone(),
    InputRange::new([0.0, 0.2, 0.4, 0.6, 0.8, 1.0]),
    OutputRange::new([0.0, 1.0, -0.7, 0.4, -0.15, 0.0]),
  );
  let z = use_transform(
    progress.clone(),
    InputRange::new([0.0, 0.2, 0.4, 0.6, 0.8, 1.0]),
    OutputRange::new([0.0, -0.4, 0.3, -0.2, 0.1, 0.0]),
  );
  let offsets = BoardShake {
    x: use_motion_expression(MotionExpression::input(x).multiply(amplitude.clone())),
    z: use_motion_expression(MotionExpression::input(z).multiply(amplitude.clone())),
  };
  let current = hooks::use_ref(None::<AnimationPlayback>);
  let disabling = current.clone();
  let publication = reactant::use_game_publication::<ChessGame>();
  hooks::use_commit_effect(
    {
      let publication = publication.clone();
      move || {
        let Some(animation) = publication.filter(|_| enabled) else {
          return;
        };
        let ChessAnimation::Movement {
          movement,
          accepted_board,
          ..
        } = animation.as_ref();
        let (amount, duration) = if accepted_board.status() == GameStatus::Won {
          (0.06, 0.28)
        } else if matches!(
          movement,
          Movement::Capture { .. }
            | Movement::Promotion {
              captured: Some(_),
              ..
            }
        ) {
          (0.03, 0.18)
        } else {
          return;
        };
        if let Some(previous) = current.get() {
          previous.complete();
        }
        progress.jump(0.0);
        amplitude.jump(amount);
        current.replace(Some(
          progress.animate(
            1.0,
            Transition::tween()
              .delay_secs(motion::arrival_duration(movement, false).as_secs_f64())
              .duration_secs(duration)
              .ease(Easing::Linear),
          ),
        ));
      }
    },
    publication,
  );
  hooks::use_commit_effect(
    move || {
      if !enabled && let Some(playback) = disabling.get() {
        playback.complete();
      }
    },
    enabled,
  );
  offsets
}

struct Presentation {
  children: Children,
}

impl Component for Presentation {
  fn render(&self) -> impl Render {
    let offset = hooks::use_required_context::<BoardShake>();
    Group::new().child(self.children.render()).animate(
      StyleTarget::new()
        .local_position_x_value(offset.x.clone())
        .local_position_z_value(offset.z.clone()),
    )
  }
}
