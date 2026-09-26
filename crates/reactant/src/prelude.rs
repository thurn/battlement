//! Ordinary component authoring with app-owned game sessions.

pub use crate::{
  application_shell::{Application, use_portal_target},
  game_app::use_game,
  game_hooks::{
    GamePresentation, GameRoot, SnapshotAnimation, SnapshotPlayback, use_animate,
    use_game_motion_ready, use_game_observation, use_game_presentation,
    use_game_presentation_receipt, use_game_prompt, use_game_publication, use_game_selector,
    use_game_selector_with, use_game_state, use_game_status,
  },
  game_presentation::{PresentationReceipt, PresentationStatus},
  game_reducer::{
    GameVersion, ReducerDispatch, ReducerHandle, ReducerSnapshot, use_game_reducer,
    use_reducer_selector,
  },
  game_session::{DispatchResult, GameHandle, GameObservation, GameStatus},
  host_settings::use_host_settings,
  input::{GlobalInput, use_global_input},
  input_capture::use_input_capture,
  input_subscriptions::{InputSubscription, use_input_subscription},
  persistence::{
    PersistentState, use_host_module, use_persistent_state, use_persistent_state_with,
  },
  persistence_file::FilePersistenceBackend,
  persistence_operation::{
    PersistenceBackend, PersistenceCompletion, PersistenceId, PersistenceOperation,
    PersistenceRequest, PersistenceVersion,
  },
  persistence_store::{PersistenceSnapshot, PersistenceStatus, PersistenceStore},
  presentation_inspector::{InspectorObject, PresentationInspector},
  timers::{use_interval, use_pausable_timeout, use_timeout},
};
pub use reactant_core::prelude::*;
pub use reactant_rules::{GameReducer, ReducerOutput};

pub use crate::world::{Group as WorldGroup, Prefab, SceneRoot};
pub use reactant_core::native_host::{ObjectRef, use_object_ref};

pub use crate::{Task, TaskState, use_task};
