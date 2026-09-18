//! Imperative replacement releases only the properties claimed by the new target.

use std::collections::HashSet;

use battlement::{MotionGeneration, MotionProperty, MotionSlotId};

use crate::{motion_playbacks::Address, motion_slot::Slot};

pub(crate) fn retain_disjoint(
  descriptor: battlement::ObjectId,
  slots: &mut Vec<Slot>,
  replacements: &[Slot],
) -> Vec<(Address, Address)> {
  let claimed = replacements
    .iter()
    .flat_map(|slot| {
      slot
        .definition
        .target
        .tracks
        .iter()
        .map(|track| track.property)
        .chain(
          slot
            .definition
            .target
            .transition_end
            .iter()
            .map(|value| value.property),
        )
    })
    .collect::<HashSet<MotionProperty>>();
  let replacement_ids = replacements
    .iter()
    .map(|slot| slot.definition.slot)
    .collect::<HashSet<_>>();
  let mut occupied = slots
    .iter()
    .map(|slot| slot.definition.slot)
    .chain(replacement_ids.iter().copied())
    .collect::<HashSet<_>>();
  let mut remaps = Vec::new();
  slots.retain_mut(|slot| {
    if slot.definition.slot.0 < u64::MAX - 2048 {
      return true;
    }
    let old_count =
      slot.definition.target.tracks.len() + slot.definition.target.transition_end.len();
    slot
      .definition
      .target
      .tracks
      .retain(|track| !claimed.contains(&track.property));
    slot
      .definition
      .target
      .transition_end
      .retain(|value| !claimed.contains(&value.property));
    let remaining =
      slot.definition.target.tracks.len() + slot.definition.target.transition_end.len();
    if remaining == 0 {
      return false;
    }
    let collision = replacement_ids.contains(&slot.definition.slot);
    if remaining == old_count && !collision {
      return true;
    }
    if !slot.definition.target.transition_end.is_empty() {
      slot.completion_tracks.extend(
        slot
          .tracks
          .iter()
          .filter(|track| claimed.contains(&track.definition.property))
          .cloned(),
      );
    }
    slot
      .tracks
      .retain(|track| !claimed.contains(&track.definition.property));
    slot
      .presentation
      .retain(|(property, _)| !claimed.contains(property));
    let old = Address {
      descriptor,
      slot: slot.definition.slot,
      generation: slot.definition.generation,
    };
    slot.definition.generation = MotionGeneration(
      slot
        .definition
        .generation
        .0
        .checked_add(1)
        .expect("Motion generation overflow"),
    );
    if collision {
      let id = (u64::MAX - 2048..u64::MAX - 1024)
        .map(MotionSlotId)
        .find(|id| !occupied.contains(id))
        .expect("Motion imperative property ownership capacity exceeded");
      occupied.insert(id);
      slot.definition.slot = id;
    }
    remaps.push((
      old,
      Address {
        descriptor,
        slot: slot.definition.slot,
        generation: slot.definition.generation,
      },
    ));
    true
  });
  remaps
}
