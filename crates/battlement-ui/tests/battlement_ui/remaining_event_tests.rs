use battlement_ui::UiEventKind;

#[test]
fn only_native_propagating_remaining_events_accept_routed_phases() {
  for kind in [
    UiEventKind::LinkEnter,
    UiEventKind::LinkLeave,
    UiEventKind::LinkDown,
    UiEventKind::LinkUp,
  ] {
    assert!(kind.propagates());
  }
  for kind in [
    UiEventKind::GeometryChanged,
    UiEventKind::AttachToPanel,
    UiEventKind::DetachFromPanel,
    UiEventKind::SelectionChanged,
  ] {
    assert!(!kind.propagates());
  }
}

#[test]
#[should_panic(expected = "at least one supported property")]
fn transition_constructor_rejects_empty_property_lists() {
  let _ = battlement_ui::TransitionEvent::new(Vec::new(), 10.0);
}
