//! Motion targets shared by the selector menu, options, flash, and caret.

use battlement_reactant::motion::{Easing, MotionTarget, StyleTarget, Transition};

pub(crate) fn menu_initial(reduced: bool) -> StyleTarget {
  if reduced {
    StyleTarget::new().opacity(1.0)
  } else {
    StyleTarget::new().opacity(0.0).y(-12.0).scale_y(0.76)
  }
}

pub(crate) fn menu_visible() -> StyleTarget {
  StyleTarget::new().opacity(1.0).y(0.0).scale_y(1.0)
}

pub(crate) fn menu_exit(reduced: bool) -> MotionTarget {
  let style = if reduced {
    StyleTarget::new().opacity(0.0)
  } else {
    StyleTarget::new().opacity(0.0).y(-7.0).scale_y(0.42)
  };
  MotionTarget::new(style).transition(if reduced {
    reduced_transition()
  } else {
    Transition::tween()
      .duration_secs(0.26)
      .ease(Easing::CubicBezier([0.4, 0.0, 0.75, 0.3]))
  })
}

pub(crate) fn menu_transition(reduced: bool) -> Transition {
  if reduced {
    reduced_transition()
  } else {
    Transition::tween()
      .duration_secs(0.2)
      .ease(Easing::CubicBezier([0.2, 0.8, 0.25, 1.0]))
  }
}

pub(crate) fn option_initial(reduced: bool) -> StyleTarget {
  if reduced {
    StyleTarget::new().opacity(1.0)
  } else {
    StyleTarget::new().opacity(0.0).x(-17.0)
  }
}

pub(crate) fn option_visible() -> StyleTarget {
  StyleTarget::new().opacity(1.0).x(0.0)
}

pub(crate) fn option_exit(reduced: bool) -> StyleTarget {
  if reduced {
    StyleTarget::new().opacity(0.0)
  } else {
    StyleTarget::new().opacity(0.0).x(10.0)
  }
}

pub(crate) fn option_transition(reduced: bool, index: usize) -> Transition {
  if reduced {
    reduced_transition()
  } else {
    Transition::tween()
      .duration_secs(0.18)
      .delay_secs(index as f64 * 0.028)
      .ease(Easing::EaseOut)
  }
}

pub(crate) fn flash_initial(reduced: bool) -> StyleTarget {
  if reduced {
    StyleTarget::new().opacity(0.9)
  } else {
    StyleTarget::new().opacity(0.9).scale(0.96)
  }
}

pub(crate) fn flash_target(reduced: bool) -> MotionTarget {
  let style = if reduced {
    StyleTarget::new().opacity(0.0)
  } else {
    StyleTarget::new().opacity(0.0).scale(1.035)
  };
  MotionTarget::new(style).transition(if reduced {
    reduced_transition()
  } else {
    Transition::tween()
      .duration_secs(0.38)
      .ease(Easing::EaseOut)
  })
}

pub(crate) fn caret_transition(reduced: bool) -> Transition {
  if reduced {
    reduced_transition()
  } else {
    Transition::tween().duration_secs(0.14).ease(Easing::Ease)
  }
}

fn reduced_transition() -> Transition {
  Transition::tween().duration_secs(0.01)
}
