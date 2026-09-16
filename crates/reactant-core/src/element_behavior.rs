//! Common committed-host behavior without hand-written reference effects.

use crate::{
  element_ref::{self, ElementRef},
  hooks::{self, Dependencies},
};

/// Attaches to a focusable host and focuses it after this component mounts.
///
/// Use a key on the owning component to focus again when its content is replaced.
/// The host must allow programmatic focus; a heading normally uses tab index -1.
pub fn use_focus_on_mount() -> ElementRef {
  self::use_focus_when(Some(()))
}

/// Focuses an attached, focusable host when a nonempty request changes.
/// `None` keeps the hook mounted without requesting focus.
pub fn use_focus_when<D: Dependencies>(request: Option<D>) -> ElementRef {
  let reference = element_ref::use_element_ref();
  let target = reference.clone();
  let enabled = request.is_some();
  hooks::use_effect(
    move || {
      if enabled {
        target.focus();
      }
    },
    request,
  );
  reference
}

/// Attaches to a descendant and reveals it in `scroll` when the request changes.
///
/// `Some(key)` requests reveal on mount or whenever the key changes; `None`
/// does nothing. Repeated renders of the same request do not move the viewport.
pub fn use_scroll_reveal<D: Dependencies>(
  scroll: Option<ElementRef>,
  request: Option<D>,
) -> ElementRef {
  let reference = element_ref::use_element_ref();
  let target = reference.clone();
  let enabled = request.is_some();
  let container = scroll.clone();
  hooks::use_effect(
    move || {
      if enabled && let Some(container) = container {
        container.scroll_to(&target);
      }
    },
    (scroll, request),
  );
  reference
}
