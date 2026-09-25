//! Verified FlatBuffers readers and direct message builders for Battlement transport.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod audio;
mod connect;
mod core_action_geometry;
mod core_action_motion;
mod core_client;
mod geometry;
mod hit_region;
mod limits;
mod material;
mod message_writer;
mod motion;
mod response;
mod response_motion;
mod response_motion_descriptor;
mod response_ui;
mod response_validate;
mod retained_ui;
#[cfg(feature = "test-support")]
pub mod test_support;
mod ui_event;
mod ui_event_body;
mod ui_event_write;
mod world_pointer;
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod common_generated {
  include!("generated/common_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod connect_generated {
  include!("generated/connect_generated.rs");
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod geometry_generated {
  include!("generated/geometry_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod motion_generated {
  include!("generated/motion_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod client_message_generated {
  include!("generated/client_message_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod ui_event_generated {
  include!("generated/ui_event_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod world_generated {
  include!("generated/world_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod command_core_generated {
  include!("generated/command_core_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod response_generated {
  include!("generated/response_generated.rs");
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod accessibility_generated {
  include!("generated/accessibility_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}
#[allow(clippy::all, missing_docs, unsafe_op_in_unsafe_fn, unused_imports)]
mod ui_generated {
  include!("generated/ui_generated.rs");
  pub use self::battlement::flat_buffers::generated::*;
}

/// Generated schema modules used only by build-composed application schemas.
///
/// Ordinary callers should use Battlement's verified views and writers instead.
#[doc(hidden)]
pub mod schema_generated {
  /// Accessibility schema closure.
  pub mod accessibility_generated {
    pub use crate::accessibility_generated::*;
  }
  /// Client-message schema closure.
  pub mod client_message_generated {
    pub use crate::client_message_generated::*;
  }
  /// Core command payload schema closure.
  pub mod command_core_generated {
    pub use crate::command_core_generated::*;
  }
  /// Common schema closure.
  pub mod common_generated {
    pub use crate::common_generated::*;
  }
  /// Geometry schema closure.
  pub mod geometry_generated {
    pub use crate::geometry_generated::*;
  }
  /// Motion schema closure.
  pub mod motion_generated {
    pub use crate::motion_generated::*;
  }
  /// Core response schema closure.
  pub mod response_generated {
    pub use crate::response_generated::*;
  }
  /// UI event schema closure.
  pub mod ui_event_generated {
    pub use crate::ui_event_generated::*;
  }
  /// UI schema closure.
  pub mod ui_generated {
    pub use crate::ui_generated::*;
  }
  /// World schema closure.
  pub mod world_generated {
    pub use crate::world_generated::*;
  }
}

pub use connect::*;
pub use core_client::*;
pub use geometry::*;
pub use limits::*;
pub use message_writer::*;
pub use motion::*;
pub use response::*;
pub use retained_ui::*;
pub use ui_event::*;
