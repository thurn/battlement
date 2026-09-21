//! Ordinary component authoring with app-owned game sessions.

pub use crate::{
  application_shell::{Application, use_portal_target},
  game_app::use_game,
  game_hooks::{
    GamePresentation, GameRoot, SnapshotAnimation, use_animate, use_game_observation,
    use_game_presentation, use_game_prompt, use_game_publication, use_game_selector,
    use_game_selector_with, use_game_state, use_game_status,
  },
  game_session::{DispatchResult, GameHandle, GameObservation, GameStatus},
  input::{GlobalInput, use_global_input},
  persistence::{
    FilePersistenceBackend, PersistenceBackend, PersistentState, use_host_module,
    use_persistent_state, use_persistent_state_with,
  },
  presentation_inspector::{InspectorObject, PresentationInspector},
  timers::{use_interval, use_timeout},
};
pub use reactant_core::prelude::*;

pub use crate::world::{Group as WorldGroup, Prefab, SceneRoot};
pub use reactant_core::native_host::{ObjectRef, use_object_ref};
