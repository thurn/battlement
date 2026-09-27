use std::time::Duration;

use battlement::Vector3;
use reactant::{
  SnapshotAnimation,
  animation_controls::{self, AnimationSequence, SequencePosition, SequenceSoundOptions},
  motion_config, native_host,
  prelude::*,
  world,
};
use reactant_rules::ReducerGame;

use crate::{
  audio,
  card_table::CardTable,
  domain::{Event, Phase, Seat},
  particles,
  projection::{HumanView, VisibleCard},
  reducer::HeartsReducer,
};

#[derive(Clone, Copy, Default)]
pub(crate) struct CardTiming {
  pub travel: f64,
  pub deal: bool,
  pub reduced: bool,
}

pub(crate) struct AnimatedTable {
  pub view: HumanView,
  pub aspect: f64,
  pub inspection: Option<VisibleCard>,
  pub fresh: bool,
  pub sound: bool,
}

impl Component for AnimatedTable {
  fn render(&self) -> impl Render {
    let event = reactant::use_game_publication::<ReducerGame<HeartsReducer>>();
    let reduced = motion_config::use_reduced_motion();
    let initial_deal = self.fresh && event.is_none();
    let deal = event.as_deref() == Some(&Event::Deal)
      || (initial_deal && self.view.table.phase == Phase::Passing);
    let travel = if reduced {
      0.1
    } else {
      match event.as_deref() {
        Some(Event::PassExchanged) => 0.45,
        Some(Event::TrickCollected { .. }) => 0.4,
        _ => 0.25,
      }
    };
    let timing = CardTiming {
      travel,
      deal,
      reduced,
    };
    let scope = animation_controls::use_animation_scope();
    let animation_scope = scope.clone();
    let full_trick = self.view.trick.len() == 4;
    let sound = self.sound;
    reactant::use_animate_with_initial::<ReducerGame<HeartsReducer>>(
      deal.then_some(Event::Deal),
      move |event| {
        if matches!(
          event,
          Event::PassSubmitted { .. } | Event::MatchEnded { .. }
        ) {
          return None;
        }
        let arrival = if initial_deal && !reduced {
          Duration::from_secs_f64(51.0 * 0.045 + travel)
        } else if reduced {
          Duration::from_millis(100)
        } else {
          Duration::ZERO
        };
        let hold = if matches!(event, Event::CardPlayed { .. }) && full_trick {
          if reduced { 0.2 } else { 0.65 }
        } else {
          0.0
        };
        let mut sequence =
          AnimationSequence::new().label_at("arrived", SequencePosition::Absolute(arrival));
        if sound && let Some((address, volume)) = audio::cue(*event) {
          sequence = sequence.play_sound_with(
            address,
            SequenceSoundOptions {
              volume,
              ..SequenceSoundOptions::default()
            },
          );
        }
        Some(SnapshotAnimation::sequence(
          animation_scope,
          sequence.label_at(
            "ready",
            SequencePosition::Absolute(arrival + Duration::from_secs_f64(hold)),
          ),
        ))
      },
    );
    let anchor = native_host::use_object_ref();
    let position = anchor.clone();
    let effect_scope = scope.clone();
    reactant::use_animate::<ReducerGame<HeartsReducer>>(move |event| {
      if reduced {
        return None;
      }
      let seed = match event {
        Event::CardPlayed {
          broke_hearts: true, ..
        } => 71,
        Event::HandScored { .. } => 83,
        _ => return None,
      };
      Some(
        SnapshotAnimation::sequence(effect_scope, particles::burst(position, seed)).nonblocking(),
      )
    });
    world::Group::new()
      .child((
        CardTable::new(&self.view, self.aspect)
          .inspect(self.inspection)
          .timing(timing),
        particles::anchor(self.aspect).reference(anchor),
      ))
      .motion(MotionProps::new().animation_scope(scope))
  }
}

impl CardTiming {
  pub fn child(
    self,
    child: world::LayoutChild,
    seat: Option<Seat>,
    index: usize,
    half_width: f64,
  ) -> world::LayoutChild {
    if self.travel == 0.0 {
      return child;
    }
    let delay = if self.deal && !self.reduced {
      seat.map_or(0.0, |seat| (index * 4 + seat.index()) as f64 * 0.045)
    } else {
      0.0
    };
    let child = child.movement(
      Transition::tween()
        .duration_secs(self.travel)
        .delay_secs(delay)
        .ease(Easing::EaseOut),
    );
    if !self.deal || self.reduced {
      return child;
    }
    let portrait = half_width < 5.7;
    let side = half_width * if portrait { 0.7 } else { 0.60 };
    let origin = match seat {
      Some(Seat::South) => Vector3::new(0.0, 0.6, 0.0),
      Some(Seat::North) => Vector3::new(0.0, 0.6, 3.6),
      Some(Seat::West) => Vector3::new(0.6, 0.6, side),
      Some(Seat::East) => Vector3::new(-0.6, 0.6, side),
      None => return child,
    };
    child.initial(
      StyleTarget::new()
        .local_position_x(origin.x as f32)
        .local_position_y(origin.y as f32)
        .local_position_z(origin.z as f32),
    )
  }
}
