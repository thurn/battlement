use std::sync::atomic::{AtomicU8, Ordering};

use battlement_native::{CoreActionBodyView, CoreClientMessageView};

static PREFERENCE: AtomicU8 = AtomicU8::new(0);

pub(crate) fn record(value: u8) {
  PREFERENCE.store(value, Ordering::SeqCst);
}

pub(crate) fn submit(message: CoreClientMessageView<'_>) {
  if let CoreClientMessageView::Action(action) = message
    && let CoreActionBodyView::ReducedMotionPreferenceChanged(preference) = action.body()
  {
    self::record(preference.value());
  }
}

/// Returns the latest motion preference received through the native wire boundary.
#[unsafe(no_mangle)]
pub extern "C" fn fixture_motion_preference() -> u8 {
  PREFERENCE.load(Ordering::SeqCst)
}
