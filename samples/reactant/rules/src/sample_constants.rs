use crate::{screens, values_time_controls};
use battlement::{ObjectId, object_id};

pub(crate) const MISSING_GEOMETRY_TARGET_ID: ObjectId =
  object_id!("25300000-0000-4000-8000-000000000006");

/// A screen available in the Reactant sample.
pub type Screen = screens::Screen;

/// Address of the sample's authored content scene.
pub const CONTENT_SCENE: &str = "reactant/content";
/// Address of the sample's prepared UI shader material.
pub const MOTION_MATERIAL: &str = "reactant/assets/motion-material";
/// Address of the sample's prepared audio-playhead pulse.
pub const MOTION_AUDIO_CLIP: battlement::AudioClipAddress = values_time_controls::AUDIO_CLIP;
/// Address of the sample's prepared motion texture.
pub const MOTION_TEXTURE: &str = "reactant/assets/texture";
/// Machine-readable registry derived from the Reactant screen inventory.
pub const DITTO_VISUAL_STATE_REGISTRY: &str = include_str!("../../ditto-visual-states.toml");
/// Stable identity of the projected world specimen.
pub const GEOMETRY_TARGET_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000005");
/// Stable identity of the Reactant document root.
pub const ROOT_ID: ObjectId = object_id!("25300000-0000-4000-8000-000000000004");
