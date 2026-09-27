use battlement_ui::{PaintBlendMode, PaintStyle, UiBox, UiElement, UiValidationError};

#[test]
fn non_view_hosts_reject_blending_before_admission() {
  for mode in [PaintBlendMode::Screen, PaintBlendMode::Additive] {
    let element = UiElement::from(UiBox::new().paint(PaintStyle::new().blend_mode(mode)));
    assert_eq!(
      battlement_ui::validate_element_state(&element),
      Err(UiValidationError::InvalidProperty)
    );
  }
  let normal =
    UiElement::from(UiBox::new().paint(PaintStyle::new().blend_mode(PaintBlendMode::Normal)));
  assert!(battlement_ui::validate_element_state(&normal).is_ok());
}
